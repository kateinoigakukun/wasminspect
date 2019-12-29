use super::module::ModuleIndex;
use super::store::{FuncAddr, GlobalAddr};
use parity_wasm::elements::Internal;

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

    pub fn new_from_parity_entry(
        parity_entry: parity_wasm::elements::ExportEntry,
        module_index: ModuleIndex,
    ) -> Self {
        Self {
            name: parity_entry.field().to_string(),
            value: match parity_entry.internal() {
                Internal::Function(func_index) => {
                    let addr = FuncAddr(module_index, *func_index as usize);
                    ExternalValue::Func(addr)
                }
                _ => panic!()
            },
        }
    }
}

pub enum ExternalValue {
    Func(FuncAddr),
    Global(GlobalAddr),
}
