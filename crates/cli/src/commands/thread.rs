use super::command::{Command, CommandContext};
use super::debugger::Debugger;

pub struct ThreadCommand {}

impl ThreadCommand {
    pub fn new() -> Self {
        Self {}
    }
}

use anyhow::Result;
use structopt::StructOpt;

#[derive(StructOpt)]
enum Opts {
    #[structopt(name = "backtrace")]
    Backtrace,
    #[structopt(name = "step-in")]
    StepIn,
}

use super::list::display_source;
impl<D: Debugger> Command<D> for ThreadCommand {
    fn name(&self) -> &'static str {
        "thread"
    }
    fn run(&self, debugger: &mut D, context: &CommandContext, args: Vec<&str>) -> Result<()> {
        let opts = Opts::from_iter_safe(args.clone())?;
        match opts {
            Opts::Backtrace => {
                for (index, frame) in debugger.frame().iter().rev().enumerate() {
                    println!("{}: {}", index, frame);
                }
            }
            Opts::StepIn => {
                debugger.step()?;
                let (insts, next_index) = debugger.instructions()?;
                let current_index = if next_index == 0 { 0 } else { next_index - 1 };
                let first_inst = insts[current_index].clone();
                display_source(first_inst.offset, &context.sourcemap)?;
            }
        }
        Ok(())
    }
}
