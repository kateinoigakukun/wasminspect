use super::command::{Command, CommandContext};
use super::debugger::Debugger;
use anyhow::Result;
use wasminspect_vm::Signal;

use structopt::StructOpt;

pub struct ProcessCommand {}

impl ProcessCommand {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(StructOpt)]
enum Opts {
    #[structopt(name = "continue")]
    Continue,
}

impl<D: Debugger> Command<D> for ProcessCommand {
    fn name(&self) -> &'static str {
        "process"
    }

    fn description(&self) -> &'static str {
        "Commands for interacting with processes."
    }

    fn run(&self, debugger: &mut D, _context: &CommandContext, args: Vec<&str>) -> Result<()> {
        let opts = Opts::from_iter_safe(args)?;
        match opts {
            Opts::Continue => match debugger.process()? {
                Signal::Next => unreachable!(),
                Signal::End => {}
                Signal::Breakpoint => {
                    println!("Hit breakpoit");
                }
            },
        }
        Ok(())
    }
}
