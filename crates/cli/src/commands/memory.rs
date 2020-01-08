use super::command::{self, Command};
use super::debugger::Debugger;

use clap::{App, Arg};

pub struct MemoryCommand {}

impl MemoryCommand {
    pub fn new() -> Self {
        Self {}
    }
}

impl<D: Debugger> Command<D> for MemoryCommand {
    fn name(&self) -> &'static str {
        "memory"
    }
    fn run(&self, debugger: &mut D, args: Vec<&str>) -> Result<(), command::Error> {
        Ok(())
    }
}
