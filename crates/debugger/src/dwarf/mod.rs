use anyhow::{anyhow, Context, Result};
use gimli::{
    AttributeValue, CompilationUnitHeader, DebugAbbrev, DebugAddr, DebugInfo, DebugInfoOffset,
    DebugLine, DebugLineStr, DebugLoc, DebugLocLists, DebugRanges, DebugRngLists, DebugStr,
    DebugStrOffsets, DebugTypes, DebuggingInformationEntry, EndianSlice, EntriesTree, LineRow,
    LittleEndian, LocationLists, RangeLists, Unit, UnitOffset,
};
use log::trace;
use std::collections::{BTreeMap, HashMap};

mod format;
mod types;
mod utils;

use utils::*;

type Reader<'input> = gimli::EndianSlice<'input, LittleEndian>;
pub type Dwarf<'input> = gimli::Dwarf<Reader<'input>>;

pub fn parse_dwarf<'a>(module: &'a [u8]) -> Result<Dwarf<'a>> {
    const EMPTY_SECTION: &[u8] = &[];
    let parser = wasmparser::Parser::new(0);
    let mut sections = HashMap::new();
    for payload in parser.parse_all(module) {
        match payload? {
            wasmparser::Payload::CustomSection { name, data, .. } => {
                sections.insert(name, data);
            }
            _ => continue,
        }
    }
    let try_get = |key: &str| sections.get(key).ok_or(anyhow!("no {}", key));
    let endian = LittleEndian;
    let debug_str = DebugStr::new(try_get(".debug_str")?, endian);
    let debug_abbrev = DebugAbbrev::new(try_get(".debug_abbrev")?, endian);
    let debug_info = DebugInfo::new(try_get(".debug_info")?, endian);
    let debug_line = DebugLine::new(try_get(".debug_line")?, endian);
    let debug_addr = DebugAddr::from(EndianSlice::new(EMPTY_SECTION, endian));
    let debug_line_str = match sections.get(".debug_line_str") {
        Some(section) => DebugLineStr::from(EndianSlice::new(section, endian)),
        None => DebugLineStr::from(EndianSlice::new(EMPTY_SECTION, endian)),
    };
    let debug_str_sup = DebugStr::from(EndianSlice::new(EMPTY_SECTION, endian));
    let debug_ranges = match sections.get(".debug_ranges") {
        Some(section) => DebugRanges::new(section, endian),
        None => DebugRanges::new(EMPTY_SECTION, endian),
    };
    let debug_rnglists = DebugRngLists::new(EMPTY_SECTION, endian);
    let ranges = RangeLists::new(debug_ranges, debug_rnglists);
    let debug_loc = match sections.get(".debug_loc") {
        Some(section) => DebugLoc::new(section, endian),
        None => DebugLoc::new(EMPTY_SECTION, endian),
    };
    let debug_loclists = DebugLocLists::new(EMPTY_SECTION, endian);
    let locations = LocationLists::new(debug_loc, debug_loclists);
    let debug_str_offsets = DebugStrOffsets::from(EndianSlice::new(EMPTY_SECTION, endian));
    let debug_types = DebugTypes::from(EndianSlice::new(EMPTY_SECTION, endian));

    Ok(Dwarf {
        debug_abbrev,
        debug_addr,
        debug_info,
        debug_line,
        debug_line_str,
        debug_str,
        debug_str_offsets,
        debug_str_sup,
        debug_types,
        locations,
        ranges,
    })
}

pub struct DwarfDebugInfo<'a> {
    pub sourcemap: DwarfSourceMap,
    pub subroutine: DwarfSubroutineMap<'a>,
}
pub fn transform_dwarf<'a>(buffer: &'a [u8]) -> Result<DwarfDebugInfo<'a>> {
    let dwarf = parse_dwarf(buffer)?;
    let mut headers = dwarf.units();
    let mut sourcemaps = Vec::new();
    let mut subroutines = Vec::new();

    let mut type_hash = HashMap::new();
    while let Some(header) = headers.next()? {
        let unit = dwarf.unit(header)?;
        let mut entries = unit.entries();
        let root = match entries.next_dfs()? {
            Some((_, entry)) => entry,
            None => continue,
        };
        sourcemaps.push(transform_debug_line(
            &unit,
            root,
            &dwarf,
            &dwarf.debug_line,
        )?);
        subroutines.append(&mut transform_subprogram(&dwarf, &unit, header.offset())?);
        get_types(&dwarf, &unit, &mut type_hash)?;
    }
    Ok(DwarfDebugInfo {
        sourcemap: DwarfSourceMap::new(sourcemaps),
        subroutine: DwarfSubroutineMap {
            subroutines,
            type_hash,
            buffer: buffer.to_vec(),
        },
    })
}

#[derive(Clone)]
pub struct SymbolVariable<R>
where
    R: gimli::Reader,
{
    name: Option<String>,
    content: VariableContent<R>,
    ty_offset: Option<R::Offset>,
}

#[derive(Clone)]
enum VariableContent<R: gimli::Reader> {
    Location(gimli::AttributeValue<R>),
    ConstValue(Vec<u8>),
    Unknown { debug_info: String },
}

#[derive(Debug)]
pub struct Subroutine<Offset> {
    pub name: Option<String>,
    pub pc: std::ops::Range<u64>,
    pub entry_offset: UnitOffset<Offset>,
    pub unit_offset: DebugInfoOffset<Offset>,
    pub encoding: gimli::Encoding,
    pub frame_base: Option<WasmLoc>,
}

pub fn transform_subprogram<R: gimli::Reader>(
    dwarf: &gimli::Dwarf<R>,
    unit: &Unit<R, R::Offset>,
    unit_offset: DebugInfoOffset<R::Offset>,
) -> Result<Vec<Subroutine<R::Offset>>> {
    let mut tree = unit.entries_tree(None)?;
    let root = tree.root()?;
    let mut subroutines = vec![];
    transform_subprogram_rec(root, dwarf, unit, unit_offset, &mut subroutines)?;
    Ok(subroutines)
}

#[allow(non_camel_case_types)]
enum DwAtWasm {
    DW_OP_WASM_location = 0xed,
}

#[derive(Clone, Copy, Debug)]
pub enum WasmLoc {
    Local(u64),
    Global(u64),
    Stack(u64),
}

// See also https://yurydelendik.github.io/webassembly-dwarf/#DWARF-expressions-and-location-descriptions
fn read_wasm_location<R: gimli::Reader>(attr_value: AttributeValue<R>) -> Result<WasmLoc> {
    let mut bytes_reader = match attr_value {
        AttributeValue::Exprloc(ref expr) => expr.0.clone(),
        _ => Err(anyhow!("unexpected attribute kind: {:?}", attr_value))?,
    };

    if bytes_reader.is_empty() {
        Err(anyhow!("byte sequence should not be empty"))?
    }
    let magic = bytes_reader.read_u8()?;
    if magic != DwAtWasm::DW_OP_WASM_location as u8 {
        Err(anyhow!("invalid wasm location magic: {:?}", magic))?
    }
    let wasm_op = bytes_reader.read_u8()?;
    let loc = match wasm_op {
        0x00 => WasmLoc::Local(bytes_reader.read_uleb128()?),
        0x01 => WasmLoc::Global(bytes_reader.read_uleb128()?),
        0x02 => WasmLoc::Stack(bytes_reader.read_uleb128()?),
        0x03 => WasmLoc::Global(bytes_reader.read_u32()? as u64),
        _ => Err(anyhow!("invalid wasm location operation: {:?}", wasm_op))?,
    };
    Ok(loc)
}

fn read_subprogram_header<R: gimli::Reader>(
    node: &gimli::EntriesTreeNode<R>,
    dwarf: &gimli::Dwarf<R>,
    unit: &Unit<R, R::Offset>,
    unit_offset: DebugInfoOffset<R::Offset>,
) -> Result<Option<Subroutine<R::Offset>>> {
    match node.entry().tag() {
        gimli::DW_TAG_subprogram | gimli::DW_TAG_lexical_block => (),
        _ => return Ok(None),
    };

    let name = match node.entry().attr_value(gimli::DW_AT_name)? {
        Some(attr) => Some(clone_string_attribute(dwarf, unit, attr)?),
        None => None,
    };

    let low_pc_attr = node.entry().attr_value(gimli::DW_AT_low_pc)?;
    trace!("low_pc_attr: {:?}", low_pc_attr);
    let high_pc_attr = node.entry().attr_value(gimli::DW_AT_high_pc)?;
    trace!("high_pc_attr: {:?}", high_pc_attr);
    let frame_base_attr = node.entry().attr_value(gimli::DW_AT_frame_base)?;

    let subroutine = if let Some(AttributeValue::Addr(low_pc)) = low_pc_attr {
        let high_pc = match high_pc_attr {
            Some(AttributeValue::Udata(size)) => low_pc + size,
            Some(AttributeValue::Addr(high_pc)) => high_pc,
            Some(x) => unreachable!("high_pc can't be {:?}", x),
            None => return Ok(None),
        };
        let frame_base = if let Some(attr) = frame_base_attr {
            Some(read_wasm_location(attr)?)
        } else {
            None
        };
        Subroutine {
            pc: low_pc..high_pc,
            name,
            encoding: unit.encoding(),
            entry_offset: node.entry().offset(),
            unit_offset: unit_offset,
            frame_base: frame_base,
        }
    } else {
        return Ok(None);
    };
    Ok(Some(subroutine))
}

pub fn transform_subprogram_rec<R: gimli::Reader>(
    node: gimli::EntriesTreeNode<R>,
    dwarf: &gimli::Dwarf<R>,
    unit: &Unit<R, R::Offset>,
    unit_offset: DebugInfoOffset<R::Offset>,
    out_subroutines: &mut Vec<Subroutine<R::Offset>>,
) -> Result<()> {
    let mut subroutine = read_subprogram_header(&node, dwarf, unit, unit_offset)?;
    let mut children = node.children();
    while let Some(child) = children.next()? {
        match child.entry().tag() {
            gimli::DW_TAG_variable | gimli::DW_TAG_formal_parameter => {
                continue;
            }
            _ => {
                transform_subprogram_rec(child, dwarf, unit, unit_offset, out_subroutines)?;
            }
        }
    }

    if let Some(subroutine) = subroutine.take() {
        out_subroutines.push(subroutine);
    }

    Ok(())
}

fn transform_variable<R: gimli::Reader>(
    dwarf: &gimli::Dwarf<R>,
    unit: &Unit<R, R::Offset>,
    entry: &DebuggingInformationEntry<R>,
) -> Result<SymbolVariable<R>> {
    let mut content = VariableContent::Unknown {
        debug_info: "".to_string(), //format!("{:?}", entry.attrs()),
    };
    let mut has_explicit_location = false;
    if let Some(location) = entry.attr_value(gimli::DW_AT_location)? {
        content = VariableContent::Location(location);
        has_explicit_location = true;
    }
    if let Some(constant) = entry.attr_value(gimli::DW_AT_const_value)? {
        if !has_explicit_location {
            // TODO: support big endian
            let bytes = match constant {
                AttributeValue::Block(block) => block.to_slice()?.to_vec(),
                AttributeValue::Data1(b) => vec![b],
                AttributeValue::Data2(b) => b.to_le_bytes().to_vec(),
                AttributeValue::Data4(b) => b.to_le_bytes().to_vec(),
                AttributeValue::Data8(b) => b.to_le_bytes().to_vec(),
                AttributeValue::Sdata(b) => b.to_le_bytes().to_vec(),
                AttributeValue::Udata(b) => b.to_le_bytes().to_vec(),
                AttributeValue::String(b) => b.to_slice()?.to_vec(),
                _ => unimplemented!(),
            };
            content = VariableContent::ConstValue(bytes);
        }
    }
    let name = match entry.attr_value(gimli::DW_AT_name)? {
        Some(name_attr) => Some(clone_string_attribute(dwarf, unit, name_attr)?),
        None => None,
    };

    let ty = match entry.attr_value(gimli::DW_AT_type)? {
        Some(AttributeValue::UnitRef(ref offset)) => Some(offset.0),
        _ => None,
    };
    Ok(SymbolVariable {
        name,
        content,
        ty_offset: ty,
    })
}

#[derive(Debug)]
pub enum FrameBase {
    WasmFrameBase(u64),
    RBP(u64),
}

use gimli::Expression;
fn evaluate_variable_location<R: gimli::Reader>(
    encoding: gimli::Encoding,
    base: FrameBase,
    expr: Expression<R>,
) -> Result<Vec<gimli::Piece<R>>> {
    let mut evaluation = expr.evaluation(encoding);
    if let FrameBase::RBP(base) = base {
        evaluation.set_initial_value(base);
    }
    let mut result = evaluation.evaluate()?;
    use gimli::EvaluationResult;
    loop {
        if let EvaluationResult::Complete = result {
            return Ok(evaluation.result());
        }
        match result {
            EvaluationResult::RequiresFrameBase => {
                if let FrameBase::WasmFrameBase(base) = base {
                    result = evaluation.resume_with_frame_base(base)?;
                } else {
                    return Err(anyhow!("unexpected occurrence of DW_AT_frame_base"));
                }
            }
            ref x => Err(anyhow!("{:?}", x))?,
        }
    }
}

use std::path::Path;

pub fn transform_debug_line<R: gimli::Reader>(
    unit: &Unit<R, R::Offset>,
    root: &DebuggingInformationEntry<R>,
    dwarf: &gimli::Dwarf<R>,
    debug_line: &DebugLine<R>,
) -> Result<DwarfUnitSourceMap> {
    let offset = match root.attr_value(gimli::DW_AT_stmt_list)? {
        Some(gimli::AttributeValue::DebugLineRef(offset)) => offset,
        _ => {
            return Err(anyhow!("Debug line offset is not found"));
        }
    };

    let program = debug_line
        .program(offset, unit.header.address_size(), None, None)
        .expect("parsable debug_line");

    let header = program.header();

    let sequence_base_index: usize;
    let mut dirs = vec![];
    if header.version() <= 4 {
        dirs.push("./".to_string());
        sequence_base_index = 1;
    } else {
        sequence_base_index = 0;
    }

    for dir in header.include_directories() {
        dirs.push(clone_string_attribute(dwarf, unit, dir.clone()).expect("parsable dir string"));
    }
    let mut files = Vec::new();
    for file_entry in header.file_names() {
        let dir = dirs[file_entry.directory_index() as usize].clone();
        let dir_path = Path::new(&dir);
        let mut path = dir_path.join(clone_string_attribute(dwarf, unit, file_entry.path_name())?);
        if !path.is_absolute() {
            if let Some(comp_dir) = unit.comp_dir.clone() {
                let comp_dir = String::from_utf8(comp_dir.to_slice()?.to_vec()).unwrap();
                path = Path::new(&comp_dir).join(path);
            }
        }
        files.push(path);
    }

    let mut rows = program.rows();
    let mut sorted_rows = BTreeMap::new();
    while let Some((_, row)) = rows.next_row()? {
        sorted_rows.insert(row.address(), row.clone());
    }
    let sorted_rows: Vec<_> = sorted_rows.into_iter().collect();
    Ok(DwarfUnitSourceMap {
        address_sorted_rows: sorted_rows,
        paths: files,
        sequence_base_index,
    })
}

pub struct DwarfUnitSourceMap {
    address_sorted_rows: Vec<(u64, LineRow)>,
    paths: Vec<std::path::PathBuf>,
    sequence_base_index: usize,
}

use super::commands::sourcemap;
impl DwarfUnitSourceMap {
    fn transform_lineinfo(&self, row: &LineRow) -> sourcemap::LineInfo {
        let filepath = self.paths[row.file_index() as usize - self.sequence_base_index].clone();
        sourcemap::LineInfo {
            filepath: filepath.to_str().unwrap().to_string(),
            line: row.line(),
            column: match row.column() {
                gimli::ColumnType::Column(c) => sourcemap::ColumnType::Column(c),
                gimli::ColumnType::LeftEdge => sourcemap::ColumnType::LeftEdge,
            },
        }
    }
}

use std::cell::RefCell;
pub struct DwarfSourceMap {
    address_sorted_rows: Vec<(u64, sourcemap::LineInfo)>,
    directory_map: RefCell<HashMap<String, String>>,
}

impl DwarfSourceMap {
    fn new(units: Vec<DwarfUnitSourceMap>) -> Self {
        let mut rows = BTreeMap::new();
        for unit in &units {
            for (addr, row) in &unit.address_sorted_rows {
                let line_info = unit.transform_lineinfo(row);
                rows.insert(*addr, line_info);
            }
        }
        Self {
            address_sorted_rows: rows.into_iter().collect(),
            directory_map: RefCell::new(HashMap::new()),
        }
    }
}

impl sourcemap::SourceMap for DwarfSourceMap {
    fn set_directory_map(&self, from: String, to: String) {
        self.directory_map.borrow_mut().insert(from, to);
    }
    fn find_line_info(&self, offset: usize) -> Option<sourcemap::LineInfo> {
        let mut line_info = match self
            .address_sorted_rows
            .binary_search_by_key(&(offset as u64), |i| i.0)
        {
            Ok(i) => self.address_sorted_rows[i].1.clone(),
            Err(i) => {
                if i > 0 {
                    self.address_sorted_rows[i - 1].1.clone()
                } else {
                    return None;
                }
            }
        };
        for (from, to) in self.directory_map.borrow().iter() {
            line_info.filepath = line_info.filepath.replace(from, to);
        }
        Some(line_info)
    }
}

use super::commands::subroutine;
use types::*;
pub struct DwarfSubroutineMap<'input> {
    pub subroutines: Vec<Subroutine<usize>>,
    type_hash: HashMap<usize, TypeInfo<Reader<'input>>>,
    buffer: Vec<u8>,
}

fn header_from_offset<R: gimli::Reader>(
    dwarf: &gimli::Dwarf<R>,
    offset: DebugInfoOffset<R::Offset>,
) -> Result<Option<CompilationUnitHeader<R>>> {
    let mut headers = dwarf.units();
    while let Some(header) = headers.next()? {
        if header.offset() == offset {
            return Ok(Some(header));
        } else {
            continue;
        }
    }
    return Ok(None);
}

fn subroutine_variables<R: gimli::Reader>(
    dwarf: &gimli::Dwarf<R>,
    unit: &Unit<R>,
    subroutine: &Subroutine<R::Offset>,
) -> Result<Vec<SymbolVariable<R>>> {
    let mut tree = unit.entries_tree(Some(subroutine.entry_offset))?;
    let root = tree.root()?;
    let mut children = root.children();
    let mut variables = vec![];
    while let Some(child) = children.next()? {
        match child.entry().tag() {
            gimli::DW_TAG_variable | gimli::DW_TAG_formal_parameter => {
                let var = transform_variable(&dwarf, &unit, child.entry())?;
                variables.push(var);
            }
            _ => continue,
        }
    }
    Ok(variables)
}

fn unit_type_name<R: gimli::Reader>(
    dwarf: &gimli::Dwarf<R>,
    unit: &Unit<R>,
    type_offset: Option<R::Offset>,
) -> Result<String> {
    let type_offset = match type_offset {
        Some(offset) => offset,
        None => {
            return Ok("void".to_string());
        }
    };
    let mut tree = unit.entries_tree(Some(UnitOffset::<R::Offset>(type_offset)))?;
    let root = tree.root()?;
    if let Some(attr) = root.entry().attr_value(gimli::DW_AT_name)? {
        clone_string_attribute(dwarf, unit, attr)
    } else {
        Err(anyhow!(format!("failed to seek at {:?}", type_offset)))
    }
}

impl<'input> subroutine::SubroutineMap for DwarfSubroutineMap<'input> {
    fn variable_name_list(&self, code_offset: usize) -> Result<Vec<subroutine::Variable>> {
        let offset = &(code_offset as u64);
        let subroutine = match self
            .subroutines
            .iter()
            .filter(|s| s.pc.contains(offset))
            .next()
        {
            Some(s) => s,
            None => return Err(anyhow!("failed to determine subroutine")),
        };
        let dwarf = parse_dwarf(&self.buffer)?;
        let header = match header_from_offset(&dwarf, subroutine.unit_offset)? {
            Some(header) => header,
            None => {
                return Ok(vec![]);
            }
        };

        let unit = dwarf.unit(header)?;
        let variables = subroutine_variables(&dwarf, &unit, &subroutine)?;

        Ok(variables
            .iter()
            .map(|var| {
                let mut v = subroutine::Variable {
                    name: "<<not parsed yet>>".to_string(),
                    type_name: "<<not parsed yet>>".to_string(),
                };
                if let Some(name) = var.name.clone() {
                    v.name = name;
                }
                if let Ok(ty_name) = unit_type_name(&dwarf, &unit, var.ty_offset) {
                    v.type_name = ty_name;
                }
                v
            })
            .collect())
    }

    fn get_frame_base(&self, code_offset: usize) -> Result<Option<WasmLoc>> {
        let offset = &(code_offset as u64);
        let subroutine = match self
            .subroutines
            .iter()
            .filter(|s| s.pc.contains(offset))
            .next()
        {
            Some(s) => s,
            None => return Err(anyhow!("failed to determine subroutine")),
        };
        return Ok(subroutine.frame_base.clone());
    }
    fn display_variable(
        &self,
        code_offset: usize,
        frame_base: FrameBase,
        memory: &[u8],
        name: String,
    ) -> Result<()> {
        let offset = &(code_offset as u64);
        let subroutine = match self
            .subroutines
            .iter()
            .filter(|s| s.pc.contains(offset))
            .next()
        {
            Some(s) => s,
            None => return Err(anyhow!("failed to determine subroutine")),
        };
        let dwarf = parse_dwarf(&self.buffer)?;
        let header = match header_from_offset(&dwarf, subroutine.unit_offset)? {
            Some(header) => header,
            None => {
                return Ok(());
            }
        };

        let unit = dwarf.unit(header)?;
        let variables = subroutine_variables(&dwarf, &unit, &subroutine)?;

        let var = match variables
            .iter()
            .filter(|v| {
                if let Some(vname) = v.name.clone() {
                    vname == name
                } else {
                    false
                }
            })
            .next()
        {
            Some(v) => v,
            None => {
                return Err(anyhow!("'{}' is not valid variable name", name));
            }
        };
        let piece = match var.content {
            VariableContent::Location(location) => match location {
                AttributeValue::Exprloc(expr) => {
                    evaluate_variable_location(subroutine.encoding, frame_base, expr)?
                }
                AttributeValue::LocationListsRef(_listsref) => unimplemented!("listsref"),
                _ => panic!(),
            },
            VariableContent::ConstValue(ref _bytes) => unimplemented!(),
            VariableContent::Unknown { ref debug_info } => {
                unimplemented!("Unknown variable content found {}", debug_info)
            }
        };

        let piece = match piece.iter().next() {
            Some(p) => p,
            None => {
                println!("failed to get piece of variable");
                return Ok(());
            }
        };

        if let Some(offset) = var.ty_offset {
            use format::format_object;
            match piece.location {
                gimli::Location::Address { address } => {
                    let mut tree = unit.entries_tree(Some(UnitOffset(offset)))?;
                    let root = tree.root()?;
                    println!(
                        "{}",
                        format_object(
                            root,
                            &memory[(address as usize)..],
                            subroutine.encoding,
                            &dwarf,
                            &unit
                        )?
                    );
                }
                _ => unimplemented!(),
            }
        } else {
            println!("no explicit type");
        }
        Ok(())
    }
}
