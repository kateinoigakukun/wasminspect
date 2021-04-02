use super::commands::command::{self, AliasCommand, Command, CommandResult};
use super::commands::debugger::Debugger;
use anyhow::{Context, Result};
use linefeed::{DefaultTerminal, Interface, ReadResult};
use std::io;
use std::{collections::HashMap, time::Duration};

pub struct Process<D: Debugger> {
    pub debugger: D,
    commands: HashMap<String, Box<dyn Command<D>>>,
    aliases: HashMap<String, Box<dyn AliasCommand>>,
}

impl<D: Debugger> Process<D> {
    pub fn new(
        debugger: D,
        commands: Vec<Box<dyn Command<D>>>,
        aliases: Vec<Box<dyn AliasCommand>>,
    ) -> anyhow::Result<Self> {
        let mut cmd_map = HashMap::new();
        for cmd in commands {
            cmd_map.insert(cmd.name().to_string().clone(), cmd);
        }
        let mut alias_map = HashMap::new();
        for cmd in aliases {
            alias_map.insert(cmd.name().to_string().clone(), cmd);
        }
        Ok(Self {
            debugger,
            commands: cmd_map,
            aliases: alias_map,
        })
    }

    pub fn dispatch_command(
        &mut self,
        line: &str,
        context: &command::CommandContext,
    ) -> Result<Option<CommandResult>> {
        let cmd_name = extract_command_name(&line);
        let args = line.split_whitespace().collect();
        if let Some(cmd) = self.commands.get(cmd_name) {
            match cmd.run(&mut self.debugger, &context, args) {
                Ok(result) => Ok(result),
                Err(err) => {
                    eprintln!("{}", err);
                    Ok(None)
                }
            }
        } else if let Some(alias) = self.aliases.get(cmd_name) {
            let line = alias.run(args)?;
            return self.dispatch_command(&line, context);
        } else if cmd_name == "help" {
            println!("Available commands:");
            for (_, command) in &self.commands {
                println!("  {} -- {}", command.name(), command.description());
            }
            Ok(None)
        } else if cfg!(feature = "remote-api") && cmd_name == "start-server" {
            Ok(Some(CommandResult::Exit))
        } else {
            eprintln!("'{}' is not a valid command.", cmd_name);
            Ok(None)
        }
    }
}

pub struct Interactive {
    pub interface: Interface<DefaultTerminal>,

    history_file: String,
}

fn history_file_path() -> String {
    format!(
        "{}/.wasminspect-history",
        std::env::var_os("HOME").unwrap().to_str().unwrap()
    )
}

impl Interactive {
    pub fn new_with_loading_history() -> anyhow::Result<Self> {
        Self::new(&history_file_path())
    }

    pub fn new(history_file: &str) -> anyhow::Result<Self> {
        let interface = Interface::new("wasminspect").with_context(|| "new Interface")?;
        interface
            .set_prompt("(wasminspect) ")
            .with_context(|| "set prompt")?;
        if let Err(e) = interface.load_history(history_file) {
            if e.kind() == io::ErrorKind::NotFound {
            } else {
                eprintln!("Could not load history file {}: {}", history_file, e);
            }
        }
        Ok(Self {
            interface,
            history_file: history_file.to_string(),
        })
    }
    pub fn run_step<D: Debugger>(
        &mut self,
        context: &command::CommandContext,
        process: &mut Process<D>,
        last_line: &mut Option<String>,
        timeout: Option<Duration>,
    ) -> Result<Option<CommandResult>> {
        let line = match self.interface.read_line_step(timeout)? {
            Some(ReadResult::Input(line)) => line,
            Some(_) => return Ok(Some(CommandResult::Exit)),
            None => return Ok(None),
        };
        let result = if !line.trim().is_empty() {
            self.interface.add_history_unique(line.clone());
            *last_line = Some(line.clone());
            process.dispatch_command(&line, context)?
        } else if let Some(last_line) = last_line.as_ref() {
            process.dispatch_command(last_line, context)?
        } else {
            None
        };
        Ok(result)
    }

    pub fn run_loop<D: Debugger>(
        &mut self,
        context: &command::CommandContext,
        process: &mut Process<D>,
    ) -> Result<CommandResult> {
        let mut last_line: Option<String> = None;
        loop {
            if let Some(result) = self.run_step(context, process, &mut last_line, None)? {
                return Ok(result);
            }
        }
    }
}

fn extract_command_name(s: &str) -> &str {
    let s = s.trim();

    match s.find(|ch: char| ch.is_whitespace()) {
        Some(pos) => &s[..pos],
        None => s,
    }
}

impl Drop for Interactive {
    fn drop(&mut self) {
        if let Err(error) = self.interface.save_history(&self.history_file) {
            println!("Error while saving command history: {}", error);
        }
    }
}
