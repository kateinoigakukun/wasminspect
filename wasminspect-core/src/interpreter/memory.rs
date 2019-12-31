use super::value::FromLittleEndian;
pub struct MemoryInstance {
    data: Vec<u8>,
    max: Option<usize>,
}

#[derive(Debug)]
pub enum Error {
    OverMaximumSize(usize),
    OverLimitWasmMemory,
}

static PAGE_SIZE: usize = 65536;
impl MemoryInstance {
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

    pub fn page_size(&self) -> usize {
        self.data_len()/PAGE_SIZE
    }

    pub fn load_as<T: FromLittleEndian>(&self, offset: usize) -> T {
        let buf = &self.data[offset..offset+std::mem::size_of::<T>()];
        T::from_le(buf)
    }

    pub fn grow(&mut self, n: usize) -> Result<(), Error> {
        let len = self.page_size() + n;
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
