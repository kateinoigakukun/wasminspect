#![recursion_limit = "1024"]

mod address;
mod config;
mod data;
mod elem;
mod executor;
mod export;
mod func;
mod global;
mod host;
mod inst;
mod instance;
mod interceptor;
mod linker;
mod memory;
mod module;
mod stack;
mod store;
mod table;
mod value;

pub use self::address::*;
pub use self::config::Config;
pub use self::executor::{Executor, Signal};
pub use self::executor::{Trap, WasmError};
pub use self::func::{FunctionInstance, InstIndex};
pub use self::global::DefaultGlobalInstance;
pub use self::host::{HostContext, HostFuncBody, HostValue};
pub use self::inst::{Instruction, InstructionKind};
pub use self::instance::WasmInstance;
pub use self::interceptor::{Interceptor, NopInterceptor};
pub use self::memory::MemoryInstance as HostMemory;
pub use self::module::DefinedModuleInstance;
pub use self::module::ModuleIndex;
pub use self::stack::{CallFrame, ProgramCounter};
pub use self::store::Store;
pub use self::table::TableInstance as HostTable;
pub use self::value::NumVal;
pub use self::value::RefType;
pub use self::value::RefVal;
pub use self::value::Value as WasmValue;
pub use self::value::{F32, F64};

pub const WASM_PAGE_SIZE: usize = 0x10000;

pub fn invoke_func_ignoring_break(
    func_addr: FuncAddr,
    arguments: Vec<WasmValue>,
    store: &mut Store,
    config: &Config,
) -> Result<Vec<WasmValue>, WasmError> {
    match store
        .func(func_addr)
        .ok_or(WasmError::ExecutionError(Trap::UndefinedFunc(func_addr.1)))?
    {
        (FunctionInstance::Host(host), _) => {
            let mut results = Vec::new();
            match host
                .code()
                .call(&arguments, &mut results, store, func_addr.module_index())
            {
                Ok(_) => Ok(results),
                Err(_) => Err(WasmError::HostExecutionError),
            }
        }
        (FunctionInstance::Defined(func), exec_addr) => {
            let (frame, ret_types) = {
                let ret_types = &func.ty().returns;
                let frame = CallFrame::new_from_func(exec_addr, func, arguments, None);
                (frame, ret_types)
            };
            let pc = ProgramCounter::new(func.module_index(), exec_addr, InstIndex::zero());
            let interceptor = NopInterceptor::new();
            let mut executor = Executor::new(frame, ret_types.len(), pc);
            loop {
                let result = executor.execute_step(store, &interceptor, config);
                match result {
                    Ok(Signal::Next) => continue,
                    Ok(Signal::Breakpoint) => continue,
                    Ok(Signal::End) => match executor.pop_result(ret_types.to_vec()) {
                        Ok(values) => return Ok(values),
                        Err(err) => return Err(WasmError::ReturnValueError(err)),
                    },
                    Err(err) => return Err(WasmError::ExecutionError(err)),
                }
            }
        }
    }
}
