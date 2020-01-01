use wasminspect_core::interpreter::{HostValue, WasmValue};
use std::collections::HashMap;

pub fn instantiate_spectest() -> HashMap<String, HostValue> {
    let mut module = HashMap::new();
    module.insert("memory".to_string(), HostValue::Mem());
    module.insert("global_i32".to_string(), HostValue::Global(WasmValue::I32(666)));
    module
}