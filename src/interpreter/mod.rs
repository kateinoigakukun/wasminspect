mod environment;
mod executor;
mod module;

use self::environment::Environment;


fn read_module(module_filename: String) {
    let mut env = Environment::new();
    let module = parity_wasm::deserialize_file(module_filename).unwrap();
    env.load_module(&module);
}
pub fn read_and_run_module(module_filename: String) {
    read_module(module_filename);
}
 