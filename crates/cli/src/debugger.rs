use super::commands::debugger;
use wasminspect_core::vm::{ModuleIndex, WasmInstance, WasmValue};
use wasminspect_wasi::instantiate_wasi;

pub struct MainDebugger {
    instance: WasmInstance,
    module_index: Option<ModuleIndex>,
}

impl MainDebugger {
    pub fn new(file: Option<String>) -> Result<Self, String> {
        let mut instance = WasmInstance::new();
        let (ctx, wasi_snapshot_preview) = instantiate_wasi();
        instance.add_embed_context(ctx);
        instance.load_host_module("wasi_unstable".to_string(), wasi_snapshot_preview);
        let module_index = if let Some(file) = file {
            Some(
                instance
                    .load_module_from_file(None, file)
                    .map_err(|err| format!("{}", err))?,
            )
        } else {
            None
        };
        Ok(Self {
            instance,
            module_index,
        })
    }
}

impl debugger::Debugger for MainDebugger {
    fn run(&mut self, name: Option<String>) -> Result<Vec<WasmValue>, String> {
        if let Some(module_index) = self.module_index {
            self.instance
                .run(module_index, name, vec![])
                .map_err(|err| format!("{}", err))
        } else {
            Err("No module loaded".to_string())
        }
    }
}
