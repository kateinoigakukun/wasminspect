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

    pub fn as_i32(self) -> Option<i32> {
        match self {
            Value::I32(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_i64(self) -> Option<i64> {
        match self {
            Value::I64(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_f32(self) -> Option<f32> {
        match self {
            Value::F32(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_f64(self) -> Option<f64> {
        match self {
            Value::F64(v) => Some(v),
            _ => None,
        }
    }
}

pub enum ValueConversionError {
    InvalidType(String),
}

impl From<i32> for Value {
    fn from(val: i32) -> Self {
        Self::I32(val)
    }
}

impl From<i64> for Value {
    fn from(val: i64) -> Self {
        Self::I64(val as i64)
    }
}

impl From<u32> for Value {
    fn from(val: u32) -> Self {
        Self::I32(val as i32)
    }
}

impl From<u64> for Value {
    fn from(val: u64) -> Self {
        Self::I64(val as i64)
    }
}

impl From<f32> for Value {
    fn from(val: f32) -> Self {
        Self::F32(val)
    }
}

impl From<f64> for Value {
    fn from(val: f64) -> Self {
        Self::F64(val)
    }
}

pub trait NativeValue: Sized {
    fn from_value(val: Value) -> Option<Self>;
}

macro_rules! impl_native_value {
    ($type:ty, $case:ident) => {
        impl NativeValue for $type {
            fn from_value(val: Value) -> Option<Self> {
                match val {
                    Value::$case(val) => Some(val as $type),
                    _ => None,
                }
            }
        }
    };
}

impl_native_value!(i32, I32);
impl_native_value!(i64, I64);
impl_native_value!(u32, I32);
impl_native_value!(u64, I64);
impl_native_value!(f32, F32);
impl_native_value!(f64, F64);

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

little_endian_conversion!(u8, 1);
little_endian_conversion!(u16, 2);
little_endian_conversion!(u32, 4);
little_endian_conversion!(u64, 8);

little_endian_conversion!(i8, 1);
little_endian_conversion!(i16, 2);
little_endian_conversion!(i32, 4);
little_endian_conversion!(i64, 8);

impl IntoLittleEndian for f32 {
    fn into_le(self, buf: &mut [u8]) {
        buf.copy_from_slice(&self.to_bits().to_le_bytes());
    }
}

impl FromLittleEndian for f32 {
    fn from_le(buf: &[u8]) -> Self {
        let mut b: [u8; 4] = Default::default();
        b.copy_from_slice(&buf[0..4]);
        Self::from_bits(u32::from_le_bytes(b))
    }
}

impl IntoLittleEndian for f64 {
    fn into_le(self, buf: &mut [u8]) {
        buf.copy_from_slice(&self.to_bits().to_le_bytes());
    }
}

impl FromLittleEndian for f64 {
    fn from_le(buf: &[u8]) -> Self {
        let mut b: [u8; 8] = Default::default();
        b.copy_from_slice(&buf[0..8]);
        Self::from_bits(u64::from_le_bytes(b))
    }
}

pub trait ExtendInto<T> {
    fn extend_into(self) -> T;
}

macro_rules! extend_conversion {
    ($from:ty, $to:ty) => {
        impl ExtendInto<$to> for $from {
            fn extend_into(self) -> $to {
                self as $to
            }
        }
    };
}

extend_conversion!(u8, i32);
extend_conversion!(u16, i32);
extend_conversion!(i8, i32);
extend_conversion!(i16, i32);

extend_conversion!(u8, i64);
extend_conversion!(u16, i64);
extend_conversion!(u32, i64);
extend_conversion!(i8, i64);
extend_conversion!(i16, i64);
extend_conversion!(i32, i64);

pub enum F32 {}
pub enum F64 {}

macro_rules! impl_copysign {
    ($type:ty, $orig:ty, $size:ty) => {
        impl $type {
            pub fn copysign(lhs: $orig, rhs: $orig) -> $orig {
                let sign_mask: $size = 1 << (std::mem::size_of::<$orig>() * 8 - 1);
                let sign = rhs.to_bits() & sign_mask;
                <$orig>::from_bits((lhs.to_bits() & (!sign_mask)) | sign)
            }
        }
    };
}

impl_copysign!(F32, f32, u32);
impl_copysign!(F64, f64, u64);
