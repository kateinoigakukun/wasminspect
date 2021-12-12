use std::ops::{BitAnd, BitOr, Not};

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

/// Runtime representation of a value
/// Spec: https://webassembly.github.io/spec/core/exec/runtime.html#values
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Value {
    /// Basic number value
    Num(NumVal),
    /// Reference value
    Ref(RefVal),
}

/// Runtime representation of a basic number value
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum NumVal {
    /// Value of 32-bit signed or unsigned integer.
    I32(i32),
    /// Value of 64-bit signed or unsigned integer.
    I64(i64),
    /// Value of 32-bit IEEE 754-2008 floating point number.
    F32(F32),
    /// Value of 64-bit IEEE 754-2008 floating point number.
    F64(F64),
}

/// A wrapper to represent f32 (32-bit IEEE 754-2008) in WebAssembly runtime, used to keep internal bit pattern.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct F32(u32);

impl F32 {
    pub fn to_bits(&self) -> u32 {
        self.0
    }
    pub fn to_float(&self) -> f32 {
        f32::from_bits(self.0)
    }
}

/// A wrapper to represent f64 (64-bit IEEE 754-2008) in WebAssembly runtime, used to keep internal bit pattern.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct F64(u64);

impl F64 {
    pub fn to_bits(&self) -> u64 {
        self.0
    }
    pub fn to_float(&self) -> f64 {
        f64::from_bits(self.0)
    }
}

/// Runtime representation of a reference type
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RefType {
    FuncRef,
    ExternRef,
}

impl From<RefType> for wasmparser::Type {
    fn from(from: RefType) -> Self {
        match from {
            RefType::FuncRef => wasmparser::Type::FuncRef,
            RefType::ExternRef => wasmparser::Type::ExternRef,
        }
    }
}

/// Runtime representation of a reference value
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RefVal {
    NullRef(RefType),
    FuncRef(crate::FuncAddr),
    ExternRef(u32),
}

impl Value {
    #[allow(non_snake_case)]
    pub fn I32(v: i32) -> Value {
        Value::Num(NumVal::I32(v))
    }
    #[allow(non_snake_case)]
    pub fn I64(v: i64) -> Value {
        Value::Num(NumVal::I64(v))
    }
    #[allow(non_snake_case)]
    pub fn F32(v: u32) -> Value {
        Value::Num(NumVal::F32(F32(v)))
    }
    #[allow(non_snake_case)]
    pub fn F64(v: u64) -> Value {
        Value::Num(NumVal::F64(F64(v)))
    }

    pub fn null_ref(ty: wasmparser::Type) -> Option<Value> {
        let r = match ty {
            wasmparser::Type::FuncRef => RefVal::NullRef(RefType::FuncRef),
            wasmparser::Type::ExternRef => RefVal::NullRef(RefType::ExternRef),
            _ => return None,
        };
        Some(Value::Ref(r))
    }

    pub fn isa(&self, ty: wasmparser::Type) -> bool {
        match self {
            Value::Num(_) => self.value_type() == ty,
            Value::Ref(r) => matches!((r, ty), (RefVal::ExternRef(_), wasmparser::Type::ExternRef)
                | (RefVal::FuncRef(_), wasmparser::Type::FuncRef)
                | (RefVal::NullRef(RefType::ExternRef), wasmparser::Type::ExternRef)
                | (RefVal::NullRef(RefType::FuncRef), wasmparser::Type::FuncRef)),
        }
    }

    pub fn value_type(&self) -> wasmparser::Type {
        match self {
            Value::Num(NumVal::I32(_)) => wasmparser::Type::I32,
            Value::Num(NumVal::I64(_)) => wasmparser::Type::I64,
            Value::Num(NumVal::F32(_)) => wasmparser::Type::F32,
            Value::Num(NumVal::F64(_)) => wasmparser::Type::F64,
            Value::Ref(RefVal::NullRef(_)) => wasmparser::Type::FuncRef,
            Value::Ref(RefVal::FuncRef(_)) => wasmparser::Type::FuncRef,
            Value::Ref(RefVal::ExternRef(_)) => wasmparser::Type::ExternRef,
        }
    }

    pub fn as_i32(self) -> Option<i32> {
        match self {
            Value::Num(NumVal::I32(v)) => Some(v),
            _ => None,
        }
    }

    pub fn as_i64(self) -> Option<i64> {
        match self {
            Value::Num(NumVal::I64(v)) => Some(v),
            _ => None,
        }
    }

    pub fn as_f32(self) -> Option<f32> {
        match self {
            Value::Num(NumVal::F32(v)) => Some(f32::from_bits(v.0)),
            _ => None,
        }
    }

    pub fn as_f64(self) -> Option<f64> {
        match self {
            Value::Num(NumVal::F64(v)) => Some(f64::from_bits(v.0)),
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

impl From<F32> for Value {
    fn from(val: F32) -> Self {
        Self::Num(NumVal::F32(val))
    }
}

impl From<F64> for Value {
    fn from(val: F64) -> Self {
        Self::Num(NumVal::F64(val))
    }
}

/// A trait to represent an inner value representation of a WebAssembly value
pub trait NativeValue: Sized {
    /// An attempted conversion from an any value to a specific type value
    fn from_value(val: Value) -> Option<Self>;
    /// A type in WebAssembly of a value of this type
    fn value_type() -> wasmparser::Type;
}

macro_rules! impl_native_value {
    ($type:ty, $case:ident) => {
        impl NativeValue for $type {
            fn from_value(val: Value) -> Option<Self> {
                match val {
                    Value::Num(NumVal::$case(val)) => Some(val as $type),
                    _ => None,
                }
            }

            fn value_type() -> wasmparser::Type {
                wasmparser::Type::$case
            }
        }
    };
}

impl_native_value!(i32, I32);
impl_native_value!(i64, I64);
impl_native_value!(u32, I32);
impl_native_value!(u64, I64);
impl_native_value!(F32, F32);
impl_native_value!(F64, F64);

/// A trait to convert a basic number value into a bytes in little-endian byte order
pub trait IntoLittleEndian {
    fn into_le_bytes(self) -> Vec<u8>;
}

impl IntoLittleEndian for i32 {
    fn into_le_bytes(self) -> Vec<u8> {
        i32::to_le_bytes(self).to_vec()
    }
}

impl IntoLittleEndian for i64 {
    fn into_le_bytes(self) -> Vec<u8> {
        i64::to_le_bytes(self).to_vec()
    }
}

impl IntoLittleEndian for F32 {
    fn into_le_bytes(self) -> Vec<u8> {
        self.0.to_le_bytes().to_vec()
    }
}

impl IntoLittleEndian for F64 {
    fn into_le_bytes(self) -> Vec<u8> {
        self.0.to_le_bytes().to_vec()
    }
}

/// A trait to convert a bytes in little-endian byte order to a basic number value
pub trait FromLittleEndian {
    fn from_le(buf: &[u8]) -> Self;
}

impl F32 {
    fn from_le_bytes(bytes: [u8; 4]) -> F32 {
        Self(u32::from_le_bytes(bytes))
    }
}

impl F64 {
    fn from_le_bytes(bytes: [u8; 8]) -> F64 {
        Self(u64::from_le_bytes(bytes))
    }
}

macro_rules! impl_from_little_endian {
    ($type:ty, $size:expr) => {
        impl FromLittleEndian for $type {
            fn from_le(buf: &[u8]) -> Self {
                let mut b: [u8; $size] = Default::default();
                b.copy_from_slice(&buf[0..$size]);
                Self::from_le_bytes(b)
            }
        }
    };
}

impl_from_little_endian!(u8, 1);
impl_from_little_endian!(u16, 2);
impl_from_little_endian!(u32, 4);
impl_from_little_endian!(u64, 8);

impl_from_little_endian!(i8, 1);
impl_from_little_endian!(i16, 2);
impl_from_little_endian!(i32, 4);
impl_from_little_endian!(i64, 8);

impl_from_little_endian!(F32, 4);
impl_from_little_endian!(F64, 8);

/// A trait to extend a basic number value into a larger size of number type.
/// `To` must be larger basic number value than `Self`.
pub trait ExtendInto<To> {
    fn extend_into(self) -> To;
}

macro_rules! impl_extend_into {
    ($from:ty, $to:ty) => {
        impl ExtendInto<$to> for $from {
            fn extend_into(self) -> $to {
                self as $to
            }
        }
    };
}

impl_extend_into!(u8, i32);
impl_extend_into!(u16, i32);
impl_extend_into!(i8, i32);
impl_extend_into!(i16, i32);

impl_extend_into!(u8, i64);
impl_extend_into!(u16, i64);
impl_extend_into!(u32, i64);
impl_extend_into!(i8, i64);
impl_extend_into!(i16, i64);
impl_extend_into!(i32, i64);

/// An attempted truncation from a basic number value into a smaller number value
pub trait TruncTo<To> {
    fn trunc_to(self) -> Result<To, Error>;
}

macro_rules! impl_trunc_to {
    ($self:ty, $to:ty) => {
        impl TruncTo<$to> for $self {
            fn trunc_to(self) -> Result<$to, Error> {
                let this = self.to_float();
                if this.is_nan() {
                    Err(Error::InvalidConversionToInt)
                } else if InRange::<$to>::in_range(<$self>::from_le_bytes(
                    this.trunc().to_le_bytes(),
                )) != InRangeResult::InRange
                {
                    Err(Error::IntegerOverflow)
                } else {
                    Ok(this.trunc() as $to)
                }
            }
        }
    };
}

impl_trunc_to!(F32, i32);
impl_trunc_to!(F32, i64);
impl_trunc_to!(F64, i32);
impl_trunc_to!(F64, i64);

impl_trunc_to!(F32, u32);
impl_trunc_to!(F32, u64);
impl_trunc_to!(F64, u32);
impl_trunc_to!(F64, u64);

/// A trait to perform saturating truncation.
/// This trait corresponds to `To_trunc_sat_Self` instruction semantics.
/// - https://webassembly.github.io/spec/core/exec/numerics.html#op-trunc-sat-u
/// - https://webassembly.github.io/spec/core/exec/numerics.html#op-trunc-sat-s
pub trait TruncSat<To> {
    fn trunc_sat(self) -> To;
}

macro_rules! impl_trunc_sat_to {
    ($self:ty, $to:ty) => {
        impl TruncSat<$to> for $self {
            fn trunc_sat(self) -> $to {
                let this = self.to_native();
                if this.is_nan() {
                    0
                } else if this == <$self as IEEE754>::NativeType::INFINITY {
                    <$to>::MAX
                } else if this == <$self as IEEE754>::NativeType::NEG_INFINITY {
                    <$to>::MIN
                } else {
                    let trunc = this.trunc();
                    if trunc < <$to>::MIN as <$self as IEEE754>::NativeType {
                        <$to>::MIN
                    } else if trunc > <$to>::MAX as <$self as IEEE754>::NativeType {
                        <$to>::MAX
                    } else {
                        trunc as $to
                    }
                }
            }
        }
    };
}

impl_trunc_sat_to!(F32, i32);
impl_trunc_sat_to!(F32, i64);
impl_trunc_sat_to!(F64, i32);
impl_trunc_sat_to!(F64, i64);

impl_trunc_sat_to!(F32, u32);
impl_trunc_sat_to!(F32, u64);
impl_trunc_sat_to!(F64, u32);
impl_trunc_sat_to!(F64, u64);

/// Check this value is in range of `Target` basic number type
trait InRange<Target> {
    fn in_range(self) -> InRangeResult;
}

#[derive(PartialEq, Eq)]
enum InRangeResult {
    /// Too large value to fit in the target basic number type
    TooLarge,
    /// Too small value to fit in the target basic number type
    TooSmall,
    /// Fit in the target basic number type
    InRange,
}

pub(crate) trait IEEE754 {
    const SIGN_BITS: usize;
    const EXP_BITS: usize;
    const FRAC_BITS: usize;
    const BIAS: Self::BitsType;

    const BITS: usize = Self::SIGN_BITS + Self::EXP_BITS + Self::FRAC_BITS;

    type BitsType;
    type NativeType;

    fn from_bits(v: Self::BitsType) -> Self;
    fn to_bits(&self) -> Self::BitsType;
    fn from_native(v: Self::NativeType) -> Self;
    fn to_native(&self) -> Self::NativeType;
    fn sign_mask() -> Self::BitsType;
}

impl IEEE754 for F32 {
    const SIGN_BITS: usize = 1;
    const EXP_BITS: usize = 8;
    const FRAC_BITS: usize = 23;
    const BIAS: u32 = 127;

    type BitsType = u32;
    type NativeType = f32;

    fn from_bits(v: u32) -> Self {
        Self(v)
    }
    fn to_bits(&self) -> Self::BitsType {
        self.0
    }
    fn from_native(v: Self::NativeType) -> Self {
        Self::from_bits(v.to_bits())
    }
    fn to_native(&self) -> Self::NativeType {
        f32::from_bits(self.0)
    }
    fn sign_mask() -> Self::BitsType {
        1 << (Self::EXP_BITS + Self::FRAC_BITS)
    }
}

impl IEEE754 for F64 {
    const SIGN_BITS: usize = 1;
    const EXP_BITS: usize = 11;
    const FRAC_BITS: usize = 52;
    const BIAS: u64 = 1023;

    type BitsType = u64;
    type NativeType = f64;

    fn to_bits(&self) -> Self::BitsType {
        self.0
    }
    fn from_bits(v: u64) -> Self {
        Self(v)
    }
    fn from_native(v: Self::NativeType) -> Self {
        Self::from_bits(v.to_bits())
    }
    fn to_native(&self) -> Self::NativeType {
        f64::from_bits(self.0)
    }
    fn sign_mask() -> Self::BitsType {
        1 << (Self::EXP_BITS + Self::FRAC_BITS)
    }
}

macro_rules! impl_in_range_signed {
    // FIXME: `target_bits` will be replaced with `<$target>::BITS` after stablized
    ($target:ty, $target_bits:expr, $self:ty) => {
        impl InRange<$target> for $self {
            fn in_range(self) -> InRangeResult {
                // -1 * 1.0 * 2^($target_bits - 1)
                let min = (1 << (<$self>::EXP_BITS + <$self>::FRAC_BITS))
                    | (($target_bits - 1 + <$self>::BIAS) << <$self>::FRAC_BITS);
                // +1 * 1.0 * 2^($target_bits - 1)
                let max_plus_one = (0 << (<$self>::EXP_BITS + <$self>::FRAC_BITS))
                    | (($target_bits - 1 + <$self>::BIAS) << <$self>::FRAC_BITS);
                if <$self>::from_bits(min).to_native() > self.to_float() {
                    InRangeResult::TooSmall
                } else if self.to_float() >= <$self>::from_bits(max_plus_one).to_native() {
                    InRangeResult::TooLarge
                } else {
                    InRangeResult::InRange
                }
            }
        }
    };
}

impl_in_range_signed!(i32, 32, F32);
impl_in_range_signed!(i32, 32, F64);
impl_in_range_signed!(i64, 64, F32);
impl_in_range_signed!(i64, 64, F64);

macro_rules! impl_in_range_unsigned {
    // FIXME: `target_bits` will be replaced with `<$target>::BITS` after stablized
    ($target:ty, $target_bits:expr, $self:ty) => {
        impl InRange<$target> for $self {
            fn in_range(self) -> InRangeResult {
                let negative_zero = 1 << (<$self>::EXP_BITS + <$self>::FRAC_BITS);
                let negative_one = 1 << (<$self>::EXP_BITS + <$self>::FRAC_BITS)
                    | (<$self>::BIAS + 0) << <$self>::FRAC_BITS;
                // +1 * 1.0 * 2^($target_bits - 1)
                let max_plus_one = (0 << (<$self>::EXP_BITS + <$self>::FRAC_BITS))
                    | (($target_bits + <$self>::BIAS) << <$self>::FRAC_BITS);
                if <$self>::from_bits(negative_zero).to_native() > self.to_float()
                    || <$self>::from_bits(negative_one).to_native() >= self.to_float()
                {
                    InRangeResult::TooSmall
                } else if self.to_float() >= <$self>::from_bits(max_plus_one).to_native() {
                    InRangeResult::TooLarge
                } else {
                    InRangeResult::InRange
                }
            }
        }
    };
}

impl_in_range_unsigned!(u32, 32, F32);
impl_in_range_unsigned!(u32, 32, F64);
impl_in_range_unsigned!(u64, 64, F32);
impl_in_range_unsigned!(u64, 64, F64);

pub(crate) enum I32 {}
pub(crate) enum I64 {}
pub(crate) enum U32 {}
pub(crate) enum U64 {}

pub(crate) trait Copysign {
    fn copysign(&self, other: Self) -> Self;
}

impl<F: IEEE754> Copysign for F
where
    F::BitsType: BitAnd,
    F::BitsType: BitOr,
    F::BitsType: Not,
    F::BitsType: Copy,
    <F::BitsType as BitAnd>::Output: Into<F::BitsType>,
    <F::BitsType as BitOr>::Output: Into<F::BitsType>,
    <F::BitsType as Not>::Output: Into<F::BitsType>,
{
    fn copysign(&self, rhs: F) -> F {
        let sign_mask = F::sign_mask();
        let sign: F::BitsType = (rhs.to_bits() & sign_mask).into();
        let result: F::BitsType = ((self.to_bits() & (!sign_mask).into()).into() | sign).into();
        F::from_bits(result)
    }
}

macro_rules! impl_try_wrapping {
    ($type:ty, $orig:ty) => {
        impl $type {
            pub(crate) fn try_wrapping_div(this: $orig, another: $orig) -> Result<$orig, Error> {
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

            pub(crate) fn try_wrapping_rem(this: $orig, another: $orig) -> Result<$orig, Error> {
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
    ($type:ty) => {
        impl $type {
            pub(crate) fn min(lhs: $type, rhs: $type) -> $type {
                let this = lhs.to_native();
                let another = rhs.to_native();
                if this.is_nan() {
                    let bits = this.to_bits() | <$type>::arithmetic_bits();
                    return <$type>::from_bits(bits);
                }

                if another.is_nan() {
                    let bits = another.to_bits() | <$type>::arithmetic_bits();
                    return <$type>::from_bits(bits);
                }
                // min(0.0, -0.0) returns 0.0 in rust, but wasm expects
                // to return -0.0
                // spec: https://webassembly.github.io/spec/core/exec/numerics.html#op-fmin
                #[allow(clippy::float_cmp)]
                if this == another {
                    return <$type>::from_bits(this.to_bits() | another.to_bits());
                }
                return <$type>::from_native(this.min(another));
            }

            pub(crate) fn max(lhs: $type, rhs: $type) -> $type {
                let this = lhs.to_native();
                let another = rhs.to_native();
                if this.is_nan() {
                    let bits = this.to_bits() | <$type>::arithmetic_bits();
                    return <$type>::from_bits(bits);
                }

                if another.is_nan() {
                    let bits = another.to_bits() | <$type>::arithmetic_bits();
                    return <$type>::from_bits(bits);
                }
                // max(-0.0, 0.0) returns -0.0 in rust, but wasm expects
                // to return 0.0
                // spec: https://webassembly.github.io/spec/core/exec/numerics.html#op-fmax
                #[allow(clippy::float_cmp)]
                if this == another {
                    return <$type>::from_bits(this.to_bits() & another.to_bits());
                }
                return <$type>::from_native(this.max(another));
            }
        }
    };
}

impl_min_max!(F32);
impl_min_max!(F64);

pub(crate) trait Nearest {
    fn nearest(&self) -> Self;
}

macro_rules! impl_nearest {
    ($type:ty) => {
        impl Nearest for $type {
            fn nearest(&self) -> Self {
                let this = self.to_float();
                let round = this.round();
                #[allow(clippy::float_cmp)]
                if this.fract().abs() != 0.5 {
                    return Self::from_native(round);
                }

                use core::ops::Rem;
                #[allow(clippy::float_cmp)]
                let result = if round.rem(2.0) == 1.0 {
                    this.floor()
                } else if round.rem(2.0) == -1.0 {
                    this.ceil()
                } else {
                    round
                };
                Self::from_native(result)
            }
        }
    };
}

impl_nearest!(F32);
impl_nearest!(F64);

impl I32 {
    pub(crate) fn extend_with_width(x: i32, to_bits: usize) -> i32 {
        let shift = 32 - to_bits;
        (x << shift) >> shift
    }
}

impl I64 {
    pub(crate) fn extend_with_width(x: i64, to_bits: usize) -> i64 {
        let shift = 64 - to_bits;
        (x << shift) >> shift
    }
}

#[cfg(test)]
mod tests {
    use crate::value::IEEE754;

    use super::F32;

    #[test]
    fn floating_value_min() {
        assert_eq!(
            F32::min(F32::from_native(0.0), F32::from_native(-0.0)).to_bits(),
            (-0.0_f32).to_bits()
        );
    }
}
