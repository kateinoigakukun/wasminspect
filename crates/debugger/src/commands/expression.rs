use super::super::dwarf::{FrameBase, WasmLoc};
use super::command::{Command, CommandContext, CommandResult};
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

    fn run(
        &self,
        debugger: &mut D,
        context: &CommandContext,
        args: Vec<&str>,
    ) -> Result<Option<CommandResult>> {
        let opts = Opts::from_iter_safe(args)?;
        let (insts, next_index) = debugger.instructions()?;
        let current_index = if next_index == 0 { 0 } else { next_index - 1 };
        let current_inst = insts[current_index].clone();
        let locals = debugger.locals();
        use wasminspect_vm::*;
        let store: &Store = debugger.store();
        let mod_index = match debugger.current_frame() {
            Some(frame) => frame.module_index,
            None => return Err(anyhow!("function frame not found")),
        };
        let frame_base = match context.subroutine.get_frame_base(current_inst.offset)? {
            Some(loc) => {
                let offset = match loc {
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
                let offset = match offset {
                    WasmValue::I32(v) => v as u64,
                    WasmValue::I64(v) => v as u64,
                    _ => Err(anyhow!("unexpected frame base value: {:?}", offset))?,
                };
                FrameBase::WasmFrameBase(offset)
            }
            None => {
                let argument_count = debugger
                    .current_frame()
                    .ok_or(anyhow!("function frame not found"))?
                    .argument_count;
                let offset = locals
                    .get(argument_count + 2)
                    .ok_or(anyhow!("failed to get rbp"))?
                    .clone();
                let offset = match offset {
                    WasmValue::I32(v) => v as u64,
                    _ => Err(anyhow!("unexpected frame base value: {:?}", offset))?,
                };
                FrameBase::RBP(offset)
            }
        };
        let output = format!("frame_base is {:?}", frame_base);
        context.printer.println(&output);
        context.subroutine.display_variable(
            current_inst.offset,
            frame_base,
            &debugger.memory()?,
            opts.symbol,
        )?;
        Ok(None)
    }
}
