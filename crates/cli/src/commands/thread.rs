use super::command::{Command, CommandContext};
use super::debugger::Debugger;

pub struct ThreadCommand {}

impl ThreadCommand {
    pub fn new() -> Self {
        Self {}
    }
}

use anyhow::Result;
use structopt::StructOpt;

#[derive(StructOpt)]
enum Opts {
    #[structopt(name = "backtrace")]
    Backtrace,
}

impl<D: Debugger> Command<D> for ThreadCommand {
    fn name(&self) -> &'static str {
        "thread"
    }
    fn run(
        &self,
        debugger: &mut D,
        _context: &CommandContext,
        args: Vec<&str>,
    ) -> Result<()> {
        let opts = Opts::from_iter_safe(args)?;
        match opts {
            Opts::Backtrace => {
        for (index, frame) in debugger.frame().iter().rev().enumerate() {
            println!("{}: {}", index, frame);
        }
            }
        }
        Ok(())
    }
}
