use super::module::ModuleIndex;

// Addresses
#[derive(Clone, Copy)]
pub struct FuncAddr(pub ModuleIndex, pub usize);
pub struct GlobalAddr(pub ModuleIndex, pub usize);
pub struct TableAddr(pub ModuleIndex, pub usize);
