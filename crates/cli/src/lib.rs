mod commands;
mod debugger;
mod dwarf;
mod process;

use std::env;

fn history_file_path() -> String {
    format!(
        "{}/.wasminspect-history",
        env::var_os("HOME").unwrap().to_str().unwrap()
    )
}

pub fn run_loop(file: Option<String>) -> Result<(), String> {
    let mut debugger = debugger::MainDebugger::new()?;
    if let Some(file) = file {
        let parity_module = parity_wasm::deserialize_file(file)
            .unwrap()
            .parse_names()
            .map_err(|_| format!("Failed to parse name section"))?;
        debugger.load_module(&parity_module)?;
        use dwarf::parse_dwarf;
        let dwarf = parse_dwarf(&parity_module);
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
    )
    .map_err(|e| format!("{}", e))?;
    process.run_loop().map_err(|e| format!("{}", e))?;
    Ok(())
}
