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

use std::rc::Rc;
use std::cell::RefCell;

use super::stack::Stack;
use parity_wasm::elements::FunctionType;

type Ref<T> = Rc<RefCell<T>>;

pub enum HostValue {
    Func(HostFunc),
    Mem(),
    Global(Value),
}

pub struct HostFunc {
    code: Box<dyn Fn(&Stack) + 'static>,
    ty: FunctionType,
}