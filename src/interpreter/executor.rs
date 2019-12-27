use super::environment::{Environment};
use super::module::*;
use parity_wasm::elements::{Instruction, BrTableData};

struct ProgramCounter {
    func_index: Index,
    inst_index: Index,
}

pub struct Executor<'a, 'b> {
    env: &'a Environment<'b>,
    module: &'a DefinedModule,
    thread: Thread<'a, 'b>,
    pc: ProgramCounter,
}

impl<'a, 'b> Executor<'a, 'b> {
    pub fn new(module: &'a DefinedModule, env: &'a Environment<'b>) -> Self {
        Self {
            env,
            module,
            thread: Thread::new(env),
            pc: ProgramCounter {
                func_index: panic!(),
                inst_index: panic!(),
            },
        }
    }

    pub fn init_segments(module: &DefinedModule, env: &Environment) {
        #[derive(PartialOrd, PartialEq, Debug)]
        enum Pass { Check, Init, }
        impl Iterator for Pass {
            type Item = Pass;
            fn next(&mut self) -> Option<Pass> {
                match self {
                    Pass::Check => Some(Pass::Init),
                    Pass::Init => None,
                }
            }
        }
        // let mut pass = Some(if env.get_features().is_bulk_memory_enabled() {
        //     Pass::Init
        // } else {
        //     Pass::Check
        // });

        // let module = module.get_module();
        // // TODO: bulk
        // if let Some(elem_section) = module.elements_section() {
        //     for current_pass in pass {
        //         for elem_seg in elem_section.entries() {
        //             elem_seg.index();
        //         }
        //     }
        // }
    }

    pub fn run_function(&mut self, func_index: Index) {
        // let func = &self.env.get_func(func_index);
        // let sig = self.env.get_func_signature(func.sig_index());
        // match func {
        //     Func::Defined(defined_func) => self.run_defined_function(defined_func),
        // }
    }

    fn run_defined_function(&mut self, func: &DefinedFunc) {
    }

    fn execute_step(&mut self) {
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

    fn execute_inst(&mut self, inst: Instruction) {
        match inst {
	        Instruction::Unreachable => panic!(),
	        Instruction::Block(blockType) => {
            }
	        Instruction::Loop(blockType) => {}
	        Instruction::If(blockType) => {}
	        Instruction::BrTable(brTableData) => {}
	        Instruction::CallIndirect(u32, u8) => {}
	        Instruction::GetLocal(u32) => {}
	        Instruction::SetLocal(u32) => {}
	        Instruction::TeeLocal(u32) => {}
	        Instruction::GetGlobal(u32) => {}
	        Instruction::SetGlobal(u32) => {}
	        Instruction::I32Load(_, _) => {}
	        Instruction::I64Load(_, _) => {}
	        Instruction::F32Load(_, _) => {}
	        Instruction::F64Load(_, _) => {}
	        Instruction::I32Load8S(_, _) => {}
	        Instruction::I32Load8U(_, _) => {}
	        Instruction::I32Load16S(_, _) => {}
	        Instruction::I32Load16U(_, _) => {}
	        Instruction::I64Load8S(_, _) => {}
	        Instruction::I64Load8U(_, _) => {}
	        Instruction::I64Load16S(_, _) => {}
	        Instruction::I64Load16U(_, _) => {}
	        Instruction::I64Load32S(_, _) => {}
	        Instruction::I64Load32U(_, _) => {}
	        Instruction::I32Store(_, _) => {}
	        Instruction::I64Store(_, _) => {}
	        Instruction::F32Store(_, _) => {}
	        Instruction::F64Store(_, _) => {}
	        Instruction::I32Store8(_, _) => {}
	        Instruction::I32Store16(_, _) => {}
	        Instruction::I64Store8(_, _) => {}
	        Instruction::I64Store16(_, _) => {}
	        Instruction::I64Store32(_, _) => {}
	        Instruction::CurrentMemory(_) => {}
	        Instruction::GrowMemory(_) => {}
	        Instruction::I32Const(_) => {}
	        Instruction::I64Const(_) => {}
	        Instruction::F32Const(_) => {}
            Instruction::F64Const(_) => {}
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