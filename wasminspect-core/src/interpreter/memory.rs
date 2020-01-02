use super::module::ModuleInstance;
use super::store::Store;
use super::value::FromLittleEndian;
use super::utils::*;
use parity_wasm::elements::ResizableLimits;

pub enum MemoryInstance {
    Defined(DefinedMemoryInstance),
    External(HostMemoryInstance),
}



impl MemoryInstance {
    pub fn grow(&mut self, n: usize, store: &Store) -> Result<(), Error> {
        match self {
            Self::Defined(defined) => defined.grow(n),
            Self::External(external) => {
                let module = store.module_by_name(external.module_name.clone());
                match module {
                    ModuleInstance::Defined(defined) => {
                        let addr = defined.exported_memory(external.name.clone());
                        store.memory(addr.unwrap()).borrow_mut().grow(n, store)
                    }
                    ModuleInstance::Host(host) => host
                        .memory_by_name(external.name.clone())
                        .unwrap()
                        .borrow_mut()
                        .grow(n),
                }
            }
        }
    }

    pub fn store(&mut self, offset: usize, data: &[u8], store: &Store) {
        match self {
            Self::Defined(defined) => defined.store(offset, data),
            Self::External(external) => {
                let module = store.module_by_name(external.module_name.clone());
                match module {
                    ModuleInstance::Defined(defined) => {
                        let addr = defined.exported_memory(external.name.clone());
                        store
                            .memory(addr.unwrap())
                            .borrow_mut()
                            .store(offset, data, store)
                    }
                    ModuleInstance::Host(host) => host
                        .memory_by_name(external.name.clone())
                        .unwrap()
                        .borrow_mut()
                        .store(offset, data),
                }
            }
        }
    }
    pub fn data_len(&self, store: &Store) -> usize {
        match self {
            Self::Defined(defined) => defined.data_len(),
            Self::External(external) => {
                let module = store.module_by_name(external.module_name.clone());
                match module {
                    ModuleInstance::Defined(defined) => {
                        let addr = defined.exported_memory(external.name.clone());
                        store.memory(addr.unwrap()).borrow().data_len(store)
                    }
                    ModuleInstance::Host(host) => host
                        .memory_by_name(external.name.clone())
                        .unwrap()
                        .borrow()
                        .data_len(),
                }
            }
        }
    }
    pub fn page_count(&self, store: &Store) -> usize {
        self.data_len(store) / PAGE_SIZE
    }

    pub fn load_as<T: FromLittleEndian>(&self, offset: usize, store: &Store) -> T {
        match self {
            Self::Defined(defined) => defined.load_as(offset),
            Self::External(external) => {
                let module = store.module_by_name(external.module_name.clone());
                match module {
                    ModuleInstance::Defined(defined) => {
                        let addr = defined.exported_memory(external.name.clone());
                        store.memory(addr.unwrap()).borrow().load_as(offset, store)
                    }
                    ModuleInstance::Host(host) => host
                        .memory_by_name(external.name.clone())
                        .unwrap()
                        .borrow()
                        .load_as(offset),
                }
            }
        }
    }
}

pub struct DefinedMemoryInstance {
    data: Vec<u8>,
    max: Option<usize>,
}

pub struct HostMemoryInstance {
    module_name: String,
    name: String,
    limit: ResizableLimits,
}

impl HostMemoryInstance {
    pub fn new(module_name: String, name: String, limit: ResizableLimits) -> Self {
        Self {
            module_name,
            name,
            limit,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    OverMaximumSize(usize),
    OverLimitWasmMemory,
}

static PAGE_SIZE: usize = 65536;
impl DefinedMemoryInstance {
    pub fn new(initial: usize, maximum: Option<usize>) -> Self {
        Self {
            data: std::iter::repeat(0).take(initial * PAGE_SIZE).collect(),
            max: maximum,
        }
    }

    pub fn store(&mut self, offset: usize, data: &[u8]) {
        for (index, byte) in data.into_iter().enumerate() {
            self.data[offset + index] = *byte;
        }
    }
    pub fn data_len(&self) -> usize {
        self.data.len()
    }

    pub fn load_as<T: FromLittleEndian>(&self, offset: usize) -> T {
        let buf = &self.data[offset..offset + std::mem::size_of::<T>()];
        T::from_le(buf)
    }

    fn page_count(&self) -> usize {
        self.data_len() / PAGE_SIZE
    }

    pub fn grow(&mut self, n: usize) -> Result<(), Error> {
        let len = self.page_count() + n;
        if len > 65536 {
            return Err(Error::OverLimitWasmMemory);
        }

        if let Some(max) = self.max {
            if len > max {
                return Err(Error::OverMaximumSize(max));
            }
        }
        let mut extra: Vec<u8> = std::iter::repeat(0).take(n * PAGE_SIZE).collect();
        self.data.append(&mut extra);
        return Ok(());
    }
}
