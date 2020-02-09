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
pub enum ModifierKind {
    Atomic,
    Const,
    Immutable,
    Packed,
    Pointer,
    Reference,
    Restrict,
    RvalueReference,
    Shared,
    Volatile,
}
#[derive(Debug)]
pub struct ModifiedTypeInfo<Offset> {
    pub content_ty_offset: Option<Offset>,
    pub kind: ModifierKind,
}

#[derive(Debug)]
pub enum TypeInfo<Offset> {
    BaseType(BaseTypeInfo),
    ModifiedType(ModifiedTypeInfo<Offset>),
}

pub fn get_types<R: gimli::Reader>(
    dwarf: &gimli::Dwarf<R>,
    unit: &gimli::Unit<R, R::Offset>,
    out_type_hash: &mut HashMap<R::Offset, TypeInfo<R::Offset>>,
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
    out_type_hash: &mut HashMap<R::Offset, TypeInfo<R::Offset>>,
) -> Result<()> {
    match node.entry().tag() {
        gimli::DW_TAG_base_type => {
            out_type_hash.insert(
                node.entry().offset().0,
                TypeInfo::BaseType(parse_base_type(&node, dwarf, unit)?),
            );
        }
        gimli::DW_TAG_atomic_type
        | gimli::DW_TAG_const_type
        | gimli::DW_TAG_immutable_type
        | gimli::DW_TAG_packed_type
        | gimli::DW_TAG_pointer_type
        | gimli::DW_TAG_reference_type
        | gimli::DW_TAG_restrict_type
        | gimli::DW_TAG_rvalue_reference_type
        | gimli::DW_TAG_shared_type
        | gimli::DW_TAG_volatile_type => {
            let kind = match node.entry().tag() {
                gimli::DW_TAG_atomic_type => ModifierKind::Atomic,
                gimli::DW_TAG_const_type => ModifierKind::Const,
                gimli::DW_TAG_immutable_type => ModifierKind::Immutable,
                gimli::DW_TAG_packed_type => ModifierKind::Packed,
                gimli::DW_TAG_pointer_type => ModifierKind::Pointer,
                gimli::DW_TAG_reference_type => ModifierKind::Reference,
                gimli::DW_TAG_restrict_type => ModifierKind::Restrict,
                gimli::DW_TAG_rvalue_reference_type => ModifierKind::RvalueReference,
                gimli::DW_TAG_shared_type => ModifierKind::Shared,
                gimli::DW_TAG_volatile_type => ModifierKind::Volatile,
                _ => unreachable!(),
            };
            out_type_hash.insert(
                node.entry().offset().0,
                TypeInfo::ModifiedType(parse_modified_type(kind, &node, dwarf, unit)?),
            );
        }
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

fn parse_modified_type<R: gimli::Reader>(
    kind: ModifierKind,
    node: &gimli::EntriesTreeNode<R>,
    dwarf: &gimli::Dwarf<R>,
    unit: &gimli::Unit<R, R::Offset>,
) -> Result<ModifiedTypeInfo<R::Offset>> {
    let ty = match node.entry().attr_value(gimli::DW_AT_type)? {
        Some(gimli::AttributeValue::UnitRef(ref offset)) => Some(offset.0),
        x => {
            println!("Failed to get pointee type: {:?} {:?} {:?}", node.entry().offset(), x, kind);
            None
        },
    };
    Ok(ModifiedTypeInfo {
        content_ty_offset: ty,
        kind,
    })
}
