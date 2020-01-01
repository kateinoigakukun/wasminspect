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

use self::executor::{ExecSuccess, Executor};
use self::func::InstIndex;
use self::module::ModuleIndex;
use self::stack::{ProgramCounter, CallFrame};
use self::store::Store;

pub use self::host::HostValue;
pub use self::memory::HostMemoryInstance;
pub use self::value::Value as WasmValue;
use std::collections::HashMap;
use std::fmt;

pub struct WasmInstance {
    store: Store,
    module_index: ModuleIndex,
}

pub struct WasmInstanceBuilder {
    store: Store,
}

impl WasmInstanceBuilder {
    pub fn load_main_module_from_file(self, module_filename: String) -> WasmInstance {
        let parity_module = parity_wasm::deserialize_file(module_filename).unwrap();
        self.load_main_module_from_parity_module(parity_module)
    }
    pub fn load_main_module_from_parity_module(self, parity_module: parity_wasm::elements::Module) -> WasmInstance {
        let mut store = self.store;
        let module_index = store.load_parity_module(None, parity_module);
        WasmInstance {
            store,
            module_index,
        }
    }
    pub fn load_host_module(mut self, name: String, module: HashMap<String, HostValue>) -> Self {
        self.store.load_host_module(name, module);
        self
    }
}

impl WasmInstance {
    pub fn new() -> WasmInstanceBuilder {
        WasmInstanceBuilder { store: Store::new() }
    }

    pub fn run(
        &mut self,
        func_name: Option<String>,
        arguments: Vec<WasmValue>,
    ) -> Result<Vec<WasmValue>, WasmError> {
        let module = self.store.module(self.module_index).defined().unwrap();
        let pc = if let Some(func_name) = func_name {
            if let Some(func_addr) = module.exported_func(func_name.clone()) {
                ProgramCounter::new(func_addr, InstIndex::zero())
            } else {
                return Err(WasmError::EntryFunctionNotFound(func_name.clone()));
            }
        } else if let Some(start_func_addr) = module.start_func_addr() {
            ProgramCounter::new(*start_func_addr, InstIndex::zero())
        } else {
            panic!()
        };

        let (frame, ret_types) = {
            let func = self.store.func(pc.func_addr()).defined().unwrap();
            let ret_types = func.ty().return_type().map(|ty| vec![ty]).unwrap_or(vec![]);
            let mut local_tys = func.ty().params().to_vec();
            local_tys.append(&mut func.code().locals().clone());
            let frame = CallFrame::new(pc.func_addr(), &local_tys, arguments, None);
            (frame, ret_types)
        };
        let mut executor = Executor::new(
            frame,
            ret_types.len(),
            pc,
            &mut self.store,
        );
        loop {
            let result = executor.execute_step();
            match result {
                Ok(ExecSuccess::Next) => continue,
                Ok(ExecSuccess::End) => match executor.pop_result(ret_types) {
                    Ok(values) => return Ok(values),
                    Err(err) => return Err(WasmError::ReturnValueError(err)),
                },
                Err(err) => return Err(WasmError::ExecutionError(err)),
            }
        }
    }
}

pub enum WasmError {
    ExecutionError(executor::ExecError),
    EntryFunctionNotFound(String),
    ReturnValueError(executor::ReturnValError),
}

impl std::fmt::Display for WasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmError::ExecutionError(err) => write!(f, "Failed to execute: {:?}", err),
            WasmError::EntryFunctionNotFound(func_name) => {
                write!(f, "Entry function \"{}\" not found", func_name)
            }
            WasmError::ReturnValueError(err) => write!(f, "Failed to get returned value: {:?}", err),
        }
    }
}
