use super::command::{Command, CommandContext};
use super::debugger::Debugger;

use anyhow::Result;

pub struct DisassembleCommand {}

impl DisassembleCommand {
    pub fn new() -> Self {
        Self {}
    }
}

impl<D: Debugger> Command<D> for DisassembleCommand {
    fn name(&self) -> &'static str {
        "disassemble"
    }

    fn description(&self) -> &'static str {
        "Disassemble instructions in the current function."
    }

    fn run(&self, debugger: &mut D, _context: &CommandContext, _args: Vec<&str>) -> Result<()> {
        display_asm(debugger)
    }
}

pub fn display_asm<D: Debugger>(debugger: &D) -> Result<()> {
    let (insts, next_index) = debugger.instructions()?;
    for (index, inst) in insts.iter().enumerate() {
        let prefix = if index + 1 == next_index { "->" } else { "  " };
        println!("{} 0x{:>08x}: {:?}", prefix, inst.offset, inst.kind)
    }
    Ok(())
}
