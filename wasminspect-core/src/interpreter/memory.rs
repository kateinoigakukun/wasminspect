use super::module::ModuleInstance;
use super::store::Store;
use super::value::FromLittleEndian;
use parity_wasm::elements::ResizableLimits;

pub enum MemoryInstance {
    Defined(std::rc::Rc<std::cell::RefCell<DefinedMemoryInstance>>),
    External(ExternalMemoryInstance),
}

impl MemoryInstance {
    fn resolve_memory_instance(
        &self,
        store: &Store,
    ) -> std::rc::Rc<std::cell::RefCell<DefinedMemoryInstance>> {
        match self {
            Self::Defined(defined) => defined.clone(),
            Self::External(external) => {
                let module = store.module_by_name(external.module_name.clone());
                match module {
                    ModuleInstance::Defined(defined_module) => {
                        let addr = defined_module
                            .exported_memory(external.name.clone())
                            .unwrap();
                        let memory = store.memory(addr);
                        return memory.borrow_mut().resolve_memory_instance(store);
                    }
                    ModuleInstance::Host(host_module) => host_module
                        .memory_by_name(external.name.clone())
                        .unwrap()
                        .clone(),
                }
            }
        }
    }

    pub fn grow(&mut self, n: usize, store: &Store) -> Result<(), Error> {
        self.resolve_memory_instance(store).borrow_mut().grow(n)
    }

    pub fn store(&mut self, offset: usize, data: &[u8], store: &Store) {
        self.resolve_memory_instance(store)
            .borrow_mut()
            .store(offset, data)
    }

    pub fn data_len(&self, store: &Store) -> usize {
        self.resolve_memory_instance(store).borrow().data_len()
    }

    pub fn page_count(&self, store: &Store) -> usize {
        self.data_len(store) / PAGE_SIZE
    }

    pub fn load_as<T: FromLittleEndian>(&self, offset: usize, store: &Store) -> T {
        self.resolve_memory_instance(store).borrow().load_as(offset)
    }
}

pub struct DefinedMemoryInstance {
    data: Vec<u8>,
    max: Option<usize>,
}

pub struct ExternalMemoryInstance {
    module_name: String,
    name: String,
    limit: ResizableLimits,
}

impl ExternalMemoryInstance {
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
