use parity_wasm::elements::{FunctionType, ValueType};
use std::collections::HashMap;
use wasminspect_core::interpreter::*;

pub fn instantiate_spectest() -> HashMap<String, HostValue> {
    let mut module = HashMap::new();
    let ty = FunctionType::new(vec![], None);
    let func = HostValue::Func(HostFunc::new(ty, |_, _| Ok(())));
    module.insert("print".to_string(), func);

    let ty = FunctionType::new(vec![ValueType::I32], None);
    let func = HostValue::Func(HostFunc::new(ty, |params, _| {
        println!("{}: i32", params[0].as_i32().unwrap());
        Ok(())
    }));
    module.insert("print_i32".to_string(), func);

    let ty = FunctionType::new(vec![ValueType::I64], None);
    let func = HostValue::Func(HostFunc::new(ty, |params, _| {
        println!("{}: i64", params[0].as_i64().unwrap());
        Ok(())
    }));
    module.insert("print_i64".to_string(), func);

    let ty = FunctionType::new(vec![ValueType::F32], None);
    let func = HostValue::Func(HostFunc::new(ty, |params, _| {
        println!("{}: f32", params[0].as_f32().unwrap());
        Ok(())
    }));
    module.insert("print_f32".to_string(), func);

    let ty = FunctionType::new(vec![ValueType::F64], None);
    let func = HostValue::Func(HostFunc::new(ty, |params, _| {
        println!("{}: f64", params[0].as_f64().unwrap());
        Ok(())
    }));
    module.insert("print_f64".to_string(), func);

    let ty = FunctionType::new(vec![ValueType::I32, ValueType::F32], None);
    let func = HostValue::Func(HostFunc::new(ty, |params, _| {
        println!("{}: i32", params[0].as_i32().unwrap());
        println!("{}: f32", params[1].as_f32().unwrap());
        Ok(())
    }));
    module.insert("print_i32_f32".to_string(), func);

    let ty = FunctionType::new(vec![ValueType::F64, ValueType::F64], None);
    let func = HostValue::Func(HostFunc::new(ty, |params, _| {
        println!("{}: f64", params[0].as_f64().unwrap());
        println!("{}: f64", params[1].as_f64().unwrap());
        Ok(())
    }));
    module.insert("print_f64_f64".to_string(), func);

    module.insert(
        "global_i32".to_string(),
        HostValue::Global(WasmValue::I32(666)),
    );
    module.insert(
        "global_i64".to_string(),
        HostValue::Global(WasmValue::I32(666)),
    );
    module.insert(
        "global_f32".to_string(),
        HostValue::Global(WasmValue::F32(f32::from_bits(0x44268000))),
    );
    module.insert(
        "global_f64".to_string(),
        HostValue::Global(WasmValue::F64(f64::from_bits(0x4084d00000000000))),
    );

    let mut table = HostTable::new(10, Some(20));
    let init_func_addr = FuncAddr(ModuleIndex(0), 0);
    table.initialize(0, std::iter::repeat(init_func_addr).take(10).collect());
    module.insert("table".to_string(), HostValue::Table(table));

    module.insert("memory".to_string(), HostValue::Mem());
    module
}
