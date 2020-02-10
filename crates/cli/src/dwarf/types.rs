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
pub struct Member<Offset> {
    pub name: Option<String>,
    pub offset: usize,
    pub ty: Offset,
    pub byte_size: u64,
}

#[derive(Debug)]
pub struct StructTypeInfo<Offset> {
    pub name: Option<String>,
    pub members: Vec<Member<Offset>>,
    pub byte_size: u64,
}

#[derive(Debug)]
pub enum TypeInfo<Offset> {
    BaseType(BaseTypeInfo),
    ModifiedType(ModifiedTypeInfo<Offset>),
    StructType(StructTypeInfo<Offset>),
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
    let mut ty = match node.entry().tag() {
        gimli::DW_TAG_base_type => Some(TypeInfo::BaseType(parse_base_type(&node, dwarf, unit)?)),
        gimli::DW_TAG_class_type | gimli::DW_TAG_structure_type => Some(TypeInfo::StructType(
            parse_partial_struct_type(&node, dwarf, unit)?,
        )),
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
            Some(TypeInfo::ModifiedType(parse_modified_type(
                kind, &node, dwarf, unit,
            )?))
        }
        gimli::DW_TAG_member => unreachable!(),
        _ => None,
    };

    let mut children = node.children();
    let mut members = vec![];
    while let Some(child) = children.next()? {
        match child.entry().tag() {
            gimli::DW_TAG_member => members.push(parse_member(&child, dwarf, unit)?),
            _ => parse_types_rec(child, dwarf, unit, out_type_hash)?,
        }
    }

    if let Some(TypeInfo::StructType(ty)) = ty.as_mut() {
        ty.members.append(&mut members);
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
    let name = match node.entry().attr_value(gimli::DW_AT_name)? {
        Some(attr) => clone_string_attribute(dwarf, unit, attr)?,
        None => return Err(anyhow!("Failed to get name")),
    };
    let ty = match node.entry().attr_value(gimli::DW_AT_type)? {
        Some(gimli::AttributeValue::UnitRef(ref offset)) => Some(offset.0),
        x => {
            println!(
                "Failed to get pointee type: {:?} {:?} {:?}",
                node.entry().offset(),
                x,
                kind
            );
            None
        }
    };
    Ok(ModifiedTypeInfo {
        content_ty_offset: ty,
        kind,
    })
}

fn parse_partial_struct_type<R: gimli::Reader>(
    node: &gimli::EntriesTreeNode<R>,
    dwarf: &gimli::Dwarf<R>,
    unit: &gimli::Unit<R, R::Offset>,
) -> Result<StructTypeInfo<R::Offset>> {
    let name = match node.entry().attr_value(gimli::DW_AT_name)? {
        Some(attr) => Some(clone_string_attribute(dwarf, unit, attr)?),
        None => None,
    };
    let byte_size = match node
        .entry()
        .attr_value(gimli::DW_AT_byte_size)?
        .and_then(|attr| attr.udata_value())
    {
        Some(s) => s,
        None => return Err(anyhow!("Failed to get byte_size")),
    };
    let members = vec![];
    Ok(StructTypeInfo {
        name,
        byte_size,
        members,
    })
}

fn parse_member<R: gimli::Reader>(
    node: &gimli::EntriesTreeNode<R>,
    dwarf: &gimli::Dwarf<R>,
    unit: &gimli::Unit<R, R::Offset>,
) -> Result<Member<R::Offset>> {
    let name = match node.entry().attr_value(gimli::DW_AT_name)? {
        Some(attr) => Some(clone_string_attribute(dwarf, unit, attr)?),
        None => None,
    };
    let ty = match node.entry().attr_value(gimli::DW_AT_type)? {
        Some(gimli::AttributeValue::UnitRef(ref offset)) => offset.0,
        _ => return Err(anyhow!("Failed to get type offset")),
    };
    let byte_size = match node
        .entry()
        .attr_value(gimli::DW_AT_byte_size)?
        .and_then(|attr| attr.udata_value())
    {
        Some(s) => s,
        None => return Err(anyhow!("Failed to get byte_size")),
    };
    Ok(Member {
        name,
        byte_size,
        ty,
    })
}
