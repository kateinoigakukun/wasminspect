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
    stack: Stack<'a>,
    last_ret_frame: Option<CallFrame<'a>>,
}

impl<'a> Executor<'a> {
    pub fn new(initial_args: Vec<Value>, pc: ProgramCounter, store: Store) -> Self {
        Self {
            store,
            pc: pc,
            stack: Stack::default(),
            last_ret_frame: None,
        }
    }

    pub fn peek_result(&self) -> ReturnValResult {
        let frame = match &self.last_ret_frame {
            Some(frame) => frame,
            None => return Err(ReturnValError::NoCallFrame),
        };
        let return_ty = frame.func.ty().return_type();
        // TODO: support multi value
        match (self.stack.peek_last_value(), return_ty) {
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

    pub fn current_func_insts(&self) -> &[Instruction] {
        self.stack.current_instructions()
    }

    pub fn execute_step(&mut self) -> ExecResult {
        let func = self.store.func(self.pc.func_addr());
        match func {
            FunctionInstance::Defined(defined) => self.execute_defined_func_step(defined),
        }
    }

    fn execute_defined_func_step(&mut self, func: &DefinedFunctionInstance) -> ExecResult {
        let inst = func.code().inst(self.pc.inst_index());
        return self.execute_inst(inst, func.module_index());
    }

    fn execute_inst(&mut self, inst: &Instruction, module_index: ModuleIndex) -> ExecResult {
        self.pc.inc_inst_index();
        println!("{}", inst.clone());
        let result = match *inst {
            Instruction::Unreachable => panic!(),
            Instruction::GetGlobal(index) => {
                let addr = GlobalAddr(module_index, index as usize);
                let global = self.store.global(addr);
                self.push(global.value());
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
                self.stack.push_value(Value::I32(val));
                Ok(ExecSuccess::Next)
            }
            Instruction::I32Add => self.int_op::<i32, _>(|a, b| Value::I32(a + b)),
            Instruction::I32LtS => {
                self.int_op::<i32, _>(|a, b| Value::I32(if a < b { 1 } else { 0 }))
            }
            Instruction::Block(_) => {
                self.stack.push_label(Label::Block);
                Ok(ExecSuccess::Next)
            }
            Instruction::Loop(_) => {
                self.stack.push_label(Label::new_loop(self.pc.inst_index()));
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
                let addr = FuncAddr(module_index, func_index as usize);
                let func = self.store.func(addr);
                let pc = ProgramCounter::new(addr, InstIndex::zero());
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

pub fn eval_const_expr(init_expr: &InitExpr) -> Value {
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
