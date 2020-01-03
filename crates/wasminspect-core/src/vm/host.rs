use super::value::Value;

use std::cell::RefCell;
use std::rc::Rc;

use super::executor::Trap;
use super::global::DefinedGlobalInstance;
use super::memory::DefinedMemoryInstance;
use super::table::DefinedTableInstance;
use parity_wasm::elements::FunctionType;

type Ref<T> = Rc<RefCell<T>>;

pub enum HostValue {
    Func(HostFuncBody),
    Global(Rc<RefCell<DefinedGlobalInstance>>),
    Mem(Ref<DefinedMemoryInstance>),
    Table(Ref<DefinedTableInstance>),
}

pub struct HostFuncBody {
    ty: FunctionType,
    code: Box<dyn Fn(&[Value], &mut [Value]) -> Result<(), Trap>>,
}

impl HostFuncBody {
    pub fn new<F>(ty: FunctionType, code: F) -> Self
    where
        F: Fn(&[Value], &mut [Value]) -> Result<(), Trap>,
        F: 'static,
    {
        Self {
            ty,
            code: Box::new(code),
        }
    }

    pub fn call(&self, param: &[Value], results: &mut [Value]) -> Result<(), Trap> {
        (self.code)(param, results)
    }

    pub fn ty(&self) -> &FunctionType {
        &self.ty
    }
}
