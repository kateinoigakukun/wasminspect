use super::command::{Command, CommandContext};
use super::debugger::Debugger;
use anyhow::Result;

pub struct ExpressionCommand {}

impl ExpressionCommand {
    pub fn new() -> Self {
        Self {}
    }
}

use structopt::StructOpt;
#[derive(StructOpt)]
struct Opts {
    #[structopt(name = "SYMBOL")]
    symbol: String,
}

impl<D: Debugger> Command<D> for ExpressionCommand {
    fn name(&self) -> &'static str {
        "expression"
    }

    fn run(&self, debugger: &mut D, context: &CommandContext, args: Vec<&str>) -> Result<()> {
        let opts = Opts::from_iter_safe(args)?;
        let (insts, next_index) = debugger.instructions()?;
        let current_index = if next_index == 0 { 0 } else { next_index - 1 };
        let current_inst = insts[current_index].clone();
        context
            .subroutine
            .display_variable(current_inst.offset, opts.symbol);
        Ok(())
    }
}
