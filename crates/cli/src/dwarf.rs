use anyhow::{anyhow, Result};
use gimli::{
    AttributeValue, DebugAbbrev, DebugAddr, DebugInfo, DebugLine, DebugLineStr, DebugLoc,
    DebugLocLists, DebugRanges, DebugRngLists, DebugStr, DebugStrOffsets, DebugTypes,
    DebuggingInformationEntry, EndianSlice, LineRow, LittleEndian, LocationLists, RangeLists, Unit,
};
use std::collections::{BTreeMap, HashMap};
use wasmparser::{ModuleReader, SectionCode};

type Reader<'input> = gimli::EndianSlice<'input, LittleEndian>;
pub type Dwarf<'input> = gimli::Dwarf<Reader<'input>>;


pub fn parse_dwarf<'a>(module: &'a [u8]) -> Result<Dwarf<'a>> {
    const EMPTY_SECTION: &[u8] = &[];
    let mut reader = ModuleReader::new(module)?;
    let mut sections = HashMap::new();
    while !reader.eof() {
        let section = reader.read().expect("section");
        match section.code {
            SectionCode::Custom { name, kind: _ } => {
                let mut reader = section.get_binary_reader();
                let len = reader.bytes_remaining();
                sections.insert(name, reader.read_bytes(len).expect("bytes"));
            }
            _ => (),
        }
    }
    let endian = LittleEndian;
    let debug_str = DebugStr::new(sections.get(".debug_str").unwrap(), endian);
    let debug_abbrev = DebugAbbrev::new(sections.get(".debug_abbrev").unwrap(), endian);
    let debug_info = DebugInfo::new(sections.get(".debug_info").unwrap(), endian);
    let debug_line = DebugLine::new(sections.get(".debug_line").unwrap(), endian);
    let debug_addr = DebugAddr::from(EndianSlice::new(EMPTY_SECTION, endian));
    let debug_line_str = DebugLineStr::from(EndianSlice::new(EMPTY_SECTION, endian));
    let debug_str_sup = DebugStr::from(EndianSlice::new(EMPTY_SECTION, endian));
    let debug_ranges = match sections.get(".debug_ranges") {
        Some(section) => DebugRanges::new(section, endian),
        None => DebugRanges::new(EMPTY_SECTION, endian),
    };
    let debug_rnglists = DebugRngLists::new(EMPTY_SECTION, endian);
    let ranges = RangeLists::new(debug_ranges, debug_rnglists);
    let debug_loc = match sections.get(".debug_loc") {
        Some(section) => DebugLoc::new(section, endian),
        None => DebugLoc::new(EMPTY_SECTION, endian),
    };
    let debug_loclists = DebugLocLists::new(EMPTY_SECTION, endian);
    let locations = LocationLists::new(debug_loc, debug_loclists);
    let debug_str_offsets = DebugStrOffsets::from(EndianSlice::new(EMPTY_SECTION, endian));
    let debug_types = DebugTypes::from(EndianSlice::new(EMPTY_SECTION, endian));

    Ok(Dwarf {
        debug_abbrev,
        debug_addr,
        debug_info,
        debug_line,
        debug_line_str,
        debug_str,
        debug_str_offsets,
        debug_str_sup,
        debug_types,
        locations,
        ranges,
    })
}

pub struct DwarfDebugInfo<'a> {
    pub sourcemap: DwarfSourceMap,
    pub subroutine: DwarfSubroutineMap<'a>,
}
pub fn transform_dwarf<'a>(dwarf: Dwarf<'a>) -> Result<DwarfDebugInfo<'a>> {
    let mut headers = dwarf.units();
    let mut sourcemaps = Vec::new();
    let mut subroutines = Vec::new();
    while let Some(header) = headers.next()? {
        let unit = dwarf.unit(header)?;
        let mut entries = unit.entries();
        let root = match entries.next_dfs()? {
            Some((_, entry)) => entry,
            None => continue,
        };
        sourcemaps.push(transform_debug_line(
            &unit,
            root,
            &dwarf,
            &dwarf.debug_line,
        )?);
        subroutines.append(&mut transform_subprogram(&dwarf, &unit)?);
    }
    Ok(DwarfDebugInfo {
        sourcemap: DwarfSourceMap::new(sourcemaps),
        subroutine: DwarfSubroutineMap { subroutines },
    })
}

struct SubroutineBuilder<R: gimli::Reader> {
    name: Option<String>,
    pc: std::ops::Range<u64>,
    variables: Vec<SymbolVariable<R>>,
}

impl<R: gimli::Reader> SubroutineBuilder<R> {
    fn new(pc: std::ops::Range<u64>, name: Option<String>) -> Self {
        Self {
            pc,
            name,
            variables: vec![],
        }
    }

    fn add_variable(&mut self, var: SymbolVariable<R>) {
        self.variables.push(var);
    }

    fn build(&self) -> Subroutine<R> {
        Subroutine {
            pc: self.pc.clone(),
            name: self.name.clone(),
            variables: self.variables.clone(),
        }
    }
}

#[derive(Clone)]
pub struct SymbolVariable<R>
where
    R: gimli::Reader,
{
    name: String,
    location: gimli::AttributeValue<R>,
}

pub struct Subroutine<R: gimli::Reader> {
    pub name: Option<String>,
    pub pc: std::ops::Range<u64>,
    pub variables: Vec<SymbolVariable<R<>>>
}

pub fn transform_subprogram<R: gimli::Reader>(
    dwarf: &gimli::Dwarf<R>,
    unit: &Unit<R, R::Offset>,
) -> Result<Vec<Subroutine<R>>> {
    let mut entries = unit.entries();
    let root_cu = entries.next_dfs();

    let mut subroutines = vec![];

    let mut current: Option<SubroutineBuilder<R>> = None;
    while let Some((depth_delta, entry)) = entries.next_dfs()? {
        println!("[Parse DIE for collect subprograms] {:?}", entry);
        match entry.tag() {
            gimli::DW_TAG_subprogram => {
                let name = match entry.attr_value(gimli::DW_AT_name)? {
                    Some(attr) => Some(
                        dwarf
                            .attr_string(unit, attr)?
                            .to_string()?
                            .as_ref()
                            .to_string(),
                    ),
                    None => None,
                };
                if let Some(ref builder) = current {
                    subroutines.push(builder.build())
                }

                let low_pc_attr = entry.attr_value(gimli::DW_AT_low_pc)?;
                println!("low_pc_attr: {:?}", low_pc_attr);
                let high_pc_attr = entry.attr_value(gimli::DW_AT_high_pc)?;
                println!("high_pc_attr: {:?}", high_pc_attr);
                if let Some(AttributeValue::Addr(low_pc)) = low_pc_attr {
                    let high_pc = match high_pc_attr {
                        Some(AttributeValue::Udata(size)) => Some(low_pc + size),
                        Some(AttributeValue::Addr(high_pc)) => Some(high_pc),
                        Some(x) => unreachable!("high_pc can't be {:?}", x),
                        None => None,
                    };
                    if let Some(high_pc) = high_pc {
                        current = Some(SubroutineBuilder::new(low_pc..high_pc, name));
                    }
                }
            }
            gimli::DW_TAG_variable => {
                let location = entry.attr_value(gimli::DW_AT_location)?.unwrap();
                let name = dwarf
                    .attr_string(unit, entry.attr_value(gimli::DW_AT_name)?.unwrap())?
                    .to_string()?
                    .as_ref()
                    .to_string();
                let var = SymbolVariable { name, location };
                current.as_mut().unwrap().add_variable(var);
            }
            gimli::DW_TAG_formal_parameter => {}
            _ => {}
        }
    }
    Ok(subroutines)
}

use gimli::Expression;
fn evaluate_variable_location<R: gimli::Reader>(
    unit: &Unit<R, R::Offset>,
    expr: Expression<R>,
) -> Result<Vec<gimli::Piece<R>>> {
    let mut evaluation = expr.evaluation(unit.encoding());
    let result = evaluation.evaluate()?;
    use gimli::EvaluationResult;
    match result {
        EvaluationResult::Complete => Ok(evaluation.result()),
        x => unimplemented!("{:?}", x),
    }
}

use std::path::Path;

pub fn transform_debug_line<R: gimli::Reader>(
    unit: &Unit<R, R::Offset>,
    root: &DebuggingInformationEntry<R>,
    dwarf: &gimli::Dwarf<R>,
    debug_line: &DebugLine<R>,
) -> Result<DwarfUnitSourceMap> {
    let offset = match root.attr_value(gimli::DW_AT_stmt_list)? {
        Some(gimli::AttributeValue::DebugLineRef(offset)) => offset,
        _ => {
            return Err(anyhow!("Debug line offset is not found"));
        }
    };

    let program = debug_line
        .program(offset, unit.header.address_size(), None, None)
        .expect("parsable debug_line");

    let header = program.header();

    let sequence_base_index: usize;
    let mut dirs = vec![];
    if header.version() <= 4 {
        dirs.push("./".to_string());
        sequence_base_index = 1;
    } else {
        sequence_base_index = 0;
    }
    for dir in header.include_directories() {
        let dir_str =
            String::from_utf8(dwarf.attr_string(unit, dir.clone())?.to_slice()?.to_vec()).unwrap();
        dirs.push(dir_str)
    }
    let mut files = Vec::new();
    for file_entry in header.file_names() {
        let dir = dirs[file_entry.directory_index() as usize].clone();
        let dir_path = Path::new(&dir);
        let mut path = dir_path.join(
            dwarf
                .attr_string(unit, file_entry.path_name())?
                .to_string()?
                .as_ref(),
        );
        if !path.is_absolute() {
            if let Some(comp_dir) = unit.comp_dir.clone() {
                let comp_dir = String::from_utf8(comp_dir.to_slice()?.to_vec()).unwrap();
                path = Path::new(&comp_dir).join(path);
            }
        }
        files.push(path);
    }

    let mut rows = program.rows();
    let mut sorted_rows = BTreeMap::new();
    while let Some((_, row)) = rows.next_row()? {
        sorted_rows.insert(row.address(), row.clone());
    }
    let sorted_rows: Vec<_> = sorted_rows.into_iter().collect();
    Ok(DwarfUnitSourceMap {
        address_sorted_rows: sorted_rows,
        paths: files,
        sequence_base_index,
    })
}

pub struct DwarfUnitSourceMap {
    address_sorted_rows: Vec<(u64, LineRow)>,
    paths: Vec<std::path::PathBuf>,
    sequence_base_index: usize,
}

use super::commands::sourcemap;
impl DwarfUnitSourceMap {
    fn transform_lineinfo(&self, row: &LineRow) -> sourcemap::LineInfo {
        let filepath = self.paths[row.file_index() as usize - self.sequence_base_index].clone();
        sourcemap::LineInfo {
            filepath: filepath.to_str().unwrap().to_string(),
            line: row.line(),
            column: match row.column() {
                gimli::ColumnType::Column(c) => sourcemap::ColumnType::Column(c),
                gimli::ColumnType::LeftEdge => sourcemap::ColumnType::LeftEdge,
            },
        }
    }
}

pub struct DwarfSourceMap {
    address_sorted_rows: Vec<(u64, sourcemap::LineInfo)>,
}

impl DwarfSourceMap {
    fn new(units: Vec<DwarfUnitSourceMap>) -> Self {
        let mut rows = BTreeMap::new();
        for unit in &units {
            for (addr, row) in &unit.address_sorted_rows {
                let line_info = unit.transform_lineinfo(row);
                rows.insert(*addr, line_info);
            }
        }
        Self {
            address_sorted_rows: rows.into_iter().collect(),
        }
    }
}

impl sourcemap::SourceMap for DwarfSourceMap {
    fn find_line_info(&self, offset: usize) -> Option<sourcemap::LineInfo> {
        match self
            .address_sorted_rows
            .binary_search_by_key(&(offset as u64), |i| i.0)
        {
            Ok(i) => Some(self.address_sorted_rows[i].1.clone()),
            Err(i) => {
                if i > 0 {
                    Some(self.address_sorted_rows[i - 1].1.clone())
                } else {
                    None
                }
            }
        }
    }
}

use super::commands::subroutine;
pub struct DwarfSubroutineMap<'input> {
    pub subroutines: Vec<Subroutine<Reader<'input>>>,
}

impl<'input> subroutine::SubroutineMap for DwarfSubroutineMap<'input> {
    fn display_variable(&self, code_offset: usize, name: String) {
        let offset = &(code_offset as u64);
        let subroutine = self
            .subroutines
            .iter()
            .filter(|s| s.pc.contains(offset))
            .next();
        // subroutine.
    }
}
