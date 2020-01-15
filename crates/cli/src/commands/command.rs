use super::debugger::Debugger;
use linefeed;

pub enum Error {
    Command(String),
}

pub type Interface = linefeed::Interface<linefeed::DefaultTerminal>;

pub trait Command<D: Debugger> {
    fn name(&self) -> &'static str;
    fn run(&self, debugger: &mut D, interface: &Interface, args: Vec<&str>) -> Result<(), Error>;
}
