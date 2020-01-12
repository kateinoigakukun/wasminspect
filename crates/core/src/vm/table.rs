use super::address::FuncAddr;

pub type TableInstance = DefinedTableInstance;

#[derive(Debug)]
pub enum Error {
    AccessOutOfBounds(
        /* try to access */ Option<usize>,
        /* memory size */ usize,
    ),
    UninitializedElement(usize),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccessOutOfBounds(Some(addr), size) => write!(
                f,
                "undefined element, try to access {} but size of memory is {}",
                addr, size
            ),
            Self::AccessOutOfBounds(None, size) => write!(
                f,
                "undefined element, try to access over size of usize but size of memory is {}",
                size
            ),
            Self::UninitializedElement(addr) => {
                write!(f, "uninitialized element, try to access {}", addr)
            }
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

pub struct DefinedTableInstance {
    buffer: Vec<Option<FuncAddr>>,
    pub max: Option<usize>,
    pub initial: usize,
}

impl DefinedTableInstance {
    pub fn new(initial: usize, maximum: Option<usize>) -> Self {
        Self {
            buffer: std::iter::repeat(None).take(initial).collect(),
            initial,
            max: maximum,
        }
    }

    pub fn initialize(&mut self, offset: usize, data: Vec<FuncAddr>) -> Result<()> {
        {
            if let Some(max_addr) = offset.checked_add(data.len()) {
                if max_addr > self.buffer_len() {
                    return Err(Error::AccessOutOfBounds(Some(max_addr), self.buffer_len()));
                }
            } else {
                return Err(Error::AccessOutOfBounds(None, self.buffer_len()));
            }
        }
        for (index, func_addr) in data.into_iter().enumerate() {
            self.buffer[offset + index] = Some(func_addr);
        }
        Ok(())
    }

    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    pub fn get_at(&self, index: usize) -> Result<FuncAddr> {
        self.buffer
            .get(index)
            .ok_or(Error::AccessOutOfBounds(Some(index), self.buffer_len()))
            .and_then(|addr| addr.ok_or(Error::UninitializedElement(index)))
            .map(|addr| addr.clone())
    }
}
