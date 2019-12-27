mod interpreter;
use interpreter::{read_and_run_module};

fn main() {
    read_and_run_module("example/hello.wasm".to_string())
}
