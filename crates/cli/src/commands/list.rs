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

    fn run(
        &self,
        debugger: &mut D,
        context: &CommandContext,
        args: Vec<&str>,
    ) -> Result<()> {
        let (insts, next_index) = debugger.instructions()?;
        for (index, inst) in insts.iter().enumerate() {
            if index + 1 == next_index {
                print!("> ")
            } else {
                print!("  ")
            }
            println!("{:?}", inst.kind)
        }
        Ok(())
    }
}
