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
    let context = commands::command::CommandContext {
        sourcemap: Box::new(dwarf::DwarfSourceMap {})
    };
    if let Some(file) = file {
        let mut f = ::std::fs::File::open(file)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        debugger.load_module(&buffer)?;
        use dwarf::parse_dwarf;
        let _dwarf = parse_dwarf(&buffer);
    }
    let mut process = process::Process::new(
        debugger,
        vec![
            Box::new(commands::run::RunCommand::new()),
            Box::new(commands::backtrace::BacktraceCommand::new()),
            Box::new(commands::list::ListCommand::new()),
            Box::new(commands::memory::MemoryCommand::new()),
            Box::new(commands::stack::StackCommand::new()),
            Box::new(commands::breakpoint::BreakpointCommand::new()),
        ],
        &history_file_path(),
    )?;
    process.run_loop(context)?;
    Ok(())
}
