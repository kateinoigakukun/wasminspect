use anyhow::Result;
use gimli::{
    DebugAbbrev, DebugAddr, DebugInfo, DebugLine, DebugLineStr, DebugLoc, DebugLocLists,
    DebugRanges, DebugRngLists, DebugStr, DebugStrOffsets, DebugTypes, EndianSlice, LittleEndian,
    LocationLists, RangeLists,
};
use std::collections::HashMap;
use wasmparser::{ModuleReader, SectionCode};

type Reader<'input> = gimli::EndianSlice<'input, LittleEndian>;
pub type Dwarf<'input> = gimli::Dwarf<Reader<'input>>;
pub type Unit<'input> = gimli::Unit<Reader<'input>>;

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
        transform_unit(unit)?
    }
    Ok(())
}

pub fn transform_unit<'input>(unit: Unit<'input>) -> Result<()> {
    let mut entries = unit.entries();
    if let Some((_depth, _cu_die)) = entries.next_dfs()? {}
    Ok(())
}

pub fn find_debug_line<'input>(_unit: Unit<'input>) {}
