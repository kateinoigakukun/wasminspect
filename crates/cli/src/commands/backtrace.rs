use super::command::{Command, CommandContext};
use super::debugger::Debugger;
use anyhow::Result;

pub struct BacktraceCommand {}

impl BacktraceCommand {
    pub fn new() -> Self {
        Self {}
    }
}

impl<D: Debugger> Command<D> for BacktraceCommand {
    fn name(&self) -> &'static str {
        "bt"
    }
    fn run(
        &self,
        debugger: &mut D,
        context: &CommandContext,
        args: Vec<&str>,
    ) -> Result<()> {
        for (index, frame) in debugger.frame().iter().rev().enumerate() {
            println!("{}: {}", index, frame);
        }
        Ok(())
    }
}
