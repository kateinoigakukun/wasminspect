use super::command::{Command, CommandContext};
use super::debugger::Debugger;
use anyhow::Result;

use structopt::StructOpt;

pub struct SettingsCommand {}

impl SettingsCommand {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(StructOpt)]
enum Opts {
    #[structopt(name = "set")]
    Set {
        key: String,
        operand1: String,
        operand2: String,
    },
}

impl<D: Debugger> Command<D> for SettingsCommand {
    fn name(&self) -> &'static str {
        "settings"
    }

    fn description(&self) -> &'static str {
        "Commands for setting environment"
    }

    fn run(&self, _debugger: &mut D, context: &CommandContext, args: Vec<&str>) -> Result<()> {
        let opts = Opts::from_iter_safe(args)?;
        match opts {
            Opts::Set {
                key,
                operand1,
                operand2,
            } => match key.as_str() {
                "directory.map" => {
                    context.sourcemap.set_directory_map(operand1, operand2);
                }
                _ => {
                    let output = format!("'{}' is not valid key", key);
                    context.printer.eprintln(&output);
                },
            },
        }
        Ok(())
    }
}
