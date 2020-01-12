use wasminspect_api::value::Value;

use parity_wasm::elements::Instruction;

pub trait Debugger {
    fn run(&mut self, name: Option<String>) -> Result<Vec<Value>, String>;
    fn frame(&self) -> Vec<String>;
    fn instructions(&self) -> Result<(&[Instruction], usize), String>;
}
