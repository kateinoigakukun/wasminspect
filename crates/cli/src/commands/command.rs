use super::debugger::Debugger;
use super::sourcemap::SourceMap;
use anyhow::Result;

pub struct CommandContext {
    pub sourcemap: Box<dyn SourceMap>,
}

pub trait Command<D: Debugger> {
    fn name(&self) -> &'static str;
    fn run(&self, debugger: &mut D, context: &CommandContext, args: Vec<&str>) -> Result<()>;
}
