mod address;
mod executor;
mod export;
mod func;
mod global;
mod host;
mod memory;
mod module;
mod stack;
mod store;
mod table;
mod validator;
mod value;
mod utils;

use self::executor::{invoke_func, Signal, Executor, WasmError};
use self::func::{FunctionInstance, InstIndex};
use self::module::ModuleInstance;
use self::stack::{CallFrame, ProgramCounter};
use self::store::Store;

pub use self::address::*;
pub use self::host::{HostFuncBody, HostValue};
pub use self::memory::DefinedMemoryInstance as HostMemory;
pub use self::module::ModuleIndex;
pub use self::table::DefinedTableInstance as HostTable;
pub use self::value::Value as WasmValue;
use std::collections::HashMap;
use std::fmt;

enum Either<L, R> {
    Left(L),
    Right(R),
}

pub struct WasmInstance {
    store: Store,
}

impl WasmInstance {
    pub fn load_module_from_file(
        &mut self,
        name: Option<String>,
        module_filename: String,
    ) -> ModuleIndex {
        let parity_module = parity_wasm::deserialize_file(module_filename).unwrap();
        self.load_module_from_parity_module(name, parity_module)
    }

    pub fn load_module_from_parity_module(
        &mut self,
        name: Option<String>,
        parity_module: parity_wasm::elements::Module,
    ) -> ModuleIndex {
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

    pub fn get_global(&self, module_index: ModuleIndex, field: &str) -> Option<WasmValue> {
        self.store
            .scan_global_by_name(module_index, field)
            .map(|g| g.value(&self.store))
    }

    pub fn run(
        &mut self,
        module_index: ModuleIndex,
        func_name: Option<String>,
        arguments: Vec<WasmValue>,
    ) -> Result<Vec<WasmValue>, WasmError> {
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
