mod environment;
mod executor;
mod module;

use self::executor::Executor;
use self::environment::Environment;
use self::module::Index;
use std::convert::TryFrom;

pub fn read_and_run_module(module_filename: String) {
    let mut env = Environment::new();
    let module = parity_wasm::deserialize_file(module_filename).unwrap();
    env.load_module(&module);
    let main_module = env.main_module();
    let mut executor = Executor::new(main_module, &env);
    executor.run_function(Index::try_from(0).unwrap());
}
 