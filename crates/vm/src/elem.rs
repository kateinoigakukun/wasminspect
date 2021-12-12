use crate::value::{RefType, RefVal};

#[derive(Debug)]
pub enum Error {
    AccessOutOfBounds {
        try_to_access: Option<usize>,
        size: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccessOutOfBounds { try_to_access: Some(addr), size } => write!(
                f,
                "out of bounds table access, try to access {} but size of memory is {}",
                addr, size
            ),
            Self::AccessOutOfBounds { try_to_access: None, size } => write!(
                f,
                "out of bounds table access, try to access over size of usize but size of memory is {}",
                size
            ),
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

pub struct ElementInstance {
    _ty: RefType,
    elem: Vec<RefVal>,
}

impl ElementInstance {
    pub fn new(ty: RefType, elem: Vec<RefVal>) -> Self {
        Self { _ty: ty, elem }
    }

    pub fn validate_region(&self, offset: usize, size: usize) -> Result<()> {
        let len = self.elem.len();
        if let Some(max_addr) = offset.checked_add(size) {
            if max_addr > len {
                return Err(Error::AccessOutOfBounds {
                    try_to_access: Some(max_addr),
                    size: len,
                });
            }
        } else {
            return Err(Error::AccessOutOfBounds {
                try_to_access: None,
                size: len,
            });
        }
        Ok(())
    }

    pub fn get_at(&self, index: usize) -> Result<RefVal> {
        self.elem
            .get(index)
            .ok_or_else(|| Error::AccessOutOfBounds {
                try_to_access: Some(index),
                size: self.elem.len(),
            })
            .map(|addr| *addr)
    }

    pub fn drop_elem(&mut self) {
        self.elem = vec![];
    }
}
