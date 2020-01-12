use super::command::{self, Command};
use super::debugger::Debugger;



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
    fn run(&self, debugger: &mut D, _args: Vec<&str>) -> Result<(), command::Error> {
        let (insts, next_index) = debugger.instructions().map_err(command::Error::Command)?;
        for (index, inst) in insts.iter().enumerate() {
            if index == next_index - 1 {
                print!("> ")
            } else {
                print!("  ")
            }
            println!("{}", inst)
        }
        Ok(())
    }
}
