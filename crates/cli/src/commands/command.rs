use super::debugger::Debugger;

pub enum Error {
    Command(String),
}

pub trait Command<'a, D: Debugger<'a>> {
    fn name(&self) -> &'static str;
    fn run(&self, debugger: &'a mut D, args: Vec<&str>) -> Result<(), Error>;
}
