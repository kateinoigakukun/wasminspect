use super::Environment;
use parity_wasm::elements::Module as PModule;
use parity_wasm::elements::*;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::error::Error;
use std::iter;

struct BaseModule {
    name: String,
    exports: Vec<Export>,
    export_bindings: HashMap<String, Index>,
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
        func_type: &FunctionType,
    ) -> Option<&Export> {
        let module = &self.get_base_module();
        for export in &module.exports {
            if export.name == name && export.kind == ExternalKind::Func {
                let func = env.get_func(export.index);
                if func.func_type() == func_type {
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

    pub fn name(&self) -> &String {
        &self.get_base_module().name
    }
}

pub struct DefinedModule {
    base_module: BaseModule,
    pmodule: PModule,
    start_func: Option<u32>,
}

impl DefinedModule {
    pub fn read_from_parity_wasm<'a, 'b>(module: PModule, env: &'a mut Environment) -> Self {
        let module_name = module
            .names_section()
            .and_then(|sec| sec.module())
            .map(|module| module.name())
            .unwrap_or("wasminspect_main");
        let reader = &mut ModuleReader::new(env);
        reader.walk(&module);
        let start_func = module.start_section();
        Self {
            base_module: BaseModule {
                name: module_name.to_string(),
                exports: vec![],
                export_bindings: HashMap::new(),
            },
            pmodule: module,
            start_func: start_func,
        }
    }

    pub fn start_func_index(&self) -> Option<Index> {
        self.start_func.map(Index)
    }

    pub fn globals(&self) -> &[GlobalEntry] {
        self.pmodule
            .global_section()
            .map(|sec| sec.entries())
            .unwrap_or(&[])
    }
}

struct ModuleReader<'a> {
    env: &'a mut Environment,

    // legacy
    sig_index_mapping: IndexVector,
    func_index_mapping: IndexVector,
    table_index_mapping: IndexVector,

    has_table: bool,
}

impl<'a> ModuleReader<'a> {
    fn new(env: &'a mut Environment) -> Self {
        Self {
            env: env,
            sig_index_mapping: vec![],
            func_index_mapping: vec![],
            table_index_mapping: vec![],
            has_table: false,
        }
    }
}

impl<'a> ModuleReader<'a> {
    fn walk(&mut self, module: &PModule) {
        let types = self.walk_types(module);
        self.walk_imports(module, &types);
        self.walk_functions(module, &types);
        self.walk_tables(module);
        self.walk_memory(module);
        self.walk_global(module);
        self.walk_export(module);
        self.walk_start(module);
        self.walk_elem(module);
        self.walk_code(module);
        self.walk_data(module);
    }

    fn walk_types(&mut self, module: &PModule) -> Vec<FunctionType> {
        let type_sec = match module.type_section() {
            Some(type_sec) => type_sec,
            None => return vec![],
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
        return type_sec
            .types()
            .into_iter()
            .map(|t| match t {
                Type::Function(func_type) => func_type.clone(),
            })
            .collect();
    }

    fn walk_imports(&mut self, module: &PModule, types: &[FunctionType]) {
        let import_sec: &ImportSection = match module.import_section() {
            Some(import_sec) => import_sec,
            None => return,
        };
        for entry in import_sec.entries() {
            match entry.external() {
                External::Function(sig_index) => {
                    let func_type = &types[sig_index.clone() as usize];
                    self.walk_import_fun(entry, func_type);
                }
                External::Table(table) => self.walk_import_table(entry, table),
                External::Memory(memory) => self.walk_import_memory(entry, memory),
                External::Global(global) => self.walk_import_global(entry, global),
            }
        }
        panic!();
    }

    fn walk_import_fun(&mut self, entry: &ImportEntry, func_type: &FunctionType) {
        let imported_module = &self.env.find_registered_module(entry.module());
        let export =
            match imported_module.get_func_export(self.env, entry.field().to_string(), func_type) {
                Some(e) => e,
                None => panic!("Imported func {} not found", entry.field()),
            };
        self.func_index_mapping.push(export.index);
    }

    fn walk_import_table(&mut self, entry: &ImportEntry, table: &TableType) {
        self.has_table = true;
        let module = self.env.find_registered_module(entry.module());
        let export = module
            .get_export(&entry.field().to_string())
            .expect("Imported table not found");
        let exported_table = self.env.get_table(export.index);
        let _ = table;
        let _ = exported_table;
        self.table_index_mapping.push(export.index);
    }

    fn walk_import_memory(&mut self, entry: &ImportEntry, memory: &MemoryType) {
        panic!()
    }

    fn walk_import_global(&mut self, entry: &ImportEntry, memory: &GlobalType) {
        panic!()
    }

    fn walk_functions(&mut self, module: &PModule, types: &[FunctionType]) {
        let function_sec = match module.function_section() {
            Some(function_sec) => function_sec,
            None => return,
        };
        let code_sec = match module.code_section() {
            Some(code_sec) => code_sec,
            None => return,
        };
        let func_count = self.env.get_func_count();
        for ((i, entry), body) in function_sec
            .entries()
            .into_iter()
            .enumerate()
            .zip(code_sec.bodies())
        {
            let env_func_index = Index::try_from(func_count + i).unwrap();
            self.func_index_mapping.push(env_func_index);
            let func_type = types[entry.type_ref() as usize].clone();
            let locals: Vec<ValueType> = body
                .locals()
                .iter()
                .flat_map(|locals| iter::repeat(locals.value_type()).take(locals.count() as usize))
                .collect();
            let instructions = body.code().elements().to_vec();
            let fun = DefinedFunc::new("TODO".to_string(), func_type, locals, instructions);
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

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Value {
    I32(i32),
    I64(i64),
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

    pub fn is_host(&self) -> bool {
        match self {
            Func::Defined(_) => true,
        }
    }

    pub fn func_type(&self) -> &FunctionType {
        &self.base().func_type
    }

    pub fn locals(&self) -> &Vec<ValueType> {
        &self.base().locals
    }
}

pub struct FuncBase {
    name: String,
    func_type: FunctionType,
    locals: Vec<ValueType>,
    is_host: bool,
}
pub struct DefinedFunc {
    base: FuncBase,
    pub instructions: Vec<Instruction>,
}

impl DefinedFunc {
    fn new(
        name: String,
        func_type: FunctionType,
        locals: Vec<ValueType>,
        instructions: Vec<Instruction>,
    ) -> Self {
        Self {
            base: FuncBase {
                name,
                func_type,
                locals: locals,
                is_host: false,
            },
            instructions: instructions,
        }
    }

    pub fn inst(&self, index: Index) -> &Instruction {
        &self.instructions[index.0 as usize]
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

impl Index {
    pub fn zero() -> Index {
        Index(0)
    }

    pub fn inc(&mut self) {
        self.0 += 1;
    }
}

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
