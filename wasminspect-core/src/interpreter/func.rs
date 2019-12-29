use parity_wasm::elements::*;
use super::module::*;

pub struct FunctionInstance {
    r#type: FunctionType,
    // module: ModuleInstance,
}

pub enum Func {
    Defined(DefinedFunc),
}

impl Func {
    fn base(&self) -> &FuncBase {
        match self {
            Func::Defined(defined) => &defined.base,
        }
    }

    pub fn func_type(&self) -> &FunctionType {
        &self.base().func_type
    }

    pub fn locals(&self) -> &Vec<ValueType> {
        &self.base().locals
    }
}

pub struct FuncBase {
    name: String,
    func_type: FunctionType,
    locals: Vec<ValueType>,
}
pub struct DefinedFunc {
    base: FuncBase,
    pub instructions: Vec<Instruction>,
}

impl DefinedFunc {
    pub fn new(
        name: String,
        func_type: FunctionType,
        locals: Vec<ValueType>,
        instructions: Vec<Instruction>,
    ) -> Self {
        Self {
            base: FuncBase {
                name,
                func_type,
                locals: locals,
            },
            instructions: instructions,
        }
    }

    pub fn inst(&self, index: Index) -> &Instruction {
        &self.instructions[index.0 as usize]
    }
}

pub struct ImportedFunc {
    base: FuncBase,
}

impl ImportedFunc {
    fn new(
        name: String,
        func_type: FunctionType,
        locals: Vec<ValueType>,
    ) -> Self {
        Self {
            base: FuncBase {
                name,
                func_type,
                locals: locals,
            },
        }
    }
}
