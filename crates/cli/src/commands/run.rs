use super::command::{self, Command, Interface};
use super::debugger::Debugger;


use clap::{App, Arg};

pub struct RunCommand {}

impl RunCommand {
    pub fn new() -> Self {
        Self {}
    }
}

const ARG_FUNCTION_NAME_KEY: &str = "function_name";
impl<D: Debugger> Command<D> for RunCommand {
    fn name(&self) -> &'static str {
        "run"
    }
    fn run(
        &self,
        debugger: &mut D,
        _interface: &Interface,
        args: Vec<&str>,
    ) -> Result<(), command::Error> {
        let mut app = App::new("run").arg(Arg::with_name(ARG_FUNCTION_NAME_KEY).takes_value(true));
        let matches = match app.get_matches_from_safe_borrow(args) {
            Ok(m) => m,
            Err(_) => {
                let _ = app.print_long_help();
                return Ok(());
            }
        };
        match debugger.run(
            matches
                .value_of(ARG_FUNCTION_NAME_KEY)
                .map(|name| name.to_string()),
        ) {
            Ok(values) => {
                println!("{:?}", values);
            }
            Err(msg) => {
                eprintln!("{}", msg);
            }
        }
        Ok(())
    }
}
