use wasminspect_core::vm::WasmValue;
use wasminspect_core::vm::Store;
use parity_wasm::elements::Instruction;

pub trait Debugger {
    fn run(&mut self, name: Option<String>) -> Result<Vec<WasmValue>, String>;
    fn frame(&self) -> Vec<String>;
    fn instructions(&self) -> Result<(&[Instruction], usize), String>;
}
