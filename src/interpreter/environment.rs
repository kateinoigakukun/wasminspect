use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::hash::Hash;
use super::module::*;


pub struct Environment {
    modules: HashMap<String, Module>,
    sigs: Vec<FuncSignature>,
    funcs: Vec<Func>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            sigs: vec![],
            funcs: vec![],
        }
    }
    pub fn load_module(&mut self, pmodule: parity_wasm::elements::Module) {
        let module = DefinedModule::read_from_parity_wasm(&pmodule);
        let module_name = &pmodule.names_section()
                                  .map(|sec| { sec.module().unwrap() })
                                  .map(|module| { module.name() })
                                  .unwrap();
        self.modules.insert(module_name.to_string(), Module::Defined(module));
    }

    pub fn get_module_by_name<T: Into<String>>(&self, name: T) -> &Module {
        &self.modules[&name.into()]
    }

    pub fn get_func(&self, index: Index) -> &Func {
        let index: usize = index.try_into().unwrap();
        &self.funcs[index]
    }

    pub fn is_func_sigs_equal(&self, lhs: Index, rhs: Index) -> bool {
        if lhs == rhs {
            true
        } else {
            let lhs_index: usize = lhs.try_into().unwrap();
            let rhs_index: usize = rhs.try_into().unwrap();
            let lhs_sig = &self.sigs[lhs_index];
            let rhs_sig = &self.sigs[rhs_index];
            lhs_sig == rhs_sig
        }
    }
}
