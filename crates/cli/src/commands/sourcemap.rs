pub struct LineInfo {
    pub filepath: String,
    pub line: u64,
    pub column: u64,
}

pub trait SourceMap {
    fn find_line_info(&self, offset: usize) -> Option<LineInfo>;
}