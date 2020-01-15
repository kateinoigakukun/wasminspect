use gimli::{DebugStr, DebugAbbrev, DebugInfo, DebugLine, LittleEndian};
use parity_wasm::elements::{Module};
use std::collections::HashMap;
pub fn parse_dwarf(module: &Module) {
    let mut sections = HashMap::new();
    for section in module.custom_sections() {
        sections.insert(section.name(), section.payload());
    }
    let endian = LittleEndian;
    let debug_str = DebugStr::new(sections.get(".debug_str").unwrap(), endian);
    let debug_abbrev = DebugAbbrev::new(sections.get(".debug_abbrev").unwrap(), endian);
    let debug_info = DebugInfo::new(sections.get(".debug_info").unwrap(), endian);
    let debug_line = DebugLine::new(sections.get(".debug_line").unwrap(), endian);
}
