use wasminspect_vm::{Instruction, WasmValue, Signal};
use anyhow::Result;

pub enum Breakpoint {
    Function { name: String },
}

pub enum RunResult {
    Finish(Vec<WasmValue>),
    Breakpoint,
}

pub trait Debugger {
    fn run(&mut self, name: Option<String>) -> Result<RunResult>;
    fn is_running(&self) -> bool;
    fn frame(&self) -> Vec<String>;
    fn memory(&self) -> Result<Vec<u8>>;
    fn set_breakpoint(&mut self, breakpoint: Breakpoint);
    fn stack_values(&self) -> Vec<String>;
    fn instructions(&self) -> Result<(&[Instruction], usize)>;
    fn step(&self) -> Result<Signal>;
    // fn program_counter(&self) -> Result<usize>;
}
