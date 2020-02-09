use super::types::TypeInfo;

pub fn format_object(ty: &TypeInfo, memory: &[u8]) -> String {
    match ty {
        TypeInfo::BaseType(base_type) => {
            let type_name: &str = &base_type.name;
            match type_name {
                "int" => {
                    let mut bytes: [u8; 4] = Default::default();
                    bytes.copy_from_slice(&memory[0..(base_type.byte_size as usize)]);
                    format!("{}({})", base_type.name, i32::from_le_bytes(bytes))
                }
                _ => unimplemented!()
            }
        }
    }
}