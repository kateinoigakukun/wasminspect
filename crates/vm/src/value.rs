use wasmparser::Type;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Value {
    I32(i32),
    I64(i64),
    F32(u32),
    F64(u64),
}

impl Value {
    pub fn value_type(&self) -> Type {
        match self {
            Value::I32(_) => Type::I32,
            Value::I64(_) => Type::I64,
            Value::F32(_) => Type::F32,
            Value::F64(_) => Type::F64,
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
            Value::F32(v) => Some(f32::from_bits(v)),
            _ => None,
        }
    }

    pub fn as_f64(self) -> Option<f64> {
        match self {
            Value::F64(v) => Some(f64::from_bits(v)),
            _ => None,
        }
    }
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
        Self::F32(val.to_bits())
    }
}

impl From<f64> for Value {
    fn from(val: f64) -> Self {
        Self::F64(val.to_bits())
    }
}

pub trait NativeValue: Sized {
    fn from_value(val: Value) -> Option<Self>;
    fn value_type() -> Type;
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

            fn value_type() -> Type {
                Type::$case
            }
        }
    };
}

impl_native_value!(i32, I32);
impl_native_value!(i64, I64);
impl_native_value!(u32, I32);
impl_native_value!(u64, I64);

impl NativeValue for f32 {
    fn from_value(val: Value) -> Option<Self> {
        match val {
            Value::F32(val) => Some(f32::from_bits(val)),
            _ => None,
        }
    }

    fn value_type() -> Type {
        Type::F32
    }
}

impl NativeValue for f64 {
    fn from_value(val: Value) -> Option<Self> {
        match val {
            Value::F64(val) => Some(f64::from_bits(val)),
            _ => None,
        }
    }

    fn value_type() -> Type {
        Type::F64
    }
}

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
pub enum I32 {}
pub enum I64 {}
pub enum U32 {}
pub enum U64 {}

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

macro_rules! impl_trunc {
    ($type:ty, $orig:ty) => {
        impl $type {
            pub fn trunc_to_i32(self_float: $orig) -> Result<i32, Error> {
                if self_float.is_nan() {
                    Err(Error::InvalidConversionToInt)
                } else if !<$type>::in_range_i32(self_float.to_bits()) {
                    Err(Error::IntegerOverflow)
                } else {
                    Ok(self_float.trunc() as i32)
                }
            }

            pub fn trunc_to_i64(self_float: $orig) -> Result<i64, Error> {
                if self_float.is_nan() {
                    Err(Error::InvalidConversionToInt)
                } else if !<$type>::in_range_i64(self_float.to_bits()) {
                    Err(Error::IntegerOverflow)
                } else {
                    Ok(self_float.trunc() as i64)
                }
            }

            pub fn trunc_to_u32(self_float: $orig) -> Result<u32, Error> {
                if self_float.is_nan() {
                    Err(Error::InvalidConversionToInt)
                } else if !<$type>::in_range_u32(self_float.to_bits()) {
                    Err(Error::IntegerOverflow)
                } else {
                    Ok(self_float.trunc() as u32)
                }
            }

            pub fn trunc_to_u64(self_float: $orig) -> Result<u64, Error> {
                if self_float.is_nan() {
                    Err(Error::InvalidConversionToInt)
                } else if !<$type>::in_range_u64(self_float.to_bits()) {
                    Err(Error::IntegerOverflow)
                } else {
                    Ok(self_float.trunc() as u64)
                }
            }
        }
    };
}

impl F32 {
    const NEGATIVE_ZERO: u32 = 0x80000000u32;
    const NEGATIVE_ONE: u32 = 0xbf800000u32;
    fn in_range_i32(bits: u32) -> bool {
        return (bits < 0x4f000000u32) || (bits >= Self::NEGATIVE_ZERO && bits <= 0xcf000000u32);
    }

    fn in_range_i64(bits: u32) -> bool {
        return (bits < 0x5f000000u32) || (bits >= Self::NEGATIVE_ZERO && bits <= 0xdf000000u32);
    }

    fn in_range_u32(bits: u32) -> bool {
        return (bits < 0x4f800000u32)
            || (bits >= Self::NEGATIVE_ZERO && bits < Self::NEGATIVE_ONE);
    }

    fn in_range_u64(bits: u32) -> bool {
        return (bits < 0x5f800000u32)
            || (bits >= Self::NEGATIVE_ZERO && bits < Self::NEGATIVE_ONE);
    }
}

impl F64 {
    const NEGATIVE_ZERO: u64 = 0x8000000000000000u64;
    const NEGATIVE_ONE: u64 = 0xbff0000000000000u64;
    fn in_range_i32(bits: u64) -> bool {
        return (bits <= 0x41dfffffffc00000u64)
            || (bits >= Self::NEGATIVE_ZERO && bits <= 0xc1e0000000000000u64);
    }

    fn in_range_i64(bits: u64) -> bool {
        return (bits < 0x43e0000000000000u64)
            || (bits >= Self::NEGATIVE_ZERO && bits <= 0xc3e0000000000000u64);
    }

    fn in_range_u32(bits: u64) -> bool {
        return (bits <= 0x41efffffffe00000u64)
            || (bits >= Self::NEGATIVE_ZERO && bits < Self::NEGATIVE_ONE);
    }

    fn in_range_u64(bits: u64) -> bool {
        return (bits < 0x43f0000000000000u64)
            || (bits >= Self::NEGATIVE_ZERO && bits < Self::NEGATIVE_ONE);
    }
}

impl_trunc!(F32, f32);
impl_trunc!(F64, f64);

#[derive(Debug)]
pub enum Error {
    ZeroDivision,
    InvalidConversionToInt,
    IntegerOverflow,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroDivision => write!(f, "integer divide by zero"),
            Self::InvalidConversionToInt => write!(f, "invalid conversion to integer"),
            Self::IntegerOverflow => write!(f, "integer overflow"),
        }
    }
}

macro_rules! impl_try_wrapping {
    ($type:ty, $orig:ty) => {
        impl $type {
            pub fn try_wrapping_div(this: $orig, another: $orig) -> Result<$orig, Error> {
                if another == 0 {
                    Err(Error::ZeroDivision)
                } else {
                    let (result, overflow) = this.overflowing_div(another);
                    if overflow {
                        Err(Error::IntegerOverflow)
                    } else {
                        Ok(result)
                    }
                }
            }

            pub fn try_wrapping_rem(this: $orig, another: $orig) -> Result<$orig, Error> {
                if another == 0 {
                    Err(Error::ZeroDivision)
                } else {
                    Ok(this.wrapping_rem(another))
                }
            }
        }
    };
}

impl_try_wrapping!(I32, i32);
impl_try_wrapping!(I64, i64);
impl_try_wrapping!(U32, u32);
impl_try_wrapping!(U64, u64);

impl F32 {
    fn arithmetic_bits() -> u32 {
        0x00400000
    }
}

impl F64 {
    fn arithmetic_bits() -> u64 {
        0x0008000000000000
    }
}

macro_rules! impl_min_max {
    ($type:ty, $orig:ty) => {
        impl $type {
            pub fn min(this: $orig, another: $orig) -> $orig {
                if this.is_nan() {
                    let bits = this.to_bits() | <$type>::arithmetic_bits();
                    return <$orig>::from_bits(bits);
                }

                if another.is_nan() {
                    let bits = another.to_bits() | <$type>::arithmetic_bits();
                    return <$orig>::from_bits(bits);
                }
                if another.is_nan() {
                    return another;
                }
                // min(0.0, -0.0) returns 0.0 in rust, but wasm expects
                // to return -0.0
                // spec: https://webassembly.github.io/spec/core/exec/numerics.html#op-fmin
                if this == another {
                    return <$orig>::from_bits(this.to_bits() | another.to_bits());
                }
                return this.min(another);
            }

            pub fn max(this: $orig, another: $orig) -> $orig {
                if this.is_nan() {
                    let bits = this.to_bits() | <$type>::arithmetic_bits();
                    return <$orig>::from_bits(bits);
                }

                if another.is_nan() {
                    let bits = another.to_bits() | <$type>::arithmetic_bits();
                    return <$orig>::from_bits(bits);
                }
                // max(-0.0, 0.0) returns -0.0 in rust, but wasm expects
                // to return 0.0
                // spec: https://webassembly.github.io/spec/core/exec/numerics.html#op-fmax
                if this == another {
                    return <$orig>::from_bits(this.to_bits() & another.to_bits());
                }
                return this.max(another);
            }
        }
    };
}

impl_min_max!(F32, f32);
impl_min_max!(F64, f64);

macro_rules! impl_nearest {
    ($type:ty, $orig:ty) => {
        impl $type {
            pub fn nearest(this: $orig) -> $orig {
                let round = this.round();
                if this.fract().abs() != 0.5 {
                    return round;
                }

                use core::ops::Rem;
                if round.rem(2.0) == 1.0 {
                    this.floor()
                } else if round.rem(2.0) == -1.0 {
                    this.ceil()
                } else {
                    round
                }
            }
        }
    };
}

impl_nearest!(F32, f32);
impl_nearest!(F64, f64);
