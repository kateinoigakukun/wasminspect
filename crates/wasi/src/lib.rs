use std::cell::RefCell;
use std::collections::HashMap;
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_common::WasiCtx;
use wasminspect_vm::*;
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

pub fn instantiate_wasi(args: &[String]) -> (WasiContext, HashMap<String, HostValue>) {
    let builder = WasiCtxBuilder::new();
    let wasi_ctx = builder.inherit_stdio().args(args).unwrap().build().unwrap();
    let mut module: HashMap<String, HostValue> = HashMap::new();

    wasminspect_wasi_macro::define_wasi_fn_for_wasminspect!(
        module,
        "phases/snapshot/witx/wasi_snapshot_preview1.witx"
    );

    let context = WasiContext {
        ctx: RefCell::new(wasi_ctx),
    };
    (context, module)
}
