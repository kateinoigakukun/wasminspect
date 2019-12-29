use super::func::*;
use super::module::*;
use super::stack::*;
use super::store::*;
use super::value::*;
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
        let func = self.store.func(self.pc.func_addr()).defined().unwrap();
        let module_index = func.module_index().clone();
        let inst = func.code().inst(self.pc.inst_index()).clone();
        return self.execute_inst(&inst, module_index);
    }

    fn execute_inst(&mut self, inst: &Instruction, module_index: ModuleIndex) -> ExecResult {
        self.pc.inc_inst_index();
        println!("{}", inst.clone());
        let result = match inst {
            Instruction::Unreachable => panic!(),
            Instruction::GetGlobal(index) => {
                let addr = GlobalAddr(module_index, *index as usize);
                let global = self.store.global(addr);
                self.stack.push_value(global.value());
                Ok(ExecSuccess::Next)
            }
            Instruction::SetLocal(index) => {
                let value = self.stack.pop_value().unwrap();
                self.stack.set_local(*index as usize, value);
                Ok(ExecSuccess::Next)
            }
            Instruction::GetLocal(index) => {
                let value = self.stack.current_frame().local(*index as usize);
                self.stack.push_value(value);
                Ok(ExecSuccess::Next)
            }
            Instruction::I32Const(val) => {
                self.stack.push_value(Value::I32(*val));
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
                let val = self.stack.pop_value().unwrap();
                if val != Value::I32(0) {
                    self.branch(*depth);
                }
                Ok(ExecSuccess::Next)
            }
            Instruction::Br(depth) => {
                self.branch(*depth);
                Ok(ExecSuccess::Next)
            }
            Instruction::Call(func_index) => {
                let addr = FuncAddr(module_index, *func_index as usize);
                let func = self.store.func(addr);
                let pc = ProgramCounter::new(addr, InstIndex::zero());
                match func {
                    FunctionInstance::Defined(defined) => {
                        let mut args = Vec::new();
                        for _ in func.ty().params() {
                            args.push(self.stack.pop_value().unwrap());
                        }
                        args.reverse();
                        let frame = CallFrame::new(defined, args, pc);
                        // self.stack.set_frame(frame);
                        // self.stack.push_label(Label::Return);
                        self.pc = pc;
                        Ok(ExecSuccess::Next)
                    }
                    FunctionInstance::Host(_, _) => panic!(),
                }
            }
            Instruction::Return => {
                if let Some(Label::Return) = self.stack.pop_label() {
                    let frame = self.stack.take_current_frame().unwrap();
                    self.pc = frame.ret_pc;
                    self.last_ret_frame = Some(frame);
                    Ok(ExecSuccess::Next)
                } else {
                    panic!();
                }
            }
            Instruction::End => {
                if let Some(Label::Return) = self.stack.pop_label() {
                    if let Some(frame) = self.stack.take_current_frame() {
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
        if self.stack.is_over_top_level() {
            return Ok(ExecSuccess::End);
        } else {
            return result;
        }
    }

    fn pop_as<T: TryFrom<Value>>(&mut self) -> T {
        let value = self.stack.pop_value().unwrap();
        match T::try_from(value) {
            Ok(val) => val,
            Err(_) => panic!(),
        }
    }

    fn branch(&mut self, depth: u32) {
        self.stack.pop_labels(depth as usize);
        match self.stack.peek_last_label().unwrap() {
            Label::Loop(loop_label) => self.pc.loop_jump(loop_label),
            Label::Block => {
                let mut depth = depth + 1;
                loop {
                    let index = self.pc.inst_index().0 as usize;
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
                    self.pc.inc_inst_index();
                }
            }
            Label::Return => panic!(),
        }
    }

    fn int_op<T: TryFrom<Value>, F: Fn(T, T) -> Value>(&mut self, f: F) -> ExecResult {
        let rhs = self.pop_as();
        let lhs = self.pop_as();
        self.stack.push_value(f(lhs, rhs));
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
