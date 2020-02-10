use super::command::{Command, CommandContext};
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

    fn run(&self, debugger: &mut D, _context: &CommandContext, args: Vec<&str>) -> Result<()> {
        let opts = Opts::from_iter_safe(args)?;
        match opts {
            Opts::Read { index: None } => {
                for (index, value) in debugger.locals().iter().enumerate() {
                    println!("{: <3}: {:?}", index, value);
                }
            }
            Opts::Read { index: Some(index) } => {
                println!("{:?}", debugger.locals()[index]);
            }
        }
        Ok(())
    }
}
