use super::value::FromLittleEndian;
use super::WASM_PAGE_SIZE;

pub struct MemoryInstance {
    data: Vec<u8>,
    pub max: Option<usize>,
    pub initial: usize,
}

#[derive(Debug)]
pub enum Error {
    GrowOverMaximumSize(usize),
    GrowOverMaximumPageSize(usize),
    AccessOutOfBounds(
        /* try to access */ Option<usize>,
        /* memory size */ usize,
    ),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccessOutOfBounds(Some(addr), size) => write!(
                f,
                "out of bounds memory access, try to access {} but size of memory is {}",
                addr, size
            ),
            Self::AccessOutOfBounds(None, size) => write!(
                f,
                "out of bounds memory access, try to access over size of usize but size of memory is {}",
                size
            ),
            _ => write!(f, "{:?}", self),
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

impl MemoryInstance {
    pub fn new(initial: usize, maximum: Option<usize>) -> Self {
        Self {
            data: std::iter::repeat(0).take(initial * WASM_PAGE_SIZE).collect(),
            initial,
            max: maximum,
        }
    }

    pub fn validate_region(&self, offset: usize, size: usize) -> Result<()> {
        if let Some(max_addr) = offset.checked_add(size) {
            if max_addr > self.data_len() {
                return Err(Error::AccessOutOfBounds(Some(max_addr), self.data_len()));
            }
        } else {
            return Err(Error::AccessOutOfBounds(None, self.data_len()));
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

    pub fn page_count(&self) -> usize {
        self.data_len() / WASM_PAGE_SIZE
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
        let mut extra: Vec<u8> = std::iter::repeat(0).take(n * WASM_PAGE_SIZE).collect();
        self.data.append(&mut extra);
        return Ok(());
    }
    pub fn raw_data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn raw_data(&self) -> &[u8] {
        &self.data
    }
}
