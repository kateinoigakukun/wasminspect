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
        let mut app = App::new("frame");
        let matches = match app.get_matches_from_safe_borrow(args) {
            Ok(m) => m,
            Err(_) => {
                let _ = app.print_long_help();
                return Ok(());
            }
        };
        Ok(())
    }
}
