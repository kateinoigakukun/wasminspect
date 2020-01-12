use super::command::{self, Command};
use super::debugger::Debugger;



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
    fn run(&self, debugger: &mut D, _args: Vec<&str>) -> Result<(), command::Error> {
        for (index, frame) in debugger.frame().iter().rev().enumerate() {
            println!("{}: {}", index, frame);
        }
        Ok(())
    }
}
