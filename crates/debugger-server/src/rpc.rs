use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DebuggerRequest {
    Version,
    Init { bytes: Vec<u8> },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DebuggerResponse {
    Version { value: String },
    Init,
}

