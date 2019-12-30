use super::address::FuncAddr;
pub struct TableInstance {
    buffer: Vec<Option<FuncAddr>>,
    max: Option<usize>,
}

impl TableInstance {
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
