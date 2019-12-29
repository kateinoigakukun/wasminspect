use super::module::*;
use super::func::*;
use super::stack::*;
use super::value::*;
use super::store::*;
use parity_wasm::elements::{InitExpr, Instruction, ValueType};

use std::convert::{TryFrom, TryInto};

#[derive(Debug)]
pub enum ExecError {
    Panic(String),
    NoCallFrame,
}

pub enum ExecSuccess {
    Next,
    End,
}

pub type ExecResult = Result<ExecSuccess, ExecError>;

#[derive(Debug)]
pub enum ReturnValError {
    TypeMismatchReturnValue(Value, ValueType),
    NoValue(ValueType),
    NoCallFrame,
}

pub type ReturnValResult = Result<Vec<Value>, ReturnValError>;

pub struct Executor<'a> {
    store: Store,
    pc: ProgramCounter,
    globals: Vec<Value>,
    stack: Vec<Value>,
    call_stack: Vec<CallFrame<'a>>,
    label_stack: Vec<Label>,
    last_ret_frame: Option<CallFrame<'a>>,
}

impl<'a> Executor<'a> {
    pub fn new(initial_args: Vec<Value>, pc: ProgramCounter, store: Store) -> Self {
        let initial_call_frame = Self::init_initial_call_frame(&initial_args, &pc, store);
        Self {
            store,
            pc: pc,
            globals: Self::init_global(&store),
            stack: vec![],
            label_stack: vec![Label::Return],
            call_stack: vec![initial_call_frame],
            last_ret_frame: None,
        }
    }

    pub fn init_global(env: &Store) -> Vec<Value> {
        let mut globals = Vec::with_capacity(env.modules().len());
        for module in env.modules() {
            match module {
                Module::Defined(defined) => {
                    for entry in defined.globals() {
                        globals.push(eval_const_expr(entry.init_expr()));
                    }
                }
            }
        }
        globals
    }

    pub fn init_initial_call_frame<'b>(
        args: &Vec<Value>,
        pc: &ProgramCounter,
        env: &'b Environment,
    ) -> CallFrame<'b> {
        let func = env.get_func(pc.func_index);
        let mut frame = CallFrame::new(func, pc.clone());
        for (i, arg) in args.iter().enumerate() {
            frame.locals[i] = *arg;
        }
        frame
    }

    pub fn peek_result(&self) -> ReturnValResult {
        let frame = match &self.last_ret_frame {
            Some(frame) => frame,
            None => return Err(ReturnValError::NoCallFrame),
        };
        let return_ty = frame.func.func_type().return_type();
        // TODO: support multi value
        match (self.stack.last(), return_ty) {
            (Some(val), Some(ty)) => {
                if val.value_type() == ty {
                    return Ok(vec![val.clone()]);
                } else {
                    return Err(ReturnValError::TypeMismatchReturnValue(val.clone(), ty));
                }
            }
            (_, None) => return Ok(vec![]),
            (None, Some(ty)) => Err(ReturnValError::NoValue(ty)),
        }
    }

    pub fn current_func_insts(&self) -> &Vec<Instruction> {
        if let Some(frame) = self.call_stack.last() {
            match frame.func {
                Func::Defined(defined) => &defined.instructions,
            }
        } else {
            panic!();
        }
    }

    pub fn execute_step(&mut self) -> ExecResult {
        let func = self.env.get_func(self.pc.func_index);
        match func {
            Func::Defined(defined) => self.execute_defined_func_step(defined),
        }
    }

    fn execute_defined_func_step(&mut self, func: &DefinedFunc) -> ExecResult {
        let inst = func.inst(self.pc.inst_index);
        return self.execute_inst(inst);
    }

    fn execute_inst(&mut self, inst: &Instruction) -> ExecResult {
        self.pc.inst_index.inc();
        println!("{}", inst.clone());
        let result = match *inst {
            Instruction::Unreachable => panic!(),
            Instruction::GetGlobal(index) => {
                let value = self.globals[index as usize];
                self.push(value);
                Ok(ExecSuccess::Next)
            }
            Instruction::SetLocal(index) => {
                let value = self.pop();
                self.locals_mut()?[index as usize] = value;
                Ok(ExecSuccess::Next)
            }
            Instruction::GetLocal(index) => {
                let value = self.locals_mut()?[index as usize];
                self.push(value);
                Ok(ExecSuccess::Next)
            }
            Instruction::I32Const(val) => {
                self.stack.push(Value::I32(val));
                Ok(ExecSuccess::Next)
            }
            Instruction::I32Add => self.int_op::<i32, _>(|a, b| Value::I32(a + b)),
            Instruction::I32LtS => {
                self.int_op::<i32, _>(|a, b| Value::I32(if a < b { 1 } else { 0 }))
            }
            Instruction::Block(_) => {
                self.label_stack.push(Label::Block);
                Ok(ExecSuccess::Next)
            }
            Instruction::Loop(_) => {
                self.label_stack.push(Label::new_loop(self.pc.inst_index));
                Ok(ExecSuccess::Next)
            }
            Instruction::BrIf(depth) => {
                let val = self.pop();
                if val != Value::I32(0) {
                    self.branch(depth);
                }
                Ok(ExecSuccess::Next)
            }
            Instruction::Br(depth) => {
                self.branch(depth);
                Ok(ExecSuccess::Next)
            }
            Instruction::Call(func_index) => {
                let func_index = Index::try_from(func_index as usize).unwrap();
                let func = self.env.get_func(func_index);
                let pc = ProgramCounter::new(func_index, Index::zero());
                let mut locals: Vec<Value> = Vec::new();

                for _ in func.func_type().params() {
                    locals.push(self.pop());
                }
                locals.reverse();

                let frame = CallFrame::new_with_locals(func, locals, self.pc);
                self.call_stack.push(frame);
                self.label_stack.push(Label::Return);

                self.pc = pc;
                Ok(ExecSuccess::Next)
            }
            Instruction::Return => {
                if let Some(Label::Return) = self.label_stack.pop() {
                    let frame = self.call_stack.pop().unwrap();
                    self.pc = frame.ret_pc;
                    self.last_ret_frame = Some(frame);
                    Ok(ExecSuccess::Next)
                } else {
                    panic!();
                }
            }
            Instruction::End => {
                if let Some(Label::Return) = self.label_stack.pop() {
                    if let Some(frame) = self.call_stack.pop() {
                        self.pc = frame.ret_pc;
                        self.last_ret_frame = Some(frame);
                        Ok(ExecSuccess::Next)
                    } else {
                        Err(ExecError::NoCallFrame)
                    }
                } else {
                    Ok(ExecSuccess::Next)
                }
            }
            _ => {
                debug_assert!(false, format!("{} not supported yet", inst));
                ExecResult::Err(ExecError::Panic(format!("{} not supported yet", inst)))
            }
        };
        if self.label_stack.is_empty() {
            return Ok(ExecSuccess::End);
        } else {
            return result;
        }
    }

    fn locals_mut(&mut self) -> Result<&mut Vec<Value>, ExecError> {
        if let Some(frame) = self.call_stack.last_mut() {
            return Ok(&mut frame.locals);
        } else {
            debug_assert!(false, "No func frame");
            Err(ExecError::NoCallFrame)
        }
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value)
    }
    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap()
    }

    fn pop_as<T: TryFrom<Value>>(&mut self) -> T {
        let value = self.pop();
        match T::try_from(value) {
            Ok(val) => val,
            Err(_) => panic!(),
        }
    }

    fn branch(&mut self, depth: u32) {
        self.label_stack
            .truncate(self.label_stack.len() - depth as usize);
        match self.label_stack.last().unwrap() {
            Label::Loop(loop_label) => self.pc.inst_index = loop_label.inst_index,
            Label::Block => {
                let mut depth = depth + 1;
                loop {
                    let index: usize = self.pc.inst_index.try_into().unwrap();
                    match self.current_func_insts()[index] {
                        Instruction::End => depth -= 1,
                        Instruction::Block(_) => depth += 1,
                        Instruction::If(_) => depth += 1,
                        Instruction::Loop(_) => depth += 1,
                        _ => (),
                    }
                    if depth == 0 {
                        break;
                    }
                    self.pc.inst_index.inc();
                }
            }
            Label::Return => panic!(),
        }
    }

    fn int_op<T: TryFrom<Value>, F: Fn(T, T) -> Value>(&mut self, f: F) -> ExecResult {
        let rhs = self.pop_as();
        let lhs = self.pop_as();
        self.push(f(lhs, rhs));
        Ok(ExecSuccess::Next)
    }
}

fn eval_const_expr(init_expr: &InitExpr) -> Value {
    let inst = &init_expr.code()[0];
    match *inst {
        Instruction::I32Const(val) => Value::I32(val),
        Instruction::I64Const(val) => Value::I64(val),
        Instruction::F32Const(_) => panic!(),
        Instruction::F64Const(_) => panic!(),
        Instruction::GetGlobal(_) => panic!(),
        _ => panic!("Unsupported init_expr {}", inst),
    }
}
