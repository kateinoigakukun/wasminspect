use super::module::ModuleInstance;
use super::store::Store;
use super::value::Value;
use super::address::GlobalAddr;
use parity_wasm::elements::GlobalType;

pub enum GlobalInstance {
    Defined(std::rc::Rc<std::cell::RefCell<DefinedGlobalInstance>>),
    External(ExternalGlobalInstance),
}

pub fn resolve_global_instance(
    addr: GlobalAddr,
    store: &Store,
) -> std::rc::Rc<std::cell::RefCell<DefinedGlobalInstance>> {
    let this = store.global(addr);
    match *this.borrow() {
        GlobalInstance::Defined(defined) => defined,
        GlobalInstance::External(external) => {
            let module = store.module_by_name(external.module_name.clone());
            match module {
                ModuleInstance::Defined(defined_module) => {
                    let addr = defined_module
                        .exported_global(external.name.clone())
                        .unwrap();
                    resolve_global_instance(addr, store)
                }
                ModuleInstance::Host(host_module) => *host_module
                    .global_by_name(external.name.clone()).unwrap(),
            }
        }
    }
}

impl GlobalInstance {
    pub fn value(&self, store: &Store) -> Value {
        match self {
            GlobalInstance::Defined(defined) => defined.borrow().value(),
            GlobalInstance::External(external) => {
                let module = store.module_by_name(external.module_name.clone());
                match module {
                    ModuleInstance::Host(host) => {
                        host.global_by_name(external.name.clone()).unwrap().borrow().value()
                    }
                    ModuleInstance::Defined(defined) => {
                        let addr = defined.exported_global(external.name.clone());
                        store.global(addr.unwrap()).borrow().value(store)
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

    pub fn ty(&self) -> &GlobalType {
        &self.ty
    }
}

pub struct ExternalGlobalInstance {
    pub module_name: String,
    pub name: String,
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
