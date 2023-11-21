use crate::commands::debugger::{self, Debugger, DebuggerOpts, RawHostModule, RunResult};
use anyhow::{anyhow, Context, Result};
use log::{trace, warn};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{cell::RefCell, usize};
use wasminspect_vm::{
    CallFrame, DefinedModuleInstance, Executor, FuncAddr, FunctionInstance, InstIndex, Instruction,
    Interceptor, MemoryAddr, ModuleIndex, ProgramCounter, Signal, Store, Trap, WasmValue,
};
use wasminspect_wasi::instantiate_wasi;
use wasmparser::WasmFeatures;

type RawModule = Vec<u8>;

pub struct Instance {
    main_module_index: ModuleIndex,
    pub store: Store,
    pub executor: Option<Rc<RefCell<Executor>>>,
}

pub struct MainDebugger {
    pub instance: Option<Instance>,

    main_module: Option<(RawModule, String)>,

    opts: DebuggerOpts,
    preopen_dirs: Vec<(String, String)>,
    envs: Vec<(String, String)>,

    config: wasminspect_vm::Config,
    breakpoints: Breakpoints,
    is_interrupted: Arc<AtomicBool>,
}

#[derive(Default)]
struct Breakpoints {
    function_map: HashMap<String, debugger::Breakpoint>,
    inst_map: HashMap<usize, debugger::Breakpoint>,
}

impl Breakpoints {
    fn should_break_func(&self, name: &str) -> bool {
        // FIXME
        self.function_map
            .keys()
            .any(|k| name.contains(Clone::clone(&k)))
    }

    fn should_break_inst(&self, inst: &Instruction) -> bool {
        self.inst_map.contains_key(&inst.offset)
    }

    fn insert(&mut self, breakpoint: debugger::Breakpoint) {
        match &breakpoint {
            debugger::Breakpoint::Function { name } => {
                self.function_map.insert(name.clone(), breakpoint);
            }
            debugger::Breakpoint::Instruction { inst_offset } => {
                self.inst_map.insert(*inst_offset, breakpoint);
            }
        }
    }
}

impl MainDebugger {
    pub fn load_main_module(&mut self, module: &[u8], name: String) -> Result<()> {
        if let Err(err) = wasmparser::validate(module) {
            warn!("{}", err);
            return Err(err.into());
        }
        self.main_module = Some((module.to_vec(), name));
        Ok(())
    }

    pub fn new(preopen_dirs: Vec<(String, String)>, envs: Vec<(String, String)>) -> Result<Self> {
        let is_interrupted = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&is_interrupted))?;
        Ok(Self {
            instance: None,
            main_module: None,
            opts: DebuggerOpts::default(),
            config: wasminspect_vm::Config {
                features: WasmFeatures::default(),
            },
            breakpoints: Default::default(),
            is_interrupted,
            preopen_dirs,
            envs,
        })
    }

    pub fn main_module(&self) -> Result<&DefinedModuleInstance> {
        if let Some(ref instance) = self.instance {
            let module = match instance.store.module(instance.main_module_index).defined() {
                Some(module) => module,
                None => return Err(anyhow::anyhow!("Main module is not loaded correctly")),
            };
            Ok(module)
        } else {
            Err(anyhow::anyhow!("No instance"))
        }
    }

    fn executor(&self) -> Result<Rc<RefCell<Executor>>> {
        let instance = self.instance()?;
        if let Some(ref executor) = instance.executor {
            Ok(executor.clone())
        } else {
            Err(anyhow::anyhow!("No execution context"))
        }
    }
    fn instance(&self) -> Result<&Instance> {
        if let Some(ref instance) = self.instance {
            Ok(instance)
        } else {
            Err(anyhow::anyhow!("No instance"))
        }
    }

    pub fn func_type(&self, func_addr: FuncAddr) -> Result<wasmparser::FuncType> {
        let (func, _) = self
            .store()?
            .func(func_addr)
            .with_context(|| "Function not found".to_string())?;
        return Ok(func.ty().clone());
    }

    pub fn with_module<T, F: FnOnce(&DefinedModuleInstance) -> Result<T>>(
        &self,
        f: F,
    ) -> Result<T> {
        let module = self.main_module()?;
        f(module)
    }

    pub fn lookup_func(&self, name: &str) -> Result<FuncAddr> {
        self.with_module(|module| {
            if let Ok(Some(func_addr)) = module.exported_func(name) {
                Ok(func_addr)
            } else {
                Err(anyhow!("Entry function {} not found", name))
            }
        })
    }

    pub fn execute_func(
        &mut self,
        func_addr: FuncAddr,
        args: Vec<WasmValue>,
    ) -> Result<debugger::RunResult> {
        let instance = self
            .instance
            .as_mut()
            .with_context(|| "No instance".to_string())?;
        let func = instance
            .store
            .func(func_addr)
            .with_context(|| "Function not found".to_string())?;
        match func {
            (FunctionInstance::Native(host), _) => {
                let mut results = Vec::new();
                match host.code().call(
                    &args,
                    &mut results,
                    &instance.store,
                    func_addr.module_index(),
                ) {
                    Ok(_) => Ok(debugger::RunResult::Finish(results)),
                    Err(_) => Err(anyhow!("Failed to execute host func")),
                }
            }
            (FunctionInstance::Defined(func), exec_addr) => {
                let ret_types = &func.ty().results();
                let frame = CallFrame::new_from_func(exec_addr, func, args, None);
                let pc = ProgramCounter::new(func.module_index(), exec_addr, InstIndex::zero());
                let executor = Rc::new(RefCell::new(Executor::new(frame, ret_types.len(), pc)));
                instance.executor = Some(executor);
                Ok(self.process()?)
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
        let executor = self.executor()?;
        let executor = executor.borrow();
        let insts = executor.current_func_insts(self.store()?)?;
        Ok((insts, executor.pc.inst_index().0 as usize))
    }

    fn set_breakpoint(&mut self, breakpoint: debugger::Breakpoint) {
        self.breakpoints.insert(breakpoint)
    }

    fn stack_values(&self) -> Vec<WasmValue> {
        if let Ok(ref executor) = self.executor() {
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

    fn store(&self) -> Result<&Store> {
        let instance = self.instance()?;
        Ok(&instance.store)
    }

    fn locals(&self) -> Vec<WasmValue> {
        if let Ok(ref executor) = self.executor() {
            let executor = executor.borrow();
            executor.stack.current_frame().unwrap().locals.clone()
        } else {
            Vec::new()
        }
    }
    fn current_frame(&self) -> Option<debugger::FunctionFrame> {
        let executor = if let Ok(executor) = self.executor() {
            executor
        } else {
            return None;
        };
        let executor = executor.borrow();
        let frame = executor.stack.current_frame().unwrap();
        let func = match self.store() {
            Ok(store) => store.func_global(frame.exec_addr),
            Err(_) => return None,
        };

        Some(debugger::FunctionFrame {
            module_index: frame.module_index,
            argument_count: func.ty().params().len(),
        })
    }
    fn frame(&self) -> Vec<String> {
        let instance = if let Ok(instance) = self.instance() {
            instance
        } else {
            return vec![];
        };
        let executor = if let Some(executor) = instance.executor.clone() {
            executor
        } else {
            return vec![];
        };
        let executor = executor.borrow();
        let frames = executor.stack.peek_frames();
        return frames
            .iter()
            .map(|frame| instance.store.func_global(frame.exec_addr).name().clone())
            .collect();
    }
    fn memory(&self) -> Result<Vec<u8>> {
        let instance = self.instance()?;
        let store = &instance.store;
        if store.memory_count(instance.main_module_index) == 0 {
            return Ok(vec![]);
        }
        let addr = MemoryAddr::new_unsafe(instance.main_module_index, 0);
        Ok(store.memory(addr).borrow().raw_data().to_vec())
    }

    fn is_running(&self) -> bool {
        self.executor().is_ok()
    }

    fn step(&self, style: debugger::StepStyle) -> Result<Signal> {
        let store = self.store()?;
        let executor = self.executor()?;
        use debugger::StepStyle::*;

        fn frame_depth(executor: &Executor) -> usize {
            executor.stack.peek_frames().len()
        }
        match style {
            InstIn => {
                return Ok(executor
                    .borrow_mut()
                    .execute_step(store, self, &self.config)?)
            }
            InstOver => {
                let initial_frame_depth = frame_depth(&executor.borrow());
                let mut last_signal =
                    executor
                        .borrow_mut()
                        .execute_step(store, self, &self.config)?;
                while initial_frame_depth < frame_depth(&executor.borrow()) {
                    last_signal = executor
                        .borrow_mut()
                        .execute_step(store, self, &self.config)?;
                    if let Signal::Breakpoint = last_signal {
                        return Ok(last_signal);
                    }
                }
                Ok(last_signal)
            }
            Out => {
                let initial_frame_depth = frame_depth(&executor.borrow());
                let mut last_signal =
                    executor
                        .borrow_mut()
                        .execute_step(store, self, &self.config)?;
                while initial_frame_depth <= frame_depth(&executor.borrow()) {
                    last_signal = executor
                        .borrow_mut()
                        .execute_step(store, self, &self.config)?;
                    if let Signal::Breakpoint = last_signal {
                        return Ok(last_signal);
                    }
                }
                Ok(last_signal)
            }
        }
    }

    fn process(&self) -> Result<RunResult> {
        let store = self.store()?;
        let executor = self.executor()?;
        loop {
            let result = executor
                .borrow_mut()
                .execute_step(store, self, &self.config);
            match result {
                Ok(Signal::Next) => continue,
                Ok(Signal::Breakpoint) => return Ok(RunResult::Breakpoint),
                Ok(Signal::End) => {
                    let pc = executor.borrow().pc;
                    let func = store.func_global(pc.exec_addr());
                    let results = executor
                        .borrow_mut()
                        .pop_result(func.ty().results().to_vec())?;
                    return Ok(RunResult::Finish(results));
                }
                Err(err) => return Err(anyhow!("Function exec failure {}", err)),
            }
        }
    }

    fn run(&mut self, name: Option<&str>, args: Vec<WasmValue>) -> Result<debugger::RunResult> {
        let main_module = self.main_module()?;
        let start_func_addr = *main_module.start_func_addr();
        let func_addr = {
            if let Some(name) = name {
                self.lookup_func(name)?
            } else if let Some(start_func_addr) = start_func_addr {
                start_func_addr
            } else {
                self.lookup_func("_start")?
            }
        };

        self.execute_func(func_addr, args)
    }

    fn instantiate(
        &mut self,
        host_modules: HashMap<String, RawHostModule>,
        wasi_args: Option<&[String]>,
    ) -> Result<()> {
        let mut store = Store::new();
        for (name, host_module) in host_modules {
            store.load_host_module(name, host_module);
        }

        let (main_module, basename) = if let Some((main_module, basename)) = &self.main_module {
            (main_module, basename.clone())
        } else {
            return Err(anyhow::anyhow!("No main module registered"));
        };

        if let Some(wasi_args) = wasi_args {
            let mut wasi_args = wasi_args.to_vec();
            wasi_args.insert(0, basename);

            fn collect_preopen_dirs(
                preopen_dirs: &[(String, String)],
            ) -> anyhow::Result<Vec<(String, cap_std::fs::Dir)>> {
                preopen_dirs
                    .iter()
                    .map(|(guest, host)| {
                        let dir = unsafe { cap_std::fs::Dir::open_ambient_dir(host) }?;
                        Ok((guest.clone(), dir))
                    })
                    .collect::<anyhow::Result<Vec<_>>>()
            }

            let (ctx, wasi_snapshot_preview) = instantiate_wasi(
                &wasi_args,
                collect_preopen_dirs(&self.preopen_dirs)?,
                &self.envs,
            )?;
            let (_, wasi_unstable) = instantiate_wasi(
                &wasi_args,
                collect_preopen_dirs(&self.preopen_dirs)?,
                &self.envs,
            )?;
            store.add_embed_context(Box::new(ctx));
            store.load_host_module("wasi_snapshot_preview1".to_string(), wasi_snapshot_preview);
            store.load_host_module("wasi_unstable".to_string(), wasi_unstable);
        }

        let main_module_index = store.load_module(None, main_module)?;

        self.instance = Some(Instance {
            main_module_index,
            store,
            executor: None,
        });
        Ok(())
    }
}

impl Interceptor for MainDebugger {
    fn invoke_func(
        &self,
        name: &str,
        _executor: &Executor,
        _store: &Store,
    ) -> Result<Signal, Trap> {
        trace!("Invoke function '{}'", name);
        if self.breakpoints.should_break_func(name) {
            Ok(Signal::Breakpoint)
        } else {
            Ok(Signal::Next)
        }
    }

    fn execute_inst(&self, inst: &Instruction) -> Result<Signal, Trap> {
        if self.breakpoints.should_break_inst(inst) {
            Ok(Signal::Breakpoint)
        } else if self.is_interrupted.swap(false, Ordering::Relaxed) {
            println!("Interrupted by signal");
            Ok(Signal::Breakpoint)
        } else {
            Ok(Signal::Next)
        }
    }

    fn after_store(&self, _addr: usize, _bytes: &[u8]) -> Result<Signal, Trap> {
        Ok(Signal::Next)
    }
}
