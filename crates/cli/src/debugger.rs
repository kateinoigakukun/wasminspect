use super::commands::debugger;
use anyhow::Result;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasminspect_vm::{
    CallFrame, Executor, FunctionInstance, InstIndex, Instruction, Interceptor,
    MemoryAddr, ModuleIndex, ProgramCounter, Signal, Store, Trap,
};
use wasminspect_wasi::instantiate_wasi;
use wasmparser::ModuleReader;

pub struct MainDebugger {
    store: Store,
    executor: Option<Rc<RefCell<Executor>>>,
    module_index: Option<ModuleIndex>,

    function_breakpoints: HashMap<String, debugger::Breakpoint>,
}

impl MainDebugger {
    pub fn load_module(&mut self, module: &[u8]) -> Result<()> {
        let mut reader = ModuleReader::new(module)?;
        self.module_index = Some(self.store.load_parity_module(None, &mut reader)?);
        Ok(())
    }
    pub fn new() -> Result<Self> {
        let (ctx, wasi_snapshot_preview) = instantiate_wasi();
        let (_, wasi_unstable) = instantiate_wasi();

        let mut store = Store::new();
        store.add_embed_context(Box::new(ctx));
        store.load_host_module("wasi_snapshot_preview1".to_string(), wasi_snapshot_preview);
        store.load_host_module("wasi_unstable".to_string(), wasi_unstable);

        Ok(Self {
            store,
            executor: None,
            module_index: None,
            function_breakpoints: HashMap::new(),
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

    fn set_breakpoint(&mut self, breakpoint: debugger::Breakpoint) {
        match &breakpoint {
            debugger::Breakpoint::Function { name } => {
                self.function_breakpoints.insert(name.clone(), breakpoint);
            }
        }
    }

    fn stack_values(&self) -> Vec<String> {
        if let Some(ref executor) = self.executor {
            let executor = executor.borrow();
            let values = executor.stack.peek_values();
            values.iter().map(|v| format!("{:?}", v)).collect()
        } else {
            Vec::new()
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
    fn memory(&self) -> Result<Vec<u8>, String> {
        if let Some(ref executor) = self.executor {
            let executor = executor.borrow();
            let frame = executor
                .stack
                .current_frame()
                .map_err(|e| format!("Failed to get current frame: {}", e))?;
            let addr = MemoryAddr::new_unsafe(frame.module_index(), 0);
            Ok(self.store.memory(addr).borrow().raw_data().to_vec())
        } else {
            Ok(vec![])
        }
    }

    fn is_running(&self) -> bool {
        self.executor.is_some()
    }

    fn run(&mut self, name: Option<String>) -> Result<debugger::RunResult, String> {
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
                        Ok(_) => return Ok(debugger::RunResult::Finish(results)),
                        Err(_) => return Err(format!("Failed to execute host func")),
                    }
                }
                (FunctionInstance::Defined(func), exec_addr) => {
                    let ret_types = &func.ty().returns;
                    let frame = CallFrame::new_from_func(exec_addr, func, vec![], None);
                    let pc = ProgramCounter::new(func.module_index(), exec_addr, InstIndex::zero());
                    let executor = Rc::new(RefCell::new(Executor::new(frame, ret_types.len(), pc)));
                    self.executor = Some(executor.clone());
                    loop {
                        let result = executor.borrow_mut().execute_step(&self.store, self);
                        match result {
                            Ok(Signal::Next) => continue,
                            Ok(Signal::Breakpoint) => return Ok(debugger::RunResult::Breakpoint),
                            Ok(Signal::End) => {
                                match executor.borrow_mut().pop_result(ret_types.to_vec()) {
                                    Ok(values) => {
                                        self.executor = None;
                                        return Ok(debugger::RunResult::Finish(values));
                                    }
                                    Err(err) => {
                                        self.executor = None;
                                        return Err(format!("Return value failure {:?}", err));
                                    }
                                }
                            }
                            Err(err) => {
                                let err = Err(format!("Function exec failure {:?}", err));
                                return err;
                            }
                        }
                    }
                }
            }
        } else {
            Err("No module loaded".to_string())
        }
    }
}

impl Interceptor for MainDebugger {
    fn invoke_func(&self, name: &String) -> Result<Signal, Trap> {
        if self.function_breakpoints.contains_key(name) {
            Ok(Signal::Breakpoint)
        } else {
            Ok(Signal::Next)
        }
    }
}
