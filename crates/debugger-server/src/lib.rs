mod debugger_proxy;
mod rpc;
mod serialization;
mod socket;

use hyper::{
    service::{make_service_fn, service_fn},
    Method, Request, StatusCode,
};
use hyper::{Body, Response, Server};

use std::net::SocketAddr;

pub async fn start(addr: SocketAddr) {
    run(addr).await;
}

async fn remote_api(req: Request<Body>) -> Result<Response<Body>, anyhow::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/debugger") => {
            let res =
                socket::socket_handshake(req, socket::establish_connection)
                    .await;
            match res {
                Ok(res) => Ok(res),
                Err(e) => Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(e.to_string()))
                    .unwrap()),
            }
        }
        _ => {
            // Return 404 not found response.
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap())
        }
    }
}

async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

async fn run(addr: SocketAddr) {
    let make_service = make_service_fn(move |_| async move {
        Ok::<_, anyhow::Error>(service_fn(move |req| {
            log::trace!("request: {:?}", req);
            remote_api(req)
        }))
    });

    let server = Server::bind(&addr).serve(make_service);

    println!("Listening on http://{}", server.local_addr());
    let graceful = server.with_graceful_shutdown(shutdown_signal());

    if let Err(e) = graceful.await {
        eprintln!("server error: {}", e);
    }
}
