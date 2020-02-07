use anyhow::Result;
use wasminspect_vm::{Instruction, Signal, WasmValue};

pub enum Breakpoint {
    Function { name: String },
}

pub enum RunResult {
    Finish(Vec<WasmValue>),
    Breakpoint,
}

#[derive(Clone, Copy)]
pub enum StepStyle {
    StepInstIn,
    StepInstOver,
}

pub trait Debugger {
    fn run(&mut self, name: Option<String>) -> Result<RunResult>;
    fn is_running(&self) -> bool;
    fn frame(&self) -> Vec<String>;
    fn memory(&self) -> Result<Vec<u8>>;
    fn set_breakpoint(&mut self, breakpoint: Breakpoint);
    fn stack_values(&self) -> Vec<String>;
    fn instructions(&self) -> Result<(&[Instruction], usize)>;
    fn step(&self, style: StepStyle) -> Result<Signal>;
}
