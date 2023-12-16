use cap_std::fs::Dir;
use std::cell::RefCell;
use std::collections::HashMap;
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_common::WasiCtx;
use wasminspect_vm::*;
use wasmparser::{FuncType, ValType};
mod borrow;

pub struct WasiContext {
    ctx: RefCell<WasiCtx>,
}

#[derive(Debug)]
struct WasiError(std::string::String);
impl std::error::Error for WasiError {}
impl std::fmt::Display for WasiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

fn wasi_proc_exit(status: i32) -> Result<(), Trap> {
    std::process::exit(status);
}

pub fn instantiate_wasi(
    args: &[String],
    preopen_dirs: Vec<(String, Dir)>,
    envs: &[(String, String)],
) -> anyhow::Result<(WasiContext, HashMap<String, HostValue>)> {
    let builder = WasiCtxBuilder::new();
    let mut builder = builder.inherit_stdio().args(args)?.envs(envs)?;

    for (name, dir) in preopen_dirs.into_iter() {
        builder = builder.preopened_dir(dir, name)?;
    }

    let wasi_ctx = builder.build()?;

    let mut module: HashMap<String, HostValue> = HashMap::new();

    wasminspect_wasi_macro::define_wasi_fn_for_wasminspect!(
        module,
        "phases/snapshot/witx/wasi_snapshot_preview1.witx"
    );

    module.insert(
        "sock_accept".to_string(),
        HostValue::Func(HostFuncBody::new(
            FuncType::new(vec![ValType::I32, ValType::I32, ValType::I32], vec![ValType::I32]),
            move |_, _, _, _| {
                Err(Trap::HostFunctionError(Box::new(WasiError(
                    "sock_accept is not supported".to_string(),
                ))))
            }
        )),
    );

    let context = WasiContext {
        ctx: RefCell::new(wasi_ctx),
    };
    Ok((context, module))
}
