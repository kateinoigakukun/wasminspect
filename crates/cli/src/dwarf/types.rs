use gimli;
use anyhow::Result;

use super::utils::*;

pub fn get_types<R: gimli::Reader>(
    dwarf: &gimli::Dwarf<R>,
    unit: &gimli::Unit<R, R::Offset>,
) -> Result<()> {
    let mut entries = unit.entries();
    let _root_cu = entries.next_dfs();

    while let Some((_depth_delta, entry)) = entries.next_dfs()? {
        match entry.tag() {
            gimli::DW_TAG_base_type => {
                let name = match entry.attr_value(gimli::DW_AT_name)? {
                    Some(attr) => Some(clone_string_attribute(dwarf, unit, attr)?),
                    None => None,
                };
                let byte_size = entry
                    .attr_value(gimli::DW_AT_byte_size)?
                    .map(|attr| attr.udata_value());
            }
            _ => {}
        }
    }

    Ok(())
}
