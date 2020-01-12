use super::host::HostFuncBody;
use super::module::*;
use parity_wasm::elements::*;

use std::iter;

#[derive(Clone, Copy, Debug)]
pub struct InstIndex(pub u32);

impl InstIndex {
    pub fn zero() -> InstIndex {
        InstIndex(0)
    }
}

pub enum FunctionInstance {
    Defined(DefinedFunctionInstance),
    Host(HostFunctionInstance),
}

impl FunctionInstance {
    pub fn ty(&self) -> &FunctionType {
        match self {
            Self::Defined(defined) => defined.ty(),
            Self::Host(host) => host.ty(),
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
    code: HostFuncBody,
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

    pub fn code(&self) -> &HostFuncBody {
        &self.code
    }

    pub fn new(
        ty: FunctionType,
        module_name: String,
        field_name: String,
        code: HostFuncBody,
    ) -> Self {
        Self {
            ty,
            module_name,
            field_name,
            code,
        }
    }
}
