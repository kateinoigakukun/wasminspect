use crate::value::{RefType, RefVal};

#[derive(Debug)]
pub enum Error {
    AccessOutOfBounds {
        try_to_access: Option<usize>,
        size: usize,
    },
    UninitializedElement(usize),
    GrowOverMaximumSize {
        base: usize,
        growing: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccessOutOfBounds { try_to_access: Some(addr), size } => write!(
                f,
                "undefined element: out of bounds table access, try to access {} but size of memory is {}",
                addr, size
            ),
            Self::AccessOutOfBounds { try_to_access: None, size } => write!(
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

/// Runtime representation of a table. It records its type and holds a vector of `RefVal`
/// https://webassembly.github.io/spec/core/exec/runtime.html#table-instances
pub struct TableInstance {
    buffer: Vec<RefVal>,
    pub max: Option<usize>,
    pub initial: usize,
    pub ty: RefType,
}

impl TableInstance {
    pub fn new(initial: usize, maximum: Option<usize>, ty: RefType) -> Self {
        Self {
            buffer: std::iter::repeat(RefVal::NullRef(ty))
                .take(initial)
                .collect(),
            initial,
            max: maximum,
            ty,
        }
    }

    pub fn validate_region(&self, offset: usize, size: usize) -> Result<()> {
        if let Some(max_addr) = offset.checked_add(size) {
            if max_addr > self.buffer_len() {
                return Err(Error::AccessOutOfBounds {
                    try_to_access: Some(max_addr),
                    size: self.buffer_len(),
                });
            }
        } else {
            return Err(Error::AccessOutOfBounds {
                try_to_access: None,
                size: self.buffer_len(),
            });
        }
        Ok(())
    }

    pub fn initialize(&mut self, offset: usize, data: Vec<RefVal>) -> Result<()> {
        self.validate_region(offset, data.len())?;
        for (index, func_addr) in data.into_iter().enumerate() {
            self.buffer[offset + index] = func_addr;
        }
        Ok(())
    }

    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    pub fn get_at(&self, index: usize) -> Result<RefVal> {
        self.buffer
            .get(index)
            .ok_or_else(|| Error::AccessOutOfBounds {
                try_to_access: Some(index),
                size: self.buffer_len(),
            })
            .map(|addr| *addr)
    }

    pub fn set_at(&mut self, index: usize, val: RefVal) -> Result<()> {
        let buffer_len = self.buffer_len();
        let entry = self.buffer.get_mut(index).ok_or(Error::AccessOutOfBounds {
            try_to_access: Some(index),
            size: buffer_len,
        })?;
        *entry = val;
        Ok(())
    }

    /// https://webassembly.github.io/spec/core/exec/modules.html#growing-tables
    pub fn grow(&mut self, n: usize, val: RefVal) -> Result<()> {
        let base_len = self.buffer_len();
        let len = base_len.checked_add(n).ok_or(Error::GrowOverMaximumSize {
            base: base_len,
            growing: n,
        })?;

        if let Some(max) = self.max {
            if len > max {
                return Err(Error::GrowOverMaximumSize {
                    base: base_len,
                    growing: n,
                });
            }
        }
        let mut extra = std::iter::repeat(val).take(n).collect();
        self.buffer.append(&mut extra);
        Ok(())
    }
}
