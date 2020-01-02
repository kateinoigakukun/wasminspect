use super::value::Value;
use parity_wasm::elements::ValueType;

pub struct BuiltinPrintI32 {}

impl BuiltinPrintI32 {
    pub fn dispatch(args: &[Value]) -> Option<Value> {
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].value_type(), ValueType::I32);
        match args[0] {
            Value::I32(val) => println!("{}", val),
            _ => panic!("Invalid argument type {}", args[0].value_type()),
        }
        None
    }
}

use std::cell::RefCell;
use std::rc::Rc;

use super::stack::Stack;
use super::table::DefinedTableInstance;
use super::memory::DefinedMemoryInstance;
use parity_wasm::elements::FunctionType;

type Ref<T> = Rc<RefCell<T>>;

pub enum HostValue {
    Func(HostFunc),
    Global(Value),
    Mem(Ref<DefinedMemoryInstance>),
    Table(Ref<DefinedTableInstance>),
}

pub struct HostFunc {
    ty: FunctionType,
    code: Box<dyn Fn(&[Value], &mut [Value]) -> Result<(), ()>>,
}

impl HostFunc {
    pub fn new<F>(ty: FunctionType, code: F) -> Self
    where
        F: Fn(&[Value], &mut [Value]) -> Result<(), ()>,
        F: 'static,
    {
        Self {
            ty,
            code: Box::new(code),
        }
    }

    pub fn call(&self, param: &[Value], results: &mut [Value]) -> Result<(), ()> {
        (self.code)(param, results)
    }

    pub fn ty(&self) -> &FunctionType {
        &self.ty
    }
}

