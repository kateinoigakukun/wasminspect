use super::commands::command::{self, Command, AliasCommand};
use super::commands::debugger::Debugger;
use linefeed::{DefaultTerminal, Interface, ReadResult};
use std::collections::HashMap;
use std::io;
use anyhow::Result;

pub struct Process<D: Debugger> {
    interface: Interface<DefaultTerminal>,
    debugger: D,
    commands: HashMap<String, Box<dyn Command<D>>>,
    aliases: HashMap<String, Box<dyn AliasCommand>>,

    history_file: String,
}

impl<D: Debugger> Process<D> {
    pub fn new(
        debugger: D,
        commands: Vec<Box<dyn Command<D>>>,
        aliases: Vec<Box<dyn AliasCommand>>,
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
            cmd_map.insert(cmd.name().to_string().clone(), cmd);
        }
        let mut alias_map = HashMap::new();
        for cmd in aliases {
            alias_map.insert(cmd.name().to_string().clone(), cmd);
        }
        Ok(Self {
            interface,
            debugger,
            commands: cmd_map,
            aliases: alias_map,
            history_file: history_file.to_string(),
        })
    }

    pub fn run_loop(&mut self, context: command::CommandContext) -> Result<()> {
        while let ReadResult::Input(line) = self.interface.read_line()? {
            if !line.trim().is_empty() {
                self.interface.add_history_unique(line.clone());
            }
            self.dispatch_command(line, &context)?;
        }
        Ok(())
    }

    fn dispatch_command(&mut self, line: String, context: &command::CommandContext) -> Result<()> {
        let cmd_name = extract_command_name(&line);
        let args = line.split_whitespace().collect();
        if let Some(cmd) = self.commands.get(cmd_name) {
            match cmd.run(&mut self.debugger, &context, args) {
                Ok(()) => (),
                Err(err) => {
                    eprintln!("{}", err);
                }
            }
        } else if let Some(alias) = self.aliases.get(cmd_name) {
            let line = alias.run(args)?.clone();
            self.dispatch_command(line, context)?
        } else if cmd_name == "help" {
            println!("Available commands:");
            for (_, command) in &self.commands {
                println!("  {} -- {}", command.name(), command.description());
            }
        } else {
            eprintln!("'{}' is not a valid command.", cmd_name);
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

impl<'a, D: Debugger> Drop for Process<D> {
    fn drop(&mut self) {
        if let Err(error) = self.interface.save_history(&self.history_file) {
            println!("Error while saving command history: {}", error);
        }
    }
}
