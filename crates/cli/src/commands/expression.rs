use super::command::{Command, CommandContext};
use super::debugger::Debugger;
use anyhow::{anyhow, Result};
use std::convert::TryInto;
use wasminspect_vm::WasmValue;

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

    fn description(&self) -> &'static str {
        "Evaluate an expression on the process (only support variable name now)."
    }

    fn run(&self, debugger: &mut D, context: &CommandContext, args: Vec<&str>) -> Result<()> {
        let opts = Opts::from_iter_safe(args)?;
        let (insts, next_index) = debugger.instructions()?;
        let current_index = if next_index == 0 { 0 } else { next_index - 1 };
        let current_inst = insts[current_index].clone();
        let argument_count = debugger
            .current_frame()
            .ok_or(anyhow!("function frame not found"))?
            .argument_count;
        let locals = debugger.locals();
        let rbp = match locals
            .get(argument_count + 2)
            .ok_or(anyhow!("failed to get rbp"))?
        {
            WasmValue::I32(v) => v,
            x => return Err(anyhow!("invalid type rbp: '{:?}'", x)),
        };
        context.subroutine.display_variable(
            current_inst.offset,
            TryInto::<u32>::try_into(*rbp)?,
            &debugger.memory()?,
            opts.symbol,
        )?;
        Ok(())
    }
}
