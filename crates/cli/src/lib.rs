mod commands;
mod debugger;
mod process;

use std::env;

fn history_file_path() -> String {
    format!(
        "{}/.wasminspect-history",
        env::var_os("HOME").unwrap().to_str().unwrap()
    )
}

pub fn run_loop(file: Option<String>) -> Result<(), String> {
    let debugger = debugger::MainDebugger::new(file)?;
    let mut process = process::Process::new(
        debugger,
        vec![
            Box::new(commands::run::RunCommand::new()),
            Box::new(commands::backtrace::BacktraceCommand::new()),
            Box::new(commands::list::ListCommand::new()),
        ],
        &history_file_path(),
    )
    .map_err(|e| format!("{}", e))?;
    process.run_loop().map_err(|e| format!("{}", e))?;
    Ok(())
}
