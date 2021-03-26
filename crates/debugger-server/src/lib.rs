mod rpc;

use anyhow::anyhow;
use futures::{FutureExt, SinkExt, StreamExt};
use headers::{
    Connection, Header, HeaderMapExt, SecWebsocketAccept, SecWebsocketKey, SecWebsocketVersion,
    Upgrade,
};
use hyper::{
    header::{self, AsHeaderName, HeaderName, ToStrError},
    upgrade::Upgraded,
    Body, Response, Server,
};
use hyper::{
    service::{make_service_fn, service_fn},
    Method, Request, StatusCode,
};
use tokio;
use tokio_tungstenite::tungstenite::{protocol, Message};
use tokio_tungstenite::WebSocketStream;
use wasminspect_debugger::*;

use std::{cell::RefCell, net::SocketAddr, rc::Rc};

static VERSION: &str = "0.1.0";

struct Context {
    process: RefCell<Process<MainDebugger>>,
    dbg_context: RefCell<CommandContext>,
}

pub async fn start(addr: SocketAddr) {
    run(addr).await;
}

// let mut data: DebuggerRequest = serde_json::from_reader(body.reader())?;
async fn establish_connection(upgraded: Upgraded) -> Result<(), anyhow::Error> {
    let mut ws = WebSocketStream::from_raw_socket(upgraded, protocol::Role::Server, None).await;
    use std::sync::mpsc::{self, Receiver, Sender};
    let (request_tx, request_rx): (Sender<Option<Message>>, Receiver<Option<Message>>) =
        mpsc::channel();

    tokio::task::spawn(async move {
        let (process, dbg_context) = wasminspect_debugger::start_debugger(None).unwrap();
        let context = Rc::new(Context {
            process: RefCell::new(process),
            dbg_context: RefCell::new(dbg_context),
        });

        match init_process(context.as_ref()) {
            Ok(_) => {}
            Err(err) => eprintln!("{}", err),
        }
        loop {
            let msg = match request_rx.recv() {
                Ok(Some(msg)) => msg,
                Ok(None) => break,
                Err(err) => {
                    log::error!("Receiving error: {}", err);
                    break;
                }
            };
        }
    });
    while let Some(msg) = ws.next().await {
        request_tx.send(Some(msg?))?;
    }
    request_tx.send(None)?;

    Ok(())
}
async fn socket_handshake(req: Request<Body>) -> Result<Response<Body>, anyhow::Error> {
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
    tokio::task::spawn(async move {
        match upgrade.await {
            Ok(upgraded) => match establish_connection(upgraded).await {
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

async fn remote_api(
    req: Request<Body>,
    context: Rc<Context>,
) -> Result<Response<Body>, anyhow::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/version") => Ok(Response::new(VERSION.into())),
        (&Method::POST, "/init") => {
            let body = hyper::body::to_bytes(req.into_body()).await?;
            let wasm_bytes = body.to_vec();
            context.process.borrow_mut().debugger.reset_store();
            context
                .process
                .borrow_mut()
                .debugger
                .load_module(&wasm_bytes)?;
            Ok(Response::new(Body::empty()))
        }
        (&Method::GET, "debug") => socket_handshake(req).await,
        _ => {
            // Return 404 not found response.
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(header::UPGRADE, "foobar")
                .body(Body::empty())
                .unwrap())
        }
    }
}

fn init_process(context: &Context) -> anyhow::Result<()> {
    context
        .process
        .borrow_mut()
        .run_loop(&context.dbg_context.borrow())
}

async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

async fn run(addr: SocketAddr) {
    let (process, dbg_context) = { wasminspect_debugger::start_debugger(None).unwrap() };
    let context = Rc::new(Context {
        process: RefCell::new(process),
        dbg_context: RefCell::new(dbg_context),
    });

    match init_process(context.as_ref()) {
        Ok(_) => {}
        Err(err) => eprintln!("{}", err),
    }
    let ctx = context.clone();
    let make_service = make_service_fn(move |_| {
        let ctx = ctx.clone();
        async move { Ok::<_, anyhow::Error>(service_fn(move |req| remote_api(req, ctx.clone()))) }
    });

    let server = Server::bind(&addr).executor(LocalExec).serve(make_service);

    println!("Listening on http://{}", addr);
    let graceful = server.with_graceful_shutdown(shutdown_signal());

    if let Err(e) = graceful.await {
        eprintln!("server error: {}", e);
    }
}

#[derive(Clone, Copy, Debug)]
struct LocalExec;

impl<F> hyper::rt::Executor<F> for LocalExec
where
    F: std::future::Future + 'static,
{
    fn execute(&self, fut: F) {
        tokio::task::spawn_local(fut);
    }
}
