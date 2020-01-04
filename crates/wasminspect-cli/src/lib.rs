mod debugger;
mod commands;
mod process;

use std::io;
use std::env;
use std::collections::HashMap;


fn history_file_path() -> String {
    format!("{}/.wasminspect-history", env::var_os("HOME").unwrap().to_str().unwrap())
}

pub fn run_loop() -> io::Result<()> {
    let debugger = debugger::MainDebugger::new();
    let mut cmds = HashMap::new();
    let command = commands::command::Command::new(|_| {
        println!("Yeah 👍");
        Ok(())
    });
    cmds.insert("run".to_string(), command);
    let mut process = process::Process::new(debugger, cmds, &history_file_path())?;
    process.run_loop()?;
    Ok(())
}