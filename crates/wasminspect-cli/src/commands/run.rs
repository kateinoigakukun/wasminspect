use super::command::{self, Command};
use super::debugger::Debugger;

use clap::App;

pub struct RunCommand {}

impl RunCommand {
    pub fn new() -> Self {
        Self {}
    }
}

impl<D: Debugger> Command<D> for RunCommand {
    fn name(&self) -> &str { "run" }
    fn run(&self, debugger: &mut D, args: Vec<&str>) -> Result<(), command::Error> {
        let app = App::new("run").get_matches_from(args);
        Ok(())
    }
}
