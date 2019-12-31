pub struct MemoryInstance {
    data: Vec<u8>,
    max: Option<usize>,
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
}
