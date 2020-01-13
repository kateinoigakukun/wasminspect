use super::commands::debugger;
use parity_wasm::elements::Instruction;
use std::cell::RefCell;
use std::rc::Rc;
use wasminspect_vm::{
    CallFrame, Executor, FunctionInstance, InstIndex, ModuleIndex, ProgramCounter, Signal, Store,
    WasmValue,
};
use wasminspect_wasi::instantiate_wasi;

pub struct MainDebugger {
    store: Store,
    executor: Option<Rc<RefCell<Executor>>>,
    module_index: Option<ModuleIndex>,
}

impl MainDebugger {
    pub fn new(file: Option<String>) -> Result<Self, String> {
        let (ctx, wasi_snapshot_preview) = instantiate_wasi();

        let mut store = Store::new();
        store.add_embed_context(Box::new(ctx));
        store.load_host_module("wasi_snapshot_preview1".to_string(), wasi_snapshot_preview);

        let module_index = if let Some(file) = file {
            let parity_module = parity_wasm::deserialize_file(file).unwrap();
            Some(
                store
                    .load_parity_module(None, parity_module)
                    .map_err(|err| format!("{}", err))?,
            )
        } else {
            None
        };
        Ok(Self {
            store,
            executor: None,
            module_index,
        })
    }
}

impl debugger::Debugger for MainDebugger {
    fn instructions(&self) -> Result<(&[Instruction], usize), String> {
        if let Some(ref executor) = self.executor {
            let executor = executor.borrow();
            let insts = executor
                .current_func_insts(&self.store)
                .map_err(|e| format!("Failed to get instructions: {}", e))?;
            Ok((insts, executor.pc.inst_index().0 as usize))
        } else {
            Err(format!("No execution context"))
        }
    }
    fn frame(&self) -> Vec<String> {
        if let Some(ref executor) = self.executor {
            let executor = executor.borrow();
            let frames = executor.stack.peek_frames();
            frames
                .iter()
                .map(|frame| self.store.func_global(frame.exec_addr).name().clone())
                .collect()
        } else {
            Vec::new()
        }
    }
    fn run(&mut self, name: Option<String>) -> Result<Vec<WasmValue>, String> {
        if let Some(module_index) = self.module_index {
            let module = self.store.module(module_index).defined().unwrap();
            let func_addr = if let Some(func_name) = name {
                if let Some(Some(func_addr)) = module.exported_func(func_name.clone()).ok() {
                    func_addr
                } else {
                    return Err(format!("Entry function {} not found", func_name));
                }
            } else if let Some(start_func_addr) = module.start_func_addr() {
                *start_func_addr
            } else {
                if let Some(Some(func_addr)) = module.exported_func("_start".to_string()).ok() {
                    func_addr
                } else {
                    return Err(format!("Entry function _start not found"));
                }
            };
            let func = self
                .store
                .func(func_addr)
                .ok_or(format!("Function not found"))?;
            match func {
                (FunctionInstance::Host(host), _) => {
                    let mut results = Vec::new();
                    match host.code().call(
                        &vec![],
                        &mut results,
                        &self.store,
                        func_addr.module_index(),
                    ) {
                        Ok(_) => return Ok(results),
                        Err(_) => return Err(format!("Failed to execute host func")),
                    }
                }
                (FunctionInstance::Defined(func), exec_addr) => {
                    let (frame, ret_types) = {
                        let ret_types =
                            func.ty().return_type().map(|ty| vec![ty]).unwrap_or(vec![]);
                        let frame = CallFrame::new_from_func(exec_addr, func, vec![], None);
                        (frame, ret_types)
                    };
                    let pc = ProgramCounter::new(func.module_index(), exec_addr, InstIndex::zero());
                    let executor = Rc::new(RefCell::new(Executor::new(frame, ret_types.len(), pc)));
                    self.executor = Some(executor.clone());
                    loop {
                        let result = executor.borrow_mut().execute_step(&self.store);
                        match result {
                            Ok(Signal::Next) => continue,
                            Ok(Signal::End) => match executor.borrow_mut().pop_result(ret_types) {
                                Ok(values) => return Ok(values),
                                Err(err) => return Err(format!("Return value failure {:?}", err)),
                            },
                            Err(err) => return Err(format!("Function exec failure {:?}", err)),
                        }
                    }
                }
            }
        } else {
            Err("No module loaded".to_string())
        }
    }
}
