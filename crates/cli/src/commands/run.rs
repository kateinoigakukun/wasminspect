use super::command::{Command, CommandContext};
use super::debugger::{Debugger, RunResult};
use std::io::Write;

use structopt::StructOpt;

use anyhow::Result;

pub struct RunCommand {}

impl RunCommand {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(StructOpt)]
struct Opts {
    #[structopt(name = "FUNCTION NAME")]
    name: Option<String>,
}
impl<D: Debugger> Command<D> for RunCommand {
    fn name(&self) -> &'static str {
        "run"
    }

    fn description(&self) -> &'static str {
        "Launch the executable in the debugger."
    }
    fn run(&self, debugger: &mut D, _context: &CommandContext, args: Vec<&str>) -> Result<()> {
        let opts = Opts::from_iter_safe(args)?;
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
        match debugger.run(opts.name) {
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
