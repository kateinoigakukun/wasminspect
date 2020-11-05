use anyhow::Result;
use wasminspect_vm::{Instruction, ModuleIndex, Signal, Store, WasmValue};

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
    StepOut,
}

pub struct FunctionFrame {
    pub module_index: ModuleIndex,
    pub argument_count: usize,
}

pub trait Debugger {
    fn run(&mut self, name: Option<String>) -> Result<RunResult>;
    fn is_running(&self) -> bool;
    fn frame(&self) -> Vec<String>;
    fn current_frame(&self) -> Option<FunctionFrame>;
    fn locals(&self) -> Vec<WasmValue>;
    fn memory(&self) -> Result<Vec<u8>>;
    fn store(&self) -> &Store;
    fn set_breakpoint(&mut self, breakpoint: Breakpoint);
    fn stack_values(&self) -> Vec<WasmValue>;
    fn instructions(&self) -> Result<(&[Instruction], usize)>;
    fn step(&self, style: StepStyle) -> Result<Signal>;
    fn process(&self) -> Result<Signal>;
}
