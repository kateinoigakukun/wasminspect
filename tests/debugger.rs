extern crate wasminspect_debugger;
extern crate wasminspect_vm;
use std::{collections::HashMap, io::Read};
use wasminspect_debugger::*;
use wasminspect_vm::*;
use wast_spec::instantiate_spectest;

fn load_file(filename: &str) -> anyhow::Result<Vec<u8>> {
    let mut f = ::std::fs::File::open(filename)?;
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)?;
    Ok(buffer)
}

#[test]
fn test_load_and_execute() -> anyhow::Result<()> {
    let (mut process, _) = start_debugger(None, vec![], vec![])?;
    let example_dir = std::path::Path::new(file!())
        .parent()
        .unwrap()
        .join("simple-example");
    let bytes = load_file(example_dir.join("calc.wasm").to_str().unwrap())?;
    let spectest = instantiate_spectest();
    let mut host_modules = HashMap::new();
    let args = vec![];
    host_modules.insert("spectest".to_string(), spectest);
    process
        .debugger
        .load_main_module(&bytes, String::from("calc.wasm"))?;
    process.debugger.instantiate(host_modules, Some(&args))?;
    process
        .debugger
        .run(Some("add"), vec![WasmValue::I32(1), WasmValue::I32(2)])?;
    Ok(())
}
