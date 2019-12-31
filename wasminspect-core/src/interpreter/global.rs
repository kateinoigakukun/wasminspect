use super::value::Value;
use parity_wasm::elements::{GlobalType, ValueType};

pub enum GlobalInstance {
    Defined(DefinedGlobalInstance),
    External(ExternalGlobalInstance),
}

impl GlobalInstance {
    pub fn value(&self) -> Value {
        match self {
            GlobalInstance::Defined(defined) => defined.value(),
            GlobalInstance::External(_) => unimplemented!(),
        }
    }
}

pub struct DefinedGlobalInstance {
    ty: GlobalType,
    value: Value,
}

impl DefinedGlobalInstance {
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
        self.ty.is_mutable()
    }
}

pub struct ExternalGlobalInstance {
    module_name: String,
    name: String,
    ty: GlobalType,
}

impl ExternalGlobalInstance {
    pub fn new(module_name: String, name: String, ty: GlobalType) -> Self {
        Self {
            module_name,
            name,
            ty,
        }
    }
}