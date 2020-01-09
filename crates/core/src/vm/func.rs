use super::module::*;
use parity_wasm::elements::*;

use std::iter;

pub struct TypeIndex {
    module_index: ModuleIndex,
    index: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct InstIndex(pub u32);

impl InstIndex {
    pub fn zero() -> InstIndex {
        InstIndex(0)
    }
}

pub enum FunctionInstance {
    Defined(DefinedFunctionInstance),
    External(HostFunctionInstance),
}

impl FunctionInstance {
    pub fn ty(&self) -> &FunctionType {
        match self {
            Self::Defined(defined) => defined.ty(),
            Self::External(host) => host.ty(),
        }
    }

    pub fn defined(&self) -> Option<&DefinedFunctionInstance> {
        match self {
            Self::Defined(defined) => Some(defined),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        match self {
            Self::Defined(defined) => defined.name.clone(),
            Self::External(external) => {
                format!("{}.{}", external.module_name(), external.field_name())
            }
        }
    }
}

pub struct DefinedFunctionInstance {
    name: String,
    ty: FunctionType,
    module_index: ModuleIndex,
    locals: Vec<ValueType>,
    instructions: Vec<Instruction>,
}

impl DefinedFunctionInstance {
    pub fn new(
        name: String,
        ty: FunctionType,
        module_index: ModuleIndex,
        body: parity_wasm::elements::FuncBody,
    ) -> Self {
        let locals = body
            .locals()
            .iter()
            .flat_map(|locals| iter::repeat(locals.value_type()).take(locals.count() as usize))
            .collect();
        let instructions = body.code().elements().to_vec();
        Self {
            name,
            ty,
            module_index,
            locals,
            instructions,
        }
    }

    pub fn ty(&self) -> &FunctionType {
        &self.ty
    }

    pub fn module_index(&self) -> ModuleIndex {
        self.module_index
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    pub fn locals(&self) -> &[ValueType] {
        &self.locals
    }

    pub fn inst(&self, index: InstIndex) -> &Instruction {
        &self.instructions[index.0 as usize]
    }
}

pub struct HostFunctionInstance {
    ty: FunctionType,
    module_name: String,
    field_name: String,
}

impl HostFunctionInstance {
    pub fn ty(&self) -> &FunctionType {
        &self.ty
    }

    pub fn module_name(&self) -> &String {
        &self.module_name
    }

    pub fn field_name(&self) -> &String {
        &self.field_name
    }

    pub fn new(ty: FunctionType, module_name: String, field_name: String) -> Self {
        Self {
            ty,
            module_name,
            field_name,
        }
    }
}
