extern crate wasminspect_core;
use wasminspect_core::interpreter::*;

use std::path::Path;

#[test]
fn add_example() {
    let example_dir = Path::new(file!()).parent().unwrap().join("example");
    let instance = WasmInstance::new(example_dir.join("calc.wasm").to_str().unwrap().to_string());
    match instance.run(
        Some("add".to_string()),
        vec![WasmValue::I32(1), WasmValue::I32(2)],
    ) {
        Ok(result) => assert_eq!(result[0], WasmValue::I32(3)),
        Err(err) => panic!(err.message()),
    }
}
