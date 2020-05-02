use super::executor::{simple_invoke_func, WasmError};
use super::host::HostValue;
use super::module::ModuleIndex;
use super::store::Store;
use super::value::Value;
use std::collections::HashMap;

use anyhow::Result;
use std::io::Read;

pub struct WasmInstance {
    pub store: Store,
}

impl WasmInstance {
    pub fn load_module_from_file(
        &mut self,
        name: Option<String>,
        module_filename: String,
    ) -> Result<ModuleIndex> {
        let mut f = ::std::fs::File::open(module_filename)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        let reader = wasmparser::ModuleReader::new(&buffer)?;
        self.load_module_from_module(name, reader)
    }

    pub fn load_module_from_module(
        &mut self,
        name: Option<String>,
        reader: wasmparser::ModuleReader,
    ) -> Result<ModuleIndex> {
        let mut reader = reader;
        self.store.load_module(name, &mut reader)
    }

    pub fn load_host_module(&mut self, name: String, module: HashMap<String, HostValue>) {
        self.store.load_host_module(name, module)
    }

    pub fn register_name(&mut self, name: String, module_index: ModuleIndex) {
        self.store.register_name(name, module_index)
    }

    pub fn add_embed_context<T: std::any::Any>(&mut self, ctx: T) {
        self.store.add_embed_context(Box::new(ctx))
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
            .map(|g| g.borrow().value())
    }

    pub fn run(
        &mut self,
        module_index: ModuleIndex,
        func_name: Option<String>,
        arguments: Vec<Value>,
    ) -> Result<Vec<Value>, WasmError> {
        let module = self.store.module(module_index).defined().unwrap();
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
        simple_invoke_func(func_addr, arguments, &mut self.store)
    }
}
