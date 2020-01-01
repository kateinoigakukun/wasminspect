use std::collections::HashMap;
use wasminspect_core::interpreter::{HostValue, WasmValue};

pub fn instantiate_spectest() -> HashMap<String, HostValue> {
    let mut module = HashMap::new();
    module.insert("memory".to_string(), HostValue::Mem());
    module.insert(
        "global_i32".to_string(),
        HostValue::Global(WasmValue::I32(666)),
    );
    module.insert(
        "global_i64".to_string(),
        HostValue::Global(WasmValue::I32(666)),
    );
    module.insert(
        "global_f32".to_string(),
        HostValue::Global(WasmValue::F32(f32::from_bits(0x44268000))),
    );
    module.insert(
        "global_f64".to_string(),
        HostValue::Global(WasmValue::F64(f64::from_bits(0x4084d00000000000))),
    );
    module
}
