mod rpc;

use hyper::{
    service::{make_service_fn, service_fn},
    Method, Request, StatusCode,
};
use hyper::{Body, Response, Server};
use tokio;
use wasminspect_debugger::*;


use std::{
    cell::{RefCell, RefMut},
    net::SocketAddr,
    rc::Rc,
    thread,
};

static VERSION: &str = "0.1.0";

struct Context {
    process: RefCell<Process<MainDebugger>>,
    dbg_context: RefCell<CommandContext>,
}

pub fn start(addr: SocketAddr) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    let local = tokio::task::LocalSet::new();
    local.block_on(&rt, run(addr));
}

// let mut data: DebuggerRequest = serde_json::from_reader(body.reader())?;
async fn remote_api(
    req: Request<Body>,
    context: Rc<Context>,
) -> Result<Response<Body>, anyhow::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/version") => Ok(Response::new(VERSION.into())),
        (&Method::POST, "/init") => {
            let body = hyper::body::to_bytes(req.into_body()).await?;
            let wasm_bytes = body.to_vec();
            context
                .process
                .borrow_mut()
                .debugger
                .load_module(&wasm_bytes)?;
            Ok(Response::new(Body::from("Hello")))
        }
        _ => {
            // Return 404 not found response.
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(hyper::body::Body::empty())
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

    let make_service = make_service_fn(move |_| {
        let ctx = context.clone();
        async move { Ok::<_, anyhow::Error>(service_fn(move |req| remote_api(req, ctx.clone()))) }
    });

    let server = Server::bind(&addr).executor(LocalExec).serve(make_service);

    println!("Listening on http://{}", addr);

    if let Err(e) = server.await {
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
