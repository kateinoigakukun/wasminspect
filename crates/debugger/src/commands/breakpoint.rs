use super::command::{Command, CommandContext, CommandResult};
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
    /// Sets a breakpoint for the given symbol in executable
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

    fn description(&self) -> &'static str {
        "Commands for operating on breakpoints."
    }

    fn run(
        &self,
        debugger: &mut D,
        _context: &CommandContext,
        args: Vec<&str>,
    ) -> Result<Option<CommandResult>> {
        let opts = Opts::from_iter_safe(args)?;
        match opts {
            Opts::Set { name } => {
                let breakpoint = Breakpoint::Function { name };
                debugger.set_breakpoint(breakpoint);
                Ok(None)
            }
        }
    }
}
