use super::command::{self, Command, Interface};
use super::debugger::Debugger;

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
    fn run(&self, debugger: &mut D, _interface: &Interface, _args: Vec<&str>) -> Result<(), command::Error> {
        for (index, frame) in debugger.frame().iter().rev().enumerate() {
            println!("{}: {}", index, frame);
        }
        Ok(())
    }
}
