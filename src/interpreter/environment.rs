use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use parity_wasm::elements::{FunctionType};
use super::module::*;


pub struct Environment<'a> {
    modules: HashMap<String, Module>,
    sigs: Vec<&'a FunctionType>,
    funcs: Vec<Func>,
    tables: Vec<Table>,
}

impl<'a> Environment<'a> {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            sigs: vec![],
            funcs: vec![],
            tables: vec![],
        }
    }
    pub fn load_module(&mut self, pmodule: &'a parity_wasm::elements::Module) {
        let module = DefinedModule::read_from_parity_wasm(pmodule, self);
        let module_name = pmodule.names_section()
                                  .map(|sec| { sec.module().unwrap() })
                                  .map(|module| { module.name() })
                                  .unwrap();
        self.modules.insert(module_name.to_string(), Module::Defined(module));
    }

    pub fn main_module(&self) -> &DefinedModule {
        match &self.modules["main"] {
            Module::Defined(defined) => defined,
        }
    }

    pub fn find_registered_module<T: Into<String>>(&self, name: T) -> &Module {
        &self.modules[&name.into()]
    }

    pub fn get_func_signature_count(&self) -> usize {
        self.sigs.len()
    }
    pub fn get_func_count(&self) -> usize {
        self.funcs.len()
    }

    pub fn get_func_signature(&self, index: Index) -> &FunctionType {
        let index: usize = index.try_into().unwrap();
        &self.sigs[index]
    }

    pub fn get_func(&self, index: Index) -> &Func {
        let index: usize = index.try_into().unwrap();
        &self.funcs[index]
    }

    pub fn get_table(&self, index: Index) -> &Table {
        let index: usize = index.try_into().unwrap();
        &self.tables[index]
    }

    pub fn push_back_func_signature(&mut self, sig: &'a FunctionType) {
        self.sigs.push(sig)
    }

    pub fn push_back_func(&mut self, func: Func) {
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
