use anyhow::anyhow;
use futures::StreamExt;

use headers::{
    Connection, Header, HeaderMapExt, SecWebsocketAccept, SecWebsocketKey, SecWebsocketVersion,
    Upgrade,
};
use hyper::{upgrade::Upgraded, Body, Response};
use hyper::{Request, StatusCode};

use tokio_tungstenite::tungstenite::{protocol, Message};
use tokio_tungstenite::WebSocketStream;

pub async fn socket_handshake<F, Fut>(
    req: Request<Body>,
    connect: F,
) -> Result<Response<Body>, anyhow::Error>
where
    F: Send + 'static + FnOnce(Upgraded) -> Fut,
    Fut: std::future::Future<Output = Result<(), anyhow::Error>> + Send,
{
    fn try_get_header<H>(req: &Request<Body>) -> Result<H, anyhow::Error>
    where
        H: Header,
    {
        match req.headers().typed_get::<H>() {
            Some(header_value) => Ok(header_value),
            None => {
                return Err(anyhow!(format!(
                    "Missing request header {}",
                    H::name().as_str()
                )));
            }
        }
    }
    let upgrade_to = try_get_header::<Upgrade>(&req)?;
    if upgrade_to != Upgrade::websocket() {
        return Err(anyhow!("Invalid request header value in UPGRADE"));
    }

    let ws_version = try_get_header::<SecWebsocketVersion>(&req)?;
    if ws_version != SecWebsocketVersion::V13 {
        return Err(anyhow!(format!(
            "Unsupported WebSocket version: {:?}",
            ws_version
        )));
    }

    let ws_key = try_get_header::<SecWebsocketKey>(&req)?;
    let upgrade = hyper::upgrade::on(req);
    tokio::spawn(async move {
        match upgrade.await {
            Ok(upgraded) => match connect(upgraded).await {
                Ok(_) => {}
                Err(err) => {
                    log::error!("error while connection: {}", err);
                }
            },
            Err(err) => {
                log::error!("upgrade error: {}", err);
            }
        }
    });

    let mut res = Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .body(Body::empty())
        .unwrap();

    res.headers_mut().typed_insert(Connection::upgrade());
    res.headers_mut().typed_insert(Upgrade::websocket());
    res.headers_mut()
        .typed_insert(SecWebsocketAccept::from(ws_key));
    Ok(res)
}

async fn establish_connection(upgraded: Upgraded) -> Result<(), anyhow::Error> {
    let mut ws = WebSocketStream::from_raw_socket(upgraded, protocol::Role::Server, None).await;
    use std::sync::mpsc::{self, Receiver, Sender};
    let (request_tx, _request_rx): (Sender<Option<Message>>, Receiver<Option<Message>>) =
        mpsc::channel();

    // tokio::task::spawn(async move {
    //     let (process, dbg_context) = wasminspect_debugger::start_debugger(None).unwrap();
    //     let context = Rc::new(Context {
    //         process: RefCell::new(process),
    //         dbg_context: RefCell::new(dbg_context),
    //     });

    //     match init_process(context.as_ref()) {
    //         Ok(_) => {}
    //         Err(err) => eprintln!("{}", err),
    //     }
    //     loop {
    //         let msg = match request_rx.recv() {
    //             Ok(Some(msg)) => msg,
    //             Ok(None) => break,
    //             Err(err) => {
    //                 log::error!("Receiving error: {}", err);
    //                 break;
    //             }
    //         };
    //     }
    // });
    while let Some(msg) = ws.next().await {
        request_tx.send(Some(msg?))?;
    }
    request_tx.send(None)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_socket_handshake() {
        use hyper::server::conn::Http;
        use std::{pin::Pin, task::Poll};

        use futures::SinkExt;
        use futures::{task, Future};
        use std::net::SocketAddr;
        use tokio::net::TcpListener;
        use tokio_tungstenite::tungstenite::protocol;
        use tokio_tungstenite::WebSocketStream;

        #[derive(Clone)]
        struct AddrConnect(SocketAddr);

        impl tower_service::Service<hyper::http::Uri> for AddrConnect {
            type Response = ::tokio::net::TcpStream;
            type Error = ::std::io::Error;
            type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

            fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>> {
                Poll::Ready(Ok(()))
            }

            fn call(&mut self, _: hyper::http::Uri) -> Self::Future {
                Box::pin(tokio::net::TcpStream::connect(self.0))
            }
        }

        fn tcp_bind(addr: &SocketAddr) -> ::tokio::io::Result<TcpListener> {
            use std::net::TcpListener as StdTcpListener;
            let std_listener = StdTcpListener::bind(addr).unwrap();
            std_listener.set_nonblocking(true).unwrap();
            TcpListener::from_std(std_listener)
        }

        async fn echo(upgraded: Upgraded) -> anyhow::Result<()> {
            let ws = WebSocketStream::from_raw_socket(upgraded, protocol::Role::Server, None).await;
            let (tx, rx) = ws.split();
            rx.inspect(|i| log::debug!("ws recv: {:?}", i))
                .forward(tx)
                .await?;
            Ok(())
        }

        let _ = env_logger::try_init();

        let listener = tcp_bind(&"127.0.0.1:0".parse().unwrap()).unwrap();
        let addr = listener.local_addr().unwrap();
        let (upgraded_tx, upgraded_rx) = futures::channel::oneshot::channel::<Upgraded>();

        tokio::spawn(async move {
            let uri: hyper::Uri = format!("http://{}", addr).parse().expect("valid URI");
            let req = Request::builder()
                .uri(uri)
                .header("connection", "upgrade")
                .header("upgrade", "websocket")
                .header("sec-websocket-version", "13")
                .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
                .body(Body::empty())
                .expect("connection req");
            let res = ::hyper::Client::builder()
                .build(AddrConnect(addr))
                .request(req)
                .await
                .expect("hello res");
            let upgrade = hyper::upgrade::on(res);
            match upgrade.await {
                Ok(up) => upgraded_tx.send(up).expect("send upgraded"),
                Err(err) => {
                    panic!("{}", err);
                }
            };
        });
        let svc = hyper::service::service_fn(|req| socket_handshake(req, echo));
        let (socket, _) = listener.accept().await.unwrap();
        Http::new()
            .serve_connection(socket, svc)
            .with_upgrades()
            .await
            .unwrap();
        let upgraded = upgraded_rx.await.expect("recv upgraded");
        let mut ws = WebSocketStream::from_raw_socket(upgraded, protocol::Role::Client, None).await;
        let msg = Message::Text("hello".to_string());
        ws.send(msg.clone()).await.expect("send msg");
        let recv = ws.next().await.expect("recv msg").unwrap();
        assert_eq!(recv, msg);
    }
}
