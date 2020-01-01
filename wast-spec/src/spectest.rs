use wasminspect_core::interpreter::{HostValue};
use std::collections::HashMap;

pub fn instantiate_spectest() -> HashMap<String, HostValue> {
    let mut module = HashMap::new();
    module.insert("memory".to_string(), HostValue::Mem());
    module
}