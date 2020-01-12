use super::linker::{GlobalAddress, LinkableAddress};
use super::module::ModuleIndex;

// Addresses
#[derive(Clone, Copy, Debug)]
pub struct GlobalAddr(pub ModuleIndex, pub usize);

// Internal representation of global function address to reference same function instances

use super::func::FunctionInstance;
pub type FuncAddr = LinkableAddress<FunctionInstance>;
pub type ExecutableFuncAddr = GlobalAddress<FunctionInstance>;

use super::table::TableInstance;
use std::cell::RefCell;
use std::rc::Rc;
pub type TableAddr = LinkableAddress<Rc<RefCell<TableInstance>>>;
pub type ResolvedTableAddr = GlobalAddress<Rc<RefCell<TableInstance>>>;

use super::memory::MemoryInstance;
pub type MemoryAddr = LinkableAddress<Rc<RefCell<MemoryInstance>>>;
pub type ResolvedMemoryAddr = GlobalAddress<Rc<RefCell<MemoryInstance>>>;
