use super::value::Value;
use wasmparser::GlobalType;

pub struct GlobalInstance {
    ty: GlobalType,
    value: Value,
}

impl GlobalInstance {
    pub fn new(value: Value, ty: GlobalType) -> Self {
        Self { value, ty }
    }

    pub fn value(&self) -> Value {
        self.value
    }

    pub fn set_value(&mut self, value: Value) {
        assert!(self.is_mutable());
        self.value = value
    }

    pub fn is_mutable(&self) -> bool {
        self.ty.mutable
    }

    pub fn ty(&self) -> &GlobalType {
        &self.ty
    }
}
