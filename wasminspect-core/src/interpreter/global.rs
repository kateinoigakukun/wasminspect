use super::module::ModuleInstance;
use super::store::Store;
use super::value::Value;
use parity_wasm::elements::GlobalType;

pub enum GlobalInstance {
    Defined(DefinedGlobalInstance),
    External(ExternalGlobalInstance),
}

impl GlobalInstance {
    pub fn value(&self, store: &Store) -> Value {
        match self {
            GlobalInstance::Defined(defined) => defined.value(),
            GlobalInstance::External(external) => {
                let module = store.module_by_name(external.module_name.clone());
                match module {
                    ModuleInstance::Host(host) => {
                        host.global_by_name(external.name.clone()).unwrap()
                    }
                    ModuleInstance::Defined(defined) => {
                        let addr = defined.exported_global(external.name.clone());
                        store.global(addr.unwrap()).value(store)
                    }
                }
            }
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
