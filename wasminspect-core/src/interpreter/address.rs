use super::module::ModuleIndex;

// Addresses
#[derive(Clone, Copy)]
pub struct FuncAddr(pub ModuleIndex, pub usize);
#[derive(Clone, Copy)]
pub struct GlobalAddr(pub ModuleIndex, pub usize);
#[derive(Clone, Copy)]
pub struct TableAddr(pub ModuleIndex, pub usize);
#[derive(Clone, Copy)]
pub struct MemoryAddr(pub ModuleIndex, pub usize);
