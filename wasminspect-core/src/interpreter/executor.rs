use super::environment::Environment;
use super::module::*;
use parity_wasm::elements::{InitExpr, Instruction, ValueType};

#[derive(Clone, Copy)]
pub struct ProgramCounter {
    func_index: Index,
    inst_index: Index,
}

impl ProgramCounter {
    pub fn new(func_index: Index, inst_index: Index) -> Self {
        Self {
            func_index,
            inst_index,
        }
    }
}

pub struct CallFrame<'a> {
    pub func: &'a Func,
    pub locals: Vec<Value>,
    pub ret_pc: ProgramCounter,
}

impl<'a> CallFrame<'a> {
    pub fn new(func: &'a Func, pc: ProgramCounter) -> Self {
        Self {
            func,
            locals: Vec::with_capacity(func.locals().len() + func.func_type().params().len()),
            ret_pc: pc,
        }
    }
}

pub enum Label {
    Block, Loop(LoopLabel), Return,
}
pub struct LoopLabel {
    inst_index: u32,
}

impl Label {
    pub fn new_loop(inst_index: u32) -> Self {
        Self::Loop(LoopLabel{ inst_index})
    }
}

#[derive(Debug)]
pub enum ExecError {
    Panic(String),
    NoCallFrame,
}

pub enum ExecResult {
    Ok, End, Err(ExecError)
}

#[derive(Debug)]
pub enum ReturnValError {
    TypeMismatchReturnValue(Value, ValueType),
    NoValue(ValueType),
    NoCallFrame,
}

pub type ReturnValResult = Result<Vec<Value>, ReturnValError>;

pub struct Executor<'a> {
    env: &'a Environment,
    pc: ProgramCounter,
    globals: Vec<Value>,
    stack: Vec<Value>,
    call_stack: Vec<CallFrame<'a>>,
    label_stack: Vec<Label>,
    last_ret_frame: Option<CallFrame<'a>>,
}

impl<'a> Executor<'a> {
    pub fn new(initial_args: Vec<Value>, pc: ProgramCounter, env: &'a Environment) -> Self {
        let initial_call_frame = Self::init_initial_call_frame(&initial_args, &pc, env);
        Self {
            env,
            pc: pc,
            globals: Self::init_global(env),
            stack: vec![],
            label_stack: vec![Label::Return],
            call_stack: vec![initial_call_frame],
            last_ret_frame: None,
        }
    }

    pub fn init_global(env: &Environment) -> Vec<Value> {
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
        for arg in args.iter() {
            frame.locals.push(*arg);
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
        let result = match *inst {
            Instruction::Unreachable => panic!(),
            Instruction::GetGlobal(index) => {
                let value = self.globals[index as usize];
                self.push(value);
                ExecResult::Ok
            }
            Instruction::SetLocal(index) => {
                let value = self.pop();
                self.locals_mut()[index as usize] = value;
                ExecResult::Ok
            }
            Instruction::GetLocal(index) => {
                let value = self.locals_mut()[index as usize];
                self.push(value);
                ExecResult::Ok
            }
            Instruction::I32Add => {
                let lhs = self.pop();
                let rhs = self.pop();
                if let (Value::I32(lhs), Value::I32(rhs)) = (lhs, rhs) {
                    self.push(Value::I32(lhs + rhs));
                    ExecResult::Ok
                } else {
                    debug_assert!(false, format!("Invalid inst"));
                    ExecResult::Err(ExecError::Panic(format!("Invalid inst")))
                }
            }
            Instruction::Block(block) => {
                self.label_stack.push(Label::Block);
                ExecResult::Ok
            }
            Instruction::End => {
                if let Some(Label::Return) = self.label_stack.pop() {
                    if let Some(frame) = self.call_stack.pop() {
                        self.pc = frame.ret_pc;
                        self.last_ret_frame = Some(frame);
                        ExecResult::Ok
                    } else {
                        panic!()
                    }
                } else {
                    ExecResult::Err(ExecError::NoCallFrame)
                }
            },
            _ => {
                debug_assert!(false, format!("{} not supported yet", inst));
                ExecResult::Err(ExecError::Panic(format!("{} not supported yet", inst)))
            }
        };
        if self.label_stack.is_empty() {
            return ExecResult::End;
        } else {
            return result;
        }
    }

    fn locals_mut(&mut self) -> &mut Vec<Value> {
        if let Some(frame) = self.call_stack.last_mut() {
            return &mut frame.locals;
        }
        panic!("No func frame");
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value)
    }
    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap()
    }
}

fn eval_const_expr(init_expr: &InitExpr) -> Value {
    let inst = &init_expr.code()[0];
    match *inst {
        Instruction::I32Const(val) => Value::I32(val),
        Instruction::I64Const(val) => Value::I64(val),
        Instruction::F32Const(val) => panic!(),
        Instruction::F64Const(val) => panic!(),
        Instruction::GetGlobal(index) => panic!(),
        _ => panic!("Unsupported init_expr {}", inst),
    }
}

struct InstOffset(u32);

// struct Thread<'a, 'b> {
//     env: &'a Environment<'b>,
//     value_stack: Vec<Value>,
//     call_stack: Vec<InstOffset>,
//     pc: InstOffset,
// }

// impl<'a, 'b> Thread<'a, 'b> {
//     fn new(env: &'a Environment<'b>) -> Self {
//         Self {
//             env: env,
//             value_stack: vec![],
//             call_stack: vec![],
//             pc: InstOffset(0),
//         }
//     }

//     fn set_pc(&mut self, offset: InstOffset) {
//         self.pc = offset;
//     }

//     fn run(num_instructions: usize) {
//     }

// }
