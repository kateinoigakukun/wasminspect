use super::linker::{GlobalAddress, LinkableAddress};
use std::cell::RefCell;
use std::rc::Rc;

// Internal representation of global function address to reference same function instances

use super::func::FunctionInstance;
pub type FuncAddr = LinkableAddress<FunctionInstance>;
pub type ExecutableFuncAddr = GlobalAddress<FunctionInstance>;

use super::table::TableInstance;
pub type TableAddr = LinkableAddress<Rc<RefCell<TableInstance>>>;
pub type ResolvedTableAddr = GlobalAddress<Rc<RefCell<TableInstance>>>;

use super::memory::MemoryInstance;
pub type MemoryAddr = LinkableAddress<Rc<RefCell<MemoryInstance>>>;
pub type ResolvedMemoryAddr = GlobalAddress<Rc<RefCell<MemoryInstance>>>;

use super::global::GlobalInstance;
pub type GlobalAddr = LinkableAddress<Rc<RefCell<dyn GlobalInstance>>>;
pub type ResolvedGlobalAddr = GlobalAddress<Rc<RefCell<dyn GlobalInstance>>>;

use super::elem::ElementInstance;
pub type ElemAddr = LinkableAddress<Rc<RefCell<ElementInstance>>>;