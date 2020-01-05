use parity_wasm::elements::{FunctionType, GlobalType, ValueType};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasi_common::hostcalls::*;
use wasi_common::wasi::*;
use wasi_common::*;
use wasi_common::{WasiCtx, WasiCtxBuilder};
use wasminspect_core::vm::*;

pub fn instantiate_wasi() -> (WasiCtx, HashMap<String, HostValue>) {
    let builder = WasiCtxBuilder::new().inherit_stdio();
    let wasi_ctx = builder.build().unwrap();
    let mut module: HashMap<String, HostValue> = HashMap::new();

    fn define_wasi_fn<
        F: Fn(&[WasmValue], &mut [WasmValue], &mut HostContext, &mut WasiCtx) -> Result<(), Trap> + 'static,
    >(
        args_ty: Vec<ValueType>,
        ret_ty: Option<ValueType>,
        f: F,
    ) -> HostValue {
        let ty = FunctionType::new(args_ty, ret_ty);
        return HostValue::Func(HostFuncBody::new(ty, move |args, ret, ctx, store| {
            let wasi_ctx = store.get_embed_context::<WasiCtx>().unwrap();
            f(args, ret, ctx, wasi_ctx)
        }));
    }

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = args_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("args_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = args_sizes_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("args_sizes_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, _| {
            unsafe {
                let result = clock_res_get(
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("clock_res_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I64, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, _| {
            unsafe {
                let result = clock_time_get(
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u64,
                    args[2].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("clock_time_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = environ_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("environ_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = environ_sizes_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("environ_sizes_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = fd_close(
                    wasi_ctx,
                    args[0].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("fd_close".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = args_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("args_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = args_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("args_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = args_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("args_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = args_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("args_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = args_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("args_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = args_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("args_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = args_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("args_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = args_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("args_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = args_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("args_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = args_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("args_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = args_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("args_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = args_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("args_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = args_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("args_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = args_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("args_get".to_string(), func);

    let func = define_wasi_fn(
        vec![ValueType::I32, ValueType::I32],
        Some(ValueType::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = args_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret[0] = WasmValue::I32(result as i32);
            }
            Ok(())
        },
    );
    module.insert("args_get".to_string(), func);

    (wasi_ctx, module)
}
