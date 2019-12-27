use super::environment::{Environment};
use super::module::{DefinedModule};
struct Executor {
}

impl Executor {
    fn new(module: &DefinedModule, env: &Environment) {
    }

    fn init_segments(module: &DefinedModule, env: &Environment) {
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
}