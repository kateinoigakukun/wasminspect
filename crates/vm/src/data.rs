#[derive(Debug)]
pub enum Error {
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
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

pub struct DataInstance {
    bytes: Vec<u8>,
}

impl DataInstance {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
    pub fn validate_region(&self, offset: usize, size: usize) -> Result<()> {
        let len = self.bytes.len();
        if let Some(max_addr) = offset.checked_add(size) {
            if max_addr > len {
                return Err(Error::AccessOutOfBounds {
                    try_to_access: Some(max_addr),
                    memory_size: len,
                });
            }
        } else {
            return Err(Error::AccessOutOfBounds {
                try_to_access: None,
                memory_size: len,
            });
        }
        Ok(())
    }

    pub fn raw(&self) -> &[u8] {
        &self.bytes
    }

    pub fn drop_bytes(&mut self) {
        self.bytes = vec![];
    }
}
