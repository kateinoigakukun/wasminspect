mod environment;
mod executor;
mod module;

use self::environment::Environment;
use self::executor::{Executor, ProgramCounter};
use self::module::{DefinedModule, Index, Module};
use std::convert::TryFrom;

pub fn read_and_run_module(module_filename: String) {
    let mut env = Environment::new();
    let module = parity_wasm::deserialize_file(module_filename).unwrap();
    let module = DefinedModule::read_from_parity_wasm(&module, &mut env);
    env.load_module(Module::Defined(module));
    let pc: ProgramCounter = panic!();
    let mut executor = Executor::new(&module, pc, &env);

    executor.execute_step();
}
