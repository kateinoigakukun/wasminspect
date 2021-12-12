use crate::rpc;
use tokio_tungstenite::tungstenite::Message;

pub fn deserialize_request(message: &Message) -> Result<rpc::Request, rpc::RequestError> {
    match message {
        Message::Binary(bytes) => rpc::BinaryRequest::from_bytes(bytes).map(rpc::Request::Binary),
        Message::Text(text) => match serde_json::from_str::<rpc::TextRequest>(text) {
            Ok(req) => Ok(rpc::Request::Text(req)),
            Err(e) => Err(rpc::RequestError::InvalidTextRequestJSON(Box::new(e))),
        },
        msg => Err(rpc::RequestError::InvalidMessageType(format!("{:?}", msg))),
    }
}
pub fn serialize_response(response: rpc::Response) -> Message {
    match response {
        rpc::Response::Text(response) => {
            let json = match serde_json::to_string(&response) {
                Ok(json) => json,
                Err(e) => {
                    log::error!("Failed to serialize error response: {}", e);
                    return Message::Close(None);
                }
            };
            Message::Text(json)
        }
        rpc::Response::Binary { kind, bytes } => {
            let mut bin = vec![kind as u8];
            bin.extend(bytes);
            Message::binary(bin)
        }
    }
}
