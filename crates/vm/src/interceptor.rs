use crate::executor::{ExecResult, Signal};
use crate::inst::Instruction;

pub trait Interceptor {
    fn invoke_func(&self, name: &String) -> ExecResult<Signal>;
    fn execute_inst(&self, inst: &Instruction);
    fn after_store(&self, addr: usize, bytes: &[u8]) -> ExecResult<Signal>;
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

    fn after_store(&self, _addr: usize, _bytes: &[u8]) -> ExecResult<Signal> {
        Ok(Signal::Next)
    }
}
