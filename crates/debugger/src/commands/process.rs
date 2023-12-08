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

    /// Start WASI entry point
    #[structopt(name = "launch")]
    Launch {
        /// Entry point to start
        start: Option<String>,

        /// Arguments to pass to the WASI entry point
        #[structopt(name = "ARGS", last = true)]
        args: Vec<String>,
    },
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
            Opts::Launch { start, args } => {
                return self.start_debugger(debugger, context, start, args);
            }
        }
        Ok(None)
    }
}
impl ProcessCommand {
    fn start_debugger<D: Debugger>(
        &self,
        debugger: &mut D,
        context: &CommandContext,
        start: Option<String>,
        wasi_args: Vec<String>,
    ) -> Result<Option<CommandResult>> {
        use std::io::Write;
        if debugger.is_running() {
            print!("There is a running process, kill it and restart?: [Y/n] ");
            std::io::stdout().flush().unwrap();
            let stdin = std::io::stdin();
            let mut input = String::new();
            stdin.read_line(&mut input).unwrap();
            if input != "Y\n" && input != "y\n" {
                return Ok(None);
            }
        }
        debugger.instantiate(std::collections::HashMap::new(), Some(&wasi_args))?;

        match debugger.run(start.as_deref(), vec![]) {
            Ok(RunResult::Finish(values)) => {
                let output = format!("{:?}", values);
                context.printer.println(&output);
                return Ok(Some(CommandResult::ProcessFinish(values)));
            }
            Ok(RunResult::Breakpoint) => {
                context.printer.println("Hit breakpoint");
            }
            Err(msg) => {
                let output = format!("{}", msg);
                context.printer.eprintln(&output);
            }
        }
        Ok(None)
    }
}
