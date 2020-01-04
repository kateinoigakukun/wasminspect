use super::commands::command::{self, Command};
use super::commands::debugger::Debugger;
use linefeed::{DefaultTerminal, Interface, ReadResult};
use std::collections::HashMap;
use std::io;

pub struct Process<D: Debugger> {
    interface: Interface<DefaultTerminal>,
    debugger: D,
    commands: HashMap<String, Box<dyn Command<D>>>,

    history_file: String,
}

impl<D: Debugger> Process<D> {
    pub fn new(
        debugger: D,
        commands: Vec<Box<dyn Command<D>>>,
        history_file: &str,
    ) -> io::Result<Self> {
        let interface = Interface::new("wasminspect")?;

        interface.set_prompt("(wasminspect) ")?;

        if let Err(e) = interface.load_history(history_file) {
            if e.kind() == io::ErrorKind::NotFound {
            } else {
                eprintln!("Could not load history file {}: {}", history_file, e);
            }
        }
        let mut cmd_map = HashMap::new();
        for cmd in commands {
            cmd_map.insert(cmd.name().to_string(), cmd);
        }
        Ok(Self {
            interface,
            debugger,
            commands: cmd_map,
            history_file: history_file.to_string(),
        })
    }

    pub fn run_loop(&mut self) -> io::Result<()> {
        while let ReadResult::Input(line) = self.interface.read_line()? {
            if !line.trim().is_empty() {
                self.interface.add_history_unique(line.clone());
            }
            let cmd_name = extract_command_name(&line);
            let cmd = &self.commands.get(cmd_name);
            if let Some(cmd) = self.commands.get(cmd_name) {
                let args = line.split_whitespace();
                match cmd.run(&mut self.debugger, args.collect()) {
                    Ok(()) => (),
                    Err(command::Error::Command(message)) => {
                        eprintln!("{}", message);
                    }
                }
            } else {
                eprintln!("invalid command name {}", cmd_name);
            }
        }
        Ok(())
    }
}

fn extract_command_name(s: &str) -> &str {
    let s = s.trim();

    match s.find(|ch: char| ch.is_whitespace()) {
        Some(pos) => &s[..pos],
        None => s,
    }
}

impl<D: Debugger> Drop for Process<D> {
    fn drop(&mut self) {
        if let Err(error) = self.interface.save_history(&self.history_file) {
            println!("Error while saving command history: {}", error);
        }
    }
}
