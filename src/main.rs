use std::fs::read;
use wasmtime::*;

fn main() {
    let engine = HostRef::new(Engine::default());
    let store = HostRef::new(Store::new(&engine));

    let wasm = read("example/hello.wasm").expect("wasm file");

    let module = HostRef::new(Module::new(&store, &wasm).expect("wasm module"));
    let instance = Instance::new(&store, &module, &[]).expect("wasm instance");

    let answer = instance.find_export_by_name("answer").expect("answer").func().expect("function");
    let result = answer.borrow().call(&[]).expect("success");
    println!("Answer: {}", result[0].i32());
}
