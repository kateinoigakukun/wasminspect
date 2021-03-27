use num_derive::{FromPrimitive};
use num_traits::{FromPrimitive};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WasmValue {
    I32(i32),
    I64(i64),
    F32(u32),
    F64(u64),
}

#[derive(Debug)]
pub enum RequestError {
    InvalidBinaryRequestKind(u8),
    InvalidTextRequestJSON(Box<dyn std::error::Error>),
    InvalidMessageType(String),
}

impl std::fmt::Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for RequestError {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TextRequest {
    CallExported { name: String, args: Vec<WasmValue> },
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
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, RequestError> {
        if let Some(kind) = FromPrimitive::from_u8(bytes[0]) {
            Ok(Self {
                kind,
                bytes: &bytes[1..],
            })
        } else {
            Err(RequestError::InvalidBinaryRequestKind(bytes[0]))
        }
    }
}

pub enum Request<'a> {
    Text(TextRequest),
    Binary(BinaryRequest<'a>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TextResponse {
    Init,
    CallResult { value: WasmValue },
    Error { message: String },
}
#[derive(Debug)]
#[repr(u8)]
pub enum BinaryResponseKind {
    Memory = 0,
}

#[derive(Debug)]
pub struct BinaryResponse {}

pub enum Response {
    Text(TextResponse),
    Binary {
        kind: BinaryResponseKind,
        bytes: Vec<u8>,
    },
}

impl Into<Response> for TextResponse {
    fn into(self) -> Response {
        Response::Text(self)
    }
}
