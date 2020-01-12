use super::module::ModuleIndex;
use super::linker::{GlobalAddress, LinkableAddress};

// Addresses
#[derive(Clone, Copy, Debug)]
pub struct GlobalAddr(pub ModuleIndex, pub usize);
#[derive(Clone, Copy, Debug)]
pub struct TableAddr(pub ModuleIndex, pub usize);
#[derive(Clone, Copy, Debug)]
pub struct MemoryAddr(pub ModuleIndex, pub usize);

// Internal representation of global function address to reference same function instances

use super::func::FunctionInstance;
pub type FuncAddr = LinkableAddress<FunctionInstance>;
pub type ExecutableFuncAddr = GlobalAddress<FunctionInstance>;