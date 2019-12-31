use super::address::{FuncAddr, GlobalAddr, TableAddr, MemoryAddr};
use super::executor::eval_const_expr;
use super::func::{DefinedFunc, DefinedFunctionInstance, FunctionInstance, HostFunctionInstance};
use super::global::GlobalInstance;
use super::memory::MemoryInstance;
use super::module::{ModuleIndex, ModuleInstance};
use super::table::TableInstance;
use super::value::Value;
use parity_wasm;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

/// Store
pub struct Store {
    funcs: HashMap<ModuleIndex, Vec<FunctionInstance>>,
    tables: HashMap<ModuleIndex, Vec<TableInstance>>,
    mems: HashMap<ModuleIndex, Vec<MemoryInstance>>,
    globals: HashMap<ModuleIndex, Vec<GlobalInstance>>,
    modules: Vec<ModuleInstance>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            funcs: HashMap::new(),
            tables: HashMap::new(),
            mems: HashMap::new(),
            globals: HashMap::new(),
            modules: Vec::new(),
        }
    }

    pub fn func(&self, addr: FuncAddr) -> &FunctionInstance {
        &self.funcs[&addr.0][addr.1]
    }

    pub fn set_global(&mut self, addr: GlobalAddr, value: Value) {
        let instance = self.globals.get_mut(&addr.0).unwrap();
        instance[addr.1].set_value(value);
    }

    pub fn global(&self, addr: GlobalAddr) -> &GlobalInstance {
        &self.globals[&addr.0][addr.1]
    }

    pub fn table(&self, addr: TableAddr) -> &TableInstance {
        &self.tables[&addr.0][addr.1]
    }

    pub fn memory(&self, addr: MemoryAddr) -> &MemoryInstance {
        &self.mems[&addr.0][addr.1]
    }

    pub fn memory_mut(&mut self, addr: MemoryAddr) -> &mut MemoryInstance {
        self.mems.get_mut(&addr.0).unwrap().get_mut(addr.1).unwrap()
    }

    pub fn set_memory(&mut self, addr: MemoryAddr, offset: usize, bytes: &[u8]) {
        &mut self.mems.entry(addr.0).and_modify(|mems| {
            if let Some(mem) = mems.get_mut(addr.1) {
                mem.initialize(offset, bytes)
            }
        });
    }

    pub fn module(&self, module_index: ModuleIndex) -> &ModuleInstance {
        &self.modules[module_index.0 as usize]
    }
}

impl Store {
    pub fn load_parity_module(
        &mut self,
        parity_module: parity_wasm::elements::Module,
    ) -> ModuleIndex {
        let types = Self::get_types(&parity_module);
        let elem_segs = Self::get_element_segments(&parity_module);
        let data_segs = Self::get_data_segments(&parity_module);

        let module_index = ModuleIndex(self.modules.len() as u32);
        let mut func_addrs = self.load_imports(&parity_module, module_index, types);
        func_addrs.append(&mut self.load_functions(&parity_module, module_index, types));
        self.load_globals(&parity_module, module_index);
        self.load_tables(&parity_module, module_index, elem_segs);
        self.load_mems(&parity_module, module_index, data_segs);
        let types = types.iter().map(|ty| ty.clone()).collect();

        let instance =
            ModuleInstance::new_from_parity_module(parity_module, module_index, types, func_addrs);
        self.modules.push(instance);
        return module_index;
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
        println!("data_segments is {:?}", result);
        result
    }

    fn load_imports(
        &mut self,
        parity_module: &parity_wasm::elements::Module,
        module_index: ModuleIndex,
        types: &[parity_wasm::elements::Type],
    ) -> Vec<FuncAddr> {
        let imports = parity_module
            .import_section()
            .map(|sec| sec.entries())
            .unwrap_or_default();
        let mut func_addrs = Vec::new();
        for import in imports {
            match import.external() {
                parity_wasm::elements::External::Function(type_index) => {
                    let addr = self.load_import_function(
                        module_index,
                        import,
                        *type_index as usize,
                        &types,
                    );
                    func_addrs.push(addr);
                }
                _ => panic!(),
            }
        }
        func_addrs
    }

    fn load_import_function(
        &mut self,
        module_index: ModuleIndex,
        import: &parity_wasm::elements::ImportEntry,
        type_index: usize,
        types: &[parity_wasm::elements::Type],
    ) -> FuncAddr {
        let func_ty = {
            let ty = types[type_index as usize].clone();
            match ty {
                parity_wasm::elements::Type::Function(func_ty) => func_ty,
            }
        };
        let instance = HostFunctionInstance::new(
            func_ty,
            import.module().to_string(),
            import.field().to_string(),
        );

        let map = self.funcs.entry(module_index).or_insert(Vec::new());
        let func_index = map.len();
        map.push(FunctionInstance::Host(instance));
        return FuncAddr(module_index, func_index);
    }

    fn load_functions(
        &mut self,
        parity_module: &parity_wasm::elements::Module,
        module_index: ModuleIndex,
        types: &[parity_wasm::elements::Type],
    ) -> Vec<FuncAddr> {
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
            let parity_wasm::elements::Type::Function(func_type) =
                types[func.type_ref() as usize].clone();
            let defined = DefinedFunctionInstance::new(
                func_type,
                module_index,
                DefinedFunc::new(*func, body.clone(), module_index),
            );
            let instance = FunctionInstance::Defined(defined);
            let map = self.funcs.entry(module_index).or_insert(Vec::new());
            let func_index = map.len();
            map.push(instance);
            func_addrs.push(FuncAddr(module_index, func_index));
        }
        func_addrs
    }

    fn load_globals(
        &mut self,
        parity_module: &parity_wasm::elements::Module,
        module_index: ModuleIndex,
    ) {
        let globals = parity_module
            .global_section()
            .map(|sec| sec.entries())
            .unwrap_or_default();
        for entry in globals {
            let value = eval_const_expr(entry.init_expr());
            let instance = GlobalInstance::new(value, entry.global_type().clone());
            self.globals
                .entry(module_index)
                .or_insert(Vec::new())
                .push(instance);
        }
    }

    fn load_tables(
        &mut self,
        parity_module: &parity_wasm::elements::Module,
        module_index: ModuleIndex,
        element_segments: HashMap<usize, Vec<&parity_wasm::elements::ElementSegment>>,
    ) {
        let tables = parity_module
            .table_section()
            .map(|sec| sec.entries())
            .unwrap_or_default();
        for (index, entry) in tables.iter().enumerate() {
            match entry.elem_type() {
                parity_wasm::elements::TableElementType::AnyFunc => {
                    let mut instance = TableInstance::new(
                        entry.limits().initial() as usize,
                        entry.limits().maximum().map(|mx| mx as usize),
                    );
                    let segs = &element_segments[&index];
                    for seg in segs {
                        let offset =
                            match seg.offset().as_ref().map(|e| eval_const_expr(&e)).unwrap() {
                                Value::I32(v) => v,
                                _ => panic!(),
                            };
                        let data = seg
                            .members()
                            .iter()
                            .map(|func_index| FuncAddr(module_index, *func_index as usize))
                            .collect();
                        instance.initialize(offset as usize, data);
                    }
                    self.tables
                        .entry(module_index)
                        .or_insert(Vec::new())
                        .push(instance);
                }
            }
        }
    }

    fn load_mems(
        &mut self,
        parity_module: &parity_wasm::elements::Module,
        module_index: ModuleIndex,
        data_segments: HashMap<usize, Vec<&parity_wasm::elements::DataSegment>>,
    ) {
        let mem_sec = parity_module
            .memory_section()
            .map(|sec| sec.entries())
            .unwrap_or_default();
        for (index, entry) in mem_sec.iter().enumerate() {
            let mut instance = MemoryInstance::new(
                entry.limits().initial() as usize,
                entry.limits().maximum().map(|mx| mx as usize),
            );
            if data_segments.len() > index {
                let segs = &data_segments[&index];
                for seg in segs {
                    let offset = match seg.offset().as_ref().map(|e| eval_const_expr(&e)).unwrap() {
                        Value::I32(v) => v,
                        _ => panic!(),
                    };
                    instance.initialize(offset as usize, seg.value());
                }
            }
            self.mems
                .entry(module_index)
                .or_insert(Vec::new())
                .push(instance);
        }
    }
}
