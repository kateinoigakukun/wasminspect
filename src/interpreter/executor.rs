use super::environment::{Environment};
use super::module::*;
pub struct Executor<'a, 'b> {
    env: &'a Environment<'b>,
    thread: Thread<'a, 'b>,
}

impl<'a, 'b> Executor<'a, 'b> {
    pub fn new(module: &DefinedModule, env: &'a Environment<'b>) -> Self {
        Self {
            env,
            thread: Thread::new(env),
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
        let func = &self.env.get_func(func_index);
        let sig = self.env.get_func_signature(func.sig_index());
        match func {
            Func::Defined(defined_func) => self.run_defined_function(defined_func),
        }
    }

    fn run_defined_function(&mut self, func: &DefinedFunc) {
        self.thread.set_pc(InstOffset(func.offset));
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
}