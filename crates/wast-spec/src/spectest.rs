use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasminspect_vm::*;
use wasmparser::{FuncType, GlobalType, Type};

pub fn instantiate_spectest() -> HashMap<String, HostValue> {
    let mut module = HashMap::new();
    let ty = FuncType {
        params: vec![].into_boxed_slice(),
        returns: vec![].into_boxed_slice(),
    };
    let func = HostValue::Func(HostFuncBody::new(ty, |_, _, _, _| Ok(())));
    module.insert("print".to_string(), func);

    let ty = FuncType {
        params: vec![Type::I32].into_boxed_slice(),
        returns: vec![].into_boxed_slice(),
    };
    let func = HostValue::Func(HostFuncBody::new(ty, |params, _, _, _| {
        println!("{}: i32", params[0].as_i32().unwrap());
        Ok(())
    }));
    module.insert("print_i32".to_string(), func);

    let ty = FuncType {
        params: vec![Type::I64].into_boxed_slice(),
        returns: vec![].into_boxed_slice(),
    };
    let func = HostValue::Func(HostFuncBody::new(ty, |params, _, _, _| {
        println!("{}: i64", params[0].as_i64().unwrap());
        Ok(())
    }));
    module.insert("print_i64".to_string(), func);

    let ty = FuncType {
        params: vec![Type::F32].into_boxed_slice(),
        returns: vec![].into_boxed_slice(),
    };
    let func = HostValue::Func(HostFuncBody::new(ty, |params, _, _, _| {
        println!("{}: f32", params[0].as_f32().unwrap());
        Ok(())
    }));
    module.insert("print_f32".to_string(), func);

    let ty = FuncType {
        params: vec![Type::F64].into_boxed_slice(),
        returns: vec![].into_boxed_slice(),
    };
    let func = HostValue::Func(HostFuncBody::new(ty, |params, _, _, _| {
        println!("{}: f64", params[0].as_f64().unwrap());
        Ok(())
    }));
    module.insert("print_f64".to_string(), func);

    let ty = FuncType {
        params: vec![Type::I32, Type::F32].into_boxed_slice(),
        returns: vec![].into_boxed_slice(),
    };
    let func = HostValue::Func(HostFuncBody::new(ty, |params, _, _, _| {
        println!("{}: i32", params[0].as_i32().unwrap());
        println!("{}: f32", params[1].as_f32().unwrap());
        Ok(())
    }));
    module.insert("print_i32_f32".to_string(), func);

    let ty = FuncType {
        params: vec![Type::F64, Type::F64].into_boxed_slice(),
        returns: vec![].into_boxed_slice(),
    };
    let func = HostValue::Func(HostFuncBody::new(ty, |params, _, _, _| {
        println!("{}: f64", params[0].as_f64().unwrap());
        println!("{}: f64", params[1].as_f64().unwrap());
        Ok(())
    }));
    module.insert("print_f64_f64".to_string(), func);

    let create_glbal = |value, ty| Rc::new(RefCell::new(DefaultGlobalInstance::new(value, ty)));
    module.insert(
        "global_i32".to_string(),
        HostValue::Global(create_glbal(
            WasmValue::I32(666),
            GlobalType {
                content_type: Type::I32,
                mutable: false,
            },
        )),
    );
    module.insert(
        "global_i64".to_string(),
        HostValue::Global(create_glbal(
            WasmValue::I64(666),
            GlobalType {
                content_type: Type::I64,
                mutable: false,
            },
        )),
    );
    module.insert(
        "global_f32".to_string(),
        HostValue::Global(create_glbal(
            WasmValue::F32(0x44268000),
            GlobalType {
                content_type: Type::F32,
                mutable: false,
            },
        )),
    );
    module.insert(
        "global_f64".to_string(),
        HostValue::Global(create_glbal(
            WasmValue::F64(0x4084d00000000000),
            GlobalType {
                content_type: Type::F64,
                mutable: false,
            },
        )),
    );

    let table = Rc::new(RefCell::new(HostTable::new(10, Some(20), RefType::FuncRef)));
    module.insert("table".to_string(), HostValue::Table(table));

    let mem = Rc::new(RefCell::new(HostMemory::new(1, Some(2))));
    module.insert("memory".to_string(), HostValue::Mem(mem));
    module
}
