use super::command::{self, Command};
use super::debugger::Debugger;

use clap::{App, Arg};

pub struct FrameCommand {}

impl FrameCommand {
    pub fn new() -> Self {
        Self {}
    }
}

impl<D: Debugger> Command<D> for FrameCommand {
    fn name(&self) -> &'static str {
        "frame"
    }
    fn run(&self, debugger: &mut D, args: Vec<&str>) -> Result<(), command::Error> {
        for frame in debugger.frame() {
            println!("{}", frame);
        }
        Ok(())
    }
}
