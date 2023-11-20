use crate::value::FromLittleEndian;
use crate::WASM_PAGE_SIZE;

pub struct MemoryInstance {
    data: Vec<u8>,
    pub max: Option<usize>,
    pub initial: usize,
}

#[derive(Debug)]
pub enum Error {
    GrowOverMaximumSize(usize),
    GrowOverMaximumPageSize(usize),
    AccessOutOfBounds {
        try_to_access: Option<usize>,
        memory_size: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccessOutOfBounds { try_to_access: Some(addr), memory_size } => write!(
                f,
                "out of bounds memory access, try to access {} but size of memory is {}",
                addr, memory_size
            ),
            Self::AccessOutOfBounds { try_to_access: None, memory_size } => write!(
                f,
                "out of bounds memory access, try to access over size of usize but size of memory is {}",
                memory_size
            ),
            _ => write!(f, "{:?}", self),
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

impl MemoryInstance {
    pub fn new(initial: usize, maximum: Option<usize>) -> Self {
        Self {
            data: std::iter::repeat(0)
                .take(initial * WASM_PAGE_SIZE)
                .collect(),
            initial,
            max: maximum,
        }
    }

    pub fn validate_region(&self, offset: usize, size: usize) -> Result<()> {
        if let Some(max_addr) = offset.checked_add(size) {
            if max_addr > self.data_len() {
                return Err(Error::AccessOutOfBounds {
                    try_to_access: Some(max_addr),
                    memory_size: self.data_len(),
                });
            }
        } else {
            return Err(Error::AccessOutOfBounds {
                try_to_access: None,
                memory_size: self.data_len(),
            });
        }
        Ok(())
    }

    pub fn store(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        self.validate_region(offset, data.len())?;
        for (index, byte) in data.iter().enumerate() {
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

    pub fn page_count(&self) -> usize {
        self.data_len() / WASM_PAGE_SIZE
    }

    pub fn grow(&mut self, n: usize) -> Result<()> {
        let len = self
            .page_count()
            .checked_add(n)
            .ok_or(Error::GrowOverMaximumPageSize(n))?;

        if let Some(max) = self.max {
            if len > max {
                return Err(Error::GrowOverMaximumSize(max));
            }
        }
        let zero_len = n * WASM_PAGE_SIZE;
        self.data.resize(self.data.len() + zero_len, 0);
        self.initial = len;
        Ok(())
    }
    pub fn raw_data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn raw_data(&self) -> &[u8] {
        &self.data
    }
}
