use super::host::HostFuncBody;
use super::inst::*;
use super::module::*;
use anyhow::Result;
use std::iter;
use wasmparser::{FuncType, FunctionBody, Type};

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
    pub fn ty(&self) -> &FuncType {
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

    pub fn name(&self) -> &String {
        match self {
            Self::Defined(defined) => &defined.name,
            Self::Host(host) => host.field_name(),
        }
    }
}

pub struct DefinedFunctionInstance {
    name: String,
    ty: FuncType,
    module_index: ModuleIndex,
    locals: Vec<Type>,
    instructions: Vec<Instruction>,
}

impl DefinedFunctionInstance {
    pub fn new(
        name: String,
        ty: FuncType,
        module_index: ModuleIndex,
        body: FunctionBody,
    ) -> Result<Self> {
        let locals = Vec::new();
        let reader = body.get_locals_reader()?;
        for local in reader {
            let (count, value_type) = local?;
            let elements = iter::repeat(value_type).take(count as usize);
            locals.append(&mut elements.collect());
        }
        let reader = body.get_operators_reader()?;
        let instructions = Vec::new();
        while !reader.eof() {
            let inst = transform_inst(&reader)?;
            instructions.push(inst);
        }
        Ok(Self {
            name,
            ty,
            module_index,
            locals,
            instructions,
        })
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn ty(&self) -> &FuncType {
        &self.ty
    }

    pub fn module_index(&self) -> ModuleIndex {
        self.module_index
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    pub fn locals(&self) -> &[Type] {
        &self.locals
    }

    pub fn inst(&self, index: InstIndex) -> &Instruction {
        &self.instructions[index.0 as usize]
    }
}

pub struct HostFunctionInstance {
    ty: FuncType,
    module_name: String,
    field_name: String,
    code: HostFuncBody,
}

impl HostFunctionInstance {
    pub fn ty(&self) -> &FuncType {
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

    pub fn new(ty: FuncType, module_name: String, field_name: String, code: HostFuncBody) -> Self {
        Self {
            ty,
            module_name,
            field_name,
            code,
        }
    }
}
