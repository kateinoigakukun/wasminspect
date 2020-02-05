use super::command::{self, Command, Interface};
use super::debugger::{Breakpoint, Debugger};
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
        _interface: &Interface,
        args: Vec<&str>,
    ) -> Result<(), command::Error> {
        let opts = match Opts::from_iter_safe(args) {
            Ok(opts) => opts,
            Err(e) => return Err(command::Error::Command(format!("{}", e))),
        };
        match opts {
            Opts::Set { name } => {
                let breakpoint = Breakpoint::Function { name };
                debugger.set_breakpoint(breakpoint);
                Ok(())
            }
        }
    }
}
