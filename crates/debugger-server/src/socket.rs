use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use anyhow::anyhow;
use futures::{Sink, SinkExt, StreamExt};
use wasminspect_debugger::{CommandContext, MainDebugger, Process};

use crate::rpc;
use crate::{debugger_proxy, serialization};
use headers::{
    Connection, Header, HeaderMapExt, SecWebsocketAccept, SecWebsocketKey, SecWebsocketVersion,
    Upgrade,
};
use hyper::{upgrade::Upgraded, Body, Response};
use hyper::{Request, StatusCode};

use std::sync::mpsc;
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

async fn handle_incoming_message<S: Sink<Message> + Unpin + Send + 'static>(
    message: Message,
    process: &mut Process<MainDebugger>,
    context: &CommandContext,
    tx: Arc<Mutex<S>>,
    rx: Arc<mpsc::Receiver<Option<Message>>>,
) -> Result<(), S::Error>
where
    S::Error: std::error::Error,
{
    match serialization::deserialize_request(&message) {
        Ok(req) => {
            let res = debugger_proxy::handle_request(req, process, context, tx.clone(), rx);
            let msg = serialization::serialize_response(res);
            tx.lock().unwrap().send(msg).await?;
            return Ok(());
        }
        Err(e) => {
            let response = rpc::TextResponse::Error {
                message: e.to_string(),
            };
            let msg = serialization::serialize_response(response.into());
            tx.lock().unwrap().send(msg).await?;
            return Ok(());
        }
    }
}

pub async fn establish_connection(upgraded: Upgraded) -> Result<(), anyhow::Error> {
    let ws = WebSocketStream::from_raw_socket(upgraded, protocol::Role::Server, None).await;
    let (tx, mut rx) = ws.split();
    let (request_tx, request_rx) = mpsc::channel::<Option<Message>>();
    let connection_finished = Arc::new(AtomicBool::new(false));
    let connection_finished_reader = connection_finished.clone();

    let handle = thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            log::debug!("Start debugger thread");
            let (mut process, dbg_context) = wasminspect_debugger::start_debugger(None).unwrap();

            let mut last_line: Option<String> = None;
            let step_timeout = Duration::from_millis(500);
            loop {
                if connection_finished_reader.load(Ordering::Relaxed) {
                    process.interface.cancel_read_line().unwrap();
                    log::debug!("Debugger thread interrupted");
                    break;
                }
                let result = process.run_step(&dbg_context, &mut last_line, Some(step_timeout));
                match result {
                    Ok(Some(wasminspect_debugger::CommandResult::Exit)) => {
                        break;
                    },
                    Ok(Some(_)) => unreachable!("unexpected run_step result"),
                    Ok(None) => continue,
                    Err(e) => {
                        log::error!("catch error in run_step: {:?}", e);
                        break;
                    }
                }
            }
            log::debug!("Start receiving messages");

            let tx = Arc::new(Mutex::new(tx));
            let request_rx = Arc::new(request_rx);
            loop {
                let msg = match request_rx.recv() {
                    Ok(Some(msg)) => msg,
                    Ok(None) => break,
                    Err(_) => break,
                };
                log::debug!("Received message: {}", msg);
                match handle_incoming_message(
                    msg,
                    &mut process,
                    &dbg_context,
                    tx.clone(),
                    request_rx.clone(),
                )
                .await
                {
                    Ok(()) => continue,
                    Err(err) => {
                        log::error!("Sink error: {}", err);
                        break;
                    }
                }
            }
        });
    });

    while let Some(msg) = rx.next().await {
        match msg {
            Ok(msg) => {
                request_tx.send(Some(msg))?;
            }
            Err(e) => {
                request_tx.send(None).unwrap();
                return Err(e.into());
            }
        }
    }

    log::debug!("Start epilogue of socket");
    connection_finished.store(true, Ordering::Relaxed);
    request_tx.send(None).unwrap();
    handle.join().unwrap();
    log::debug!("End epilogue of socket");
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
