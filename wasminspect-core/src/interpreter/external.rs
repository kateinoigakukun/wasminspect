use std::rc::Rc;
use std::cell::RefCell;

use super::memory::MemoryInstance;
use super::stack::Stack;
use parity_wasm::elements::FunctionType;

type Ref<T> = Rc<RefCell<T>>;

pub enum External {
    Func(Ref<Func>),
    Mem(Ref<MemoryInstance>),
}

pub struct Func {
    code: Box<dyn Fn(&Stack) + 'static>,
    ty: FunctionType,
}