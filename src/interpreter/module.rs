use super::Environment;
use parity_wasm::elements::Module as PModule;
use parity_wasm::elements::{External, ImportSection, Type, ImportEntry};
use std::convert::TryFrom;
use std::convert::TryInto;
use std::error::Error;

struct BaseModule {
    name: String,
    exports: Vec<Export>,
    // export_bindings: Vec<BindingHash>,
    // memory_index: Index,
}

pub enum Module {
    Defined(DefinedModule),
}

impl Module {
    fn get_base_module(&self) -> &BaseModule {
        match self {
            Module::Defined(defined_module) => &defined_module.base_module,
        }
    }
    fn get_func_export(&self, env: &Environment, name: String, sig_index: Index) -> Option<Export> {
        let module = &self.get_base_module();
        for export in &module.exports {
            if export.name == name && export.kind == ExternalKind::Func {
                let func = env.get_func(export.index);
                if env.is_func_sigs_equal(func.sig_index, sig_index) {
                    export;
                }
            }
        }
        // TODO: unknown
        None
    }
}

pub struct DefinedModule {
    base_module: BaseModule,
    active_elem_segments: Vec<ElemSegmentInfo>,
    active_data_segments: Vec<DataSegmentInfo>,
}

impl DefinedModule {
    pub fn read_from_parity_wasm(module: &PModule) -> DefinedModule {
        module.type_section();
        panic!();
    }
}

struct ModuleReader<'a> {
    env: &'a mut Environment,
    module: &'a mut DefinedModule,

    types: Vec<&'a Type>,
    func_index_mapping: IndexVector,
}

impl<'a> ModuleReader<'a> {
    fn walk(&mut self, module: &'a PModule) {
        self.walk_types(module);
        self.walk_imports(module);
    }
    fn walk_types(&mut self, module: &'a PModule) {
        self.types = if let Some(type_sec) = module.type_section() {
            type_sec.types().iter().collect()
        } else {
            vec![]
        };
    }

    fn walk_imports(&mut self, module: &PModule) {
        let import_sec: &ImportSection = match module.import_section() {
            Some(import_sec) => import_sec,
            None => return,
        };
        for entry in import_sec.entries() {
            match entry.external() {
                External::Function(type_ref) => self.walk_import_fun(module, entry, *type_ref),
                _ => panic!(),
            }
        }
        panic!();
    }

    fn walk_import_fun(&mut self, module: &PModule, entry: &ImportEntry, type_ref: u32) {
        let imported_module = &self.env.get_module_by_name(entry.module());
        let type_ = &self.types[type_ref as usize];
        // imported_module.
    }
}

pub type TypeVector = Vec<Type>;
pub type IndexVector = Vec<Index>;

#[derive(PartialEq)]
pub struct FuncSignature {
    param_types: TypeVector,
    result_types: TypeVector,
}

impl FuncSignature {
    fn new(param_types: TypeVector, result_types: TypeVector) -> Self {
        Self {
            param_types,
            result_types,
        }
    }
}

pub struct Func {
    sig_index: Index,
    is_host: bool,
}

#[derive(PartialEq)]
pub enum ExternalKind {
    Func = 0,
    Table = 1,
    Memory = 2,
    Global = 3,
    Event = 4,
}
pub struct Export {
    name: String,
    kind: ExternalKind,
    index: Index,
}

#[derive(PartialEq, Clone, Copy)]
pub struct Index(u32);

impl TryFrom<usize> for Index {
    type Error = Box<dyn Error>;
    fn try_from(input: usize) -> Result<Index, Box<dyn Error>> {
        Ok(u32::try_from(input).map(Index).unwrap())
    }
}

impl TryInto<usize> for Index {
    type Error = Box<dyn Error>;
    fn try_into(self) -> Result<usize, Box<dyn Error>> {
        Ok(usize::try_from(self.0).unwrap())
    }
}

struct Address(u32);

enum Ref {
    Func(Index),
    Host(Index),
    Null,
}

struct Limits {
    initial: u64,
    max: u64,
    has_max: bool,
    is_shared: bool,
}

struct Table {
    elem_type: Type,
    limits: Limits,
    entries: Vec<Ref>,
}

struct Memory {
    page_limits: Limits,
    data: Vec<u8>,
}

struct ElemSegmentInfo {
    table: Table,
    destination: Index,
    source: Vec<Ref>,
}

struct DataSegmentInfo {
    memory: Memory,
    destination: Address,
    data: Vec<u8>,
}
