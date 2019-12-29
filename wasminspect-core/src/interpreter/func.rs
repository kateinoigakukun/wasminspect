use super::module::Module as _Module;
use super::module::*;
use parity_wasm::elements::*;

use std::iter;

// TODO: move
pub struct TypeIndex(u32);

pub enum FunctionInstance<'a> {
    Defined(FunctionType, &'a _Module, DefinedFunc),
    Host(FunctionType, HostFunc),
}

impl<'a> FunctionInstance<'a> {
    pub fn r#type(&self) -> &FunctionType {
        match self {
            Self::Defined(ty, _, _) => ty,
            Self::Host(ty, _) => ty,
        }
    }
}

pub struct DefinedFunc {
    type_index: TypeIndex,
    locals: Vec<ValueType>,
    instructions: Vec<Instruction>,
}

impl DefinedFunc {
    pub fn new(func: parity_wasm::elements::Func, body: parity_wasm::elements::FuncBody) -> Self {
        let locals = body
            .locals()
            .iter()
            .flat_map(|locals| iter::repeat(locals.value_type()).take(locals.count() as usize))
            .collect();
        let instructions = body.code().elements().to_vec();
        Self {
            type_index: TypeIndex(func.type_ref()),
            locals,
            instructions,
        }
    }

    pub fn inst(&self, index: Index) -> &Instruction {
        &self.instructions[index.0 as usize]
    }
}

pub struct HostFunc {}

impl HostFunc {
    fn new(name: String, func_type: FunctionType, locals: Vec<ValueType>) -> Self {
        panic!()
    }
}
