use super::func::*;
use super::module::{ModuleInstance, ModuleIndex};
use parity_wasm;
use std::collections::HashMap;

pub struct Store {
    funcs: HashMap<ModuleIndex, Vec<FunctionInstance>>,
    // tables: Vec<TableInstance<'a>>,
    // mems: Vec<MemoryInstance<'a>>,
    // globals: Vec<GlobalInstance<'a>>,
    modules: Vec<ModuleInstance>
}

impl Store {
    pub fn new() -> Self {
        Self { funcs: HashMap::new(), modules: Vec::new() }
    }
    pub fn load_parity_module(
        &mut self,
        parity_module: parity_wasm::elements::Module
    ) {
        let types = Self::get_types(&parity_module);
        let module_index = ModuleIndex(self.modules.len() as u32);
        self.load_functions(&parity_module, module_index, types);
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
    ) {
        let functions = parity_module
            .function_section()
            .map(|sec| sec.entries())
            .unwrap_or_default();
        let bodies = parity_module
            .code_section()
            .map(|sec| sec.bodies())
            .unwrap_or_default();
        self.funcs[&module_index] = Vec::new();
        for (func, body) in functions.into_iter().zip(bodies) {
            let parity_wasm::elements::Type::Function(func_type) = types[func.type_ref() as usize];
            let instance = FunctionInstance::Defined(func_type, module_index, DefinedFunc::new(*func, *body, module_index));
            self.funcs[&module_index].push(instance);
        }
    }
}
