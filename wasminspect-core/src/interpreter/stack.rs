use super::func::{DefinedFunctionInstance, FuncIndex, InstIndex};
use super::store::FuncAddr;
use super::value::Value;
use parity_wasm::elements::Instruction;

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
}

pub struct CallFrame<'a> {
    pub func: &'a DefinedFunctionInstance,
    pub locals: Vec<Value>,
    pub ret_pc: ProgramCounter,
}

impl<'a> CallFrame<'a> {
    pub fn new(func: &'a DefinedFunctionInstance, args: Vec<Value>, pc: ProgramCounter) -> Self {
        let local_len = func.ty().params().len() + func.code().locals().len();
        let mut locals: Vec<Value> = std::iter::repeat(Value::I32(0)).take(local_len).collect();
        for (i, arg) in args.into_iter().enumerate() {
            locals[i] = arg;
        }
        Self {
            func,
            locals,
            ret_pc: pc,
        }
    }

    pub fn set_local(&mut self, index: usize, value: Value) {
        self.locals[index] = value;
    }
}

#[derive(Default)]
pub struct Stack<'a> {
    values: Vec<Value>,
    labels: Vec<Label>,
    activations: Vec<CallFrame<'a>>,
}

impl<'a> Stack<'a> {
    pub fn push_value(&mut self, val: Value) {
        self.values.push(val)
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

    pub fn set_frame(&mut self, frame: CallFrame<'a>) {
        self.activations.push(frame)
    }

    pub fn take_current_frame(&mut self) -> Option<CallFrame<'a>> {
        self.activations.pop()
    }

    pub fn current_frame(&self) -> &CallFrame<'a> {
        &self.activations.last().unwrap()
    }

    pub fn current_instructions(&self) -> &[Instruction] {
        self.current_frame().func.code().instructions()
    }

    pub fn is_over_top_level(&self) -> bool {
        self.labels.is_empty()
    }
}
