use parity_wasm::elements::ValueType;
use std::convert::TryFrom;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Value {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

impl Value {
    pub fn value_type(&self) -> ValueType {
        match self {
            Value::I32(_) => ValueType::I32,
            Value::I64(_) => ValueType::I64,
            Value::F32(_) => ValueType::F32,
            Value::F64(_) => ValueType::F64,
        }
    }
}

pub enum ValueConversionError {
    InvalidType(String),
}

impl TryFrom<Value> for i32 {
    type Error = ValueConversionError;
    fn try_from(input: Value) -> Result<i32, ValueConversionError> {
        match input {
            Value::I32(val) => Ok(val),
            _ => Err(ValueConversionError::InvalidType("i32".to_string())),
        }
    }
}

impl TryFrom<Value> for i64 {
    type Error = ValueConversionError;
    fn try_from(input: Value) -> Result<i64, ValueConversionError> {
        match input {
            Value::I64(val) => Ok(val),
            _ => Err(ValueConversionError::InvalidType("i64".to_string())),
        }
    }
}
