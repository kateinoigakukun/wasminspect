use crate::address::*;
use crate::export::{ExportInstance, ExternalValue};

use std::collections::HashMap;
use std::hash::Hash;

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
                .map(|e| ExportInstance::new_from_entry(*e, module_index))
                .collect(),
            start_func,
        }
    }

    pub fn exported_by_name(&self, name: &str) -> Option<&ExportInstance> {
        self.exports.iter().find(|e| *e.name() == name)
    }

    pub fn exported_global(&self, name: &str) -> DefinedModuleResult<Option<GlobalAddr>> {
        let export = self.exported_by_name(name);
        match export {
            Some(e) => match e.value() {
                ExternalValue::Global(addr) => Ok(Some(*addr)),
                _ => Err(DefinedModuleError::TypeMismatch(
                    "global",
                    e.value().type_name().to_string(),
                )),
            },
            None => Ok(None),
        }
    }

    pub fn exported_func(&self, name: &str) -> DefinedModuleResult<Option<FuncAddr>> {
        let export = self.exported_by_name(name);
        match export {
            Some(e) => match e.value() {
                ExternalValue::Func(addr) => Ok(Some(*addr)),
                _ => Err(DefinedModuleError::TypeMismatch(
                    "function",
                    e.value().type_name().to_string(),
                )),
            },
            None => Ok(None),
        }
    }

    pub fn exported_table(&self, name: &str) -> DefinedModuleResult<Option<TableAddr>> {
        let export = self.exported_by_name(name);
        match export {
            Some(e) => match e.value() {
                ExternalValue::Table(addr) => Ok(Some(*addr)),
                _ => Err(DefinedModuleError::TypeMismatch(
                    "table",
                    e.value().type_name().to_string(),
                )),
            },
            None => Ok(None),
        }
    }

    pub fn exported_memory(&self, name: &str) -> DefinedModuleResult<Option<MemoryAddr>> {
        let export = self.exported_by_name(name);
        match export {
            Some(e) => match e.value() {
                ExternalValue::Memory(addr) => Ok(Some(*addr)),
                _ => Err(DefinedModuleError::TypeMismatch(
                    "memory",
                    e.value().type_name().to_string(),
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

pub struct HostModuleInstance {
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
    pub(crate) fn type_name(&self) -> &str {
        match self {
            Self::Func(_) => "function",
            Self::Global(_) => "global",
            Self::Mem(_) => "memory",
            Self::Table(_) => "table",
        }
    }
}
impl HostModuleInstance {
    pub fn new(values: HashMap<String, HostExport>) -> Self {
        Self { values }
    }
}

impl HostModuleInstance {
    pub(crate) fn global_by_name(
        &self,
        name: String,
    ) -> HostModuleResult<Option<&ResolvedGlobalAddr>> {
        match &self.values.get(&name) {
            Some(HostExport::Global(global)) => Ok(Some(global)),
            Some(v) => Err(HostModuleError::TypeMismatch(
                "global",
                v.type_name().to_string(),
            )),
            _ => Ok(None),
        }
    }
    pub(crate) fn func_by_name(
        &self,
        name: String,
    ) -> HostModuleResult<Option<&ExecutableFuncAddr>> {
        match self.values.get(&name) {
            Some(HostExport::Func(ref func)) => Ok(Some(func)),
            Some(v) => Err(HostModuleError::TypeMismatch(
                "function",
                v.type_name().to_string(),
            )),
            _ => Ok(None),
        }
    }

    pub(crate) fn table_by_name(
        &self,
        name: String,
    ) -> HostModuleResult<Option<&ResolvedTableAddr>> {
        match &self.values.get(&name) {
            Some(HostExport::Table(table)) => Ok(Some(table)),
            Some(v) => Err(HostModuleError::TypeMismatch(
                "table",
                v.type_name().to_string(),
            )),
            _ => Ok(None),
        }
    }

    pub(crate) fn memory_by_name(
        &self,
        name: String,
    ) -> HostModuleResult<Option<&ResolvedMemoryAddr>> {
        match &self.values.get(&name) {
            Some(HostExport::Mem(mem)) => Ok(Some(mem)),
            Some(v) => Err(HostModuleError::TypeMismatch(
                "memory",
                v.type_name().to_string(),
            )),
            _ => Ok(None),
        }
    }
}
