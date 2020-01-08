use wasminspect_core::vm::WasmValue;
use wasminspect_core::vm::Store;

pub trait Debugger {
    fn run(&mut self, name: Option<String>) -> Result<Vec<WasmValue>, String>;
    fn frame(&self) -> Vec<String>;
}
