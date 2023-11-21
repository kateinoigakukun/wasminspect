use std::convert::TryFrom;
use wasminspect_vm_macro::{define_instr_kind, TryFromWasmParserOperator};
use wasmparser::*;
#[derive(Debug, Clone)]
pub struct Instruction {
    pub kind: InstructionKind,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct BrTableData {
    pub table: Vec<u32>,
    pub default: u32,
}

trait WasmInstPayloadFrom<T>: Sized {
    type Error;
    fn from_payload(_: T) -> Result<Self, Self::Error>;
}

impl<T, U> WasmInstPayloadFrom<T> for U
where
    U: From<T>,
{
    type Error = wasmparser::BinaryReaderError;
    fn from_payload(from: T) -> Result<U, Self::Error> {
        Ok(From::<T>::from(from))
    }
}

impl WasmInstPayloadFrom<BrTable<'_>> for BrTableData {
    type Error = wasmparser::BinaryReaderError;
    fn from_payload(table: BrTable) -> Result<Self, Self::Error> {
        Ok(BrTableData {
            table: table.targets().collect::<Result<Vec<_>, _>>()?,
            default: table.default(),
        })
    }
}

for_each_operator!(define_instr_kind);

pub fn transform_inst(
    reader: &mut OperatorsReader,
    base_offset: usize,
) -> anyhow::Result<Instruction> {
    let (op, offset) = reader.read_with_offset()?;
    let kind = TryFrom::try_from(op)?;
    Ok(Instruction {
        kind,
        offset: offset - base_offset,
    })
}
