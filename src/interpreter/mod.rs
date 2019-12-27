mod environment;
mod executor;
mod module;

use self::environment::Environment;
use self::executor::{Executor, ProgramCounter};
use self::module::{DefinedModule, Index, Module, Value};
pub use self::module::Value as WasmValue;

pub struct WasmInstance {
    filename: String,
}

impl WasmInstance {
    pub fn new(module_filename: String) -> Self {
        Self { filename: module_filename}
    }

    pub fn run(&self, func_name: Option<String>, arguments: Vec<WasmValue>) {
        let env = &mut Environment::new();
        let module = parity_wasm::deserialize_file(self.filename.clone()).unwrap();
        let module = DefinedModule::read_from_parity_wasm(module, env);
        let pc = if let Some(func_name) = func_name {
            panic!("Not implemened yet")
        } else if let Some(start_func_index) = module.start_func_index() {
            ProgramCounter::new(start_func_index, Index::zero())
        } else {
            ProgramCounter::new(Index::zero(), Index::zero())
        };
        env.load_module(Module::Defined(module));
        let mut executor = Executor::new(arguments, pc, env);
        let mut result = Ok(());
        while let Ok(_) = result {
            result = executor.execute_step();
        }
    }
}