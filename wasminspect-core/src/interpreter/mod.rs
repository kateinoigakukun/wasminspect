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

pub struct WasmInstance {
    store: Store,
}

impl WasmInstance {

    pub fn load_module_from_file(&mut self, name: Option<String>, module_filename: String) -> ModuleIndex {
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

enum Either<L, R> {
    Left(L),
    Right(R),
}

impl WasmInstance {
    pub fn new() -> Self {
        Self {
            store: Store::new(),
        }
    }

    pub fn get_global(&self, module_index: ModuleIndex, field: &str) -> Option<WasmValue> {
        self.store.scan_global_by_name(module_index, field).map(|g| g.value(&self.store))
    }

    fn resolve_func(addr: FuncAddr, store: &Store) -> Either<FuncAddr, &HostFuncBody> {
        let func = store.func(addr);
        match func {
            FunctionInstance::Defined(_) => Either::Left(addr),
            FunctionInstance::Host(func) => {
                let module = store.module_by_name(func.module_name().clone());
                match module {
                    ModuleInstance::Host(host_module) => {
                        let func = host_module.func_by_name(func.field_name().clone()).unwrap();
                        return Either::Right(func);
                    }
                    ModuleInstance::Defined(defined_module) => {
                        let addr = defined_module
                            .exported_func(func.field_name().clone())
                            .unwrap();
                        return Self::resolve_func(addr, store);
                    }
                }
            }
        }
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

        match Self::resolve_func(func_addr, &self.store) {
            Either::Right(host_func_body) => {
                let mut results = Vec::new();
                match host_func_body.call(&arguments, &mut results) {
                    Ok(_) => Ok(results),
                    Err(_) => Err(WasmError::HostExecutionError)
                }
            }
            Either::Left(func_addr) => {
                let (frame, ret_types) = {
                    let func = self.store.func(func_addr).defined().unwrap();
                    let ret_types = func.ty().return_type().map(|ty| vec![ty]).unwrap_or(vec![]);
                    let mut local_tys = func.ty().params().to_vec();
                    local_tys.append(&mut func.code().locals().clone());
                    let frame = CallFrame::new(func_addr, &local_tys, arguments, None);
                    (frame, ret_types)
                };
                let pc = ProgramCounter::new(func_addr, InstIndex::zero());
                let mut executor = Executor::new(frame, ret_types.len(), pc, &mut self.store);
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
    }
}

pub enum WasmError {
    ExecutionError(executor::ExecError),
    EntryFunctionNotFound(String),
    ReturnValueError(executor::ReturnValError),
    HostExecutionError,
}

impl std::fmt::Display for WasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmError::ExecutionError(err) => write!(f, "Failed to execute: {:?}", err),
            WasmError::EntryFunctionNotFound(func_name) => {
                write!(f, "Entry function \"{}\" not found", func_name)
            }
            WasmError::ReturnValueError(err) => {
                write!(f, "Failed to get returned value: {:?}", err)
            }
            WasmError::HostExecutionError => {
                write!(f, "Failed to execute host func")
            }
        }
    }
}
