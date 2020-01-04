use super::debugger::Debugger;

pub enum Error {
    Command(String),
}

pub trait Command<D: Debugger> {
    fn name(&self) -> &str;
    fn run(&self, debugger: &mut D, args: Vec<&str>) -> Result<(), Error>;
}
