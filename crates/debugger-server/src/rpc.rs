use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TextRequest {
    Version,
}

pub struct BinaryRequest<'a> {
    kind: BinaryRequestKind,
    bytes: &'a [u8],
}
#[repr(u8)]
pub enum BinaryRequestKind {
    Init,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DebuggerResponse {
    Version { value: String },
    Init,
}
