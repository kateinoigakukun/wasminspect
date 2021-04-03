use super::address::*;
use super::export::{ExportInstance, ExternalValue};

use std::collections::HashMap;
use std::hash::Hash;

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct ModuleIndex(pub u32);

pub enum ModuleInstance {
    Defined(DefinedModuleInstance),
    Host(Box<dyn HostModuleInstance>),
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
    types: Vec<wasmparser::FuncType>,
    pub exports: Vec<ExportInstance>,
    start_func: Option<FuncAddr>,
}

#[derive(Debug)]
pub enum DefinedModuleError {
    TypeMismatch(&'static str, String),
}

impl std::fmt::Display for DefinedModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TypeMismatch(expected, actual) => write!(
                f,
                "incompatible import type, expected {} but actual {}",
                expected, actual
            ),
        }
    }
}

impl std::error::Error for DefinedModuleError {}

type DefinedModuleResult<T> = std::result::Result<T, DefinedModuleError>;

impl DefinedModuleInstance {
    pub fn new_from_module(
        module_index: ModuleIndex,
        types: Vec<wasmparser::FuncType>,
        exports: Vec<wasmparser::Export>,
        start_func: Option<FuncAddr>,
    ) -> Self {
        Self {
            types,
            exports: exports
                .iter()
                .map(|e| ExportInstance::new_from_entry(e.clone(), module_index))
                .collect(),
            start_func: start_func,
        }
    }

    pub fn exported_by_name(&self, name: &str) -> Option<&ExportInstance> {
        self.exports.iter().filter(|e| *e.name() == name).next()
    }

    pub fn exported_global(&self, name: &str) -> DefinedModuleResult<Option<GlobalAddr>> {
        let export = self.exported_by_name(name);
        match export {
            Some(e) => match e.value() {
                ExternalValue::Global(addr) => Ok(Some(addr.clone())),
                _ => Err(DefinedModuleError::TypeMismatch(
                    "global",
                    e.value().ty().to_string(),
                )),
            },
            None => Ok(None),
        }
    }

    pub fn exported_func(&self, name: &str) -> DefinedModuleResult<Option<FuncAddr>> {
        let export = self.exported_by_name(name);
        match export {
            Some(e) => match e.value() {
                ExternalValue::Func(addr) => Ok(Some(addr.clone())),
                _ => Err(DefinedModuleError::TypeMismatch(
                    "function",
                    e.value().ty().to_string(),
                )),
            },
            None => Ok(None),
        }
    }

    pub fn exported_table(&self, name: &str) -> DefinedModuleResult<Option<TableAddr>> {
        let export = self.exported_by_name(name);
        match export {
            Some(e) => match e.value() {
                ExternalValue::Table(addr) => Ok(Some(addr.clone())),
                _ => Err(DefinedModuleError::TypeMismatch(
                    "table",
                    e.value().ty().to_string(),
                )),
            },
            None => Ok(None),
        }
    }

    pub fn exported_memory(&self, name: &str) -> DefinedModuleResult<Option<MemoryAddr>> {
        let export = self.exported_by_name(name);
        match export {
            Some(e) => match e.value() {
                ExternalValue::Memory(addr) => Ok(Some(addr.clone())),
                _ => Err(DefinedModuleError::TypeMismatch(
                    "memory",
                    e.value().ty().to_string(),
                )),
            },
            None => Ok(None),
        }
    }

    pub fn start_func_addr(&self) -> &Option<FuncAddr> {
        &self.start_func
    }

    pub fn get_type(&self, index: usize) -> &wasmparser::FuncType {
        &self.types[index]
    }
}

pub trait HostModuleInstance {
    fn global_by_name(&self, name: String) -> HostModuleResult<Option<&ResolvedGlobalAddr>>;
    fn func_by_name(&self, name: String) -> HostModuleResult<Option<&ExecutableFuncAddr>>;
    fn table_by_name(&self, name: String) -> HostModuleResult<Option<&ResolvedTableAddr>>;
    fn memory_by_name(&self, name: String) -> HostModuleResult<Option<&ResolvedMemoryAddr>>;
}

pub struct DefaultHostModuleInstance {
    values: HashMap<String, HostExport>,
}

#[derive(Debug)]
pub enum HostModuleError {
    TypeMismatch(&'static str, String),
}

impl std::fmt::Display for HostModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TypeMismatch(expected, actual) => write!(
                f,
                "incompatible import type, expected {} but actual {}",
                expected, actual
            ),
        }
    }
}

type HostModuleResult<T> = std::result::Result<T, HostModuleError>;

pub enum HostExport {
    Func(ExecutableFuncAddr),
    Global(ResolvedGlobalAddr),
    Mem(ResolvedMemoryAddr),
    Table(ResolvedTableAddr),
}

impl HostExport {
    pub fn ty(&self) -> &str {
        match self {
            Self::Func(_) => "function",
            Self::Global(_) => "global",
            Self::Mem(_) => "memory",
            Self::Table(_) => "table",
        }
    }
}
impl DefaultHostModuleInstance {
    pub fn new(values: HashMap<String, HostExport>) -> Self {
        Self { values }
    }
}

impl HostModuleInstance for DefaultHostModuleInstance {
    fn global_by_name(&self, name: String) -> HostModuleResult<Option<&ResolvedGlobalAddr>> {
        match &self.values.get(&name) {
            Some(HostExport::Global(global)) => Ok(Some(global)),
            Some(v) => Err(HostModuleError::TypeMismatch("global", v.ty().to_string())),
            _ => Ok(None),
        }
    }
    fn func_by_name(&self, name: String) -> HostModuleResult<Option<&ExecutableFuncAddr>> {
        match self.values.get(&name) {
            Some(HostExport::Func(ref func)) => Ok(Some(func)),
            Some(v) => Err(HostModuleError::TypeMismatch(
                "function",
                v.ty().to_string(),
            )),
            _ => Ok(None),
        }
    }

    fn table_by_name(&self, name: String) -> HostModuleResult<Option<&ResolvedTableAddr>> {
        match &self.values.get(&name) {
            Some(HostExport::Table(table)) => Ok(Some(table)),
            Some(v) => Err(HostModuleError::TypeMismatch("table", v.ty().to_string())),
            _ => Ok(None),
        }
    }

    fn memory_by_name(&self, name: String) -> HostModuleResult<Option<&ResolvedMemoryAddr>> {
        match &self.values.get(&name) {
            Some(HostExport::Mem(mem)) => Ok(Some(mem)),
            Some(v) => Err(HostModuleError::TypeMismatch("memory", v.ty().to_string())),
            _ => Ok(None),
        }
    }
}
