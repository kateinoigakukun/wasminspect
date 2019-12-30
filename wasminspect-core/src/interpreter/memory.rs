pub struct MemoryInstance {
    data: Vec<u8>,
    max: Option<usize>,
}

impl MemoryInstance {
    pub fn new(initial: usize, maximum: Option<usize>) -> Self {
        Self {
            data: std::iter::repeat(0).take(initial).collect(),
            max: maximum,
        }
    }

    pub fn initialize(&mut self, offset: usize, data: &[u8]) {
        for (index, byte) in data.into_iter().enumerate() {
            self.data[offset + index] = *byte;
        }
    }
}
