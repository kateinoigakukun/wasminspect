use super::func::*;
use parity_wasm;

use std::iter;

pub struct Store<'a> {
    funcs: Vec<FunctionInstance<'a>>,
    // tables: Vec<TableInstance<'a>>,
    // mems: Vec<MemoryInstance<'a>>,
    // globals: Vec<GlobalInstance<'a>>,
}

impl<'a> Store<'a> {
    pub fn new() -> Self {
        Self { funcs: Vec::new() }
    }
    pub fn load_parity_module(&mut self, parity_module: parity_wasm::elements::Module) {
        let types = Self::get_types(&parity_module);
        self.funcs
            .append(&mut Self::get_function(&parity_module, types));
    }

    fn get_types(parity_module: &parity_wasm::elements::Module) -> &[parity_wasm::elements::Type] {
        return parity_module
            .type_section()
            .map(|sec| sec.types())
            .unwrap_or_default();
    }

    fn get_function(
        parity_module: &parity_wasm::elements::Module,
        types: &[parity_wasm::elements::Type],
    ) -> Vec<FunctionInstance<'a>> {
        let functions = parity_module
            .function_section()
            .map(|sec| sec.entries())
            .unwrap_or_default();
        let bodies = parity_module
            .code_section()
            .map(|sec| sec.bodies())
            .unwrap_or_default();
        functions.into_iter().zip(bodies).map(|(func, body)| {
            let parity_wasm::elements::Type::Function(func_type) = types[func.type_ref() as usize];
            FunctionInstance::Defined(
                func_type,
                panic!(),
                DefinedFunc::new(*func, *body)
            )
        }).collect()
    }
}
