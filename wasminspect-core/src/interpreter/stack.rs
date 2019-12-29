use super::func::{FuncIndex, FunctionInstance, InstIndex};
use super::value::Value;

pub enum Label {
    Block,
    Loop(LoopLabel),
    Return,
}

pub struct LoopLabel {
    inst_index: InstIndex,
}

impl Label {
    pub fn new_loop(inst_index: InstIndex) -> Self {
        Self::Loop(LoopLabel { inst_index })
    }
}

#[derive(Clone, Copy)]
pub struct ProgramCounter {
    func_index: FuncIndex,
    inst_index: InstIndex,
}

impl ProgramCounter {
    pub fn new(func_index: FuncIndex, inst_index: InstIndex) -> Self {
        Self {
            func_index,
            inst_index,
        }
    }
}

pub struct CallFrame<'a> {
    pub func: &'a FunctionInstance,
    pub locals: Vec<Value>,
    pub ret_pc: ProgramCounter,
}

impl<'a> CallFrame<'a> {
    pub fn new(func: &'a FunctionInstance, pc: ProgramCounter) -> Self {
        match func {
            FunctionInstance::Defined(ty, module_index, defined) => {
                let local_len = defined.locals().len() + func.ty().params().len();
                let locals = std::iter::repeat(Value::I32(0)).take(local_len).collect();
                Self {
                    func,
                    locals,
                    ret_pc: pc,
                }
            }
            FunctionInstance::Host(ty, host) => panic!(),
        }
    }
    pub fn new_with_locals(
        func: &'a FunctionInstance,
        locals: Vec<Value>,
        pc: ProgramCounter,
    ) -> Self {
        Self {
            func,
            locals: locals,
            ret_pc: pc,
        }
    }
}

struct Stack<'a> {
    values: Vec<Value>,
    labels: Vec<Label>,
    activations: Vec<CallFrame<'a>>,
}
