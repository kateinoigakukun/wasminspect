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

macro_rules! primitive_conversion {
    ($case:path, $type:ty) => {
        impl TryFrom<Value> for $type {
            type Error = ValueConversionError;
            fn try_from(input: Value) -> Result<$type, ValueConversionError> {
                match input {
                    $case(val) => Ok(val),
                    _ => Err(ValueConversionError::InvalidType("$type".to_string())),
                }
            }
        }

        impl Into<Value> for $type {
            fn into(self) -> Value {
                $case(self)
            }
        }
    };
}

primitive_conversion!(Value::I32, i32);
primitive_conversion!(Value::I64, i64);
primitive_conversion!(Value::F32, f32);
primitive_conversion!(Value::F64, f64);

pub trait IntoLittleEndian {
    fn into_le(self, buf: &mut [u8]);
}

pub trait FromLittleEndian {
    fn from_le(buf: &[u8]) -> Self;
}

macro_rules! little_endian_conversion {
    ($type:ty, $size:expr) => {
        impl IntoLittleEndian for $type {
            fn into_le(self, buf: &mut [u8]) {
                buf.copy_from_slice(&self.to_le_bytes());
            }
        }

        impl FromLittleEndian for $type {
            fn from_le(buf: &[u8]) -> Self {
                let mut b: [u8; $size] = Default::default();
                b.copy_from_slice(&buf[0..$size]);
                Self::from_le_bytes(b)
            }
        }
    };
}

little_endian_conversion!(i32, 4);
little_endian_conversion!(i64, 8);
