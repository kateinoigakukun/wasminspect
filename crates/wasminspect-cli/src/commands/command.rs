use super::debugger::Debugger;

pub enum Error {}

pub struct Command<D>
where
    D: Debugger,
{
    pub name: &'static str,
    pub subcommands: Vec<Command<D>>,
    runner: Box<dyn Fn(&mut D) -> Result<(), Error>>,
}

impl<D> Command<D> where D: Debugger {
    pub fn run(&self, debugger: &mut D) -> Result<(), Error> {
        (*self.runner)(debugger)
    }
}
