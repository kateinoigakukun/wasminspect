use super::command::{Command, CommandContext};
use super::debugger::Debugger;
use anyhow::Result;

pub struct StackCommand {}

impl StackCommand {
    pub fn new() -> Self {
        Self {}
    }
}

impl<D: Debugger> Command<D> for StackCommand {
    fn name(&self) -> &'static str {
        "stack"
    }

    fn run(
        &self,
        debugger: &mut D,
        context: &CommandContext,
        args: Vec<&str>,
    ) -> Result<()> {
        for (index, value) in debugger.stack_values().iter().enumerate() {
            println!("{}: {}", index, value)
        }
        Ok(())
    }
}
