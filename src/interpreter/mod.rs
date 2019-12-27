mod environment;
mod executor;
mod module;

use self::environment::Environment;
use self::executor::{Executor, ExecResult, ProgramCounter};
pub use self::module::Value as WasmValue;
use self::module::{DefinedModule, Index, Module};

pub struct WasmInstance {
    filename: String,
}

impl WasmInstance {
    pub fn new(module_filename: String) -> Self {
        Self {
            filename: module_filename,
        }
    }

    pub fn run(&self, func_name: Option<String>, arguments: Vec<WasmValue>) -> Result<(), WasmError> {
        let env = &mut Environment::new();
        let module = parity_wasm::deserialize_file(self.filename.clone()).unwrap();
        let module = DefinedModule::read_from_parity_wasm(module, env);
        let pc = if let Some(func_name) = func_name {
            if let Some(func_index) = module.exported_func_by_name(func_name.clone()) {
                ProgramCounter::new(func_index, Index::zero())
            } else {
                return Err(WasmError::EntryFunctionNotFound(func_name.clone()));
            }
        } else if let Some(start_func_index) = module.start_func_index() {
            ProgramCounter::new(start_func_index, Index::zero())
        } else {
            ProgramCounter::new(Index::zero(), Index::zero())
        };
        env.load_module(Module::Defined(module));
        let mut executor = Executor::new(arguments, pc, env);
        let mut result = ExecResult::Ok;
        while let ExecResult::Ok = result {
            result = executor.execute_step();
        }
        return match result {
            executor::ExecResult::End => Ok(()),
            executor::ExecResult::Err(err) => {
                Err(WasmError::ExecutionError(err))
            },
            executor::ExecResult::Ok => unreachable!(),
        };
    }
}

pub enum WasmError {
    ExecutionError(executor::ExecError),
    EntryFunctionNotFound(String)
}

impl WasmError {
    pub fn message(&self) -> String {
        match self {
            WasmError::ExecutionError(err) => {
                format!("Failed to execute: {}", err.message())
            },
            WasmError::EntryFunctionNotFound(func_name) => {
                format!("Entry function \"{}\" not found", func_name)
            }
        }
    }
}