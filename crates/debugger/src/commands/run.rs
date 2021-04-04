use super::command::AliasCommand;
use anyhow::Result;

pub struct RunCommand {}

impl RunCommand {
    pub fn new() -> Self {
        Self {}
    }
}

impl AliasCommand for RunCommand {
    fn name(&self) -> &'static str {
        "run"
    }

    fn run(&self, _args: Vec<&str>) -> Result<String> {
        Ok("process launch".to_string())
    }
}
