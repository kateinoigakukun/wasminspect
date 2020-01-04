use super::debugger::Debugger;

pub enum Error {
    Command(String),
}

pub struct Command<D>
where
    D: Debugger,
{
    runner: Box<dyn Fn(&mut D) -> Result<(), Error>>,
}

impl<D> Command<D>
where
    D: Debugger,
{
    pub fn new<F>(runner: F) -> Self
    where
        F: Fn(&mut D) -> Result<(), Error> + 'static,
    {
        Self {
            runner: Box::new(runner),
        }
    }
    pub fn run(&self, debugger: &mut D, args: &str) -> Result<(), Error> {
        (*self.runner)(debugger)
    }
}
