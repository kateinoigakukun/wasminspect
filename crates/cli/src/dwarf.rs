use anyhow::{anyhow, Result};
use gimli::{
    DebugAbbrev, DebugAddr, DebugInfo, DebugLine, DebugLineStr, DebugLoc, DebugLocLists,
    DebugRanges, DebugRngLists, DebugStr, DebugStrOffsets, DebugTypes, DebuggingInformationEntry,
    EndianSlice, LineRow, LittleEndian, LocationLists, RangeLists, Unit,
};
use std::collections::{BTreeMap, HashMap};
use wasmparser::{ModuleReader, SectionCode};

type Reader<'input> = gimli::EndianSlice<'input, LittleEndian>;
pub type Dwarf<'input> = gimli::Dwarf<Reader<'input>>;

pub fn parse_dwarf(module: &[u8]) -> Result<Dwarf> {
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

pub fn transform_dwarf(dwarf: Dwarf) -> Result<()> {
    let mut headers = dwarf.units();
    while let Some(header) = headers.next()? {
        let unit = dwarf.unit(header)?;
    }
    Ok(())
}

use gimli::AttributeValue;
fn extract_attr_string<R: gimli::Reader>(
    attr_value: AttributeValue<R>,
    debug_str: &DebugStr<R>,
) -> Result<String> {
    let bytes = match attr_value {
        AttributeValue::DebugStrRef(str_offset) => debug_str.get_str(str_offset)?,
        AttributeValue::String(b) => b,
    };
    Ok(String::from_utf8(bytes.to_slice()?.to_vec())?)
}

use std::ffi::OsStr;
use std::path::Path;

pub fn transform_debug_line<R: gimli::Reader>(
    unit: &Unit<R, R::Offset>,
    root: &DebuggingInformationEntry<R>,
    dwarf: gimli::Dwarf<R>,
    debug_line: &DebugLine<R>,
) -> Result<DwarfSourceMap> {
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

    let dirs = header.include_directories();
    let mut files = Vec::new();
    for file_entry in header.file_names() {
        let dir = dirs[file_entry.directory_index() as usize];
        let dir_str = Path::new(dwarf.attr_string(unit, dir)?.to_string()?.as_ref());
        let path = dir_str.join(
            dwarf
                .attr_string(unit, file_entry.path_name())?
                .to_string()?
                .as_ref(),
        );
        files.push(path);
    }

    let mut rows = program.rows();
    let mut sorted_rows = BTreeMap::new();
    while let Some((_, row)) = rows.next_row()? {
        sorted_rows.insert(row.address(), row.clone());
    }
    let sorted_rows: Vec<_> = sorted_rows.into_iter().collect();
    Ok(DwarfSourceMap {
        address_sorted_rows: sorted_rows,
        paths: files,
    })
}

pub struct DwarfSourceMap {
    address_sorted_rows: Vec<(u64, LineRow)>,
    paths: Vec<std::path::PathBuf>,
}

use super::commands::sourcemap;
impl DwarfSourceMap {
    fn transform_lineinfo(self, row: LineRow) -> sourcemap::LineInfo {
        let filepath = self.paths[row.file_index() as usize];
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
impl sourcemap::SourceMap for DwarfSourceMap {
    fn find_line_info(&self, offset: usize) -> Option<sourcemap::LineInfo> {
        let row = match self
            .address_sorted_rows
            .binary_search_by_key(&(offset as u64), |i| i.0)
        {
            Ok(i) => Some(self.address_sorted_rows[i].1),
            Err(i) => {
                if i > 0 {
                    Some(self.address_sorted_rows[i].1)
                } else {
                    None
                }
            }
        }?;
        Some(self.transform_lineinfo(row))
    }
}
