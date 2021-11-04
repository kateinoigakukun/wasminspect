#![recursion_limit = "1024"]

mod address;
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
mod config;

pub use self::address::*;
pub use self::executor::{simple_invoke_func, Executor, Signal};
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
pub use self::value::Value as WasmValue;
pub use self::value::NumVal;
pub use self::value::RefType;
pub use self::config::Config;

pub const WASM_PAGE_SIZE: usize = 0x10000;
