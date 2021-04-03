#![allow(dead_code)]

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WasmValue {
    I32 { value: i32 },
    I64 { value: i64 },
    F32 { value: u32 },
    F64 { value: u64 },
}

pub type JSNumber = f64;

#[derive(Debug, Serialize, Deserialize)]
pub enum WasmImport {
    Func { name: String },
    Global { name: String },
    Mem { name: String },
    Table { name: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WasmExport {
    Memory {
        name: String,
        #[serde(rename = "memorySize")]
        memory_size: usize,
    },
    Function { name: String },
    Global { name: String },
    Table { name: String },
}

#[derive(Debug)]
pub enum RequestError {
    InvalidBinaryRequestKind(u8),
    InvalidTextRequestJSON(Box<dyn std::error::Error + Send + Sync>),
    InvalidMessageType(String),
    CallArgumentLengthMismatch,
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
    Version,
    CallExported { name: String, args: Vec<JSNumber> },
    CallResult { values: Vec<JSNumber> },
    LoadMemory {
        name: String,
        offset: usize,
        length: usize,
    },
    StoreMemory {
        name: String,
        offset: usize,
        bytes: Vec<u8>,
    }
}

#[derive(FromPrimitive, Debug)]
pub enum BinaryRequestKind {
    Init = 0,
}

const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6d];

#[derive(Debug)]
pub struct BinaryRequest<'a> {
    pub kind: BinaryRequestKind,
    pub bytes: &'a [u8],
}

impl<'a> BinaryRequest<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, RequestError> {
        if bytes.len() >= 4 && bytes[0..4].eq(&WASM_MAGIC) {
            Ok(Self {
                kind: BinaryRequestKind::Init,
                bytes,
            })
        } else if let Some(kind) = FromPrimitive::from_u8(bytes[0]) {
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
    Version {
        value: String,
    },
    Init {
        exports: Vec<WasmExport>
    },
    CallResult {
        values: Vec<WasmValue>,
    },
    CallHost {
        module: String,
        field: String,
        args: Vec<WasmValue>,
    },
    LoadMemoryResult {
        bytes: Vec<u8>,
    },
    StoreMemoryResult,
    Error {
        message: String,
    },
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
