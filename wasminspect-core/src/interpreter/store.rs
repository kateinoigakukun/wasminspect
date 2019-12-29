use super::executor::eval_const_expr;
use super::func::{DefinedFunc, FunctionInstance};
use super::global::GlobalInstance;
use super::module::{ModuleIndex, ModuleInstance};
use parity_wasm;
use std::collections::HashMap;

// Addresses
#[derive(Clone, Copy)]
pub struct FuncAddr(pub ModuleIndex, pub usize);
pub struct GlobalAddr(pub ModuleIndex, pub usize);


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

    pub fn global_mut(&mut self, addr: GlobalAddr) -> &mut GlobalInstance {
        &mut self.globals[&addr.0][addr.1]
    }

    pub fn global(&mut self, addr: GlobalAddr) -> GlobalInstance {
        self.globals[&addr.0][addr.1]
    }
}

impl Store {
    pub fn load_parity_module(&mut self, parity_module: parity_wasm::elements::Module) {
        let types = Self::get_types(&parity_module);
        let module_index = ModuleIndex(self.modules.len() as u32);
        self.load_functions(&parity_module, module_index, types);
        self.load_globals(&parity_module, module_index);
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
            let instance = FunctionInstance::Defined(
                func_type,
                module_index,
                DefinedFunc::new(*func, *body, module_index),
            );
            self.funcs[&module_index].push(instance);
        }
    }

    fn load_globals(
        &self,
        parity_module: &parity_wasm::elements::Module,
        module_index: ModuleIndex,
    ) {
        let globals = parity_module
            .global_section()
            .map(|sec| sec.entries())
            .unwrap_or_default();
        self.globals[&module_index] = Vec::new();
        for entry in globals {
            let value = eval_const_expr(entry.init_expr());
            let instance = GlobalInstance::new(value, entry.global_type().clone());
            self.globals[&module_index].push(instance);
        }
    }
}
