mod rpc;
use bytes::Buf;
use warp::{reply, Filter};

use rpc::{DebuggerRequest, DebuggerResponse};
use std::{
    borrow::BorrowMut,
    cell::RefCell,
    net::SocketAddr,
    rc::Rc,
    sync::{Arc, Mutex},
};

static VERSION: &str = "0.1.0";

async fn handle_rpc(req: DebuggerRequest) -> anyhow::Result<DebuggerResponse> {
    match req {
        DebuggerRequest::Version => Ok(DebuggerResponse::Version {
            value: VERSION.to_string(),
        }),
        DebuggerRequest::Init { bytes } => Ok(DebuggerResponse::Init),
    }
}

#[derive(Debug)]
struct CustomReject(anyhow::Error);

impl warp::reject::Reject for CustomReject {}

pub(crate) fn custom_reject(error: impl Into<anyhow::Error>) -> warp::Rejection {
    warp::reject::custom(CustomReject(error.into()))
}

#[derive(Debug)]
pub(crate) struct MessagePackError(rmp_serde::decode::Error);

impl ::std::fmt::Display for MessagePackError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "Request body read error: {}", self.0)
    }
}

impl warp::reject::Reject for MessagePackError {}

async fn handle_request(
    req: DebuggerRequest,
) -> std::result::Result<impl warp::Reply, warp::Rejection> {
    let response = match handle_rpc(req).await {
        Ok(res) => res,
        Err(err) => {
            return Err(custom_reject(err));
        }
    };
    Ok(warp::reply::json(&response))
}

async fn handle_version() -> std::result::Result<impl warp::Reply, warp::Rejection> {
    let res = DebuggerResponse::Version {
        value: VERSION.to_string(),
    };
    Ok(warp::reply::json(&res))
}

async fn handle_init(
    bytes: bytes::Bytes,
    context: Arc<Mutex<Context>>,
) -> std::result::Result<impl warp::Reply, warp::Rejection> {
    let res = DebuggerResponse::Init;
    context.lock().unwrap().bytes = bytes.to_vec();
    let bytes = Some(bytes.to_vec());
    let (process, context) = wasminspect_debugger::start_debugger(&bytes).unwrap();
    Ok(warp::reply::json(&res))
}

struct Context {
    bytes: Vec<u8>,
}

pub async fn start(addr: SocketAddr) {
    let context = Arc::new(Mutex::new(Context { bytes: vec![] }));
    let endpoint = warp::path::path("version")
        .and(warp::get())
        .and_then(handle_version);

    let ctx0 = Arc::clone(&context);
    let init = warp::path::path("init")
        .and(warp::post())
        .and(warp::body::bytes())
        .and_then(move |req| handle_init(req, ctx0.clone()));
    // let init = warp::path::path("init")
    //     .and(warp::post())
    //     .and(warp::body::bytes())
    //     .and_then(|req| handle_init(req, context.clone()));
    warp::serve(endpoint.or(init)).run(addr).await;
}
