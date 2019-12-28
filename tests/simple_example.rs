extern crate wasminspect_core;
use wasminspect_core::interpreter::*;

use std::path::Path;

fn run_wasm(filename: &str, func: &str, args: Vec<WasmValue>, results: Vec<WasmValue>) {
    let example_dir = Path::new(file!()).parent().unwrap().join("simple-example");
    let instance = WasmInstance::new(example_dir.join(filename).to_str().unwrap().to_string());
    match instance.run(Some(func.to_string()), args) {
        Ok(result) => assert_eq!(result, results),
        Err(err) => panic!(err.message()),
    }
}

#[test]
fn test_calc() {
    run_wasm(
        "calc.wasm",
        "add",
        vec![WasmValue::I32(1), WasmValue::I32(2)],
        vec![WasmValue::I32(3)],
    )
}
