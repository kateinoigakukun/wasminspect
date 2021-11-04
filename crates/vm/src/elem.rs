use crate::value::{Ref, RefType};

#[derive(Debug)]
pub enum Error {
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
                "out of bounds table access, try to access {} but size of memory is {}",
                addr, size
            ),
            Self::AccessOutOfBounds(None, size) => write!(
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
    elem: Vec<Ref>,
}

impl ElementInstance {
    pub fn new(ty: RefType, elem: Vec<Ref>) -> Self {
        Self { _ty: ty, elem }
    }

    pub fn get_at(&self, index: usize) -> Result<Ref> {
        self.elem
            .get(index)
            .ok_or(Error::AccessOutOfBounds(Some(index), self.elem.len()))
            .map(|addr| addr.clone())
    }

    pub fn drop_elem(&mut self) {
        self.elem = vec![];
    }
}
