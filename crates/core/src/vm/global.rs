use super::address::GlobalAddr;
use super::module::ModuleInstance;
use super::store::Store;
use super::value::Value;
use parity_wasm::elements::GlobalType;

pub type GlobalInstance = DefinedGlobalInstance;

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

    pub fn ty(&self) -> &GlobalType {
        &self.ty
    }
}

pub struct ExternalGlobalInstance {
    pub module_name: String,
    pub name: String,
    pub ty: GlobalType,
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
