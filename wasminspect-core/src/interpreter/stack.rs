use super::address::FuncAddr;
use super::func::{DefinedFunctionInstance, InstIndex};
use super::module::ModuleIndex;
use super::value::Value;

use std::fmt::{Display, Formatter, Result};

#[derive(Copy)]
pub enum Label {
    If(usize),
    Block(usize),
    Loop(LoopLabel),
}

pub struct LoopLabel {
    inst_index: InstIndex,
}

impl Label {
    pub fn new_loop(inst_index: InstIndex) -> Self {
        Self::Loop(LoopLabel { inst_index })
    }

    pub fn arity(&self) -> usize {
        match self {
            Label::If(arity) => *arity,
            Label::Block(arity) => *arity,
            Label::Loop(_) => 0,
        }
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

#[derive(Clone)]
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

    pub fn module_index(&self) -> ModuleIndex {
        self.func_addr.0
    }
}

pub enum StackValue {
    Value(Value),
    Label(Label),
    Activation(CallFrame),
}

impl StackValue {

    pub fn as_value(&self) -> Option<&Value> {
        match self {
            Self::Value(val) => Some(val),
            _ => None,
        }
    }
    fn as_label(&self) -> Option<&Label> {
        match self {
            Self::Label(val) => Some(val),
            _ => None,
        }
    }
    fn as_activation(&self) -> Option<&CallFrame> {
        match self {
            Self::Activation(val) => Some(val),
            _ => None,
        }
    }

    fn as_activation_mut(&mut self) -> Option<&mut CallFrame> {
        match self {
            Self::Activation(val) => Some(val),
            _ => None,
        }
    }
}

impl Display for StackValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Value(_) => writeln!(f, "StackValue::Value"),
            Self::Label(_) => writeln!(f, "StackValue::Label"),
            Self::Activation(_) => writeln!(f, "StackValue::Activation"),
        }
    }
}

#[derive(Default)]
pub struct Stack {
   stack: Vec<StackValue>,
   current_frame_index: usize,
}

impl Stack {
    pub fn pop_while<F: Fn(&StackValue) -> bool>(&mut self, f: F) -> Vec<StackValue> {
        let mut result = vec![];
        while f(self.latest()) {
            result.push(self.stack.pop().unwrap());
        }
        result
    }

    pub fn current_frame_labels(&self) -> Vec<&Label> {
        self.stack[self.current_frame_index..].iter().filter_map(|v| {
            match v {
                StackValue::Label(label) => Some(label),
                _ => None,
            }
        }).collect()
    }

    pub fn current_label(&self) -> &Label {
        self.stack.iter().filter_map(|v| {
            match v {
                StackValue::Label(label) => Some(label),
                _ => None,
            }
        }).next().unwrap()
    }

    pub fn latest(&self) -> &StackValue {
        self.stack.last().unwrap()
    }
    pub fn push_value(&mut self, val: Value) {
        self.stack.push(StackValue::Value(val))
    }

    pub fn pop_value(&mut self) -> Value {
        match self.stack.pop() {
            Some(StackValue::Value(val)) => val,
            Some(val) => panic!("Unexpected stack value type {}", val),
            None => panic!("Stack is empty"),
        }
    }

    pub fn peek_last_value(&self) -> &Value {
        match self.stack.last() {
            Some(StackValue::Value(val)) => val,
            Some(val) => panic!("Unexpected stack value type {}", val),
            None => panic!("Stack is empty"),
        }
    }

    pub fn push_label(&mut self, val: Label) {
        self.stack.push(StackValue::Label(val))
    }

    pub fn pop_label(&mut self) -> Label {
        match self.stack.pop() {
            Some(StackValue::Label(val)) => val,
            Some(val) => panic!("Unexpected stack value type {}", val),
            None => panic!("Stack is empty"),
        }
    }

    #[deprecated]
    pub fn pop_labels(&mut self, depth: usize) {
        panic!()
        // self.labels.truncate(self.labels.len() - depth)
    }

    pub fn peek_last_label(&self) -> &Label {
        match self.stack.last() {
            Some(StackValue::Label(val)) => val,
            Some(val) => panic!("Unexpected stack value type {}", val),
            None => panic!("Stack is empty"),
        }
    }

    pub fn set_frame(&mut self, frame: CallFrame) {
        self.current_frame_index = self.stack.len();
        self.stack.push(StackValue::Activation(frame))
    }

    pub fn current_frame(&self) -> &CallFrame {
        match &self.stack[self.current_frame_index] {
            StackValue::Activation(val) => val,
            val => panic!("Unexpected stack value type {}", val),
        }
    }

    pub fn pop_frame(&mut self) -> CallFrame {
        match self.stack.pop() {
            Some(StackValue::Activation(val)) => val,
            Some(val) => panic!("Unexpected stack value type {}", val),
            None => panic!("Stack is empty"),
        }
    }

    pub fn current_func_addr(&self) -> FuncAddr {
        self.current_frame().func_addr
    }

    pub fn is_over_top_level(&self) -> bool {
        match self
            .stack
            .iter()
            .filter(|v| match v {
                StackValue::Label(_) => true,
                _ => false,
            })
            .next()
        {
            None => true,
            Some(_) => false,
        }
    }

    pub fn set_local(&mut self, index: usize, value: Value) {
        if let Some(stack) = self.stack.get_mut(self.current_frame_index) {
            if let Some(frame) = stack.as_activation_mut() {
                frame.set_local(index, value);
            }
        }
    }
}
