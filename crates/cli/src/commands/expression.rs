use super::command::{Command, CommandContext};
use super::debugger::Debugger;
use anyhow::Result;

pub struct ExpressionCommand {}

impl ExpressionCommand {
    pub fn new() -> Self {
        Self {}
    }
}

impl<D: Debugger> Command<D> for ExpressionCommand {
    fn name(&self) -> &'static str {
        "expression"
    }

    fn run(&self, debugger: &mut D, _context: &CommandContext, _args: Vec<&str>) -> Result<()> {
        Ok(())
    }
}
