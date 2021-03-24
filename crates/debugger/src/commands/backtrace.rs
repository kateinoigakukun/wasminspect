use super::command::AliasCommand;
use anyhow::Result;

pub struct BacktraceCommand {}

impl BacktraceCommand {
    pub fn new() -> Self {
        Self {}
    }
}

impl AliasCommand for BacktraceCommand {
    fn name(&self) -> &'static str {
        "bt"
    }

    fn run(&self, _args: Vec<&str>) -> Result<String> {
        Ok("thread backtrace".to_string())
    }
}
