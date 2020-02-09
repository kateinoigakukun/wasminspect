mod commands;
mod debugger;
mod dwarf;
mod process;

use anyhow::Result;
use std::env;
use std::io::Read;

fn history_file_path() -> String {
    format!(
        "{}/.wasminspect-history",
        env::var_os("HOME").unwrap().to_str().unwrap()
    )
}

pub fn run_loop(file: Option<String>) -> Result<()> {
    let mut debugger = debugger::MainDebugger::new()?;
    let mut buffer = Vec::new();
    let mut context = commands::command::CommandContext {
        sourcemap: Box::new(commands::sourcemap::EmptySourceMap::new()),
        subroutine: Box::new(commands::subroutine::EmptySubroutineMap::new()),
    };

    if let Some(file) = file {
        let mut f = ::std::fs::File::open(file)?;
        f.read_to_end(&mut buffer)?;
        debugger.load_module(&buffer)?;
        use dwarf::{parse_dwarf, transform_dwarf};
        let dwarf = parse_dwarf(&buffer)?;
        let debug_info = transform_dwarf(dwarf)?;
        context.sourcemap = Box::new(debug_info.sourcemap);
        context.subroutine = Box::new(debug_info.subroutine);
    }
    let mut process = process::Process::new(
        debugger,
        vec![
            Box::new(commands::run::RunCommand::new()),
            Box::new(commands::thread::ThreadCommand::new()),
            Box::new(commands::list::ListCommand::new()),
            Box::new(commands::memory::MemoryCommand::new()),
            Box::new(commands::stack::StackCommand::new()),
            Box::new(commands::breakpoint::BreakpointCommand::new()),
            Box::new(commands::disassemble::DisassembleCommand::new()),
            Box::new(commands::expression::ExpressionCommand::new()),
            Box::new(commands::global::GlobalCommand::new()),
            Box::new(commands::local::LocalCommand::new()),
            Box::new(commands::frame::FrameCommand::new()),
        ],
        vec![Box::new(commands::backtrace::BacktraceCommand::new())],
        &history_file_path(),
    )?;
    process.run_loop(context)?;
    Ok(())
}
