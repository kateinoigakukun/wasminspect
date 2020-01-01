use super::address::FuncAddr;
use super::module::ModuleInstance;
use super::store::Store;
use parity_wasm::elements::ResizableLimits;

pub enum TableInstance {
    Defined(DefinedTableInstance),
    External(ExternalTableInstance),
}

impl TableInstance {
    pub fn initialize(&mut self, offset: usize, data: Vec<FuncAddr>, store: &mut Store) {
        match self {
            Self::Defined(defined) => defined.initialize(offset, data),
            Self::External(external) => {
                let module = store.module_by_name(external.module_name.clone());
                match module {
                    ModuleInstance::Defined(defined) => {
                        let addr = defined.exported_table(external.name.clone());
                        store
                            .table(addr.unwrap())
                            .borrow_mut()
                            .initialize(offset, data, store)
                    }
                    ModuleInstance::Host(host) => host
                        .table_by_name(external.name.clone())
                        .unwrap()
                        .borrow_mut()
                        .initialize(offset, data),
                }
            }
        }
    }

    pub fn buffer_len(&self, store: &Store) -> usize {
        match self {
            Self::Defined(defined) => defined.buffer_len(),
            Self::External(external) => {
                let module = store.module_by_name(external.module_name.clone());
                match module {
                    ModuleInstance::Defined(defined) => {
                        let addr = defined.exported_table(external.name.clone());
                        store.table(addr.unwrap()).borrow().buffer_len(store)
                    }
                    ModuleInstance::Host(host) => host
                        .table_by_name(external.name.clone())
                        .unwrap()
                        .borrow()
                        .buffer_len(),
                }
            }
        }
    }

    pub fn get_at(&self, index: usize, store: &Store) -> Option<FuncAddr> {
        match self {
            Self::Defined(defined) => defined.get_at(index),
            Self::External(external) => {
                let module = store.module_by_name(external.module_name.clone());
                match module {
                    ModuleInstance::Defined(defined) => {
                        let addr = defined.exported_table(external.name.clone());
                        store.table(addr.unwrap()).borrow().get_at(index, store)
                    }
                    ModuleInstance::Host(host) => host
                        .table_by_name(external.name.clone())
                        .unwrap()
                        .borrow()
                        .get_at(index),
                }
            }
        }
    }
}

pub struct DefinedTableInstance {
    buffer: Vec<Option<FuncAddr>>,
    max: Option<usize>,
}

impl DefinedTableInstance {
    pub fn new(initial: usize, maximum: Option<usize>) -> Self {
        Self {
            buffer: std::iter::repeat(None).take(initial).collect(),
            max: maximum,
        }
    }

    pub fn initialize(&mut self, offset: usize, data: Vec<FuncAddr>) {
        for (index, func_addr) in data.into_iter().enumerate() {
            self.buffer[offset + index] = Some(func_addr);
        }
    }

    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    pub fn get_at(&self, index: usize) -> Option<FuncAddr> {
        self.buffer[index]
    }
}

pub struct ExternalTableInstance {
    module_name: String,
    name: String,
    limit: ResizableLimits,
}

impl ExternalTableInstance {
    pub fn new(module_name: String, name: String, limit: ResizableLimits) -> Self {
        Self {
            module_name,
            name,
            limit,
        }
    }
}
