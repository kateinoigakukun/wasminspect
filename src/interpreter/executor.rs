use super::environment::{Environment};
use super::module::*;
use parity_wasm::elements::{Instruction, InitExpr};

pub struct ProgramCounter {
    func_index: Index,
    inst_index: Index,
}

impl ProgramCounter {
    pub fn new(func_index: Index, inst_index: Index) -> Self {
        Self { func_index, inst_index }
    }
}

pub struct CallFrame {
    pub locals: Vec<Value>,
}

impl CallFrame {
    pub fn new(func: &Func) -> Self {
        Self {
            locals: Vec::with_capacity(func.locals().len() + func.func_type().params().len())
        }
    }
}

pub enum Error {
    Panic(String),
    End
}

pub type ExecResult = Result<(), Error>;

pub struct Executor<'a> {
    env: &'a Environment,
    pc: ProgramCounter,
    globals: Vec<Value>,
    stack: Vec<Value>,
    call_stack: Vec<CallFrame>,
}

impl<'a> Executor<'a> {
    pub fn new(initial_args: Vec<Value>, pc: ProgramCounter, env: &'a Environment) -> Self {
        let initial_call_frame = Self::init_initial_call_frame(&initial_args, &pc, env);
        Self {
            env,
            pc: pc,
            globals: Self::init_global(env),
            stack: vec![],
            call_stack: vec![initial_call_frame],
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

    pub fn init_initial_call_frame(args: &Vec<Value>, pc: &ProgramCounter, env: &Environment) -> CallFrame {
        let func = env.get_func(pc.func_index);
        let mut frame = CallFrame::new(func);
        for arg in args.iter() {
            frame.locals.push(*arg);
        }
        frame
    }


    pub fn execute_step(&mut self) -> ExecResult {
        let func = self.env.get_func(self.pc.func_index);
        match func {
            Func::Defined(defined) => {
                self.execute_defined_func_step(defined)
            }
        }
    }

    fn execute_defined_func_step(&mut self, func: &DefinedFunc) -> ExecResult {
        let inst = func.inst(self.pc.inst_index);
        return self.execute_inst(inst);
    }

    fn execute_inst(&mut self, inst: &Instruction) -> ExecResult {
        self.pc.inst_index.inc();
        match *inst {
            Instruction::Unreachable => panic!(),
            Instruction::GetGlobal(index) => {
                let value = self.globals[index as usize];
                self.push(value);
                Ok(())
            }
            Instruction::SetLocal(index) => {
                let value = self.pop();
                self.locals_mut()[index as usize] = value;
                Ok(())
            }
            Instruction::GetLocal(index) => {
                let value = self.locals_mut()[index as usize];
                self.push(value);
                Ok(())
            }
            Instruction::I32Add => {
                let lhs = self.pop();
                let rhs = self.pop();
                if let (Value::I32(lhs), Value::I32(rhs)) = (lhs, rhs) {
                    self.push(Value::I32(lhs + rhs));
                    Ok(())
                } else {
                    debug_assert!(false, format!("Invalid inst"));
                    Err(Error::Panic(format!("Invalid inst")))
                }
            }
            Instruction::End => Err(Error::End),
            _ => {
                debug_assert!(false, format!("{} not supported yet", inst));
                Err(Error::Panic(format!("{} not supported yet", inst)))
            }
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