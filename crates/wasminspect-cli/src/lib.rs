mod commands;
mod debugger;
mod process;

use std::collections::HashMap;
use std::env;
use std::io;

fn history_file_path() -> String {
    format!(
        "{}/.wasminspect-history",
        env::var_os("HOME").unwrap().to_str().unwrap()
    )
}

pub fn run_loop() -> io::Result<()> {
    let debugger = debugger::MainDebugger::new();
    let mut process = process::Process::new(
        debugger,
        vec![Box::new(commands::run::RunCommand::new())],
        &history_file_path(),
    )?;
    process.run_loop()?;
    Ok(())
}
