use super::command::{Command, CommandContext};
use super::debugger::Debugger;
use anyhow::Result;

pub struct ListCommand {}

impl ListCommand {
    pub fn new() -> Self {
        Self {}
    }
}

impl<D: Debugger> Command<D> for ListCommand {
    fn name(&self) -> &'static str {
        "list"
    }

    fn run(&self, debugger: &mut D, context: &CommandContext, args: Vec<&str>) -> Result<()> {
        let (insts, next_index) = debugger.instructions()?;
        let first_inst = insts[0].clone();
        let current_inst = if next_index != 0 {
            insts[next_index - 1].clone()
        } else {
            insts[0].clone()
        };
        Ok(())
    }
}
