use super::command::{self, Command, Interface};
use super::debugger::Debugger;



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
    fn run(&self, debugger: &mut D, _interface: &Interface, _args: Vec<&str>) -> Result<(), command::Error> {
        for (index, value) in debugger.stack_values().iter().enumerate() {
            println!("{}: {}", index, value)
        }
        Ok(())
    }
}
