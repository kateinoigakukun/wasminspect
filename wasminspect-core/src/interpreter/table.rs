use parity_wasm::elements::ResizableLimits;
use super::address::FuncAddr;

pub enum TableInstance {
    Defined(DefinedTableInstance),
    External(ExternalTableInstance),
}

impl TableInstance {

    pub fn buffer_len(&self) -> usize {
        match self {
            Self::Defined(defined) => defined.buffer_len(),
            Self::External(_) => unimplemented!(),
        }
    }

    pub fn get_at(&self, index: usize) -> Option<FuncAddr> {
        match self {
            Self::Defined(defined) => defined.get_at(index),
            Self::External(_) => unimplemented!(),
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