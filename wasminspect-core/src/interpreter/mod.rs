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
mod external;

use self::executor::{ExecSuccess, Executor};
use self::export::ExternalValue;
use self::func::InstIndex;
use self::module::ModuleIndex;
use self::stack::ProgramCounter;
use self::store::Store;
pub use self::value::Value as WasmValue;

use std::cell::RefCell;
use std::rc::Rc;

pub struct WasmInstance {
    store: Store,
    module_index: ModuleIndex,
}

impl WasmInstance {
    pub fn new(module_filename: String) -> Self {
        let parity_module = parity_wasm::deserialize_file(module_filename).unwrap();
        Self::new_from_parity_module(parity_module)
    }

    pub fn new_from_parity_module(parity_module: parity_wasm::elements::Module) -> Self {
        let mut store = Store::new();
        let module_index = store.load_parity_module(parity_module);
        Self {
            store: store,
            module_index,
        }
    }

    pub fn run(
        &mut self,
        func_name: Option<String>,
        arguments: Vec<WasmValue>,
    ) -> Result<Vec<WasmValue>, WasmError> {
        let module = self.store.module(self.module_index);
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

        let (ret_types, local_len) = {
            let func = self.store.func(pc.func_addr()).defined().unwrap();
            let ret_types = func.ty().return_type().map(|ty| vec![ty]).unwrap_or(vec![]);
            let local_len = func.ty().params().len() + func.code().locals().len();
            (ret_types, local_len)
        };
        let mut executor = Executor::new(
            local_len,
            pc.func_addr(),
            arguments,
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
