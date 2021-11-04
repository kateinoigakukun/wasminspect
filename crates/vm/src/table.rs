use crate::value::{Ref, RefType};

#[derive(Debug)]
pub enum Error {
    AccessOutOfBounds(
        /* try to access */ Option<usize>,
        /* memory size */ usize,
    ),
    UninitializedElement(usize),
    GrowOverMaximumSize(usize),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccessOutOfBounds(Some(addr), size) => write!(
                f,
                "out of bounds table access, try to access {} but size of memory is {}",
                addr, size
            ),
            Self::AccessOutOfBounds(None, size) => write!(
                f,
                "out of bounds table access, try to access over size of usize but size of memory is {}",
                size
            ),
            Self::UninitializedElement(addr) => {
                write!(f, "uninitialized element, try to access {}", addr)
            }
            other => write!(f, "{:?}", other)
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

pub struct TableInstance {
    buffer: Vec<Ref>,
    pub max: Option<usize>,
    pub initial: usize,
}

impl TableInstance {
    pub fn new(initial: usize, maximum: Option<usize>, ty: RefType) -> Self {
        Self {
            buffer: std::iter::repeat(Ref::NullRef(ty)).take(initial).collect(),
            initial,
            max: maximum,
        }
    }

    pub fn initialize(&mut self, offset: usize, data: Vec<Ref>) -> Result<()> {
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
            self.buffer[offset + index] = func_addr;
        }
        Ok(())
    }

    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    pub fn get_at(&self, index: usize) -> Result<Ref> {
        self.buffer
            .get(index)
            .ok_or(Error::AccessOutOfBounds(Some(index), self.buffer_len()))
            .map(|addr| addr.clone())
    }

    pub fn set_at(&mut self, index: usize, val: Ref) -> Result<()> {
        let buffer_len = self.buffer_len();
        let entry = self
            .buffer
            .get_mut(index)
            .ok_or(Error::AccessOutOfBounds(Some(index), buffer_len))?;
        *entry = val;
        Ok(())
    }

    pub fn grow(&mut self, n: usize, val: Ref) -> Result<()> {
        let len = self.buffer_len() + n;
        if self.buffer_len() > (1 << 32) {
            return Err(Error::GrowOverMaximumSize(len));
        }

        if let Some(max) = self.max {
            if len > max {
                return Err(Error::GrowOverMaximumSize(max));
            }
        }
        let mut extra = std::iter::repeat(val).take(n).collect();
        self.buffer.append(&mut extra);
        return Ok(());
    }
}
