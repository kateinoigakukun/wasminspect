use crate::RunResult;

use super::command::{Command, CommandContext, CommandResult};
use super::debugger::Debugger;
use anyhow::Result;

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

    fn run(
        &self,
        debugger: &mut D,
        context: &CommandContext,
        args: Vec<&str>,
    ) -> Result<Option<CommandResult>> {
        let opts = Opts::from_iter_safe(args)?;
        match opts {
            Opts::Continue => match debugger.process()? {
                RunResult::Finish(result) => {
                    return Ok(Some(CommandResult::ProcessFinish(result)));
                }
                RunResult::Breakpoint => {
                    context.printer.println("Hit breakpoint");
                }
            },
        }
        Ok(None)
    }
}
