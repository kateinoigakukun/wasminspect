use crate::linker::{GlobalAddress, LinkableAddress};
use std::cell::RefCell;
use std::rc::Rc;

// Internal representation of global function address to reference same function instances

use crate::func::FunctionInstance;
pub type FuncAddr = LinkableAddress<FunctionInstance>;
pub type ExecutableFuncAddr = GlobalAddress<FunctionInstance>;

use crate::table::TableInstance;
pub type TableAddr = LinkableAddress<Rc<RefCell<TableInstance>>>;
pub type ResolvedTableAddr = GlobalAddress<Rc<RefCell<TableInstance>>>;

use crate::memory::MemoryInstance;
pub type MemoryAddr = LinkableAddress<Rc<RefCell<MemoryInstance>>>;
pub type ResolvedMemoryAddr = GlobalAddress<Rc<RefCell<MemoryInstance>>>;

use crate::global::GlobalInstance;
pub type GlobalAddr = LinkableAddress<Rc<RefCell<GlobalInstance>>>;
pub type ResolvedGlobalAddr = GlobalAddress<Rc<RefCell<GlobalInstance>>>;

use crate::elem::ElementInstance;
pub type ElemAddr = LinkableAddress<Rc<RefCell<ElementInstance>>>;

use crate::data::DataInstance;
pub type DataAddr = LinkableAddress<Rc<RefCell<DataInstance>>>;
