use std::cell::RefCell;
use std::collections::HashMap;
use wasi_common::hostcalls::*;
use wasi_common::{WasiCtx, WasiCtxBuilder};
use wasminspect_vm::*;
use wasmparser::{FuncType, Type};

pub struct WasiContext {
    ctx: RefCell<WasiCtx>,
}

pub fn instantiate_wasi() -> (WasiContext, HashMap<String, HostValue>) {
    let builder = WasiCtxBuilder::new().inherit_stdio();
    let wasi_ctx = builder.build().unwrap();
    let mut module: HashMap<String, HostValue> = HashMap::new();

    fn define_wasi_fn<
        F: Fn(
                &[WasmValue],
                &mut Vec<WasmValue>,
                &mut HostContext,
                &mut WasiCtx,
            ) -> Result<(), Trap>
            + 'static,
    >(
        args_ty: Vec<Type>,
        ret_ty: Option<Type>,
        f: F,
    ) -> HostValue {
        let ty = FuncType {
            params: args_ty.into_boxed_slice(),
            returns: ret_ty
                .map(|t| vec![t])
                .unwrap_or_default()
                .into_boxed_slice(),
        };
        return HostValue::Func(HostFuncBody::new(ty, move |args, ret, ctx, store| {
            let wasi_ctx = store.get_embed_context::<WasiContext>().unwrap();
            let mut wasi_ctx = wasi_ctx.ctx.borrow_mut();
            f(args, ret, ctx, &mut *wasi_ctx)
        }));
    }

    let func = define_wasi_fn(vec![Type::I32], None, |args, _ret, _ctx, _wasi_ctx| {
        unsafe {
            proc_exit(args[0].as_i32().unwrap() as u32);
        }
        Ok(())
    });
    module.insert("proc_exit".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = args_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("args_get".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = args_sizes_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("args_sizes_get".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, _| {
            unsafe {
                let result = clock_res_get(
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("clock_res_get".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I64, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, _| {
            unsafe {
                let result = clock_time_get(
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i64().unwrap() as u64,
                    args[2].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("clock_time_get".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = environ_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("environ_get".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = environ_sizes_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("environ_sizes_get".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32],
        Some(Type::I32),
        |args, ret, _ctx, wasi_ctx| {
            unsafe {
                let result = fd_close(wasi_ctx, args[0].as_i32().unwrap() as u32);
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_close".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = fd_fdstat_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_fdstat_get".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, _ctx, wasi_ctx| {
            unsafe {
                let result = fd_fdstat_set_flags(
                    wasi_ctx,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u16,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_fdstat_set_flags".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = fd_tell(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_tell".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I64, Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = fd_seek(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i64().unwrap(),
                    args[2].as_i32().unwrap() as u8,
                    args[3].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_seek".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = fd_prestat_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_prestat_get".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = fd_prestat_dir_name(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                    args[2].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_prestat_dir_name".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32, Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = fd_read(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                    args[2].as_i32().unwrap() as u32,
                    args[3].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_read".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32, Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = fd_write(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                    args[2].as_i32().unwrap() as u32,
                    args[3].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_write".to_string(), func);

    let func = define_wasi_fn(
        vec![
            Type::I32,
            Type::I32,
            Type::I32,
            Type::I32,
            Type::I32,
            Type::I64,
            Type::I64,
            Type::I32,
            Type::I32,
        ],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = path_open(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                    args[2].as_i32().unwrap() as u32,
                    args[3].as_i32().unwrap() as u32,
                    args[4].as_i32().unwrap() as u16,
                    args[5].as_i64().unwrap() as u64,
                    args[6].as_i64().unwrap() as u64,
                    args[7].as_i32().unwrap() as u16,
                    args[8].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("path_open".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, _wasi_ctx| {
            unsafe {
                let result = random_get(
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("random_get".to_string(), func);

    let func = define_wasi_fn(vec![], Some(Type::I32), |_args, ret, _ctx, _wasi_ctx| {
        unsafe {
            let result = sched_yield();
            ret.push(WasmValue::I32(result as i32));
        }
        Ok(())
    });
    module.insert("sched_yield".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32, Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = poll_oneoff(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                    args[2].as_i32().unwrap() as u32,
                    args[3].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("poll_oneoff".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = fd_filestat_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_filestat_get".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32, Type::I32, Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = path_filestat_get(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                    args[2].as_i32().unwrap() as u32,
                    args[3].as_i32().unwrap() as u32,
                    args[4].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("path_filestat_get".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = path_create_directory(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                    args[2].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("path_create_directory".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = path_unlink_file(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                    args[2].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("path_unlink_file".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I64, Type::I64],
        Some(Type::I32),
        |args, ret, _ctx, wasi_ctx| {
            unsafe {
                let result = fd_allocate(
                    wasi_ctx,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i64().unwrap() as u64,
                    args[2].as_i64().unwrap() as u64,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_allocate".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I64, Type::I64, Type::I32],
        Some(Type::I32),
        |args, ret, _ctx, wasi_ctx| {
            unsafe {
                let result = fd_advise(
                    wasi_ctx,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i64().unwrap() as u64,
                    args[2].as_i64().unwrap() as u64,
                    args[3].as_i32().unwrap() as u8,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_advise".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32],
        Some(Type::I32),
        |args, ret, _ctx, wasi_ctx| {
            unsafe {
                let result = fd_datasync(wasi_ctx, args[0].as_i32().unwrap() as u32);
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_datasync".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32],
        Some(Type::I32),
        |args, ret, _ctx, wasi_ctx| {
            unsafe {
                let result = fd_sync(wasi_ctx, args[0].as_i32().unwrap() as u32);
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_sync".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I64, Type::I64],
        Some(Type::I32),
        |args, ret, _ctx, wasi_ctx| {
            unsafe {
                let result = fd_fdstat_set_rights(
                    wasi_ctx,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i64().unwrap() as u64,
                    args[2].as_i64().unwrap() as u64,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_fdstat_set_rights".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I64],
        Some(Type::I32),
        |args, ret, _ctx, wasi_ctx| {
            unsafe {
                let result = fd_filestat_set_size(
                    wasi_ctx,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i64().unwrap() as u64,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_filestat_set_size".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I64, Type::I64, Type::I32],
        Some(Type::I32),
        |args, ret, _ctx, wasi_ctx| {
            unsafe {
                let result = fd_filestat_set_times(
                    wasi_ctx,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i64().unwrap() as u64,
                    args[2].as_i64().unwrap() as u64,
                    args[3].as_i32().unwrap() as u16,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_filestat_set_times".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32, Type::I32, Type::I64, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = fd_pread(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                    args[2].as_i32().unwrap() as u32,
                    args[3].as_i64().unwrap() as u64,
                    args[4].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_pread".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32, Type::I32, Type::I64, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = fd_pwrite(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                    args[2].as_i32().unwrap() as u32,
                    args[3].as_i64().unwrap() as u64,
                    args[4].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_pwrite".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32, Type::I32, Type::I64, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = fd_readdir(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                    args[2].as_i32().unwrap() as u32,
                    args[3].as_i64().unwrap() as u64,
                    args[4].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_readdir".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, _ctx, wasi_ctx| {
            unsafe {
                let result = fd_renumber(
                    wasi_ctx,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("fd_renumber".to_string(), func);

    let func = define_wasi_fn(
        vec![
            Type::I32,
            Type::I32,
            Type::I32,
            Type::I32,
            Type::I64,
            Type::I64,
            Type::I32,
        ],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = path_filestat_set_times(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                    args[2].as_i32().unwrap() as u32,
                    args[3].as_i32().unwrap() as u32,
                    args[4].as_i64().unwrap() as u64,
                    args[5].as_i64().unwrap() as u64,
                    args[6].as_i32().unwrap() as u16,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("path_filestat_set_times".to_string(), func);

    let func = define_wasi_fn(
        vec![
            Type::I32,
            Type::I32,
            Type::I32,
            Type::I32,
            Type::I32,
            Type::I32,
            Type::I32,
        ],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = path_link(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                    args[2].as_i32().unwrap() as u32,
                    args[3].as_i32().unwrap() as u32,
                    args[4].as_i32().unwrap() as u32,
                    args[5].as_i32().unwrap() as u32,
                    args[6].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("path_link".to_string(), func);

    let func = define_wasi_fn(
        vec![
            Type::I32,
            Type::I32,
            Type::I32,
            Type::I32,
            Type::I32,
            Type::I32,
        ],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = path_readlink(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                    args[2].as_i32().unwrap() as u32,
                    args[3].as_i32().unwrap() as u32,
                    args[4].as_i32().unwrap() as u32,
                    args[5].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("path_readlink".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = path_remove_directory(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                    args[2].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("path_remove_directory".to_string(), func);

    let func = define_wasi_fn(
        vec![
            Type::I32,
            Type::I32,
            Type::I32,
            Type::I32,
            Type::I32,
            Type::I32,
        ],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = path_rename(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                    args[2].as_i32().unwrap() as u32,
                    args[3].as_i32().unwrap() as u32,
                    args[4].as_i32().unwrap() as u32,
                    args[5].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("path_rename".to_string(), func);

    let func = define_wasi_fn(
        vec![Type::I32, Type::I32, Type::I32, Type::I32, Type::I32],
        Some(Type::I32),
        |args, ret, ctx, wasi_ctx| {
            unsafe {
                let result = path_symlink(
                    wasi_ctx,
                    ctx.mem,
                    args[0].as_i32().unwrap() as u32,
                    args[1].as_i32().unwrap() as u32,
                    args[2].as_i32().unwrap() as u32,
                    args[3].as_i32().unwrap() as u32,
                    args[4].as_i32().unwrap() as u32,
                );
                ret.push(WasmValue::I32(result as i32));
            }
            Ok(())
        },
    );
    module.insert("path_symlink".to_string(), func);
    let context = WasiContext {
        ctx: RefCell::new(wasi_ctx),
    };
    (context, module)
}
