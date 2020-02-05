use wasminspect_vm::{Instruction, WasmValue};

pub enum Breakpoint {
    Function { name: String },
}

pub enum RunResult {
    Finish(Vec<WasmValue>),
    Breakpoint,
}

pub trait Debugger {
    fn run(&mut self, name: Option<String>) -> Result<RunResult, String>;
    fn is_running(&self) -> bool;
    fn frame(&self) -> Vec<String>;
    fn memory(&self) -> Result<Vec<u8>, String>;
    fn set_breakpoint(&mut self, breakpoint: Breakpoint);
    fn stack_values(&self) -> Vec<String>;
    fn instructions(&self) -> Result<(&[Instruction], usize), String>;
}
