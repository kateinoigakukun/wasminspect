use super::executor::eval_const_expr;
use super::func::{DefinedFunc, DefinedFunctionInstance, FunctionInstance};
use super::global::GlobalInstance;
use super::module::{ModuleIndex, ModuleInstance};
use super::value::Value;
use super::address::{FuncAddr, GlobalAddr};
use parity_wasm;
use std::collections::HashMap;

/// Store
pub struct Store {
    funcs: HashMap<ModuleIndex, Vec<FunctionInstance>>,
    // tables: Vec<TableInstance<'a>>,
    // mems: Vec<MemoryInstance<'a>>,
    globals: HashMap<ModuleIndex, Vec<GlobalInstance>>,
    modules: Vec<ModuleInstance>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            funcs: HashMap::new(),
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

    pub fn global(&mut self, addr: GlobalAddr) -> &GlobalInstance {
        &self.globals[&addr.0][addr.1]
    }
}

impl Store {
    pub fn load_parity_module(
        &mut self,
        parity_module: parity_wasm::elements::Module,
    ) -> &ModuleInstance {
        let types = Self::get_types(&parity_module);
        let module_index = ModuleIndex(self.modules.len() as u32);
        let func_addrs = self.load_functions(&parity_module, module_index, types);
        self.load_globals(&parity_module, module_index);
        let types = types.iter().map(|ty| ty.clone()).collect();

        let instance =
            ModuleInstance::new_from_parity_module(parity_module, module_index, types, func_addrs);
        self.modules.push(instance);
        &self.modules.last().unwrap()
    }

    fn get_types(parity_module: &parity_wasm::elements::Module) -> &[parity_wasm::elements::Type] {
        return parity_module
            .type_section()
            .map(|sec| sec.types())
            .unwrap_or_default();
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
}
