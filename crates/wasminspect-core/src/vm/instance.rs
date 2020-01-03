use super::executor::{invoke_func, WasmError};
use super::host::HostValue;
use super::module::ModuleIndex;
use super::store;
use super::store::Store;
use super::value::Value;
use std::collections::HashMap;

pub struct WasmInstance {
    store: Store,
}

impl WasmInstance {
    pub fn load_module_from_file(
        &mut self,
        name: Option<String>,
        module_filename: String,
    ) -> Result<ModuleIndex, store::Error> {
        let parity_module = parity_wasm::deserialize_file(module_filename).unwrap();
        self.load_module_from_parity_module(name, parity_module)
    }

    pub fn load_module_from_parity_module(
        &mut self,
        name: Option<String>,
        parity_module: parity_wasm::elements::Module,
    ) -> Result<ModuleIndex, store::Error> {
        self.store.load_parity_module(name, parity_module)
    }

    pub fn load_host_module(&mut self, name: String, module: HashMap<String, HostValue>) {
        self.store.load_host_module(name, module)
    }

    pub fn register_name(&mut self, name: String, module_index: ModuleIndex) {
        self.store.register_name(name, module_index)
    }
}

impl WasmInstance {
    pub fn new() -> Self {
        Self {
            store: Store::new(),
        }
    }

    pub fn get_global(&self, module_index: ModuleIndex, field: &str) -> Option<Value> {
        self.store
            .scan_global_by_name(module_index, field)
            .map(|g| g.value(&self.store))
    }

    pub fn run(
        &mut self,
        module_index: ModuleIndex,
        func_name: Option<String>,
        arguments: Vec<Value>,
    ) -> Result<Vec<Value>, WasmError> {
        let module = self.store.module(module_index).defined().unwrap();
        let func_addr = if let Some(func_name) = func_name {
            if let Some(func_addr) = module.exported_func(func_name.clone()) {
                func_addr
            } else {
                return Err(WasmError::EntryFunctionNotFound(func_name.clone()));
            }
        } else if let Some(start_func_addr) = module.start_func_addr() {
            *start_func_addr
        } else {
            panic!()
        };
        invoke_func(func_addr, arguments, &mut self.store)
    }
}
