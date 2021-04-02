use super::value::Value;
use wasmparser::GlobalType;

pub trait GlobalInstance {
    fn value(&self) -> Value;
    fn set_value(&mut self, value: Value);
    fn is_mutable(&self) -> bool;
    fn ty(&self) -> &GlobalType;
}

pub struct DefaultGlobalInstance {
    ty: GlobalType,
    value: Value,
}

impl DefaultGlobalInstance {
    pub fn new(value: Value, ty: GlobalType) -> Self {
        Self { value, ty }
    }
}

impl GlobalInstance for DefaultGlobalInstance {
    fn value(&self) -> Value {
        self.value
    }

    fn set_value(&mut self, value: Value) {
        assert!(self.is_mutable());
        self.value = value
    }

    fn is_mutable(&self) -> bool {
        self.ty.mutable
    }

    fn ty(&self) -> &GlobalType {
        &self.ty
    }
}
