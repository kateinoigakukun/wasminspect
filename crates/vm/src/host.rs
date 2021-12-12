use super::value::Value;

use std::cell::RefCell;
use std::rc::Rc;

use super::address::MemoryAddr;
use super::executor::Trap;
use super::global::GlobalInstance;
use super::memory::MemoryInstance;
use super::module::ModuleIndex;
use super::store::Store;
use super::table::TableInstance;
use wasmparser::FuncType;

type Ref<T> = Rc<RefCell<T>>;

pub struct HostContext<'a> {
    pub mem: &'a mut [u8],
}

pub enum HostValue {
    Func(HostFuncBody),
    Global(Rc<RefCell<dyn GlobalInstance>>),
    Mem(Ref<MemoryInstance>),
    Table(Ref<TableInstance>),
}

impl HostValue {
    pub fn ty(&self) -> &str {
        match self {
            Self::Func(_) => "function",
            Self::Global(_) => "global",
            Self::Mem(_) => "memory",
            Self::Table(_) => "table",
        }
    }
}

type HostCode = dyn Fn(&[Value], &mut Vec<Value>, &mut HostContext, &Store) -> Result<(), Trap>;

pub struct HostFuncBody {
    ty: FuncType,
    code: Box<HostCode>,
}

impl HostFuncBody {
    pub fn new<F>(ty: FuncType, code: F) -> Self
    where
        F: Fn(&[Value], &mut Vec<Value>, &mut HostContext, &Store) -> Result<(), Trap>,
        F: 'static,
    {
        Self {
            ty,
            code: Box::new(code),
        }
    }

    pub fn call(
        &self,
        param: &[Value],
        results: &mut Vec<Value>,
        store: &Store,
        module_index: ModuleIndex,
    ) -> Result<(), Trap> {
        if store.memory_count(module_index) > 0 {
            let mem_addr = MemoryAddr::new_unsafe(module_index, 0);
            let mem = store.memory(mem_addr);
            let mem = &mut mem.borrow_mut();
            let raw_mem = mem.raw_data_mut();
            let mut ctx = HostContext { mem: raw_mem };
            (self.code)(param, results, &mut ctx, store)
        } else {
            let mut ctx = HostContext { mem: &mut [] };
            (self.code)(param, results, &mut ctx, store)
        }
    }

    pub fn ty(&self) -> &FuncType {
        &self.ty
    }
}
