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
    #[structopt(name = "launch")]
    Launch {
        #[structopt(name = "FUNCTION NAME")]
        name: Option<String>,
    },

    /// Start WASI entry point
    #[structopt(name = "start")]
    Start {
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
            Opts::Launch { name } => {
                return self.start_debugger(debugger, context, name, None);
            }
            Opts::Start { args } => {
                return self.start_debugger(debugger, context, None, Some(args));
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
        name: Option<String>,
        wasi_args: Option<Vec<String>>
    ) -> Result<Option<CommandResult>> {
        use std::io::Write;
        if debugger.is_running() {
            print!("There is a running process, kill it and restart?: [Y/n] ");
            std::io::stdout().flush().unwrap();
            let stdin = std::io::stdin();
            let mut input = String::new();
            stdin.read_line(&mut input).unwrap();
            if input != "Y\n" {
                return Ok(None);
            }
        }
        if let Some(wasi_args) = wasi_args {
            debugger.instantiate(std::collections::HashMap::new(), Some(&wasi_args))?;
        } else {
            debugger.instantiate(std::collections::HashMap::new(), None)?;
        }

        match debugger.run(name.as_ref().map(String::as_str), vec![]) {
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
