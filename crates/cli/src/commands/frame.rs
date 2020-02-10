use super::command::{Command, CommandContext};
use super::debugger::Debugger;
use anyhow::Result;

use structopt::StructOpt;

pub struct FrameCommand {}

impl FrameCommand {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(StructOpt)]
enum Opts {
    #[structopt(name = "variable")]
    Variable,
}

impl<D: Debugger> Command<D> for FrameCommand {
    fn name(&self) -> &'static str {
        "frame"
    }

    fn description(&self) -> &'static str {
        "Commands for selecting current stack frame."
    }

    fn run(&self, debugger: &mut D, context: &CommandContext, args: Vec<&str>) -> Result<()> {
        let opts = Opts::from_iter_safe(args)?;
        match opts {
            Opts::Variable => {
                let (insts, next_index) = debugger.instructions()?;
                let current_index = if next_index == 0 { 0 } else { next_index - 1 };
                let current_inst = insts[current_index].clone();
                let variable_names = context.subroutine.variable_name_list(current_inst.offset)?;
                for variable in variable_names {
                    println!("{}: {}", variable.name, variable.type_name);
                }
                Ok(())
            }
        }
    }
}
