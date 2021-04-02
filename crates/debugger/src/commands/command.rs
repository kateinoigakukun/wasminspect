use super::debugger::{Debugger, OutputPrinter};
use super::sourcemap::SourceMap;
use super::subroutine::SubroutineMap;
use anyhow::Result;
use wasminspect_vm::WasmValue;

pub struct CommandContext {
    pub sourcemap: Box<dyn SourceMap>,
    pub subroutine: Box<dyn SubroutineMap>,
    pub printer: Box<dyn OutputPrinter>,
}

#[derive(Debug)]
pub enum CommandResult {
    ProcessFinish(Vec<WasmValue>),
    Exit,
}

pub trait Command<D: Debugger> {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str {
        "No description yet"
    }
    fn run(
        &self,
        debugger: &mut D,
        context: &CommandContext,
        args: Vec<&str>,
    ) -> Result<Option<CommandResult>>;
}

pub trait AliasCommand {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str {
        "No description yet"
    }
    fn run(&self, args: Vec<&str>) -> Result<String>;
}
