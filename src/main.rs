mod interpreter;
use interpreter::{read_and_run_module};

fn main() {
    read_and_run_module("example/calc.wasm".to_string())
}
