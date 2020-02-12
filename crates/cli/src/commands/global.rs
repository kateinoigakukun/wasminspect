use super::command::{Command, CommandContext};
use super::debugger::Debugger;
use anyhow::{anyhow, Result};

use structopt::StructOpt;

pub struct GlobalCommand {}

impl GlobalCommand {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(StructOpt)]
enum Opts {
    #[structopt(name = "read")]
    Read {
        #[structopt(name = "INDEX")]
        index: usize,
    },
}

impl<D: Debugger> Command<D> for GlobalCommand {
    fn name(&self) -> &'static str {
        "global"
    }

    fn description(&self) -> &'static str {
        "Commands for operating globals."
    }

    fn run(&self, debugger: &mut D, _context: &CommandContext, args: Vec<&str>) -> Result<()> {
        let opts = Opts::from_iter_safe(args)?;
        use wasminspect_vm::*;
        match opts {
            Opts::Read { index } => {
                let store: &Store = debugger.store();
                let mod_index = match debugger.current_frame() {
                    Some(frame) => frame.module_index,
                    None => return Err(anyhow!("function frame not found")),
                };
                let global = store.global(GlobalAddr::new_unsafe(mod_index, index));
                println!("{:?}", global.borrow().value());
                Ok(())
            }
        }
    }
}
