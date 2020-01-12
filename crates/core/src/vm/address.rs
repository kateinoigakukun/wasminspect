use super::module::ModuleIndex;

// Addresses
#[derive(Clone, Copy, Debug)]
pub struct FuncAddr(pub ModuleIndex, pub usize);
#[derive(Clone, Copy, Debug)]
pub struct GlobalAddr(pub ModuleIndex, pub usize);
#[derive(Clone, Copy, Debug)]
pub struct TableAddr(pub ModuleIndex, pub usize);
#[derive(Clone, Copy, Debug)]
pub struct MemoryAddr(pub ModuleIndex, pub usize);

// Internal representation of global function address to reference same function instances
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExecutableFuncAddr(pub usize);