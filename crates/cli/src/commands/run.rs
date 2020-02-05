use super::command::{Command, CommandContext};
use super::debugger::{Debugger, RunResult};
use std::io::Write;

use clap::{App, Arg};
use anyhow::Result;

pub struct RunCommand {}

impl RunCommand {
    pub fn new() -> Self {
        Self {}
    }
}

const ARG_FUNCTION_NAME_KEY: &str = "function_name";
impl<D: Debugger> Command<D> for RunCommand {
    fn name(&self) -> &'static str {
        "run"
    }
    fn run(
        &self,
        debugger: &mut D,
        context: &CommandContext,
        args: Vec<&str>,
    ) -> Result<()> {
        let mut app = App::new("run").arg(Arg::with_name(ARG_FUNCTION_NAME_KEY).takes_value(true));
        let matches = match app.get_matches_from_safe_borrow(args) {
            Ok(m) => m,
            Err(_) => {
                let _ = app.print_long_help();
                return Ok(());
            }
        };
        if debugger.is_running() {
            print!("There is a running process, kill it and restart?: [Y/n] ");
            std::io::stdout().flush().unwrap();
            let stdin = std::io::stdin();
            let mut input = String::new();
            stdin.read_line(&mut input).unwrap();
            if input != "Y\n" {
                return Ok(());
            }
        }
        match debugger.run(
            matches
                .value_of(ARG_FUNCTION_NAME_KEY)
                .map(|name| name.to_string()),
        ) {
            Ok(RunResult::Finish(values)) => {
                println!("{:?}", values);
            }
            Ok(RunResult::Breakpoint) => {
                println!("Hit breakpoit");
            }
            Err(msg) => {
                eprintln!("{}", msg);
            }
        }
        Ok(())
    }
}
