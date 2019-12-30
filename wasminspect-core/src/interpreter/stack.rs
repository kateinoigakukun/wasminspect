use super::address::FuncAddr;
use super::func::{DefinedFunctionInstance, InstIndex};
use super::value::Value;

pub enum Label {
    If,
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
    func_addr: FuncAddr,
    inst_index: InstIndex,
}

impl ProgramCounter {
    pub fn new(func_addr: FuncAddr, inst_index: InstIndex) -> Self {
        Self {
            func_addr,
            inst_index,
        }
    }

    pub fn func_addr(&self) -> FuncAddr {
        self.func_addr
    }

    pub fn inst_index(&self) -> InstIndex {
        self.inst_index
    }

    pub fn inc_inst_index(&mut self) {
        self.inst_index.0 += 1;
    }

    pub fn loop_jump(&mut self, loop_label: &LoopLabel) {
        self.inst_index = loop_label.inst_index;
    }
}

pub struct CallFrame {
    pub func_addr: FuncAddr,
    pub locals: Vec<Value>,
    pub ret_pc: Option<ProgramCounter>,
}

impl CallFrame {
    pub fn new(
        func_addr: FuncAddr,
        local_len: usize,
        args: Vec<Value>,
        pc: Option<ProgramCounter>,
    ) -> Self {
        let mut locals: Vec<Value> = std::iter::repeat(Value::I32(0)).take(local_len).collect();
        for (i, arg) in args.into_iter().enumerate() {
            locals[i] = arg;
        }
        Self {
            func_addr,
            locals,
            ret_pc: pc,
        }
    }

    pub fn new_from_func(
        func_addr: FuncAddr,
        func: &DefinedFunctionInstance,
        args: Vec<Value>,
        pc: Option<ProgramCounter>,
    ) -> Self {
        let local_len = func.ty().params().len() + func.code().locals().len();
        let mut locals: Vec<Value> = std::iter::repeat(Value::I32(0)).take(local_len).collect();
        for (i, arg) in args.into_iter().enumerate() {
            locals[i] = arg;
        }
        Self {
            func_addr,
            locals,
            ret_pc: pc,
        }
    }

    pub fn set_local(&mut self, index: usize, value: Value) {
        self.locals[index] = value;
    }

    pub fn local(&self, index: usize) -> Value {
        self.locals[index]
    }
}

#[derive(Default)]
pub struct Stack {
    values: Vec<Value>,
    labels: Vec<Label>,
    activations: Vec<CallFrame>,
}

impl Stack {
    pub fn push_value(&mut self, val: Value) {
        self.values.push(val)
    }

    pub fn pop_value(&mut self) -> Option<Value> {
        self.values.pop()
    }

    pub fn peek_last_value(&self) -> Option<&Value> {
        self.values.last()
    }

    pub fn push_label(&mut self, val: Label) {
        self.labels.push(val)
    }

    pub fn pop_label(&mut self) -> Option<Label> {
        self.labels.pop()
    }

    pub fn pop_labels(&mut self, depth: usize) {
        self.labels.truncate(self.labels.len() - depth)
    }

    pub fn peek_last_label(&self) -> Option<&Label> {
        self.labels.last()
    }

    pub fn set_frame(&mut self, frame: CallFrame) {
        self.activations.push(frame)
    }

    pub fn take_current_frame(&mut self) -> Option<CallFrame> {
        self.activations.pop()
    }

    pub fn current_frame(&self) -> &CallFrame {
        &self.activations.last().unwrap()
    }

    fn current_frame_mut(&mut self) -> &mut CallFrame {
        return self.activations.last_mut().unwrap();
    }

    pub fn current_func_addr(&self) -> FuncAddr {
        self.current_frame().func_addr
    }

    pub fn is_over_top_level(&self) -> bool {
        self.labels.is_empty()
    }

    pub fn set_local(&mut self, index: usize, value: Value) {
        self.current_frame_mut().set_local(index, value)
    }
}
