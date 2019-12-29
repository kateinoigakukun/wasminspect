use super::module::*;
use super::address::FuncAddr;
use parity_wasm::elements::*;

use std::iter;

pub struct TypeIndex {
    module_index: ModuleIndex,
    index: u32,
}

#[derive(Clone, Copy)]
pub struct InstIndex(pub u32);

impl InstIndex {
    pub fn zero() -> InstIndex {
        InstIndex(0)
    }
}

#[deprecated]
#[derive(Clone, Copy)]
pub struct FuncIndex(u32);

pub enum FunctionInstance {
    Defined(DefinedFunctionInstance),
    Host(FunctionType, HostFunc),
}

impl FunctionInstance {
    pub fn ty(&self) -> &FunctionType {
        match self {
            Self::Defined(defined) => defined.ty(),
            Self::Host(ty, _) => ty,
        }
    }

    pub fn defined(&self) -> Option<&DefinedFunctionInstance> {
        match self {
            Self::Defined(defined) => Some(defined),
            _ => None,
        }
    }
}

pub struct DefinedFunctionInstance {
    ty: FunctionType,
    module_index: ModuleIndex,
    code: DefinedFunc,
}

impl DefinedFunctionInstance {
    pub fn new(ty: FunctionType, module_index: ModuleIndex, code: DefinedFunc) -> Self {
        Self {
            ty,
            module_index,
            code,
        }
    }
    pub fn ty(&self) -> &FunctionType {
        &self.ty
    }

    pub fn code(&self) -> &DefinedFunc {
        &self.code
    }
    pub fn module_index(&self) -> ModuleIndex {
        self.module_index
    }
}

pub struct DefinedFunc {
    type_index: TypeIndex,
    locals: Vec<ValueType>,
    instructions: Vec<Instruction>,
}

impl DefinedFunc {
    pub fn new(
        func: parity_wasm::elements::Func,
        body: parity_wasm::elements::FuncBody,
        module_index: ModuleIndex,
    ) -> Self {
        let locals = body
            .locals()
            .iter()
            .flat_map(|locals| iter::repeat(locals.value_type()).take(locals.count() as usize))
            .collect();
        let instructions = body.code().elements().to_vec();
        Self {
            type_index: TypeIndex {
                module_index,
                index: func.type_ref(),
            },
            locals,
            instructions,
        }
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }
    pub fn inst(&self, index: InstIndex) -> &Instruction {
        &self.instructions[index.0 as usize]
    }
    pub fn locals(&self) -> &Vec<ValueType> {
        &self.locals
    }
}

pub struct HostFunc {}

impl HostFunc {
    fn new(name: String, func_type: FunctionType, locals: Vec<ValueType>) -> Self {
        panic!()
    }
}
