use std::cell::RefCell;
use std::collections::HashMap;
use wasi_common::wasi::wasi_snapshot_preview1::*;
use wasi_common::{WasiCtx, WasiCtxBuilder};
use wasminspect_vm::*;
use wiggle::GuestMemory;
mod def;

pub struct WasiContext {
    ctx: RefCell<WasiCtx>,
}

struct WasiMemory {
    mem: *mut u8,
    mem_size: u32,
}

unsafe impl GuestMemory for WasiMemory {
    fn base(&self) -> (*mut u8, u32) {
        return (self.mem, self.mem_size);
    }
}

struct WasiHostContext<'a>(&'a mut [u8]);

impl<'a> WasiHostContext<'a> {
    fn mem(self) -> WasiMemory {
        WasiMemory {
            mem: self.0.as_mut_ptr(),
            mem_size: self.0.len() as u32,
        }
    }
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
