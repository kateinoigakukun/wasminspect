use crate::address::*;
use crate::module::ModuleIndex;

pub struct ExportInstance {
    name: String,
    value: ExternalValue,
}

impl ExportInstance {
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn value(&self) -> &ExternalValue {
        &self.value
    }

    pub fn new_from_entry(entry: wasmparser::Export, module_index: ModuleIndex) -> Self {
        use wasmparser::ExternalKind;
        Self {
            name: entry.field.to_string(),
            value: match entry.kind {
                ExternalKind::Function => {
                    let addr = FuncAddr::new_unsafe(module_index, entry.index as usize);
                    ExternalValue::Func(addr)
                }
                ExternalKind::Global => {
                    let addr = GlobalAddr::new_unsafe(module_index, entry.index as usize);
                    ExternalValue::Global(addr)
                }
                ExternalKind::Memory => {
                    let addr = MemoryAddr::new_unsafe(module_index, entry.index as usize);
                    ExternalValue::Memory(addr)
                }
                ExternalKind::Table => {
                    let addr = TableAddr::new_unsafe(module_index, entry.index as usize);
                    ExternalValue::Table(addr)
                }
                ExternalKind::Type | ExternalKind::Module | ExternalKind::Instance => {
                    panic!("module type is not supported yet")
                }
                ExternalKind::Tag => {
                    panic!("event is not supported yet")
                }
            },
        }
    }
}

#[derive(Debug)]
pub enum ExternalValue {
    Func(FuncAddr),
    Global(GlobalAddr),
    Memory(MemoryAddr),
    Table(TableAddr),
}

impl ExternalValue {
    pub fn ty(&self) -> &str {
        match self {
            Self::Func(_) => "function",
            Self::Global(_) => "global",
            Self::Memory(_) => "memory",
            Self::Table(_) => "table",
        }
    }
}
