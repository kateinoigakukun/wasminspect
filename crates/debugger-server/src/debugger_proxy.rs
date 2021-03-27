use crate::rpc;
use wasminspect_debugger::{MainDebugger, Process};

static VERSION: &str = "0.1.0";

pub fn handle_request(req: rpc::Request, process: &mut Process<MainDebugger>) -> rpc::Response {
    match _handle_request(req, process) {
        Ok(res) => res,
        Err(err) => rpc::TextResponse::Error {
            message: err.to_string(),
        }
        .into(),
    }
}

fn _handle_request(
    req: rpc::Request,
    process: &mut Process<MainDebugger>,
) -> Result<rpc::Response, anyhow::Error> {
    use rpc::BinaryRequestKind::*;
    use rpc::Request::*;
    use rpc::TextRequest::*;
    use rpc::*;

    match req {
        Binary(req) => match req.kind {
            Init => {
                process.debugger.load_module(req.bytes)?;
                return Ok(rpc::Response::Text(TextResponse::Init));
            }
        },
        Text(Version) => {
            return Ok(TextResponse::Version {
                value: VERSION.to_string(),
            }
            .into());
        }
        Text(CallExported { name: _, args: _ }) => {
            unimplemented!()
        }
    }
}
