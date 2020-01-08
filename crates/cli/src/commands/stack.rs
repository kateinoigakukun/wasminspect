use super::command::{self, Command};
use super::debugger::Debugger;

use clap::{App, Arg};

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
    fn run(&self, debugger: &mut D, args: Vec<&str>) -> Result<(), command::Error> {
        for (index, value) in debugger.stack_values().iter().enumerate() {
            println!("{}: {}", index, value)
        }
        Ok(())
    }
}
