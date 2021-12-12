#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ColumnType {
    LeftEdge,
    Column(u64),
}

impl From<ColumnType> for u64 {
    fn from(val: ColumnType) -> Self {
        match val {
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
    fn set_directory_map(&self, from: String, to: String);
}

pub struct EmptySourceMap {}

impl EmptySourceMap {
    pub fn new() -> Self {
        Self {}
    }
}
impl SourceMap for EmptySourceMap {
    fn find_line_info(&self, _: usize) -> Option<LineInfo> {
        None
    }
    fn set_directory_map(&self, _: String, _: String) {}
}
