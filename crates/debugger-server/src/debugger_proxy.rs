use crate::rpc::{self, TextResponse};
use wasminspect_debugger::{MainDebugger, Process};

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
    match req {
        Binary(req) => match req.kind {
            Init => {
                process.debugger.load_module(req.bytes)?;
                return Ok(rpc::Response::Text(TextResponse::Init));
            }
        },
        Text(CallExported { name: _, args: _ }) => {
            unimplemented!()
        }
    }
}
