use anyhow::{anyhow, Result};
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
    let mut tree = unit.entries_tree(None)?;
    let root = tree.root()?;
    parse_types_rec(root, dwarf, unit, out_type_hash)?;
    Ok(())
}
pub fn parse_types_rec<R: gimli::Reader>(
    node: gimli::EntriesTreeNode<R>,
    dwarf: &gimli::Dwarf<R>,
    unit: &gimli::Unit<R, R::Offset>,
    out_type_hash: &mut HashMap<R::Offset, TypeInfo>,
) -> Result<()> {
    match node.entry().tag() {
        gimli::DW_TAG_base_type => {
            out_type_hash.insert(
                node.entry().offset().0,
                TypeInfo::BaseType(parse_base_type(&node, dwarf, unit)?),
            );
        }
        gimli::DW_TAG_pointer_type => {}
        _ => {}
    }

    let mut children = node.children();
    while let Some(child) = children.next()? {
        match child.entry().tag() {
            _ => parse_types_rec(child, dwarf, unit, out_type_hash)?,
        }
    }

    Ok(())
}

fn parse_base_type<R: gimli::Reader>(
    node: &gimli::EntriesTreeNode<R>,
    dwarf: &gimli::Dwarf<R>,
    unit: &gimli::Unit<R, R::Offset>,
) -> Result<BaseTypeInfo> {
    let name = match node.entry().attr_value(gimli::DW_AT_name)? {
        Some(attr) => clone_string_attribute(dwarf, unit, attr)?,
        None => return Err(anyhow!("Failed to get name")),
    };
    let byte_size = match node
        .entry()
        .attr_value(gimli::DW_AT_byte_size)?
        .and_then(|attr| attr.udata_value())
    {
        Some(s) => s,
        None => return Err(anyhow!("Failed to get byte_size")),
    };
    Ok(BaseTypeInfo { name, byte_size })
}
