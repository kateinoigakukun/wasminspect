use super::address::*;
use super::export::{ExportInstance, ExternalValue};
use super::table::DefinedTableInstance;
use super::memory::DefinedMemoryInstance;
use super::host::*;
use super::value::Value;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct ModuleIndex(pub u32);

pub enum ModuleInstance {
    Defined(DefinedModuleInstance),
    Host(HostModuleInstance),
}

impl ModuleInstance {
    pub fn defined(&self) -> Option<&DefinedModuleInstance> {
        match self {
            ModuleInstance::Defined(defined) => Some(defined),
            _ => None,
        }
    }
}

pub struct DefinedModuleInstance {
    types: Vec<parity_wasm::elements::Type>,
    func_addrs: Vec<FuncAddr>,
    exports: Vec<ExportInstance>,
    start_func: Option<FuncAddr>,
}

impl DefinedModuleInstance {
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

    pub fn exported_by_name(&self, name: String) -> Option<&ExportInstance> {
        self.exports.iter().filter(|e| *e.name() == name).next()
    }

    pub fn exported_global(&self, name: String) -> Option<GlobalAddr> {
        let export = self.exported_by_name(name);
        export.and_then(|e| match e.value() {
            ExternalValue::Global(addr) => Some(addr.clone()),
            _ => None,
        })
    }

    pub fn exported_func(&self, name: String) -> Option<FuncAddr> {
        let export = self.exported_by_name(name);
        export.and_then(|e| match e.value() {
            ExternalValue::Func(addr) => Some(addr.clone()),
            _ => None,
        })
    }

    pub fn exported_table(&self, name: String) -> Option<TableAddr> {
        let export = self.exported_by_name(name);
        export.and_then(|e| match e.value() {
            ExternalValue::Table(addr) => Some(addr.clone()),
            _ => None,
        })
    }

    pub fn exported_memory(&self, name: String) -> Option<MemoryAddr> {
        let export = self.exported_by_name(name);
        export.and_then(|e| match e.value() {
            ExternalValue::Memory(addr) => Some(addr.clone()),
            _ => None,
        })
    }

    pub fn start_func_addr(&self) -> &Option<FuncAddr> {
        &self.start_func
    }

    pub fn get_type(&self, index: usize) -> &parity_wasm::elements::Type {
        &self.types[index]
    }
}

pub struct HostModuleInstance {
    values: HashMap<String, HostValue>,
}

impl HostModuleInstance {
    pub fn new(values: HashMap<String, HostValue>) -> Self {
        Self { values }
    }

    pub fn global_by_name(&self, name: String) -> Option<Value> {
        assert!(
            self.values.contains_key(&name),
            "Global {} was not loaded",
            name
        );
        match self.values[&name] {
            HostValue::Global(global) => Some(global.clone()),
            _ => None,
        }
    }

    pub fn func_by_name(&self, name: String) -> Option<&HostFuncBody> {
        assert!(
            self.values.contains_key(&name),
            "Func {} was not loaded",
            name
        );
        match self.values[&name] {
            HostValue::Func(ref func) => Some(func),
            _ => None,
        }
    }

    pub fn table_by_name(&self, name: String) -> Option<&Rc<RefCell<DefinedTableInstance>>> {
        assert!(
            self.values.contains_key(&name),
            "Table {} was not loaded",
            name
        );
        match &self.values[&name] {
            HostValue::Table(table) => Some(table),
            _ => None,
        }
    }

    pub fn memory_by_name(&self, name: String) -> Option<&Rc<RefCell<DefinedMemoryInstance>>> {
        assert!(
            self.values.contains_key(&name),
            "Memory {} was not loaded",
            name
        );
        match &self.values[&name] {
            HostValue::Mem(mem) => Some(mem),
            _ => None,
        }
    }
}
