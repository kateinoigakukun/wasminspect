use crate::{Executor, Store};
use crate::executor::{ExecResult, Signal};
use crate::inst::Instruction;

pub trait Interceptor {
    fn invoke_func(&self, name: &str, executor: &Executor, store: &Store) -> ExecResult<Signal>;
    fn execute_inst(&self, inst: &Instruction) -> ExecResult<Signal>;
    fn after_store(&self, addr: usize, bytes: &[u8]) -> ExecResult<Signal>;
}

#[derive(Default)]
pub struct NopInterceptor {}
impl NopInterceptor {
    pub fn new() -> Self {
        Default::default()
    }
}
impl Interceptor for NopInterceptor {
    fn invoke_func(&self, _name: &str, _executor: &Executor, _store: &Store) -> ExecResult<Signal> {
        Ok(Signal::Next)
    }
    fn execute_inst(&self, _inst: &Instruction) -> ExecResult<Signal> {
        Ok(Signal::Next)
    }

    fn after_store(&self, _addr: usize, _bytes: &[u8]) -> ExecResult<Signal> {
        Ok(Signal::Next)
    }
}
