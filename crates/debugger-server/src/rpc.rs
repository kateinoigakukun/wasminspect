use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TextRequest {
}

#[derive(FromPrimitive, Debug)]
pub enum BinaryRequestKind {
    Init = 0,
}

#[derive(Debug)]
pub struct BinaryRequest<'a> {
    pub kind: BinaryRequestKind,
    pub bytes: &'a [u8],
}

impl<'a> BinaryRequest<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Option<Self> {
        Some(Self {
            kind: FromPrimitive::from_u8(bytes[0])?,
            bytes: &bytes[1..],
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DebuggerResponse {
    Version { value: String },
    Init,
}
