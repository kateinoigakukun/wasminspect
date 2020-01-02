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

    pub fn grow(&mut self, n: usize, store: &Store) -> Result<()> {
        self.resolve_memory_instance(store).borrow_mut().grow(n)
    }

    pub fn store(&mut self, offset: usize, data: &[u8], store: &Store) -> Result<()> {
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

    pub fn load_as<T: FromLittleEndian>(&self, offset: usize, store: &Store) -> Result<T> {
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
    GrowOverMaximumSize(usize),
    GrowOverMaximumPageSize(usize),
    AccessOutOfBounds(/* try to access */ usize, /* memory size */ usize)
}

type Result<T> = std::result::Result<T, Error>;

static PAGE_SIZE: usize = 65536;
impl DefinedMemoryInstance {
    pub fn new(initial: usize, maximum: Option<usize>) -> Self {
        Self {
            data: std::iter::repeat(0).take(initial * PAGE_SIZE).collect(),
            max: maximum,
        }
    }

    fn validate_region(&self, offset: usize, size: usize) -> Result<()> {
        if (offset + size) > self.data_len() {
            return Err(Error::AccessOutOfBounds(offset + size, self.data_len()))
        }
        Ok(())
    }

    pub fn store(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        self.validate_region(offset, data.len())?;
        for (index, byte) in data.into_iter().enumerate() {
            self.data[offset + index] = *byte;
        }
        Ok(())
    }
    pub fn data_len(&self) -> usize {
        self.data.len()
    }

    pub fn load_as<T: FromLittleEndian>(&self, offset: usize) -> Result<T> {
        self.validate_region(offset, std::mem::size_of::<T>())?;
        let buf = &self.data[offset..offset + std::mem::size_of::<T>()];
        Ok(T::from_le(buf))
    }

    fn page_count(&self) -> usize {
        self.data_len() / PAGE_SIZE
    }

    pub fn grow(&mut self, n: usize) -> Result<()> {
        let len = self.page_count() + n;
        if len > 65536 {
            return Err(Error::GrowOverMaximumPageSize(len));
        }

        if let Some(max) = self.max {
            if len > max {
                return Err(Error::GrowOverMaximumSize(max));
            }
        }
        let mut extra: Vec<u8> = std::iter::repeat(0).take(n * PAGE_SIZE).collect();
        self.data.append(&mut extra);
        return Ok(());
    }
}
