use super::environment::{Environment};
use super::module::*;
use parity_wasm::elements::{Instruction};

pub struct ProgramCounter {
    func_index: Index,
    inst_index: Index,
}

impl ProgramCounter {
    fn new(func_index: Index, inst_index: Index) -> Self {
        Self { func_index, inst_index }
    }
}

pub struct Executor<'a, 'b> {
    env: &'a Environment<'b>,
    module: &'a DefinedModule,
    thread: Thread<'a, 'b>,
    pc: ProgramCounter,
}

impl<'a, 'b> Executor<'a, 'b> {
    pub fn new(module: &'a DefinedModule, pc: ProgramCounter, env: &'a Environment<'b>) -> Self {
        Self {
            env,
            module,
            thread: Thread::new(env),
            pc: pc,
        }
    }

    pub fn execute_step(&mut self) {
        let func = self.env.get_func(self.pc.func_index);
        match func {
            Func::Defined(defined) => {
                self.execute_defined_func_step(defined)
            }
        }
    }

    fn execute_defined_func_step(&mut self, func: &DefinedFunc) {
        let inst = func.inst(self.pc.inst_index);
        self.execute_inst(inst);
    }

    fn execute_inst(&mut self, inst: &Instruction) {
        match inst {
	        Instruction::Unreachable => panic!(),
            _ => panic!("{} not supported yet", inst),
        }
    }
}

enum Value {
    I32(i32), I64(i64)
}

struct InstOffset(u32);

struct Thread<'a, 'b> {
    env: &'a Environment<'b>,
    value_stack: Vec<Value>,
    call_stack: Vec<InstOffset>,
    pc: InstOffset,
}

impl<'a, 'b> Thread<'a, 'b> {
    fn new(env: &'a Environment<'b>) -> Self {
        Self {
            env: env,
            value_stack: vec![],
            call_stack: vec![],
            pc: InstOffset(0),
        }
    }

    fn set_pc(&mut self, offset: InstOffset) {
        self.pc = offset;
    }


    fn run(num_instructions: usize) {
    }

}