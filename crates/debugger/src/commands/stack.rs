use super::command::{Command, CommandContext, CommandResult};
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

    fn description(&self) -> &'static str {
        "Commands for operating stack."
    }

    fn run(&self, debugger: &mut D, context: &CommandContext, _args: Vec<&str>) -> Result<Option<CommandResult>> {
        for (index, value) in debugger.stack_values().iter().enumerate() {
            let output = format!("{}: {:?}", index, value);
            context.printer.println(&output);
        }
        Ok(None)
    }
}
