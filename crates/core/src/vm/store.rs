use super::address::{FuncAddr, GlobalAddr, MemoryAddr, TableAddr};
use super::executor::{eval_const_expr, invoke_func, resolve_func_addr, WasmError};
use super::func::{DefinedFunctionInstance, FunctionInstance, HostFunctionInstance};
use super::global::{
    resolve_global_instance, DefinedGlobalInstance, ExternalGlobalInstance, GlobalInstance,
};
use super::host::HostValue;
use super::memory::{self, DefinedMemoryInstance, ExternalMemoryInstance, MemoryInstance};
use super::module::{self, DefinedModuleInstance, HostModuleInstance, ModuleIndex, ModuleInstance};
use super::table::{
    self, resolve_table_instance, DefinedTableInstance, ExternalTableInstance, TableInstance,
};
use super::utils::*;
use super::value::Value;
use parity_wasm::elements::{FunctionType, ValueType};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Store
pub struct Store {
    funcs: HashMap<ModuleIndex, Vec<FunctionInstance>>,
    tables: HashMap<ModuleIndex, Vec<Rc<RefCell<TableInstance>>>>,
    mems: HashMap<ModuleIndex, Vec<Rc<RefCell<MemoryInstance>>>>,
    globals: HashMap<ModuleIndex, Vec<Rc<RefCell<GlobalInstance>>>>,
    modules: Vec<ModuleInstance>,
    module_index_by_name: HashMap<String, ModuleIndex>,

    embedded_contexts: HashMap<std::any::TypeId, Box<dyn std::any::Any>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            funcs: HashMap::new(),
            tables: HashMap::new(),
            mems: HashMap::new(),
            globals: HashMap::new(),
            modules: Vec::new(),
            module_index_by_name: HashMap::new(),
            embedded_contexts: HashMap::new(),
        }
    }

    pub fn func(&self, addr: FuncAddr) -> Option<&FunctionInstance> {
        self.funcs.get(&addr.0).and_then(|m| m.get(addr.1))
    }

    pub fn func_ty(&self, addr: FuncAddr) -> &FunctionType {
        let func = &self.funcs[&addr.0][addr.1];
        func.ty()
    }

    pub fn global(&self, addr: GlobalAddr) -> Rc<RefCell<GlobalInstance>> {
        self.globals[&addr.0][addr.1].clone()
    }

    pub fn scan_global_by_name(
        &self,
        module_index: ModuleIndex,
        field: &str,
    ) -> Option<Rc<RefCell<GlobalInstance>>> {
        let module = self.module(module_index).defined().unwrap();
        let global_addr = module.exported_global(field.to_string()).ok().unwrap();
        global_addr.map(|addr| self.global(addr))
    }

    pub fn table(&self, addr: TableAddr) -> Rc<RefCell<TableInstance>> {
        self.tables[&addr.0][addr.1].clone()
    }

    pub fn memory(&self, addr: MemoryAddr) -> Rc<RefCell<MemoryInstance>> {
        self.mems[&addr.0][addr.1].clone()
    }

    pub fn memory_count(&self, addr: ModuleIndex) -> usize {
        self.mems.get(&addr).map(|c| c.len()).unwrap_or_default()
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
        let instance = HostModuleInstance::new(module);
        self.modules.push(ModuleInstance::Host(instance));
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

pub enum Error {
    InvalidElementSegments(table::Error),
    InvalidDataSegments(memory::Error),
    InvalidHostImport(module::HostModuleError),
    InvalidImport(module::DefinedModuleError),
    UnknownType(/* type index: */ u32),
    UndefinedFunction(/* module: */ String, /* name: */ String),
    UndefinedMemory(String, String),
    UndefinedTable(String, String),
    UndefinedGlobal(String, String),
    FailedEntryFunction(WasmError),
    IncompatibleImportFuncType(String, FunctionType, FunctionType),
    IncompatibleImportGlobalType(ValueType, ValueType),
    IncompatibleImportGlobalMutability,
    IncompatibleImportTableType,
    IncompatibleImportMemoryType,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidElementSegments(err) => {
                write!(f, "elements segment does not fit: {:?}", err)
            }
            Self::InvalidDataSegments(err) => write!(f, "data segment does not fit: {}", err),
            Self::InvalidHostImport(err) => write!(f, "invalid host import: {}", err),
            Self::InvalidImport(err) => write!(f, "invalid import: {}", err),
            Self::UnknownType(idx) => write!(f, "Unknown type index used: {:?}", idx),
            Self::UndefinedFunction(module, name) => write!(
                f,
                "unknown import: Undefined function \"{}\" in \"{}\"",
                name, module
            ),
            Self::UndefinedMemory(module, name) => write!(
                f,
                "unknown import: Undefined memory \"{}\" in \"{}\"",
                name, module
            ),
            Self::UndefinedTable(module, name) => write!(
                f,
                "unknown import: Undefined table \"{}\" in \"{}\"",
                name, module
            ),
            Self::UndefinedGlobal(module, name) => write!(
                f,
                "unknown import: Undefined global \"{}\" in \"{}\"",
                name, module
            ),
            Self::FailedEntryFunction(e) => write!(f, "{}", e),
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

type Result<T> = std::result::Result<T, Error>;

impl Store {
    fn load_parity_module_internal(
        &mut self,
        name: Option<String>,
        parity_module: parity_wasm::elements::Module,
        module_index: ModuleIndex,
    ) -> Result<ModuleIndex> {
        let types = Self::get_types(&parity_module);
        let elem_segs = Self::get_element_segments(&parity_module);
        let data_segs = Self::get_data_segments(&parity_module);

        let (mut func_addrs, mut mem_addrs, mut table_addrs, mut global_addrs) =
            self.load_imports(&parity_module, module_index, types)?;
        func_addrs.append(&mut self.load_functions(&parity_module, module_index, types)?);

        global_addrs.append(&mut self.load_globals(&parity_module, module_index));
        table_addrs.append(&mut self.load_tables(&parity_module, module_index, elem_segs)?);

        mem_addrs.append(&mut self.load_mems(&parity_module, module_index, data_segs)?);
        let types = types.iter().map(|ty| ty.clone()).collect();

        let instance = DefinedModuleInstance::new_from_parity_module(
            parity_module,
            module_index,
            types,
            func_addrs,
        );
        self.modules.push(ModuleInstance::Defined(instance));
        if let Some(name) = name {
            self.module_index_by_name.insert(name, module_index);
        }

        Ok(module_index)
    }
    pub fn load_parity_module(
        &mut self,
        name: Option<String>,
        parity_module: parity_wasm::elements::Module,
    ) -> Result<ModuleIndex> {
        let module_index = ModuleIndex(self.modules.len() as u32);
        let start_section = parity_module.start_section().clone();

        let result: Result<ModuleIndex> =
            self.load_parity_module_internal(name.clone(), parity_module, module_index);
        if let Some(start_section) = start_section {
            let func_addr = FuncAddr(module_index, start_section as usize);
            // TODO: Handle result
            invoke_func(func_addr, vec![], self).map_err(Error::FailedEntryFunction)?;
        }
        match result {
            Ok(ok) => Ok(ok),
            Err(err) => {
                // If fail, cleanup states
                self.funcs.remove(&module_index);
                self.tables.remove(&module_index);
                self.mems.remove(&module_index);
                self.globals.remove(&module_index);
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

    fn get_types(parity_module: &parity_wasm::elements::Module) -> &[parity_wasm::elements::Type] {
        return parity_module
            .type_section()
            .map(|sec| sec.types())
            .unwrap_or_default();
    }

    fn get_element_segments(
        parity_module: &parity_wasm::elements::Module,
    ) -> HashMap<usize, Vec<&parity_wasm::elements::ElementSegment>> {
        let segments = parity_module
            .elements_section()
            .map(|sec| sec.entries())
            .unwrap_or_default();
        let mut result = HashMap::new();
        for seg in segments {
            result
                .entry(seg.index() as usize)
                .or_insert(Vec::new())
                .push(seg);
        }
        result
    }

    fn get_data_segments(
        parity_module: &parity_wasm::elements::Module,
    ) -> HashMap<usize, Vec<&parity_wasm::elements::DataSegment>> {
        let segments = parity_module
            .data_section()
            .map(|sec| sec.entries())
            .unwrap_or_default();

        let mut result = HashMap::new();
        for seg in segments {
            result
                .entry(seg.index() as usize)
                .or_insert(Vec::new())
                .push(seg);
        }
        result
    }

    fn load_imports(
        &mut self,
        parity_module: &parity_wasm::elements::Module,
        module_index: ModuleIndex,
        types: &[parity_wasm::elements::Type],
    ) -> Result<(
        Vec<FuncAddr>,
        Vec<MemoryAddr>,
        Vec<TableAddr>,
        Vec<GlobalAddr>,
    )> {
        let imports = parity_module
            .import_section()
            .map(|sec| sec.entries())
            .unwrap_or_default();
        let mut func_addrs = Vec::new();
        let mut mem_addrs = Vec::new();
        let mut table_addrs = Vec::new();
        let mut global_addrs = Vec::new();
        for import in imports {
            match import.external() {
                parity_wasm::elements::External::Function(type_index) => {
                    let addr = self.load_import_function(
                        module_index,
                        import,
                        *type_index as usize,
                        &types,
                    )?;
                    func_addrs.push(addr);
                }
                parity_wasm::elements::External::Memory(memory_ty) => {
                    let addr = self.load_import_memory(module_index, import, *memory_ty)?;
                    mem_addrs.push(addr);
                }
                parity_wasm::elements::External::Table(table_ty) => {
                    let addr = self.load_import_table(module_index, import, *table_ty)?;
                    table_addrs.push(addr);
                }
                parity_wasm::elements::External::Global(global_ty) => {
                    let addr = self.load_import_global(module_index, import, *global_ty)?;
                    global_addrs.push(addr);
                }
            }
        }
        Ok((func_addrs, mem_addrs, table_addrs, global_addrs))
    }

    fn load_import_function(
        &mut self,
        module_index: ModuleIndex,
        import: &parity_wasm::elements::ImportEntry,
        type_index: usize,
        types: &[parity_wasm::elements::Type],
    ) -> Result<FuncAddr> {
        let parity_wasm::elements::Type::Function(func_ty) = types
            .get(type_index)
            .ok_or(Error::UnknownType(type_index as u32))?
            .clone();
        let instance = HostFunctionInstance::new(
            func_ty.clone(),
            import.module().to_string(),
            import.field().to_string(),
        );
        // Validation
        {
            let actual_func_ty = {
                let name = import.field().to_string();
                let module = self.module_by_name(import.module().to_string());
                let err = || {
                    Error::UndefinedFunction(
                        import.module().clone().to_string(),
                        import.field().clone().to_string(),
                    )
                };
                match module {
                    ModuleInstance::Defined(defined) => {
                        let addr = defined
                            .exported_func(name)
                            .map_err(Error::InvalidImport)?
                            .ok_or(err())?;
                        match resolve_func_addr(addr, self).map_err(|_| err())? {
                            Either::Left((_, f)) => f.ty(),
                            Either::Right(f) => f.ty(),
                        }
                    }
                    ModuleInstance::Host(host) => host
                        .func_by_name(name)
                        .map_err(Error::InvalidHostImport)
                        .and_then(|f| f.ok_or(err()).map(|f| f.ty()))?,
                }
            };
            if *actual_func_ty != func_ty {
                return Err(Error::IncompatibleImportFuncType(
                    import.field().to_string(),
                    func_ty,
                    actual_func_ty.clone(),
                ));
            }
        }

        let map = self.funcs.entry(module_index).or_insert(Vec::new());
        let func_index = map.len();
        map.push(FunctionInstance::External(instance));
        return Ok(FuncAddr(module_index, func_index));
    }

    fn load_import_memory(
        &mut self,
        module_index: ModuleIndex,
        import: &parity_wasm::elements::ImportEntry,
        memory_ty: parity_wasm::elements::MemoryType,
    ) -> Result<MemoryAddr> {
        let instance = ExternalMemoryInstance::new(
            import.module().to_string(),
            import.field().to_string(),
            memory_ty.limits().clone(),
        );
        // Validation
        {
            let err = || {
                Error::UndefinedMemory(
                    import.module().clone().to_string(),
                    import.field().clone().to_string(),
                )
            };
            let name = import.field().to_string();
            let module = self.module_by_name(import.module().to_string());
            let (initial, max) = match module {
                ModuleInstance::Defined(defined) => {
                    let addr = defined
                        .exported_memory(name.clone())
                        .map_err(Error::InvalidImport)?
                        .ok_or(err())?
                        .clone();
                    let mem = self.memory(addr).clone();
                    let initial = mem.borrow().initial();
                    let max = mem.borrow().max();
                    (initial, max)
                }
                ModuleInstance::Host(host) => {
                    let mem = host
                        .memory_by_name(name.clone())
                        .map_err(Error::InvalidHostImport)?
                        .ok_or(err())?
                        .clone();
                    let initial = mem.borrow().initial;
                    let max = mem.borrow().max;
                    (initial, max)
                }
            };
            if initial < memory_ty.limits().initial() as usize {
                return Err(Error::IncompatibleImportMemoryType);
            }
            match (max, memory_ty.limits().maximum()) {
                (Some(found), Some(expected)) => {
                    if found > expected as usize {
                        return Err(Error::IncompatibleImportMemoryType);
                    }
                }
                (None, Some(_)) => return Err(Error::IncompatibleImportMemoryType),
                _ => (),
            }
        }
        let map = self.mems.entry(module_index).or_insert(Vec::new());
        let mem_index = map.len();
        map.push(Rc::new(RefCell::new(MemoryInstance::External(instance))));
        return Ok(MemoryAddr(module_index, mem_index));
    }

    fn load_import_table(
        &mut self,
        module_index: ModuleIndex,
        import: &parity_wasm::elements::ImportEntry,
        table_ty: parity_wasm::elements::TableType,
    ) -> Result<TableAddr> {
        let instance = ExternalTableInstance::new(
            import.module().to_string(),
            import.field().to_string(),
            table_ty.limits().clone(),
        );
        let name = import.field().to_string();
        let module = self.module_by_name(import.module().to_string());
        // Validation
        {
            let err = || {
                Error::UndefinedTable(
                    import.module().clone().to_string(),
                    import.field().clone().to_string(),
                )
            };
            let found = match module {
                ModuleInstance::Defined(defined) => {
                    let addr = defined
                        .exported_table(name.clone())
                        .map_err(Error::InvalidImport)?
                        .ok_or(err())?
                        .clone();
                    resolve_table_instance(addr, self)
                }
                ModuleInstance::Host(host) => host
                    .table_by_name(name.clone())
                    .map_err(Error::InvalidHostImport)?
                    .ok_or(err())?
                    .clone(),
            };
            if found.borrow().initial < table_ty.limits().initial() as usize {
                return Err(Error::IncompatibleImportTableType);
            }
            match (found.clone().borrow().max, table_ty.limits().maximum()) {
                (Some(found), Some(expected)) => {
                    if found > expected as usize {
                        return Err(Error::IncompatibleImportTableType);
                    }
                }
                (None, Some(_)) => return Err(Error::IncompatibleImportTableType),
                _ => (),
            }
        }
        let map = self.tables.entry(module_index).or_insert(Vec::new());
        let table_index = map.len();
        map.push(Rc::new(RefCell::new(TableInstance::External(instance))));
        return Ok(TableAddr(module_index, table_index));
    }

    fn load_import_global(
        &mut self,
        module_index: ModuleIndex,
        import: &parity_wasm::elements::ImportEntry,
        global_ty: parity_wasm::elements::GlobalType,
    ) -> Result<GlobalAddr> {
        let instance = ExternalGlobalInstance::new(
            import.module().to_string(),
            import.field().to_string(),
            global_ty.clone(),
        );
        // Validation
        {
            let actual_global = {
                let name = import.field().to_string();
                let module = self.module_by_name(import.module().to_string());
                let err = || {
                    Error::UndefinedGlobal(
                        import.module().clone().to_string(),
                        import.field().clone().to_string(),
                    )
                };
                match module {
                    ModuleInstance::Defined(defined) => {
                        let addr = defined
                            .exported_global(name)
                            .map_err(Error::InvalidImport)?
                            .ok_or(err())?;
                        resolve_global_instance(addr, self)
                    }
                    ModuleInstance::Host(host) => host
                        .global_by_name(name)
                        .map_err(Error::InvalidHostImport)
                        .and_then(|f| f.ok_or(err()))?
                        .clone(),
                }
            };
            let actual_global_ty = actual_global.borrow().ty().content_type().clone();
            let expected_global_ty = global_ty.content_type().clone();
            if actual_global.borrow().is_mutable() != global_ty.is_mutable() {
                return Err(Error::IncompatibleImportGlobalMutability);
            }
            if actual_global_ty != expected_global_ty {
                return Err(Error::IncompatibleImportGlobalType(
                    actual_global_ty,
                    expected_global_ty,
                ));
            }
        };
        let map = self.globals.entry(module_index).or_insert(Vec::new());
        let global_index = map.len();
        map.push(Rc::new(RefCell::new(GlobalInstance::External(instance))));
        return Ok(GlobalAddr(module_index, global_index));
    }

    fn load_functions(
        &mut self,
        parity_module: &parity_wasm::elements::Module,
        module_index: ModuleIndex,
        types: &[parity_wasm::elements::Type],
    ) -> Result<Vec<FuncAddr>> {
        let functions = parity_module
            .function_section()
            .map(|sec| sec.entries())
            .unwrap_or_default();
        let bodies = parity_module
            .code_section()
            .map(|sec| sec.bodies())
            .unwrap_or_default();
        let mut func_addrs = Vec::new();
        for (func, body) in functions.into_iter().zip(bodies) {
            let parity_wasm::elements::Type::Function(func_type) = types
                .get(func.type_ref() as usize)
                .ok_or(Error::UnknownType(func.type_ref()))?
                .clone();
            let map = self.funcs.entry(module_index).or_insert(Vec::new());
            let func_index = map.len();
            let defined = DefinedFunctionInstance::new(
                format!("<module defined func #{}>", func_index),
                func_type,
                module_index,
                body.clone(),
            );
            let instance = FunctionInstance::Defined(defined);
            map.push(instance);
            func_addrs.push(FuncAddr(module_index, func_index));
        }
        Ok(func_addrs)
    }

    fn load_globals(
        &mut self,
        parity_module: &parity_wasm::elements::Module,
        module_index: ModuleIndex,
    ) -> Vec<GlobalAddr> {
        let globals = parity_module
            .global_section()
            .map(|sec| sec.entries())
            .unwrap_or_default();
        let mut global_addrs = Vec::new();
        for entry in globals {
            let value = eval_const_expr(entry.init_expr(), &self, module_index);
            let instance = DefinedGlobalInstance::new(value, entry.global_type().clone());
            let map = self.globals.entry(module_index).or_insert(Vec::new());
            let global_index = map.len();
            map.push(Rc::new(RefCell::new(GlobalInstance::Defined(Rc::new(
                RefCell::new(instance),
            )))));
            global_addrs.push(GlobalAddr(module_index, global_index));
        }
        global_addrs
    }

    fn load_tables(
        &mut self,
        parity_module: &parity_wasm::elements::Module,
        module_index: ModuleIndex,
        element_segments: HashMap<usize, Vec<&parity_wasm::elements::ElementSegment>>,
    ) -> Result<Vec<TableAddr>> {
        let tables = parity_module
            .table_section()
            .map(|sec| sec.entries())
            .unwrap_or_default();
        let mut table_addrs = Vec::new();
        if tables.is_empty() && self.tables.is_empty() {
            return Ok(table_addrs);
        }
        for entry in tables.iter() {
            match entry.elem_type() {
                parity_wasm::elements::TableElementType::AnyFunc => {
                    let instance = DefinedTableInstance::new(
                        entry.limits().initial() as usize,
                        entry.limits().maximum().map(|mx| mx as usize),
                    );
                    let map = self.tables.entry(module_index).or_insert(Vec::new());
                    let table_index = map.len();
                    map.push(Rc::new(RefCell::new(TableInstance::Defined(Rc::new(
                        RefCell::new(instance),
                    )))));
                    table_addrs.push(TableAddr(module_index, table_index));
                }
            }
        }

        let tables = self
            .tables
            .entry(module_index)
            .or_insert(Vec::new())
            .clone();

        for (index, table) in tables.iter().enumerate() {
            if let Some(segs) = element_segments.get(&index) {
                for seg in segs {
                    let offset = match seg
                        .offset()
                        .as_ref()
                        .map(|e| eval_const_expr(&e, self, module_index))
                        .unwrap()
                    {
                        Value::I32(v) => v,
                        _ => panic!(),
                    };
                    let data = seg
                        .members()
                        .iter()
                        .map(|func_index| FuncAddr(module_index, *func_index as usize))
                        .collect();
                    table
                        .borrow_mut()
                        .initialize(offset as usize, data, self)
                        .map_err(Error::InvalidElementSegments)?;
                }
            }
        }
        Ok(table_addrs)
    }

    fn load_mems(
        &mut self,
        parity_module: &parity_wasm::elements::Module,
        module_index: ModuleIndex,
        data_segments: HashMap<usize, Vec<&parity_wasm::elements::DataSegment>>,
    ) -> Result<Vec<MemoryAddr>> {
        let mem_sec = parity_module
            .memory_section()
            .map(|sec| sec.entries())
            .unwrap_or_default();
        let mut mem_addrs = Vec::new();
        if mem_sec.is_empty() && self.mems.is_empty() {
            return Ok(mem_addrs);
        }
        for entry in mem_sec.iter() {
            let instance = DefinedMemoryInstance::new(
                entry.limits().initial() as usize,
                entry.limits().maximum().map(|mx| mx as usize),
            );
            let map = self.mems.entry(module_index).or_insert(Vec::new());
            let mem_index = map.len();
            map.push(Rc::new(RefCell::new(MemoryInstance::Defined(Rc::new(
                RefCell::new(instance),
            )))));
            mem_addrs.push(MemoryAddr(module_index, mem_index));
        }

        let mems = self.mems.entry(module_index).or_insert(Vec::new()).clone();

        let mut offsets_and_value = Vec::new();
        for (index, mem) in mems.iter().enumerate() {
            if let Some(segs) = data_segments.get(&index) {
                for seg in segs {
                    let offset = match seg
                        .offset()
                        .as_ref()
                        .map(|e| eval_const_expr(&e, self, module_index))
                        .unwrap()
                    {
                        Value::I32(v) => v,
                        _ => panic!(),
                    };
                    mem.borrow()
                        .validate_region(offset as usize, seg.value().len(), self)
                        .map_err(Error::InvalidDataSegments)?;
                    offsets_and_value.push((mem, offset, seg.value()));
                }
            }
        }

        for (mem, offset, value) in offsets_and_value {
            mem.borrow_mut()
                .store(offset as usize, value, self)
                .map_err(Error::InvalidDataSegments)?;
        }
        Ok(mem_addrs)
    }
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}
