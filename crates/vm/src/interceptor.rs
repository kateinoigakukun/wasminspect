use crate::executor::{ExecResult, Signal};
use crate::inst::Instruction;

pub trait Interceptor {
    fn invoke_func(&self, name: &String) -> ExecResult<Signal>;
    fn execute_inst(&self, inst: &Instruction);
}

pub struct NopInterceptor {}
impl NopInterceptor {
    pub fn new() -> Self {
        Self {}
    }
}
impl Interceptor for NopInterceptor {
    fn invoke_func(&self, _name: &String) -> ExecResult<Signal> {
        Ok(Signal::Next)
    }
    fn execute_inst(&self, _inst: &Instruction) {}
}
