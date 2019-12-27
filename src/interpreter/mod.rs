mod environment;
mod executor;
mod module;

use self::environment::Environment;
use self::executor::{Executor, ProgramCounter};
use self::module::{DefinedModule, Index, Module, Value};

pub fn read_and_run_module(module_filename: String) {
    let env = &mut Environment::new();
    let module = parity_wasm::deserialize_file(module_filename).unwrap();
    let module = DefinedModule::read_from_parity_wasm(module, env);
    let pc = if let Some(start_func_index) = module.start_func_index() {
        ProgramCounter::new(start_func_index, Index::zero())
    } else {
        ProgramCounter::new(Index::zero(), Index::zero())
    };
    env.load_module(Module::Defined(module));
    let mut executor = Executor::new(vec![Value::I32(1), Value::I32(2)], pc, env);
    let mut result = Ok(());
    while let Ok(_) = result {
        result = executor.execute_step();
    }
}
