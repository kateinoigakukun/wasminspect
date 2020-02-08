pub trait SubroutineMap {
    fn display_variable(&self, code_offset: usize, name: String);
}

pub struct EmptySubroutineMap {}

impl EmptySubroutineMap {
    pub fn new() -> Self {
        Self {}
    }
}
impl SubroutineMap for EmptySubroutineMap {
    fn display_variable(&self, code_offset: usize, name: String) {}
}
