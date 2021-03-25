use crate::commands::debugger::{self, Debugger, DebuggerOpts};
use anyhow::{anyhow, Result};
use log::{debug, trace, warn};
use std::collections::HashMap;
use std::rc::Rc;
use std::{cell::RefCell, usize};
use wasminspect_vm::{
    CallFrame, Executor, FuncAddr, FunctionInstance, InstIndex, Instruction, Interceptor,
    MemoryAddr, ModuleIndex, ProgramCounter, Signal, Store, Trap, WasmValue,
};
use wasminspect_wasi::instantiate_wasi;

pub struct MainDebugger {
    store: Store,
    executor: Option<Rc<RefCell<Executor>>>,
    module_index: Option<ModuleIndex>,

    opts: DebuggerOpts,
    function_breakpoints: HashMap<String, debugger::Breakpoint>,
}

impl MainDebugger {
    pub fn load_module(&mut self, module: &[u8]) -> Result<()> {
        if let Err(err) = wasmparser::validate(module) {
            warn!("{}", err);
        }
        self.module_index = Some(self.store.load_module(None, module)?);
        Ok(())
    }
    pub fn new() -> Result<Self> {
        Ok(Self {
            store: Self::instantiate_store(),
            executor: None,
            module_index: None,
            function_breakpoints: HashMap::new(),
            opts: DebuggerOpts::default(),
        })
    }

    pub fn reset_store(&mut self) {
        self.store = Self::instantiate_store();
    }

    fn instantiate_store() -> Store {
        let (ctx, wasi_snapshot_preview) = instantiate_wasi();
        let (_, wasi_unstable) = instantiate_wasi();
        let mut store = Store::new();
        store.add_embed_context(Box::new(ctx));
        store.load_host_module("wasi_snapshot_preview1".to_string(), wasi_snapshot_preview);
        store.load_host_module("wasi_unstable".to_string(), wasi_unstable);
        store
    }

    fn execute_func(&mut self, func_addr: FuncAddr) -> Result<debugger::RunResult> {
        let func = self
            .store
            .func(func_addr)
            .ok_or(anyhow!("Function not found"))?;
        match func {
            (FunctionInstance::Host(host), _) => {
                let mut results = Vec::new();
                match host
                    .code()
                    .call(&vec![], &mut results, &self.store, func_addr.module_index())
                {
                    Ok(_) => return Ok(debugger::RunResult::Finish(results)),
                    Err(_) => return Err(anyhow!("Failed to execute host func")),
                }
            }
            (FunctionInstance::Defined(func), exec_addr) => {
                let ret_types = &func.ty().returns;
                let frame = CallFrame::new_from_func(exec_addr, func, vec![], None);
                let pc = ProgramCounter::new(func.module_index(), exec_addr, InstIndex::zero());
                let executor = Rc::new(RefCell::new(Executor::new(frame, ret_types.len(), pc)));
                self.executor = Some(executor.clone());
                let result = self.process()?;
                match result {
                    Signal::Next => unreachable!(),
                    Signal::Breakpoint => return Ok(debugger::RunResult::Breakpoint),
                    Signal::End => match executor.borrow_mut().pop_result(ret_types.to_vec()) {
                        Ok(values) => {
                            self.executor = None;
                            return Ok(debugger::RunResult::Finish(values));
                        }
                        Err(err) => {
                            self.executor = None;
                            return Err(anyhow!("Return value failure {:?}", err));
                        }
                    },
                }
            }
        }
    }
}

impl debugger::Debugger for MainDebugger {
    fn get_opts(&self) -> DebuggerOpts {
        self.opts.clone()
    }
    fn set_opts(&mut self, opts: DebuggerOpts) {
        self.opts = opts
    }
    fn instructions(&self) -> Result<(&[Instruction], usize)> {
        if let Some(ref executor) = self.executor {
            let executor = executor.borrow();
            let insts = executor.current_func_insts(&self.store)?;
            Ok((insts, executor.pc.inst_index().0 as usize))
        } else {
            Err(anyhow!("No execution context"))
        }
    }

    fn set_breakpoint(&mut self, breakpoint: debugger::Breakpoint) {
        match &breakpoint {
            debugger::Breakpoint::Function { name } => {
                self.function_breakpoints.insert(name.clone(), breakpoint);
            }
        }
    }

    fn stack_values(&self) -> Vec<WasmValue> {
        if let Some(ref executor) = self.executor {
            let executor = executor.borrow();
            let values = executor.stack.peek_values();
            let mut new_values = Vec::<WasmValue>::new();
            for v in values {
                new_values.push(*v);
            }
            new_values
        } else {
            Vec::new()
        }
    }

    fn store(&self) -> &Store {
        &self.store
    }
    fn locals(&self) -> Vec<WasmValue> {
        if let Some(ref executor) = self.executor {
            let executor = executor.borrow();
            executor.stack.current_frame().unwrap().locals.clone()
        } else {
            Vec::new()
        }
    }
    fn current_frame(&self) -> Option<debugger::FunctionFrame> {
        let executor = if let Some(ref executor) = self.executor {
            executor
        } else {
            return None;
        };
        let executor = executor.borrow();
        let frame = executor.stack.current_frame().unwrap();
        let func = self.store.func_global(frame.exec_addr);

        self.module_index.map(|idx| debugger::FunctionFrame {
            module_index: idx,
            argument_count: func.ty().params.len(),
        })
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
    fn memory(&self) -> Result<Vec<u8>> {
        if let Some(ref executor) = self.executor {
            let executor = executor.borrow();
            let frame = executor
                .stack
                .current_frame()
                .map_err(|e| anyhow!("Failed to get current frame: {}", e))?;
            let addr = MemoryAddr::new_unsafe(frame.module_index(), 0);
            Ok(self.store.memory(addr).borrow().raw_data().to_vec())
        } else {
            Ok(vec![])
        }
    }

    fn is_running(&self) -> bool {
        self.executor.is_some()
    }

    fn step(&self, style: debugger::StepStyle) -> Result<Signal> {
        let executor = if let Some(ref executor) = self.executor {
            executor
        } else {
            return Err(anyhow!("No execution context"));
        };
        use debugger::StepStyle::*;

        fn frame_depth(executor: &Executor) -> usize {
            executor.stack.peek_frames().len()
        }
        match style {
            StepInstIn => return Ok(executor.borrow_mut().execute_step(&self.store, self)?),
            StepInstOver => {
                let initial_frame_depth = frame_depth(&executor.borrow());
                let mut last_signal = executor.borrow_mut().execute_step(&self.store, self)?;
                while initial_frame_depth < frame_depth(&executor.borrow()) {
                    last_signal = executor.borrow_mut().execute_step(&self.store, self)?;
                    if let Signal::Breakpoint = last_signal {
                        return Ok(last_signal);
                    }
                }
                return Ok(last_signal);
            }
            StepOut => {
                let initial_frame_depth = frame_depth(&executor.borrow());
                let mut last_signal = executor.borrow_mut().execute_step(&self.store, self)?;
                while initial_frame_depth <= frame_depth(&executor.borrow()) {
                    last_signal = executor.borrow_mut().execute_step(&self.store, self)?;
                    if let Signal::Breakpoint = last_signal {
                        return Ok(last_signal);
                    }
                }
                return Ok(last_signal);
            }
        }
    }

    fn process(&self) -> Result<Signal> {
        let executor = match &self.executor {
            Some(executor) => executor,
            None => return Err(anyhow!("No execution context")),
        };
        loop {
            let result = executor.borrow_mut().execute_step(&self.store, self);
            match result {
                Ok(Signal::Next) => continue,
                Ok(Signal::Breakpoint) | Ok(Signal::End) => return Ok(result.ok().unwrap()),
                Err(err) => return Err(anyhow!("Function exec failure {}", err)),
            }
        }
    }

    fn run(&mut self, name: Option<String>) -> Result<debugger::RunResult> {
        if self.is_running() {
            self.store = Self::instantiate_store();
        }
        let module_index = match self.module_index {
            Some(idx) => idx,
            None => return Err(anyhow!("No module loaded")),
        };
        {
            let module = self.store.module(module_index).defined().unwrap();
            if let Some(start_func_addr) = *module.start_func_addr() {
                return self.execute_func(start_func_addr);
            }
        }

        let module = self.store.module(module_index).defined().unwrap();
        let func_addr = if let Some(func_name) = name {
            if let Some(Some(func_addr)) = module.exported_func(func_name.clone()).ok() {
                func_addr
            } else {
                return Err(anyhow!("Entry function {} not found", func_name));
            }
        } else {
            if let Some(Some(func_addr)) = module.exported_func("_start".to_string()).ok() {
                func_addr
            } else {
                return Err(anyhow!("Entry function _start not found"));
            }
        };
        self.execute_func(func_addr)
    }
}

impl Interceptor for MainDebugger {
    fn invoke_func(&self, name: &String) -> Result<Signal, Trap> {
        debug!("Invoke function '{}'", name);
        let key = self
            .function_breakpoints
            .keys()
            .filter(|k| name.contains(k.clone()))
            .next();
        if let Some(_) = key {
            Ok(Signal::Breakpoint)
        } else {
            Ok(Signal::Next)
        }
    }

    fn execute_inst(&self, inst: &Instruction) {
        trace!("Execute {:?}", inst);
    }

    fn after_store(&self, _addr: usize, _bytes: &[u8]) -> Result<Signal, Trap> {
        Ok(Signal::Next)
    }
}
