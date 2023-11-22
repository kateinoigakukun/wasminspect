use anyhow::Result;
use wasminspect_vm::{HostValue, Instruction, ModuleIndex, Signal, Store, WasmValue};

#[derive(Default, Clone)]
pub struct DebuggerOpts {
    pub watch_memory: bool,
}

pub enum Breakpoint {
    Function { name: String },
    Instruction { inst_offset: usize },
}

pub enum RunResult {
    Finish(Vec<WasmValue>),
    Breakpoint,
}

#[derive(Clone, Copy)]
pub enum StepStyle {
    InstIn,
    InstOver,
    Out,
}

pub struct FunctionFrame {
    pub module_index: ModuleIndex,
    pub argument_count: usize,
}

pub trait OutputPrinter {
    fn println(&self, _: &str);
    fn eprintln(&self, _: &str);
}
pub type RawHostModule = std::collections::HashMap<String, HostValue>;

pub trait Debugger {
    fn get_opts(&self) -> DebuggerOpts;
    fn set_opts(&mut self, opts: DebuggerOpts);
    fn instantiate(
        &mut self,
        host_modules: std::collections::HashMap<String, RawHostModule>,
        wasi_args: Option<&[String]>,
    ) -> Result<()>;
    fn run(&mut self, name: Option<&str>, args: Vec<WasmValue>) -> Result<RunResult>;
    fn is_running(&self) -> bool;
    fn frame(&self) -> Vec<String>;
    fn current_frame(&self) -> Option<FunctionFrame>;
    fn locals(&self) -> Vec<WasmValue>;
    fn memory(&self) -> Result<Vec<u8>>;
    fn store(&self) -> Result<&Store>;
    fn set_breakpoint(&mut self, breakpoint: Breakpoint);
    fn stack_values(&self) -> Vec<WasmValue>;
    fn selected_instructions(&self) -> Result<(&[Instruction], usize)>;
    fn step(&self, style: StepStyle) -> Result<Signal>;
    fn process(&mut self) -> Result<RunResult>;
    fn select_frame(&mut self, frame_index: Option<usize>) -> Result<()>;
}
