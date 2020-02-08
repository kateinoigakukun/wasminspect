use anyhow::Result;

pub trait SubroutineMap {
    fn display_variable(
        &self,
        code_offset: usize,
        rbp: u32,
        memory: &[u8],
        name: String,
    ) -> Result<()>;
}

pub struct EmptySubroutineMap {}

impl EmptySubroutineMap {
    pub fn new() -> Self {
        Self {}
    }
}
impl SubroutineMap for EmptySubroutineMap {
    fn display_variable(&self, _: usize, _: u32, _: &[u8], _: String) -> Result<()> {
        Ok(())
    }
}
