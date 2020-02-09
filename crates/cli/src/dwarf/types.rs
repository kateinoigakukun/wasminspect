use anyhow::Result;
use gimli;
use std::collections::HashMap;

use super::utils::*;

#[derive(Debug)]
pub struct BaseTypeInfo {
    pub name: String,
    pub byte_size: u64,
}

#[derive(Debug)]
pub enum TypeInfo {
    BaseType(BaseTypeInfo),
}

pub fn get_types<R: gimli::Reader>(
    dwarf: &gimli::Dwarf<R>,
    unit: &gimli::Unit<R, R::Offset>,
    out_type_hash: &mut HashMap<R::Offset, TypeInfo>,
) -> Result<()> {
    let mut entries = unit.entries();
    let _root_cu = entries.next_dfs();

    while let Some((_depth_delta, entry)) = entries.next_dfs()? {
        match entry.tag() {
            gimli::DW_TAG_base_type => {
                let name = match entry.attr_value(gimli::DW_AT_name)? {
                    Some(attr) => clone_string_attribute(dwarf, unit, attr)?,
                    None => continue,
                };
                let byte_size = match entry
                    .attr_value(gimli::DW_AT_byte_size)?
                    .and_then(|attr| attr.udata_value())
                {
                    Some(s) => s,
                    None => continue,
                };
                out_type_hash.insert(
                    entry.offset().0,
                    TypeInfo::BaseType(BaseTypeInfo { name, byte_size }),
                );
            }
            _ => {}
        }
    }

    Ok(())
}
