use super::evaluate_variable_location;
use super::types::*;
use super::Reader;
use std::collections::HashMap;

use anyhow::{anyhow, Result};

pub fn type_name<'input>(
    ty_offset: Option<usize>,
    type_hash: &HashMap<usize, TypeInfo<Reader<'input>>>,
) -> Result<String> {
    let ty_offset = match ty_offset {
        Some(o) => o,
        None => return Ok("void".to_string()),
    };
    let ty = type_hash
        .get(&ty_offset)
        .ok_or(anyhow!("Failed to get type from offset '{}'", ty_offset))?;
    let result = match ty {
        TypeInfo::BaseType(base_type) => base_type.name.clone(),
        TypeInfo::StructType(struct_type) => struct_type
            .name
            .clone()
            .unwrap_or("struct <<not parsed yet>>".to_string()),
        TypeInfo::EnumerationType(enum_type) => enum_type
            .name
            .clone()
            .unwrap_or("enum <<not parsed yet>>".to_string()),
        TypeInfo::TypeDef(type_def) => type_def
            .name
            .clone()
            .unwrap_or("typedef <<not parsed yet>>".to_string()),
        TypeInfo::ModifiedType(mod_type) => match mod_type.kind {
            ModifierKind::Atomic => format!(
                "std::atomic<{}>",
                type_name(mod_type.content_ty_offset, type_hash)?
            ),
            ModifierKind::Const => format!(
                "const {}",
                type_name(mod_type.content_ty_offset, type_hash)?
            ),
            ModifierKind::Immutable => format!(
                "immutable {}",
                type_name(mod_type.content_ty_offset, type_hash)?
            ),
            ModifierKind::Packed => format!(
                "packed {}",
                type_name(mod_type.content_ty_offset, type_hash)?
            ),
            ModifierKind::Pointer => {
                format!("{}*", type_name(mod_type.content_ty_offset, type_hash)?)
            }
            ModifierKind::Reference => {
                format!("{}&", type_name(mod_type.content_ty_offset, type_hash)?)
            }
            ModifierKind::Restrict => format!(
                "restrict {}",
                type_name(mod_type.content_ty_offset, type_hash)?
            ),
            ModifierKind::RvalueReference => {
                format!("{}&&", type_name(mod_type.content_ty_offset, type_hash)?)
            }
            ModifierKind::Shared => format!(
                "shared {}",
                type_name(mod_type.content_ty_offset, type_hash)?
            ),
            ModifierKind::Volatile => format!(
                "violate {}",
                type_name(mod_type.content_ty_offset, type_hash)?
            ),
        },
    };
    Ok(result)
}

pub fn format_object<'input>(
    ty_offset: usize,
    memory: &[u8],
    encoding: gimli::Encoding,
    type_hash: &HashMap<usize, TypeInfo<Reader<'input>>>,
) -> Result<String> {
    let ty = type_hash
        .get(&ty_offset)
        .ok_or(anyhow!("Failed to get type from offset '{}'", ty_offset))?;
    match ty {
        TypeInfo::BaseType(base_type) => {
            let type_name: &str = &base_type.name;
            match type_name {
                "int" => {
                    let mut bytes: [u8; 4] = Default::default();
                    bytes.copy_from_slice(&memory[0..(base_type.byte_size as usize)]);
                    Ok(format!("{}({})", base_type.name, i32::from_le_bytes(bytes)))
                }
                "long unsigned int" => {
                    let mut bytes: [u8; 4] = Default::default();
                    bytes.copy_from_slice(&memory[0..(base_type.byte_size as usize)]);
                    Ok(format!("{}({})", base_type.name, u32::from_le_bytes(bytes)))
                }
                "long long unsigned int" => {
                    let mut bytes: [u8; 8] = Default::default();
                    bytes.copy_from_slice(&memory[0..(base_type.byte_size as usize)]);
                    Ok(format!("{}({})", base_type.name, u64::from_le_bytes(bytes)))
                }
                "unsigned __int128" => {
                    let mut bytes: [u8; 16] = Default::default();
                    bytes.copy_from_slice(&memory[0..(base_type.byte_size as usize)]);
                    Ok(format!(
                        "{}({})",
                        base_type.name,
                        u128::from_le_bytes(bytes)
                    ))
                }
                "char" => Ok(String::from_utf8(vec![memory[0]])
                    .unwrap_or("<<invalid utf8 char>>".to_string())),
                _ => unimplemented!(),
            }
        }
        TypeInfo::StructType(struct_type) => {
            if let Some(type_name) = struct_type.name.clone() {
                let type_name: &str = &type_name;
                // For Swift Support
                match type_name {
                    "UnsafeRawPointer" | "UnsafeMutableRawPointer" => {
                    let mut bytes: [u8; 4] = Default::default();
                    bytes.copy_from_slice(&memory[0..4]);
                    return Ok(format!(
                        "{} (0x{:x})",
                        type_name,
                        u32::from_le_bytes(bytes)
                    ));
                    }
                    _ => (),
                }
            }

            let mut members_str = vec![];
            for member in &struct_type.members {
                let offset: usize = match member.location {
                    MemberLocation::ConstOffset(offset) => offset as usize,
                    MemberLocation::LocationDescription(expr) => {
                        let pieces = evaluate_variable_location(encoding, 0, expr)?;
                        let piece = match pieces.iter().next() {
                            Some(p) => p,
                            None => panic!(),
                        };
                        match piece.location {
                            gimli::Location::Address { address } => address as usize,
                            _ => unimplemented!(),
                        }
                    }
                };
                members_str.push(format!(
                    "{}: {}",
                    member
                        .name
                        .clone()
                        .unwrap_or("<<not parsed yet>>".to_string()),
                    format_object(member.ty, &memory[offset..], encoding, type_hash)?
                ))
            }
            Ok(format!(
                "{} {{\n{}\n}}",
                struct_type
                    .name
                    .clone()
                    .unwrap_or("<<not parsed yet>>".to_string()),
                members_str.join(",\n"),
            ))
        }
        TypeInfo::EnumerationType(enum_type) => {
            if let Some(offset) = enum_type.ty {
                match type_hash.get(&offset) {
                    Some(TypeInfo::BaseType(base_ty)) => {
                        if base_ty.name != "int" {
                            return Err(anyhow!(
                                "{} is not supported as enum content type",
                                base_ty.name
                            ));
                        }
                    }
                    Some(_) => {
                        return Err(anyhow!(
                            "enum content type '{}' should be base type",
                            type_name(Some(offset), type_hash)?
                        ))
                    }
                    None => return Err(anyhow!("failed to get enum content type")),
                }
            }
            let mut bytes: [u8; 4] = Default::default();
            bytes.copy_from_slice(&memory[0..4]);
            let value = i32::from_le_bytes(bytes);
            for enumerator in &enum_type.enumerators {
                if let Some(const_value) = enumerator.value {
                    if (const_value as i32) == value {
                        return Ok(format!(
                            "{} ({})",
                            enum_type
                                .name
                                .clone()
                                .unwrap_or("<<not parsed yet>>".to_string()),
                            enumerator
                                .name
                                .clone()
                                .unwrap_or("<<not parsed yet>>".to_string())
                        ));
                    }
                }
            }
            Err(anyhow!("Failed to find enumerator case for '{}'"))
        }
        TypeInfo::TypeDef(type_def) => {
            if let Some(ty_offset) = type_def.ty {
                Ok(format!(
                    "typedef {} {}",
                    type_def
                        .name
                        .clone()
                        .unwrap_or("<<not parsed yet>>".to_string()),
                    format_object(ty_offset, &memory, encoding, type_hash)?
                ))
            } else {
                Ok(format!(
                    "typedef {} <<not parsed yet>>",
                    type_def
                        .name
                        .clone()
                        .unwrap_or("<<not parsed yet>>".to_string())
                ))
            }
        }
        TypeInfo::ModifiedType(mod_type) => match mod_type.kind {
            ModifierKind::Pointer | ModifierKind::Reference => {
                let modifier = match mod_type.kind {
                    ModifierKind::Pointer => "*",
                    ModifierKind::Reference => "&",
                    _ => unreachable!(),
                };
                let mut bytes: [u8; 4] = Default::default();
                bytes.copy_from_slice(&memory[0..4]);
                Ok(format!(
                    "{}{} (0x{:x})",
                    type_name(mod_type.content_ty_offset, type_hash)?,
                    modifier,
                    u32::from_le_bytes(bytes)
                ))
            }
            _ => {
                if let Some(offset) = mod_type.content_ty_offset {
                    return Ok(format!(
                        "{}({})",
                        type_name(Some(ty_offset), type_hash)?,
                        format_object(offset, memory, encoding, type_hash)?
                    ));
                } else {
                    return Ok(format!(
                        "{}(unknown)",
                        type_name(Some(ty_offset), type_hash)?,
                    ));
                }
            }
        },
    }
}
