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
        None => return Ok("none".to_string()),
    };
    let ty = type_hash
        .get(&ty_offset)
        .ok_or(anyhow!("Failed to get type from offset '{}'", ty_offset))?;
    let result = match ty {
        TypeInfo::BaseType(base_type) => base_type.name.clone(),
        TypeInfo::StructType(struct_type) => struct_type.name.clone().unwrap_or("none".to_string()),
        TypeInfo::ModifiedType(mod_type) => {
            match mod_type.kind {
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
            }
        }
    };
    Ok(result)
}

pub fn format_object<'input>(
    ty_offset: usize,
    memory: &[u8],
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
                _ => unimplemented!(),
            }
        }
        TypeInfo::StructType(struct_type) => {
            unimplemented!();
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
                    i32::from_le_bytes(bytes)
                ))
            }
            _ => {
                if let Some(offset) = mod_type.content_ty_offset {
                    return Ok(format!(
                        "{}({})",
                        type_name(Some(ty_offset), type_hash)?,
                        format_object(offset, memory, type_hash)?
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
