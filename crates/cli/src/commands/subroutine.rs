use super::super::dwarf::WasmLoc;
use anyhow::Result;

pub struct Variable {
    pub name: String,
    pub type_name: String,
}

pub trait SubroutineMap {
    fn variable_name_list(&self, code_offset: usize) -> Result<Vec<Variable>>;
    fn get_frame_base(&self, code_offset: usize) -> Result<WasmLoc>;
    fn display_variable(
        &self,
        code_offset: usize,
        frame_base: u64,
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
    fn variable_name_list(&self, _code_offset: usize) -> Result<Vec<Variable>> {
        Ok(vec![])
    }
    fn get_frame_base(&self, code_offset: usize) -> Result<WasmLoc> {
        Ok(WasmLoc::Global(0))
    }
    fn display_variable(&self, _: usize, _: u64, _: &[u8], _: String) -> Result<()> {
        Ok(())
    }
}
