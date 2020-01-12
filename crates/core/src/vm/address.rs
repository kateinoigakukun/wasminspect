use super::linker::{GlobalAddress, LinkableAddress};
use super::module::ModuleIndex;

// Addresses
#[derive(Clone, Copy, Debug)]
pub struct GlobalAddr(pub ModuleIndex, pub usize);
#[derive(Clone, Copy, Debug)]
pub struct MemoryAddr(pub ModuleIndex, pub usize);

// Internal representation of global function address to reference same function instances

use super::func::FunctionInstance;
pub type FuncAddr = LinkableAddress<FunctionInstance>;
pub type ExecutableFuncAddr = GlobalAddress<FunctionInstance>;

use super::table::TableInstance;
use std::rc::Rc;
use std::cell::RefCell;
pub type TableAddr = LinkableAddress<Rc<RefCell<TableInstance>>>;
pub type ResolvedTableAddr = GlobalAddress<Rc<RefCell<TableInstance>>>;