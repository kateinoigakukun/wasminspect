use super::address::*;
use super::export::ExportInstance;
use super::store::*;
use std::hash::Hash;

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct ModuleIndex(pub u32);

pub struct ModuleInstance {
    types: Vec<parity_wasm::elements::Type>,
    func_addrs: Vec<FuncAddr>,
    exports: Vec<ExportInstance>,
    start_func: Option<FuncAddr>,
}

impl ModuleInstance {
    pub fn new_from_parity_module(
        module: parity_wasm::elements::Module,
        module_index: ModuleIndex,
        types: Vec<parity_wasm::elements::Type>,
        func_addrs: Vec<FuncAddr>,
    ) -> Self {
        Self {
            types,
            func_addrs,
            exports: module
                .export_section()
                .map(|sec| sec.entries().iter())
                .map(|entries| {
                    entries.map(|e| ExportInstance::new_from_parity_entry(e.clone(), module_index))
                })
                .map(|s| s.collect())
                .unwrap_or_default(),
            start_func: module
                .start_section()
                .map(|func_index| FuncAddr(module_index, func_index as usize)),
        }
    }

    pub fn exported_func(&self, name: String) -> Option<&ExportInstance> {
        self.exports.iter().filter(|e| *e.name() == name).next()
    }

    pub fn start_func_addr(&self) -> &Option<FuncAddr> {
        &self.start_func
    }
}