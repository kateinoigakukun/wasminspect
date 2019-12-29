use super::value::Value;
use parity_wasm::elements::ValueType;

pub trait HostFunc {
    fn call(args: &[Value]) -> Option<Value>;
}

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

// struct ModuleRegistory 