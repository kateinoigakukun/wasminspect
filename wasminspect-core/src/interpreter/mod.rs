mod environment;
mod executor;
mod module;
mod func;
mod address;
mod store;
mod value;
mod stack;
mod global;

use self::stack::ProgramCounter;
use self::executor::{ExecSuccess, Executor};
pub use self::value::Value as WasmValue;
use self::module::{ModuleInstance};
use self::store::{Store, FuncAddr};
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
        // let env = &mut Environment::new();
        let module = parity_wasm::deserialize_file(self.filename.clone()).unwrap();
        let mut store = Store::new();
        store.load_parity_module(module);
        // let pc = if let Some(func_name) = func_name {
        //     if let Some(func_index) = module.exported_func_by_name(func_name.clone()) {
        //         ProgramCounter::new(func_index, Index::zero())
        //     } else {
        //         return Err(WasmError::EntryFunctionNotFound(func_name.clone()));
        //     }
        // } else if let Some(start_func_index) = module.start_func_index() {
        //     ProgramCounter::new(start_func_index, Index::zero())
        // } else {
        // let pc = ProgramCounter::new(FuncAddr, Index::zero())
        // };
        let mut executor = Executor::new(arguments, panic!(), store);
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
