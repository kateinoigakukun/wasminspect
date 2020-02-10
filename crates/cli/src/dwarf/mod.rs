use anyhow::{anyhow, Result};
use gimli::{
    AttributeValue, DebugAbbrev, DebugAddr, DebugInfo, DebugLine, DebugLineStr, DebugLoc,
    DebugLocLists, DebugRanges, DebugRngLists, DebugStr, DebugStrOffsets, DebugTypes,
    DebuggingInformationEntry, EndianSlice, LineRow, LittleEndian, LocationLists, RangeLists, Unit,
};
use std::collections::{BTreeMap, HashMap};
use wasmparser::{ModuleReader, SectionCode};

mod format;
mod types;
mod utils;

use utils::*;

type Reader<'input> = gimli::EndianSlice<'input, LittleEndian>;
pub type Dwarf<'input> = gimli::Dwarf<Reader<'input>>;

pub fn parse_dwarf<'a>(module: &'a [u8]) -> Result<Dwarf<'a>> {
    const EMPTY_SECTION: &[u8] = &[];
    let mut reader = ModuleReader::new(module)?;
    let mut sections = HashMap::new();
    while !reader.eof() {
        let section = reader.read().expect("section");
        match section.code {
            SectionCode::Custom { name, kind: _ } => {
                let mut reader = section.get_binary_reader();
                let len = reader.bytes_remaining();
                sections.insert(name, reader.read_bytes(len).expect("bytes"));
            }
            _ => (),
        }
    }
    let endian = LittleEndian;
    let debug_str = DebugStr::new(sections.get(".debug_str").unwrap(), endian);
    let debug_abbrev = DebugAbbrev::new(sections.get(".debug_abbrev").unwrap(), endian);
    let debug_info = DebugInfo::new(sections.get(".debug_info").unwrap(), endian);
    let debug_line = DebugLine::new(sections.get(".debug_line").unwrap(), endian);
    let debug_addr = DebugAddr::from(EndianSlice::new(EMPTY_SECTION, endian));
    let debug_line_str = DebugLineStr::from(EndianSlice::new(EMPTY_SECTION, endian));
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
pub fn transform_dwarf<'a>(dwarf: Dwarf<'a>) -> Result<DwarfDebugInfo<'a>> {
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
        subroutines.append(&mut transform_subprogram(&dwarf, &unit)?);
        get_types(&dwarf, &unit, &mut type_hash)?;
    }
    Ok(DwarfDebugInfo {
        sourcemap: DwarfSourceMap::new(sourcemaps),
        subroutine: DwarfSubroutineMap {
            subroutines,
            type_hash,
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

pub struct Subroutine<R: gimli::Reader> {
    pub name: Option<String>,
    pub pc: std::ops::Range<u64>,
    pub variables: Vec<SymbolVariable<R>>,
    pub encoding: gimli::Encoding,
}

pub fn transform_subprogram<R: gimli::Reader>(
    dwarf: &gimli::Dwarf<R>,
    unit: &Unit<R, R::Offset>,
) -> Result<Vec<Subroutine<R>>> {
    let mut tree = unit.entries_tree(None)?;
    let root = tree.root()?;
    let mut subroutines = vec![];
    transform_subprogram_rec(root, dwarf, unit, &mut subroutines)?;
    Ok(subroutines)
}

pub fn transform_subprogram_rec<R: gimli::Reader>(
    node: gimli::EntriesTreeNode<R>,
    dwarf: &gimli::Dwarf<R>,
    unit: &Unit<R, R::Offset>,
    out_subroutines: &mut Vec<Subroutine<R>>,
) -> Result<()> {
    let mut subroutine = None;
    if node.entry().tag() == gimli::DW_TAG_subprogram {
        let name = match node.entry().attr_value(gimli::DW_AT_name)? {
            Some(attr) => Some(clone_string_attribute(dwarf, unit, attr)?),
            None => None,
        };

        let low_pc_attr = node.entry().attr_value(gimli::DW_AT_low_pc)?;
        // println!("low_pc_attr: {:?}", low_pc_attr);
        let high_pc_attr = node.entry().attr_value(gimli::DW_AT_high_pc)?;
        // println!("high_pc_attr: {:?}", high_pc_attr);
        if let Some(AttributeValue::Addr(low_pc)) = low_pc_attr {
            let high_pc = match high_pc_attr {
                Some(AttributeValue::Udata(size)) => Some(low_pc + size),
                Some(AttributeValue::Addr(high_pc)) => Some(high_pc),
                Some(x) => unreachable!("high_pc can't be {:?}", x),
                None => None,
            };
            if let Some(high_pc) = high_pc {
                subroutine = Some(Subroutine {
                    pc: low_pc..high_pc,
                    name,
                    encoding: unit.encoding(),
                    variables: vec![],
                });
            }
        }
    }

    let mut children = node.children();
    while let Some(child) = children.next()? {
        match child.entry().tag() {
            gimli::DW_TAG_variable | gimli::DW_TAG_formal_parameter => {
                let var = transform_variable(dwarf, unit, child.entry())?;
                if let Some(current) = subroutine.as_mut() {
                    current.variables.push(var)
                }
            }
            _ => {
                transform_subprogram_rec(child, dwarf, unit, out_subroutines)?;
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

use gimli::Expression;
fn evaluate_variable_location<R: gimli::Reader>(
    encoding: gimli::Encoding,
    rbp: u32,
    expr: Expression<R>,
) -> Result<Vec<gimli::Piece<R>>> {
    let mut evaluation = expr.evaluation(encoding);
    evaluation.set_initial_value(rbp.into());
    let result = evaluation.evaluate()?;
    use gimli::EvaluationResult;
    match result {
        EvaluationResult::Complete => Ok(evaluation.result()),
        x => unimplemented!("{:?}", x),
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
        dirs.push(clone_string_attribute(dwarf, unit, dir.clone())?);
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
    pub subroutines: Vec<Subroutine<Reader<'input>>>,
    type_hash: HashMap<usize, TypeInfo<Reader<'input>>>,
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
        Ok(subroutine
            .variables
            .iter()
            .map(|var| {
                let mut v = subroutine::Variable {
                    name: "<<not parsed yet>>".to_string(),
                    type_name: "<<not parsed yet>>".to_string(),
                };
                if let Some(name) = var.name.clone() {
                    v.name = name;
                }
                use format::type_name;
                if let Ok(ty_name) = type_name(var.ty_offset, &self.type_hash) {
                    v.type_name = ty_name;
                }
                v
            })
            .collect())
    }
    fn display_variable(
        &self,
        code_offset: usize,
        rbp: u32,
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
        let var = match subroutine
            .variables
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
                    evaluate_variable_location(subroutine.encoding, rbp, expr)?
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
                    println!("{}", format_object(offset, &memory[(address as usize)..], &self.type_hash)?);
                }
                _ => unimplemented!(),
            }
        } else {
            println!("no explicit type");
        }
        Ok(())
    }
}
