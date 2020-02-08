#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ColumnType {
    LeftEdge,
    Column(u64),
}

impl Into<u64> for ColumnType {
    fn into(self) -> u64 {
        match self {
            ColumnType::Column(c) => c,
            ColumnType::LeftEdge => 0,
        }
    }
}

#[derive(Clone)]
pub struct LineInfo {
    pub filepath: String,
    pub line: Option<u64>,
    pub column: ColumnType,
}

pub trait SourceMap {
    fn find_line_info(&self, offset: usize) -> Option<LineInfo>;
}

pub struct EmptySourceMap {}

impl EmptySourceMap {
    pub fn new() -> Self {
        Self {}
    }
}
impl SourceMap for EmptySourceMap {
    fn find_line_info(&self, _offset: usize) -> Option<LineInfo> {
        None
    }
}
