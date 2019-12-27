mod interpreter;
use interpreter::{WasmInstance, WasmValue};

fn main() {
    let mut instance = WasmInstance::new("example/calc.wasm".to_string());
    instance.run(
        Some("add".to_string()),
        vec![WasmValue::I32(1), WasmValue::I32(2)],
    );
}
