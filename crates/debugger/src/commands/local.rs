use super::command::{Command, CommandContext, CommandResult};
use super::debugger::Debugger;
use anyhow::Result;

use structopt::StructOpt;

pub struct LocalCommand {}

impl LocalCommand {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(StructOpt)]
enum Opts {
    #[structopt(name = "read")]
    Read {
        #[structopt(name = "INDEX")]
        index: Option<usize>,
    },
}

impl<D: Debugger> Command<D> for LocalCommand {
    fn name(&self) -> &'static str {
        "local"
    }

    fn description(&self) -> &'static str {
        "Commands for operating locals."
    }

    fn run(&self, debugger: &mut D, context: &CommandContext, args: Vec<&str>) -> Result<Option<CommandResult>> {
        let opts = Opts::from_iter_safe(args)?;
        match opts {
            Opts::Read { index: None } => {
                for (index, value) in debugger.locals().iter().enumerate() {
                    let output = format!("{: <3}: {:?}", index, value);
                    context.printer.println(&output);
                }
            }
            Opts::Read { index: Some(index) } => {
                let output = format!("{:?}", debugger.locals()[index]);
                context.printer.println(&output);
            }
        }
        Ok(None)
    }
}
