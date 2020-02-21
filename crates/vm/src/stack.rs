use super::address::*;
use super::func::{DefinedFunctionInstance, InstIndex};
use super::module::ModuleIndex;
use super::value::Value;

#[derive(Debug)]
pub enum StackValueType {
    Label,
    Value,
    Activation,
}

const DEFAULT_CALL_STACK_LIMIT: usize = 1024;

#[derive(Debug)]
pub enum Error {
    PopEmptyStack,
    MismatchStackValueType(
        /* expected: */ StackValueType,
        /* actual: */ StackValueType,
    ),
    NoCallFrame,
    Overflow,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Overflow => write!(f, "call stack exhausted"),
            _ => write!(f, "{:?}", self),
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug)]
pub enum Label {
    If(usize),
    Block(usize),
    Loop(LoopLabel),
    Return(usize),
}

#[derive(Clone, Copy, Debug)]
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
            Label::Return(arity) => *arity,
        }
    }
}

#[derive(Clone, Copy)]
pub struct ProgramCounter {
    module_index: ModuleIndex,
    exec_addr: ExecutableFuncAddr,
    inst_index: InstIndex,
}

impl ProgramCounter {
    pub fn new(
        module_index: ModuleIndex,
        exec_addr: ExecutableFuncAddr,
        inst_index: InstIndex,
    ) -> Self {
        Self {
            module_index,
            exec_addr,
            inst_index,
        }
    }

    pub fn module_index(&self) -> ModuleIndex {
        self.module_index
    }

    pub fn exec_addr(&self) -> ExecutableFuncAddr {
        self.exec_addr
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
    pub module_index: ModuleIndex,
    pub locals: Vec<Value>,
    pub ret_pc: Option<ProgramCounter>,

    // Only for debug use
    pub exec_addr: ExecutableFuncAddr,
}

impl CallFrame {
    fn new(
        module_index: ModuleIndex,
        exec_addr: ExecutableFuncAddr,
        local_inits: &Vec<Value>,
        args: Vec<Value>,
        pc: Option<ProgramCounter>,
    ) -> Self {
        let mut locals = local_inits.clone();
        for (i, arg) in args.into_iter().enumerate() {
            locals[i] = arg;
        }
        Self {
            module_index,
            exec_addr,
            locals,
            ret_pc: pc,
        }
    }

    pub fn new_from_func(
        exec_addr: ExecutableFuncAddr,
        func: &DefinedFunctionInstance,
        args: Vec<Value>,
        pc: Option<ProgramCounter>,
    ) -> Self {
        Self::new(func.module_index(), exec_addr, &func.cached_local_inits, args, pc)
    }

    pub fn set_local(&mut self, index: usize, value: Value) {
        self.locals[index] = value;
    }

    pub fn local(&self, index: usize) -> Value {
        self.locals[index]
    }

    pub fn module_index(&self) -> ModuleIndex {
        self.module_index
    }
}

pub enum StackValue {
    Value(Value),
    Label(Label),
    Activation(CallFrame),
}

impl StackValue {
    pub fn value_type(&self) -> StackValueType {
        match self {
            Self::Value(_) => StackValueType::Value,
            Self::Label(_) => StackValueType::Label,
            Self::Activation(_) => StackValueType::Activation,
        }
    }
    pub fn as_value(self) -> Result<Value> {
        match self {
            Self::Value(val) => Ok(val),
            _ => Err(Error::MismatchStackValueType(
                StackValueType::Value,
                self.value_type(),
            )),
        }
    }
    fn as_label(self) -> Result<Label> {
        match self {
            Self::Label(val) => Ok(val),
            _ => Err(Error::MismatchStackValueType(
                StackValueType::Label,
                self.value_type(),
            )),
        }
    }
    fn as_activation(self) -> Result<CallFrame> {
        match self {
            Self::Activation(val) => Ok(val),
            _ => Err(Error::MismatchStackValueType(
                StackValueType::Activation,
                self.value_type(),
            )),
        }
    }

    fn as_activation_ref(&self) -> Result<&CallFrame> {
        match self {
            Self::Activation(val) => Ok(val),
            _ => Err(Error::MismatchStackValueType(
                StackValueType::Activation,
                self.value_type(),
            )),
        }
    }

    fn as_activation_mut(&mut self) -> Result<&mut CallFrame> {
        match self {
            Self::Activation(val) => Ok(val),
            _ => Err(Error::MismatchStackValueType(
                StackValueType::Activation,
                self.value_type(),
            )),
        }
    }
}

impl std::fmt::Display for StackValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    frame_index: Vec<usize>,
}

// Debugger
impl Stack {
    pub fn peek_frames(&self) -> Vec<&CallFrame> {
        self.stack
            .iter()
            .filter_map(|v| match v {
                StackValue::Activation(f) => Some(f),
                _ => None,
            })
            .collect()
    }

    pub fn peek_values(&self) -> Vec<&Value> {
        self.stack
            .iter()
            .filter_map(|v| match v {
                StackValue::Value(v) => Some(v),
                _ => None,
            })
            .collect()
    }
}

impl Stack {
    pub fn pop_while<F: Fn(&StackValue) -> bool>(&mut self, f: F) -> Vec<StackValue> {
        let mut result = vec![];
        while f(self.latest()) {
            result.push(self.stack.pop().unwrap());
        }
        result
    }

    pub fn current_frame_index(&self) -> Result<usize> {
        self.frame_index
            .last()
            .map(|v| *v)
            .ok_or(Error::NoCallFrame)
    }

    pub fn is_func_top_level(&self) -> Result<bool> {
        match self.stack[self.current_frame_index()?..]
            .iter()
            .filter(|v| match v {
                StackValue::Label(Label::Return(_)) => false,
                StackValue::Label(_) => true,
                _ => false,
            })
            .next()
        {
            Some(_) => Ok(false),
            None => Ok(true),
        }
    }

    pub fn current_frame_labels(&self) -> Result<Vec<&Label>> {
        Ok(self.stack[self.current_frame_index()?..]
            .iter()
            .filter_map(|v| match v {
                StackValue::Label(label) => Some(label),
                _ => None,
            })
            .collect())
    }

    fn latest(&self) -> &StackValue {
        self.stack.last().unwrap()
    }
    pub fn push_value(&mut self, val: Value) {
        self.stack.push(StackValue::Value(val))
    }

    pub fn pop_value(&mut self) -> Result<Value> {
        match self.stack.pop() {
            Some(val) => val.as_value(),
            None => Err(Error::PopEmptyStack),
        }
    }

    pub fn push_label(&mut self, val: Label) {
        self.stack.push(StackValue::Label(val))
    }

    pub fn pop_label(&mut self) -> Result<Label> {
        match self.stack.pop() {
            Some(val) => val.as_label(),
            None => Err(Error::PopEmptyStack),
        }
    }

    pub fn set_frame(&mut self, frame: CallFrame) -> Result<()> {
        if self.frame_index.len() > DEFAULT_CALL_STACK_LIMIT {
            return Err(Error::Overflow);
        }
        self.frame_index.push(self.stack.len());
        self.stack.push(StackValue::Activation(frame));
        Ok(())
    }

    pub fn current_frame(&self) -> Result<&CallFrame> {
        self.stack[self.current_frame_index()?].as_activation_ref()
    }

    pub fn pop_frame(&mut self) -> Result<CallFrame> {
        match self.stack.pop() {
            Some(val) => {
                self.frame_index.pop();
                val.as_activation()
            }
            None => Err(Error::PopEmptyStack),
        }
    }

    pub fn is_over_top_level(&self) -> bool {
        self.frame_index.is_empty()
    }

    pub fn set_local(&mut self, index: usize, value: Value) -> Result<()> {
        let size = self.current_frame_index()?;
        if let Some(stack) = self.stack.get_mut(size) {
            let frame = stack.as_activation_mut()?;
            frame.set_local(index, value);
            Ok(())
        } else {
            Err(Error::NoCallFrame)
        }
    }
}

impl std::fmt::Debug for Stack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "┌-------------------------┐")?;
        writeln!(f, "|--------- Stack ---------|")?;
        writeln!(f, "|     ty     |     val    |")?;
        for v in &self.stack {
            match v {
                StackValue::Value(value) => {
                    writeln!(f, "| Value({:?})|{:?}|", value.value_type(), value)?;
                }
                StackValue::Label(label) => {
                    writeln!(f, "| Label |{:?}|", label)?;
                }
                StackValue::Activation(_) => {
                    writeln!(f, "| Frame |   -   |")?;
                }
            }
        }
        writeln!(f, "└-------------------------┘")?;
        Ok(())
    }
}
