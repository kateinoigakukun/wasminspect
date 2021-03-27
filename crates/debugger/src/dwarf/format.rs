use super::utils::*;

use anyhow::{anyhow, Result};
use gimli::Unit;
use num_bigint::{BigInt, BigUint, Sign};

pub fn format_object<R: gimli::Reader>(
    node: gimli::EntriesTreeNode<R>,
    memory: &[u8],
    _encoding: gimli::Encoding,
    dwarf: &gimli::Dwarf<R>,
    unit: &Unit<R>,
) -> Result<String> {
    match node.entry().tag() {
        gimli::DW_TAG_base_type => {
            let entry = node.entry();
            let name = match entry.attr_value(gimli::DW_AT_name)? {
                Some(attr) => clone_string_attribute(dwarf, unit, attr)?,
                None => "<no type name>".to_string(),
            };
            let byte_size = entry
                .attr_value(gimli::DW_AT_byte_size)?
                .and_then(|attr| attr.udata_value())
                .ok_or(anyhow!("Failed to get byte_size"))?;
            let encoding = entry
                .attr_value(gimli::DW_AT_encoding)?
                .and_then(|attr| match attr {
                    gimli::AttributeValue::Encoding(encoding) => Some(encoding),
                    _ => None,
                })
                .ok_or(anyhow!("Failed to get type encoding"))?;
            let mut bytes = Vec::with_capacity(8);
            bytes.copy_from_slice(&memory[0..(byte_size as usize)]);

            match encoding {
                gimli::DW_ATE_signed => {
                    let value = BigInt::from_bytes_le(Sign::NoSign, &bytes);
                    Ok(format!("{}({})", name, value))
                }
                gimli::DW_ATE_unsigned => {
                    let value = BigUint::from_bytes_le(&bytes);
                    Ok(format!("{}({})", name, value))
                }
                _ => unimplemented!(),
            }
        }
        gimli::DW_TAG_class_type | gimli::DW_TAG_structure_type => {
            let entry = node.entry();
            let type_name = match entry.attr_value(gimli::DW_AT_name)? {
                Some(attr) => clone_string_attribute(dwarf, unit, attr)?,
                None => "<no type name>".to_string(),
            };
            let mut children = node.children();
            let mut members = vec![];
            while let Some(child) = children.next()? {
                match child.entry().tag() {
                    gimli::DW_TAG_member => {
                        let name = match child.entry().attr_value(gimli::DW_AT_name)? {
                            Some(attr) => clone_string_attribute(dwarf, unit, attr)?,
                            None => "<no member name>".to_string(),
                        };
                        // let ty = match entry.attr_value(gimli::DW_AT_type)? {
                        //     Some(gimli::AttributeValue::UnitRef(ref offset)) => offset.0,
                        //     _ => return Err(anyhow!("Failed to get type offset")),
                        // };
                        members.push(name);
                    }
                    _ => continue,
                }
            }
            Ok(format!("{} {{\n{}\n}}", type_name, members.join(",\n")))
        }
        _ => Err(anyhow!("unsupported DIE type")),
    }
}
