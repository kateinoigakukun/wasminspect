mod interpreter;
use interpreter::{WasmInstance, WasmValue};

fn main() {
    let instance = WasmInstance::new("example/calc.wasm".to_string());
    match instance.run(
        Some("add".to_string()),
        vec![WasmValue::I32(1), WasmValue::I32(2)],
    ) {
        Ok(_) => return,
        Err(err) => panic!(err.message()),
    }
}
