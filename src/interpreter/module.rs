use super::Environment;
use parity_wasm::elements::Module as PModule;
use parity_wasm::elements::*;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::error::Error;

struct BaseModule {
    name: String,
    exports: Vec<Export>,
    export_bindings: HashMap<String, Index>,
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
    fn get_func_export(
        &self,
        env: &Environment,
        name: String,
        sig_index: Index,
    ) -> Option<&Export> {
        let module = &self.get_base_module();
        for export in &module.exports {
            if export.name == name && export.kind == ExternalKind::Func {
                let func = env.get_func(export.index);
                if env.is_func_sigs_equal(func.sig_index(), sig_index) {
                    Some(export);
                }
            }
        }
        // TODO: unknown
        None
    }

    fn get_export(&self, name: &String) -> Option<&Export> {
        let module = self.get_base_module();
        let index = module.export_bindings[name];
        Some(&module.exports[index.0 as usize])
    }
}

pub struct DefinedModule {
    base_module: BaseModule,
    active_elem_segments: Vec<ElemSegmentInfo>,
    active_data_segments: Vec<DataSegmentInfo>,
}

impl DefinedModule {
    pub fn read_from_parity_wasm<'a, 'b>(
        module: &'b PModule,
        env: &'a mut Environment<'b>,
    ) -> DefinedModule {
        let reader = &mut ModuleReader::new(env);
        reader.walk(module);
        panic!();
    }
}

struct ModuleReader<'a, 'b> {
    env: &'a mut Environment<'b>,

    sig_index_mapping: IndexVector,
    func_index_mapping: IndexVector,
    table_index_mapping: IndexVector,

    has_table: bool,
}

impl<'a, 'b> ModuleReader<'a, 'b> {
    fn new(env: &'a mut Environment<'b>) -> Self {
        Self {
            env: env,
            sig_index_mapping: vec![],
            func_index_mapping: vec![],
            table_index_mapping: vec![],
            has_table: false,
        }
    }
    fn translate_sig_index_to_env(&self, index: Index) -> Index {
        let index: usize = index.try_into().unwrap();
        self.sig_index_mapping[index]
    }
}

impl<'a, 'b> ModuleReader<'a, 'b> {
    fn walk(&mut self, module: &'b PModule) {
        self.walk_types(module);
        self.walk_imports(module);
        self.walk_functions(module);
        self.walk_tables(module);
        self.walk_memory(module);
        self.walk_global(module);
        self.walk_export(module);
        self.walk_start(module);
        self.walk_elem(module);
        self.walk_code(module);
        self.walk_data(module);
    }

    fn walk_types(&mut self, module: &'b PModule) {
        let type_sec = match module.type_section() {
            Some(type_sec) => type_sec,
            None => return,
        };

        let sig_count = self.env.get_func_signature_count();
        for (i, type_) in type_sec.types().into_iter().enumerate() {
            let env_sig_index = Index::try_from(sig_count + i).unwrap();
            self.sig_index_mapping.push(env_sig_index);

            match type_ {
                Type::Function(func_type) => {
                    self.env.push_back_func_signature(func_type);
                }
            }
        }
    }

    fn walk_imports(&mut self, module: &PModule) {
        let import_sec: &ImportSection = match module.import_section() {
            Some(import_sec) => import_sec,
            None => return,
        };
        for entry in import_sec.entries() {
            match entry.external() {
                External::Function(sig_index) => self.walk_import_fun(entry, Index(*sig_index)),
                External::Table(table) => self.walk_import_table(entry, table),
                External::Memory(memory) => self.walk_import_memory(entry, memory),
                External::Global(global) => self.walk_import_global(entry, global),
            }
        }
        panic!();
    }

    fn walk_import_fun(&mut self, entry: &ImportEntry, sig_index: Index) {
        let env_sig_index = self.translate_sig_index_to_env(sig_index);
        let imported_module = &self.env.find_registered_module(entry.module());
        let export = match imported_module.get_func_export(
            self.env,
            entry.field().to_string(),
            env_sig_index,
        ) {
            Some(e) => e,
            None => panic!("Imported func {} not found", entry.field()),
        };
        let func = self.env.get_func(export.index);
        self.func_index_mapping.push(export.index);
    }

    fn walk_import_table(&mut self, entry: &ImportEntry, table: &TableType) {
        self.has_table = true;
        let module = self.env.find_registered_module(entry.module());
        let export = module
            .get_export(&entry.field().to_string())
            .expect("Imported table not found");
        let exported_table = self.env.get_table(export.index);
        // assert_eq!(table.elem_type, exported_table.elem_type)
        self.table_index_mapping.push(export.index);
    }

    fn walk_import_memory(&mut self, entry: &ImportEntry, memory: &MemoryType) {
        panic!()
    }

    fn walk_import_global(&mut self, entry: &ImportEntry, memory: &GlobalType) {
        panic!()
    }

    fn walk_functions(&mut self, module: &PModule) {
        let function_sec = match module.function_section() {
            Some(function_sec) => function_sec,
            None => return,
        };
        let func_count = self.env.get_func_count();
        for (i, entry) in function_sec.entries().into_iter().enumerate() {
            let env_func_index = Index::try_from(func_count + i).unwrap();
            self.func_index_mapping.push(env_func_index);
            let fun = DefinedFunc::new(env_func_index);
            self.env.push_back_func(Func::Defined(fun));
        }
    }

    fn walk_tables(&mut self, module: &PModule) {}
    fn walk_memory(&mut self, module: &PModule) {}
    fn walk_global(&mut self, module: &PModule) {}
    fn walk_export(&mut self, module: &PModule) {}
    fn walk_start(&mut self, module: &PModule) {}
    fn walk_elem(&mut self, module: &PModule) {}
    fn walk_code(&mut self, module: &PModule) {}
    fn walk_data(&mut self, module: &PModule) {}
}

pub type TypeVector = Vec<Type>;
pub type IndexVector = Vec<Index>;

pub enum Func {
    Defined(DefinedFunc),
}

impl Func {
    fn base(&self) -> &FuncBase {
        match self {
            Func::Defined(defined) => &defined.base,
        }
    }
    fn sig_index(&self) -> Index {
        self.base().sig_index
    }
}

pub struct FuncBase {
    sig_index: Index,
    is_host: bool,
}
pub struct DefinedFunc {
    base: FuncBase,
    offset: u32,
    local_decl_count: Index,
    local_count: Index,
    param_and_loca_types: TypeVector,
}

impl DefinedFunc {
    fn new(sig_index: Index) -> Self {
        Self {
            base: FuncBase {
                sig_index,
                is_host: false,
            },
            offset: 0, // Invalid offset
            local_decl_count: Index(0),
            local_count: Index(0),
            param_and_loca_types: vec![],
        }
    }
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

pub struct Table {
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
