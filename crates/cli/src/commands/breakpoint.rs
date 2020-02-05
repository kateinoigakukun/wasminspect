use super::command::{Command, CommandContext};
use super::debugger::{Breakpoint, Debugger};
use anyhow::Result;
use structopt::StructOpt;

pub struct BreakpointCommand {}

impl BreakpointCommand {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(StructOpt)]
enum Opts {
    #[structopt(name = "set")]
    Set {
        #[structopt(name = "SYMBOL NAME")]
        name: String,
    },
}

impl<D: Debugger> Command<D> for BreakpointCommand {
    fn name(&self) -> &'static str {
        "breakpoint"
    }
    fn run(
        &self,
        debugger: &mut D,
        context: &CommandContext,
        args: Vec<&str>,
    ) -> Result<()> {
        let opts = Opts::from_iter_safe(args)?;
        match opts {
            Opts::Set { name } => {
                let breakpoint = Breakpoint::Function { name };
                debugger.set_breakpoint(breakpoint);
                Ok(())
            }
        }
    }
}
