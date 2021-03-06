use super::super::dwarf::WasmLoc;
use super::command::{Command, CommandContext};
use super::debugger::Debugger;
use anyhow::{anyhow, Result};

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
        let locals = debugger.locals();
        let loc = context.subroutine.get_frame_base(current_inst.offset)?;
        use wasminspect_vm::*;
        let store: &Store = debugger.store();
        let mod_index = match debugger.current_frame() {
            Some(frame) => frame.module_index,
            None => return Err(anyhow!("function frame not found")),
        };
        let frame_base = match loc {
            WasmLoc::Global(idx) => store
                .global(GlobalAddr::new_unsafe(mod_index, idx as usize))
                .borrow()
                .value(),
            WasmLoc::Local(idx) => *locals
                .get(idx as usize)
                .ok_or(anyhow!("failed to get base local"))?,
            WasmLoc::Stack(idx) => *debugger
                .stack_values()
                .get(idx as usize)
                .ok_or(anyhow!("failed to get base local"))?,
        };
        println!("frame_base is {:?}", frame_base);
        let frame_base_value = match frame_base {
            WasmValue::I32(v) => v as u64,
            WasmValue::I64(v) => v as u64,
            _ => Err(anyhow!("unexpected frame base value: {:?}", frame_base))?,
        };
        context.subroutine.display_variable(
            current_inst.offset,
            frame_base_value,
            &debugger.memory()?,
            opts.symbol,
        )?;
        Ok(())
    }
}
