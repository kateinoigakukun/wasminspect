use super::address::FuncAddr;
use super::executor::{invoke_func, WasmError};
use super::host::HostValue;
use super::module::ModuleIndex;
use super::store;
use super::store::Store;
use super::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct WasmInstance {
    store: Rc<RefCell<Store>>,
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
        let start_section = parity_module.start_section().clone();
        let module_index = self
            .store
            .borrow_mut()
            .load_parity_module(name, parity_module)?;
        if let Some(start_section) = start_section {
            let func_addr = FuncAddr(module_index, start_section as usize);
            // TODO: Handle result
            invoke_func(func_addr, vec![], self.store.clone())
                .map_err(store::Error::FailedEntryFunction)?;
        }
        Ok(module_index)
    }

    pub fn load_host_module(&mut self, name: String, module: HashMap<String, HostValue>) {
        self.store.borrow_mut().load_host_module(name, module)
    }

    pub fn register_name(&mut self, name: String, module_index: ModuleIndex) {
        self.store.borrow_mut().register_name(name, module_index)
    }

    pub fn add_embed_context<T: std::any::Any>(&mut self, ctx: Box<T>) {
        self.store.borrow_mut().add_embed_context(ctx)
    }
}

impl WasmInstance {
    pub fn new() -> Self {
        Self {
            store: Rc::new(RefCell::new(Store::new())),
        }
    }

    pub fn get_global(&self, module_index: ModuleIndex, field: &str) -> Option<Value> {
        self.store
            .borrow()
            .scan_global_by_name(module_index, field)
            .map(|g| g.borrow().value(&self.store.borrow()))
    }

    pub fn run(
        &mut self,
        module_index: ModuleIndex,
        func_name: Option<String>,
        arguments: Vec<Value>,
    ) -> Result<Vec<Value>, WasmError> {
        let store = self.store.borrow();
        let module = store.module(module_index).defined().unwrap();
        let func_addr = if let Some(func_name) = func_name {
            if let Some(Some(func_addr)) = module.exported_func(func_name.clone()).ok() {
                func_addr
            } else {
                return Err(WasmError::EntryFunctionNotFound(func_name.clone()));
            }
        } else if let Some(start_func_addr) = module.start_func_addr() {
            *start_func_addr
        } else {
            if let Some(Some(func_addr)) = module.exported_func("_start".to_string()).ok() {
                func_addr
            } else {
                return Err(WasmError::EntryFunctionNotFound("_start".to_string()));
            }
        };
        invoke_func(func_addr, arguments, self.store.clone())
    }
}
