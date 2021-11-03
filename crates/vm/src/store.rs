use crate::module::DefaultHostModuleInstance;

use super::address::*;
use super::executor::eval_const_expr;
use super::func::{DefinedFunctionInstance, FunctionInstance, HostFunctionInstance};
use super::global::{DefaultGlobalInstance, GlobalInstance};
use super::host::HostValue;
use super::linker::LinkableCollection;
use super::memory::{self, MemoryInstance};
use super::module::{self, DefinedModuleInstance, HostExport, ModuleIndex, ModuleInstance};
use super::table::{self, TableInstance};
use super::value::Value;
use anyhow::{Context, Result};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasmparser::{
    Data, DataKind, Element, ElementItem, ElementKind, FuncType, FunctionBody, Global, GlobalType,
    Import, MemoryType, NameSectionReader, TableType, Type, TypeDef,
};

/// Store
pub struct Store {
    funcs: LinkableCollection<FunctionInstance>,
    tables: LinkableCollection<Rc<RefCell<TableInstance>>>,
    mems: LinkableCollection<Rc<RefCell<MemoryInstance>>>,
    globals: LinkableCollection<Rc<RefCell<dyn GlobalInstance>>>,
    modules: Vec<ModuleInstance>,
    module_index_by_name: HashMap<String, ModuleIndex>,

    embedded_contexts: HashMap<std::any::TypeId, Box<dyn std::any::Any>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            funcs: LinkableCollection::new(),
            tables: LinkableCollection::new(),
            mems: LinkableCollection::new(),
            globals: LinkableCollection::new(),
            modules: Vec::new(),
            module_index_by_name: HashMap::new(),
            embedded_contexts: HashMap::new(),
        }
    }

    pub fn func_global(&self, addr: ExecutableFuncAddr) -> &FunctionInstance {
        self.funcs.get_global(addr)
    }

    pub fn func(&self, addr: FuncAddr) -> Option<(&FunctionInstance, ExecutableFuncAddr)> {
        self.funcs.get(addr)
    }

    pub fn global(&self, addr: GlobalAddr) -> Rc<RefCell<dyn GlobalInstance>> {
        self.globals.get(addr).unwrap().0.clone()
    }

    pub fn scan_global_by_name(
        &self,
        module_index: ModuleIndex,
        field: &str,
    ) -> Option<Rc<RefCell<dyn GlobalInstance>>> {
        let module = self.module(module_index).defined().unwrap();
        let global_addr = module.exported_global(field).ok().unwrap();
        global_addr.map(|addr| self.global(addr))
    }

    pub fn table(&self, addr: TableAddr) -> Rc<RefCell<TableInstance>> {
        self.tables.get(addr).unwrap().0.clone()
    }

    pub fn memory(&self, addr: MemoryAddr) -> Rc<RefCell<MemoryInstance>> {
        self.mems.get(addr).unwrap().0.clone()
    }

    pub fn memory_count(&self, addr: ModuleIndex) -> usize {
        self.mems.items(addr).map(|c| c.len()).unwrap_or(0)
    }

    pub fn module(&self, module_index: ModuleIndex) -> &ModuleInstance {
        &self.modules[module_index.0 as usize]
    }

    pub fn module_by_name(&self, name: String) -> &ModuleInstance {
        if let Some(index) = self.module_index_by_name.get(&name) {
            self.module(*index)
        } else {
            panic!("Module {} was not loaded", name);
        }
    }

    pub fn register_name(&mut self, name: String, module_index: ModuleIndex) {
        self.module_index_by_name.insert(name, module_index);
    }
}

impl Store {
    pub fn load_host_module(&mut self, name: String, module: HashMap<String, HostValue>) {
        let module_index = ModuleIndex(self.modules.len() as u32);
        let mut values = HashMap::new();
        for (field, entry) in module {
            match entry {
                HostValue::Func(f) => {
                    let instance =
                        HostFunctionInstance::new(f.ty().clone(), name.clone(), field.clone(), f);
                    let addr = self.funcs.push_global(FunctionInstance::Host(instance));
                    values.insert(field, HostExport::Func(addr));
                }
                HostValue::Global(g) => {
                    let addr = self.globals.push_global(g);
                    values.insert(field, HostExport::Global(addr));
                }
                HostValue::Table(t) => {
                    let addr = self.tables.push_global(t);
                    values.insert(field, HostExport::Table(addr));
                }
                HostValue::Mem(m) => {
                    let addr = self.mems.push_global(m);
                    values.insert(field, HostExport::Mem(addr));
                }
            }
        }
        let instance = DefaultHostModuleInstance::new(values);
        self.modules.push(ModuleInstance::Host(Box::new(instance)));
        self.module_index_by_name.insert(name, module_index);
    }

    pub fn add_embed_context<T: std::any::Any>(&mut self, ctx: Box<T>) {
        let type_id = std::any::TypeId::of::<T>();
        self.embedded_contexts.insert(type_id, ctx);
    }

    pub fn get_embed_context<T: std::any::Any>(&self) -> Option<&T> {
        let type_id = std::any::TypeId::of::<T>();
        self.embedded_contexts
            .get(&type_id)
            .map(|v| v.downcast_ref::<T>().unwrap())
    }
}

#[derive(Debug)]
pub enum StoreError {
    InvalidElementSegments(table::Error),
    InvalidDataSegments(memory::Error),
    InvalidHostImport(module::HostModuleError),
    InvalidImport(module::DefinedModuleError),
    UnknownType(/* type index: */ u32),
    UndefinedFunction {
        module: String,
        field: Option<String>,
    },
    UndefinedMemory {
        module: String,
        field: Option<String>,
    },
    UndefinedTable {
        module: String,
        field: Option<String>,
    },
    UndefinedGlobal {
        module: String,
        field: Option<String>,
    },
    IncompatibleImportFuncType(String, FuncType, FuncType),
    IncompatibleImportGlobalType(Type, Type),
    IncompatibleImportGlobalMutability,
    IncompatibleImportTableType,
    IncompatibleImportMemoryType,
}
impl std::error::Error for StoreError {}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidElementSegments(err) => {
                write!(f, "elements segment does not fit: {:?}", err)
            }
            Self::InvalidDataSegments(err) => write!(f, "data segment does not fit: {}", err),
            Self::InvalidHostImport(err) => write!(f, "invalid host import: {}", err),
            Self::InvalidImport(err) => write!(f, "invalid import: {}", err),
            Self::UnknownType(idx) => write!(f, "Unknown type index used: {:?}", idx),
            Self::UndefinedFunction { module, field } => write!(
                f,
                "unknown import: Undefined function \"{:?}\" in \"{}\"",
                field, module
            ),
            Self::UndefinedMemory { module, field } => write!(
                f,
                "unknown import: Undefined memory \"{:?}\" in \"{}\"",
                field, module
            ),
            Self::UndefinedTable { module, field } => write!(
                f,
                "unknown import: Undefined table \"{:?}\" in \"{}\"",
                field, module
            ),
            Self::UndefinedGlobal { module, field } => write!(
                f,
                "unknown import: Undefined global \"{:?}\" in \"{}\"",
                field, module
            ),
            Self::IncompatibleImportFuncType(name, expected, actual) => write!(
                f,
                "incompatible import type, \"{}\" expected {:?} but got {:?}",
                name, expected, actual
            ),
            Self::IncompatibleImportGlobalType(expected, actual) => write!(
                f,
                "incompatible import type, expected {:?} but got {:?}",
                expected, actual
            ),
            Self::IncompatibleImportGlobalMutability => write!(f, "incompatible import type"),
            Self::IncompatibleImportTableType => write!(f, "incompatible import type"),
            Self::IncompatibleImportMemoryType => write!(f, "incompatible import type"),
        }
    }
}

fn read_name_section(mut reader: wasmparser::NameSectionReader) -> Result<HashMap<u32, String>> {
    let mut func_names = HashMap::new();
    while !reader.eof() {
        let name = match reader.read() {
            Ok(name) => name,
            Err(_) => return Ok(func_names),
        };
        match name {
            wasmparser::Name::Module(_) => continue,
            wasmparser::Name::Function(n) => {
                let mut map = n.get_map()?;
                for _ in 0..map.get_count() {
                    let naming = map.read()?;
                    func_names.insert(naming.index, String::from(naming.name));
                }
            }
            wasmparser::Name::Local(_)
            | wasmparser::Name::Label(_)
            | wasmparser::Name::Type(_)
            | wasmparser::Name::Table(_)
            | wasmparser::Name::Memory(_)
            | wasmparser::Name::Global(_)
            | wasmparser::Name::Element(_)
            | wasmparser::Name::Data(_)
            | wasmparser::Name::Unknown { .. } => continue,
        }
    }
    Ok(func_names)
}

impl Store {
    fn load_module_internal(
        &mut self,
        name: Option<String>,
        reader: &[u8],
        module_index: ModuleIndex,
    ) -> Result<ModuleIndex> {
        let mut types = Vec::new();
        let mut elem_segs = Vec::new();
        let mut data_segs = Vec::new();
        let mut func_sigs = Vec::new();
        let mut imports = Vec::new();
        let mut exports = Vec::new();
        let mut bodies = Vec::new();
        let mut tables = Vec::new();
        let mut globals = Vec::new();
        let mut mems = Vec::new();
        let mut func_names = HashMap::new();

        let mut start_func = None;

        let mut code_section_base_offset = None;

        let parser = wasmparser::Parser::new(0);

        for payload in parser.parse_all(reader) {
            use wasmparser::Payload;
            match payload? {
                Payload::Version { .. } => {}
                Payload::TypeSection(section) => {
                    types.reserve_exact(section.get_count() as usize);
                    for entry in section {
                        match entry? {
                            TypeDef::Func(fn_ty) => types.push(fn_ty),
                            _ => panic!("module type is not supported yet"),
                        }
                    }
                }
                Payload::ElementSection(section) => {
                    elem_segs.reserve_exact(section.get_count() as usize);
                    for entry in section {
                        elem_segs.push(entry?);
                    }
                }
                Payload::DataSection(section) => {
                    data_segs.reserve_exact(section.get_count() as usize);
                    for entry in section {
                        data_segs.push(entry?);
                    }
                }
                Payload::ImportSection(section) => {
                    imports.reserve_exact(section.get_count() as usize);
                    for entry in section {
                        imports.push(entry?);
                    }
                }
                Payload::ExportSection(section) => {
                    exports.reserve_exact(section.get_count() as usize);
                    for entry in section {
                        exports.push(entry?);
                    }
                }
                Payload::FunctionSection(section) => {
                    func_sigs.reserve_exact(section.get_count() as usize);
                    for entry in section {
                        func_sigs.push(entry?);
                    }
                }
                Payload::CodeSectionStart { count, range, .. } => {
                    code_section_base_offset = Some(range.start);
                    bodies.reserve_exact(count as usize);
                }
                Payload::CodeSectionEntry(entry) => {
                    bodies.push(entry);
                }
                Payload::TableSection(section) => {
                    tables.reserve_exact(section.get_count() as usize);
                    for entry in section {
                        tables.push(entry?);
                    }
                }
                Payload::MemorySection(section) => {
                    mems.reserve_exact(section.get_count() as usize);
                    for entry in section {
                        mems.push(entry?);
                    }
                }
                Payload::GlobalSection(section) => {
                    globals.reserve_exact(section.get_count() as usize);
                    for entry in section {
                        globals.push(entry?);
                    }
                }
                Payload::StartSection { func, .. } => {
                    start_func = Some(FuncAddr::new_unsafe(module_index, func as usize));
                }
                Payload::CustomSection {
                    name,
                    data,
                    data_offset,
                    ..
                } => match name {
                    "name" => {
                        let section = NameSectionReader::new(data, data_offset)?;
                        func_names = read_name_section(section)?;
                    }
                    _ => (),
                },
                Payload::ModuleSectionEntry { .. } => {
                    panic!("nested module is not supported yet");
                }
                Payload::End => {
                    break;
                }
                _ => (),
            }
            // buf.drain(..consumed);
        }

        self.load_imports(imports, module_index, &types)?;
        self.load_globals(globals, module_index)?;
        if let Some(base_offset) = code_section_base_offset {
            self.load_functions(
                module_index,
                func_sigs,
                bodies,
                func_names,
                &types,
                base_offset,
            )?;
        }
        self.load_tables(tables, module_index, elem_segs)?;
        self.load_mems(mems, module_index, data_segs)?;

        let types = types.iter().map(|ty| ty.clone()).collect();

        let instance =
            DefinedModuleInstance::new_from_module(module_index, types, exports, start_func);
        self.modules.push(ModuleInstance::Defined(instance));
        if let Some(name) = name {
            self.module_index_by_name.insert(name, module_index);
        }

        Ok(module_index)
    }
    pub fn load_module(&mut self, name: Option<String>, reader: &[u8]) -> Result<ModuleIndex> {
        let module_index = ModuleIndex(self.modules.len() as u32);

        let result: Result<ModuleIndex> =
            self.load_module_internal(name.clone(), reader, module_index);
        match result {
            Ok(ok) => Ok(ok),
            Err(err) => {
                // If fail, cleanup states
                self.funcs.remove_module(&module_index);
                self.tables.remove_module(&module_index);
                self.mems.remove_module(&module_index);
                self.globals.remove_module(&module_index);
                let module_index = module_index.0 as usize;
                if module_index < self.modules.len() {
                    self.modules.remove(module_index);
                }
                if let Some(ref name) = name.clone() {
                    self.module_index_by_name.remove(name);
                }
                Err(err)
            }
        }
    }

    fn load_imports(
        &mut self,
        imports: Vec<Import>,
        module_index: ModuleIndex,
        types: &[FuncType],
    ) -> Result<()> {
        for import in imports {
            use wasmparser::ImportSectionEntryType::*;
            match import.ty {
                Function(type_index) => {
                    self.load_import_function(module_index, import, type_index as usize, &types)?;
                }
                Memory(memory_ty) => {
                    self.load_import_memory(module_index, import, memory_ty)?;
                }
                Table(table_ty) => {
                    self.load_import_table(module_index, import, table_ty)?;
                }
                Global(global_ty) => {
                    self.load_import_global(module_index, import, global_ty)?;
                }
                Module(_) | Instance(_) => {
                    panic!("module type is not supported yet");
                }
                Tag(_) => panic!("event type is not supported yet"),
            }
        }
        Ok(())
    }

    fn load_import_function(
        &mut self,
        module_index: ModuleIndex,
        import: Import,
        type_index: usize,
        types: &[FuncType],
    ) -> Result<()> {
        let func_ty = types
            .get(type_index)
            .ok_or(StoreError::UnknownType(type_index as u32))?
            .clone();
        let name = import
            .field
            .with_context(|| "expect non-nil field name in function import")?
            .to_string();
        let module = self.module_by_name(import.module.to_string());
        let err = || StoreError::UndefinedFunction {
            module: import.module.clone().to_string(),
            field: import.field.map(String::from),
        };
        let exec_addr = match module {
            ModuleInstance::Defined(defined) => {
                let func_addr = defined
                    .exported_func(&name)
                    .map_err(StoreError::InvalidImport)?
                    .ok_or_else(err)?;
                self.funcs.resolve(func_addr).ok_or_else(err)?.clone()
            }
            ModuleInstance::Host(host) => *host
                .func_by_name(name.clone())
                .map_err(StoreError::InvalidHostImport)?
                .ok_or_else(err)?,
        };
        let actual_func_ty = self.funcs.get_global(exec_addr).ty();
        // Validation
        if actual_func_ty != &func_ty {
            Err(StoreError::IncompatibleImportFuncType(
                name,
                func_ty,
                actual_func_ty.clone(),
            ))?;
        }
        self.funcs.link(exec_addr, module_index);
        Ok(())
    }

    fn load_import_memory(
        &mut self,
        module_index: ModuleIndex,
        import: Import,
        memory_ty: MemoryType,
    ) -> Result<()> {
        let err = || StoreError::UndefinedMemory {
            module: import.module.clone().to_string(),
            field: import.field.map(String::from),
        };
        let name = import
            .field
            .with_context(|| "expect non-nil field name in memory import")?
            .to_string();
        let module = self.module_by_name(import.module.to_string());
        let resolved_addr = match module {
            ModuleInstance::Defined(defined) => {
                let addr = defined
                    .exported_memory(&name)
                    .map_err(StoreError::InvalidImport)?
                    .ok_or(err())?
                    .clone();
                self.mems.resolve(addr).ok_or_else(err)?.clone()
            }
            ModuleInstance::Host(host) => *host
                .memory_by_name(name.clone())
                .map_err(StoreError::InvalidHostImport)?
                .ok_or(err())?,
        };

        // Validation
        {
            let memory = self.mems.get_global(resolved_addr);
            let limit_initial = memory_ty.initial;
            let limit_max = memory_ty.maximum;
            if memory.borrow().initial < limit_initial as usize {
                Err(StoreError::IncompatibleImportMemoryType)?;
            }
            match (memory.borrow().max, limit_max) {
                (Some(found), Some(expected)) => {
                    if found > expected as usize {
                        Err(StoreError::IncompatibleImportMemoryType)?;
                    }
                }
                (None, Some(_)) => Err(StoreError::IncompatibleImportMemoryType)?,
                _ => (),
            }
        }
        self.mems.link(resolved_addr, module_index);
        Ok(())
    }

    fn load_import_table(
        &mut self,
        module_index: ModuleIndex,
        import: Import,
        table_ty: TableType,
    ) -> Result<()> {
        let name = import
            .field
            .with_context(|| "expect non-nil field name in table import")?
            .to_string();
        let module = self.module_by_name(import.module.to_string());
        let err = || StoreError::UndefinedTable {
            module: import.module.clone().to_string(),
            field: import.field.map(String::from),
        };
        let resolved_addr = match module {
            ModuleInstance::Defined(defined) => {
                let addr = defined
                    .exported_table(&name)
                    .map_err(StoreError::InvalidImport)?
                    .ok_or_else(err)?;
                self.tables.resolve(addr).ok_or_else(err)?.clone()
            }
            ModuleInstance::Host(host) => host
                .table_by_name(name.clone())
                .map_err(StoreError::InvalidHostImport)?
                .ok_or_else(err)?
                .clone(),
        };
        let found = self.tables.get_global(resolved_addr);
        // Validation
        {
            if found.borrow().initial < table_ty.initial as usize {
                Err(StoreError::IncompatibleImportTableType)?;
            }
            match (found.clone().borrow().max, table_ty.maximum) {
                (Some(found), Some(expected)) => {
                    if found > expected as usize {
                        Err(StoreError::IncompatibleImportTableType)?;
                    }
                }
                (None, Some(_)) => Err(StoreError::IncompatibleImportTableType)?,
                _ => (),
            }
        }

        self.tables.link(resolved_addr, module_index);
        Ok(())
    }

    fn load_import_global(
        &mut self,
        module_index: ModuleIndex,
        import: Import,
        global_ty: GlobalType,
    ) -> Result<()> {
        let name = import
            .field
            .with_context(|| "expect non-nil field name in global import")?
            .to_string();
        let module = self.module_by_name(import.module.to_string());
        let err = || StoreError::UndefinedGlobal {
            module: import.module.clone().to_string(),
            field: import.field.map(String::from),
        };
        let resolved_addr = match module {
            ModuleInstance::Defined(defined) => {
                let addr = defined
                    .exported_global(&name)
                    .map_err(StoreError::InvalidImport)?
                    .ok_or(err())?;
                self.globals.resolve(addr).ok_or_else(err)?.clone()
            }
            ModuleInstance::Host(host) => host
                .global_by_name(name)
                .map_err(StoreError::InvalidHostImport)
                .and_then(|f| f.ok_or(err()))?
                .clone(),
        };
        // Validation
        {
            let actual_global = self.globals.get_global(resolved_addr);
            let actual_global_ty = actual_global.borrow().ty().content_type.clone();
            let expected_global_ty = global_ty.content_type.clone();
            if actual_global.borrow().is_mutable() != global_ty.mutable {
                Err(StoreError::IncompatibleImportGlobalMutability)?;
            }
            if actual_global_ty != expected_global_ty {
                Err(StoreError::IncompatibleImportGlobalType(
                    actual_global_ty,
                    expected_global_ty,
                ))?;
            }
        };
        self.globals.link(resolved_addr, module_index);
        Ok(())
    }

    fn load_functions(
        &mut self,
        module_index: ModuleIndex,
        func_sigs: Vec<u32>,
        bodies: Vec<FunctionBody>,
        names: HashMap<u32, String>,
        types: &[FuncType],
        base_offset: usize,
    ) -> Result<Vec<FuncAddr>> {
        let mut func_addrs = Vec::new();
        let imported_funcs = self.funcs.items(module_index);
        let mut index = imported_funcs.map(|items| items.len() as u32).unwrap_or(0);
        for (func_sig, body) in func_sigs.into_iter().zip(bodies) {
            let func_type = types
                .get(func_sig as usize)
                .ok_or(StoreError::UnknownType(func_sig))?
                .clone();
            let name = names.get(&index).map(|n| n.clone()).unwrap_or(format!(
                "<module #{} defined func #{}>",
                module_index.0, index
            ));
            let defined =
                DefinedFunctionInstance::new(name, func_type, module_index, body, base_offset)?;
            let instance = FunctionInstance::Defined(defined);
            let func_addr = self.funcs.push(module_index, instance);
            func_addrs.push(func_addr);
            index += 1;
        }
        Ok(func_addrs)
    }

    fn load_globals(&mut self, globals: Vec<Global>, module_index: ModuleIndex) -> Result<()> {
        for entry in globals {
            let value = eval_const_expr(&entry.init_expr, &self, module_index)?;
            let instance = DefaultGlobalInstance::new(value, entry.ty.clone());
            self.globals
                .push(module_index, Rc::new(RefCell::new(instance)));
        }
        Ok(())
    }

    fn load_tables(
        &mut self,
        tables: Vec<TableType>,
        module_index: ModuleIndex,
        element_segments: Vec<Element>,
    ) -> Result<Vec<TableAddr>> {
        let mut table_addrs = Vec::new();
        if tables.is_empty() && self.tables.is_empty(module_index) {
            return Ok(table_addrs);
        }
        for entry in tables.iter() {
            match entry.element_type {
                Type::FuncRef => {
                    let instance = TableInstance::new(
                        entry.initial as usize,
                        entry.maximum.map(|mx| mx as usize),
                    );
                    let addr = self
                        .tables
                        .push(module_index, Rc::new(RefCell::new(instance)));
                    table_addrs.push(addr);
                }
                _ => (),
            }
        }
        let tables = self.tables.items(module_index).unwrap();
        for seg in element_segments {
            match seg.kind {
                ElementKind::Active {
                    table_index,
                    init_expr,
                } => {
                    let table_addr = match tables.get(table_index as usize) {
                        Some(addr) => addr,
                        None => continue,
                    };
                    let offset = match eval_const_expr(&init_expr, self, module_index)? {
                        Value::I32(v) => v,
                        _ => panic!(),
                    };
                    let data = seg
                        .items
                        .get_items_reader()?
                        .into_iter()
                        .map(|item| match item? {
                            ElementItem::Func(index) => {
                                Ok(Some(FuncAddr::new_unsafe(module_index, index as usize)))
                            }
                            ElementItem::Expr { .. } => Ok(None),
                        })
                        .collect::<Result<Vec<Option<FuncAddr>>>>()?;
                    let table = self.tables.get_global(*table_addr);
                    table
                        .borrow_mut()
                        .initialize(offset as usize, data)
                        .map_err(StoreError::InvalidElementSegments)?;
                }
                _ => unimplemented!(),
            }
        }
        Ok(table_addrs)
    }

    fn load_mems(
        &mut self,
        mems: Vec<MemoryType>,
        module_index: ModuleIndex,
        data_segments: Vec<Data>,
    ) -> Result<Vec<MemoryAddr>> {
        let mut mem_addrs = Vec::new();
        if mems.is_empty() && self.mems.is_empty(module_index) {
            return Ok(mem_addrs);
        }
        for entry in mems.iter() {
            let instance =
                MemoryInstance::new(entry.initial as usize, entry.maximum.map(|mx| mx as usize));
            let addr = self
                .mems
                .push(module_index, Rc::new(RefCell::new(instance)));
            mem_addrs.push(addr);
        }

        let mut offsets_and_value = Vec::new();
        let mems = self.mems.items(module_index).unwrap();
        for seg in data_segments {
            match seg.kind {
                DataKind::Active {
                    memory_index,
                    init_expr,
                } => {
                    let mem_addr = match mems.get(memory_index as usize) {
                        Some(addr) => addr,
                        None => continue,
                    };
                    let offset = match eval_const_expr(&init_expr, self, module_index)? {
                        Value::I32(v) => v,
                        _ => panic!(),
                    };
                    let mem = self.mems.get_global(*mem_addr);
                    mem.borrow()
                        .validate_region(offset as usize, seg.data.len())
                        .map_err(StoreError::InvalidDataSegments)?;
                    offsets_and_value.push((mem, offset, seg.data));
                }
                other => unimplemented!("{:?}", other),
            }
        }

        for (mem, offset, value) in offsets_and_value {
            mem.borrow_mut()
                .store(offset as usize, value)
                .map_err(StoreError::InvalidDataSegments)?;
        }
        Ok(mem_addrs)
    }
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}
