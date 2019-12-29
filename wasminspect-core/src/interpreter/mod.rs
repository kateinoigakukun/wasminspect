mod address;
mod executor;
mod func;
mod global;
mod module;
mod stack;
mod store;
mod value;
mod export;

use self::executor::{ExecSuccess, Executor};
use self::module::ModuleInstance;
use self::stack::ProgramCounter;
use self::store::{FuncAddr, Store};
use self::export::ExternalValue;
use self::func::InstIndex;
pub use self::value::Value as WasmValue;
// use self::module::{DefinedModule, Index, Module};

pub struct WasmInstance {
    filename: String,
}

impl WasmInstance {
    pub fn new(module_filename: String) -> Self {
        Self {
            filename: module_filename,
        }
    }

    pub fn run(
        &self,
        func_name: Option<String>,
        arguments: Vec<WasmValue>,
    ) -> Result<Vec<WasmValue>, WasmError> {
        let module = parity_wasm::deserialize_file(self.filename.clone()).unwrap();
        let mut store = Store::new();
        let module = store.load_parity_module(module);
        let pc = if let Some(func_name) = func_name {
            if let Some(export) = module.exported_func(func_name.clone()) {
                if let ExternalValue::Func(func_addr) = export.value() {
                    ProgramCounter::new(*func_addr, InstIndex::zero())
                } else {
                    return Err(WasmError::EntryFunctionNotFound(func_name.clone()));
                }
            } else {
                return Err(WasmError::EntryFunctionNotFound(func_name.clone()));
            }
        } else if let Some(start_func_addr) = module.start_func_addr() {
            ProgramCounter::new(*start_func_addr, InstIndex::zero())
        } else {
            panic!()
        };

        let func = store.func(pc.func_addr()).defined().unwrap();
        let local_len = func.ty().params().len() + func.code().locals().len();
        let mut executor = Executor::new(local_len, pc.func_addr(), arguments, pc, store);
        loop {
            let result = executor.execute_step();
            match result {
                Ok(ExecSuccess::Next) => continue,
                Ok(ExecSuccess::End) => match executor.peek_result() {
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

impl WasmError {
    pub fn message(&self) -> String {
        match self {
            WasmError::ExecutionError(err) => format!("Failed to execute: {:?}", err),
            WasmError::EntryFunctionNotFound(func_name) => {
                format!("Entry function \"{}\" not found", func_name)
            }
            WasmError::ReturnValueError(err) => format!("Failed to get returned value: {:?}", err),
        }
    }
}
