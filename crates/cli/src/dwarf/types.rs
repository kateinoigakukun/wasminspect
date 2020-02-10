use anyhow::{anyhow, Result};
use gimli;
use log::debug;
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
pub struct Member<R: gimli::Reader> {
    pub name: Option<String>,
    pub ty: R::Offset,
    pub location: MemberLocation<R>,
}

#[derive(Debug)]
pub enum MemberLocation<R: gimli::Reader> {
    LocationDescription(gimli::Expression<R>),
    ConstOffset(u64),
}

#[derive(Debug)]
pub struct StructTypeInfo<R: gimli::Reader> {
    pub name: Option<String>,
    pub members: Vec<Member<R>>,
    pub byte_size: u64,
    pub declaration: bool,
}

#[derive(Debug)]
pub struct TypeDef<R: gimli::Reader> {
    pub name: Option<String>,
    pub ty: Option<R::Offset>,
}

#[derive(Debug)]
pub enum TypeInfo<R: gimli::Reader> {
    BaseType(BaseTypeInfo),
    ModifiedType(ModifiedTypeInfo<R::Offset>),
    StructType(StructTypeInfo<R>),
    TypeDef(TypeDef<R>),
}

pub fn get_types<R: gimli::Reader>(
    dwarf: &gimli::Dwarf<R>,
    unit: &gimli::Unit<R, R::Offset>,
    out_type_hash: &mut HashMap<R::Offset, TypeInfo<R>>,
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
    out_type_hash: &mut HashMap<R::Offset, TypeInfo<R>>,
) -> Result<()> {
    let offset = node.entry().offset();
    let mut ty = match node.entry().tag() {
        gimli::DW_TAG_base_type => Some(TypeInfo::<R>::BaseType(parse_base_type(
            &node, dwarf, unit,
        )?)),
        gimli::DW_TAG_class_type | gimli::DW_TAG_structure_type => Some(TypeInfo::<R>::StructType(
            parse_partial_struct_type(&node, dwarf, unit)?,
        )),
        gimli::DW_TAG_typedef => Some(TypeInfo::<R>::TypeDef(parse_typedef(&node, dwarf, unit)?)),
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
            Some(TypeInfo::ModifiedType(parse_modified_type(kind, &node)?))
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
    if let Some(ty) = ty {
        out_type_hash.insert(offset.0, ty);
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
) -> Result<ModifiedTypeInfo<R::Offset>> {
    let ty = match node.entry().attr_value(gimli::DW_AT_type)? {
        Some(gimli::AttributeValue::UnitRef(ref offset)) => Some(offset.0),
        x => {
            debug!(
                "Failed to get pointee type: {:?} {:?} {:?}",
                node.entry().offset(),
                x,
                kind
            );
            let mut attrs = node.entry().attrs();
            while let Some(attr) = attrs.next()? {
                debug!("The entry has '{}'", attr.name());
            }
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
) -> Result<StructTypeInfo<R>> {
    let mut ty = StructTypeInfo {
        name: None,
        members: vec![],
        byte_size: 0,
        declaration: false,
    };
    if let Some(attr) = node.entry().attr_value(gimli::DW_AT_name)? {
        ty.name = Some(clone_string_attribute(dwarf, unit, attr)?);
    }

    if let Some(byte_size) = node
        .entry()
        .attr_value(gimli::DW_AT_byte_size)?
        .and_then(|attr| attr.udata_value())
    {
        ty.byte_size = byte_size;
    };
    if let Some(declaration) = node.entry().attr_value(gimli::DW_AT_declaration)? {
        if let gimli::AttributeValue::Flag(flag) = declaration {
            ty.declaration = flag;
        }
    }
    Ok(ty)
}

fn parse_member<R: gimli::Reader>(
    node: &gimli::EntriesTreeNode<R>,
    dwarf: &gimli::Dwarf<R>,
    unit: &gimli::Unit<R, R::Offset>,
) -> Result<Member<R>> {
    let name = match node.entry().attr_value(gimli::DW_AT_name)? {
        Some(attr) => Some(clone_string_attribute(dwarf, unit, attr)?),
        None => None,
    };
    let ty = match node.entry().attr_value(gimli::DW_AT_type)? {
        Some(gimli::AttributeValue::UnitRef(ref offset)) => offset.0,
        _ => return Err(anyhow!("Failed to get type offset")),
    };
    // DWARF v5 Page 118
    let mut member_location = MemberLocation::ConstOffset(0);
    if let Some(loc_attr) = node.entry().attr_value(gimli::DW_AT_data_member_location)? {
        match loc_attr {
            gimli::AttributeValue::Udata(offset) => {
                member_location = MemberLocation::ConstOffset(offset);
            }
            gimli::AttributeValue::Exprloc(expr) => {
                member_location = MemberLocation::LocationDescription(expr);
            }
            _ => unimplemented!(),
        }
    }
    Ok(Member {
        name,
        location: member_location,
        ty,
    })
}

fn parse_typedef<R: gimli::Reader>(
    node: &gimli::EntriesTreeNode<R>,
    dwarf: &gimli::Dwarf<R>,
    unit: &gimli::Unit<R, R::Offset>,
) -> Result<TypeDef<R>> {
    let name = match node.entry().attr_value(gimli::DW_AT_name)? {
        Some(attr) => Some(clone_string_attribute(dwarf, unit, attr)?),
        None => None,
    };
    let ty = match node.entry().attr_value(gimli::DW_AT_type)? {
        Some(gimli::AttributeValue::UnitRef(ref offset)) => Some(offset.0),
        _ => None,
    };
    Ok(TypeDef { name, ty })
}
