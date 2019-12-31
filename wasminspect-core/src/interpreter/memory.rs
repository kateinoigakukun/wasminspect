use super::value::FromLittleEndian;
use parity_wasm::elements::ResizableLimits;

pub enum MemoryInstance {
    Defined(DefinedMemoryInstance),
    External(HostMemoryInstance),
}

impl MemoryInstance {
    pub fn grow(&mut self, n: usize) -> Result<(), Error> {
        match self {
            Self::Defined(defined) => defined.grow(n),
            Self::External(_) => panic!(),
        }
    }

    pub fn initialize(&mut self, offset: usize, data: &[u8]) {
        match self {
            Self::Defined(defined) => defined.initialize(offset, data),
            Self::External(_) => unimplemented!(),
        }
    }
    pub fn data_len(&self) -> usize {
        match self {
            Self::Defined(defined) => defined.data_len(),
            Self::External(_) => unimplemented!(),
        }
    }
    pub fn load_as<T: FromLittleEndian>(&self, offset: usize) -> T {
        match self {
            Self::Defined(defined) => defined.load_as(offset),
            Self::External(_) => unimplemented!(),
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

    pub fn initialize(&mut self, offset: usize, data: &[u8]) {
        for (index, byte) in data.into_iter().enumerate() {
            self.data[offset + index] = *byte;
        }
    }
    pub fn data_len(&self) -> usize {
        self.data.len()
    }

    pub fn page_count(&self) -> usize {
        self.data_len() / PAGE_SIZE
    }

    pub fn load_as<T: FromLittleEndian>(&self, offset: usize) -> T {
        let buf = &self.data[offset..offset + std::mem::size_of::<T>()];
        T::from_le(buf)
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
        let mut extra: Vec<u8> = std::iter::repeat(0).take(len * PAGE_SIZE).collect();
        self.data.append(&mut extra);
        return Ok(());
    }
}
