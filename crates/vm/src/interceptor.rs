use crate::executor::{ExecResult, Signal};

pub trait Interceptor {
    fn invoke_func(&self, name: &String) -> ExecResult<Signal>;
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
}
