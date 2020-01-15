use wasminspect_vm::WasmValue;
use parity_wasm::elements::Instruction;

pub enum Breakpoint {
    Function { name: String },
}

pub trait Debugger {
    fn run(&mut self, name: Option<String>) -> Result<Vec<WasmValue>, String>;
    fn is_running(&self) -> bool;
    fn frame(&self) -> Vec<String>;
    fn memory(&self) -> Result<Vec<u8>, String>;
    fn set_breakpoint(&mut self, breakpoint: Breakpoint);
    fn stack_values(&self) -> Vec<String>;
    fn instructions(&self) -> Result<(&[Instruction], usize), String>;
}
