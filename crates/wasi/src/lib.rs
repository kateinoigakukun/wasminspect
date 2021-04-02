use std::cell::RefCell;
use std::collections::HashMap;
use wasi_common::{WasiCtx, WasiCtxBuilder};
use wasminspect_vm::*;

pub struct WasiContext {
    ctx: RefCell<WasiCtx>,
}

pub fn instantiate_wasi() -> (WasiContext, HashMap<String, HostValue>) {
    let mut builder = WasiCtxBuilder::new();
    let wasi_ctx = builder.inherit_stdio().build().unwrap();
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
