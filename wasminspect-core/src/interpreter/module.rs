use super::address::*;
use super::export::ExportInstance;
use super::store::*;
use std::hash::Hash;

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct ModuleIndex(pub u32);

pub struct ModuleInstance {
    types: Vec<parity_wasm::elements::Type>,
    func_addrs: Vec<FuncAddr>,
    exports: Vec<ExportInstance>,
    start_func: Option<FuncAddr>,
}

impl ModuleInstance {
    pub fn new_from_parity_module(
        module: parity_wasm::elements::Module,
        module_index: ModuleIndex,
        types: Vec<parity_wasm::elements::Type>,
        func_addrs: Vec<FuncAddr>,
    ) -> Self {
        Self {
            types,
            func_addrs,
            exports: module
                .export_section()
                .map(|sec| sec.entries().iter())
                .map(|entries| {
                    entries.map(|e| ExportInstance::new_from_parity_entry(e.clone(), module_index))
                })
                .map(|s| s.collect())
                .unwrap_or_default(),
            start_func: module
                .start_section()
                .map(|func_index| FuncAddr(module_index, func_index as usize)),
        }
    }

    pub fn exported_func(&self, name: String) -> Option<&ExportInstance> {
        self.exports.iter().filter(|e| *e.name() == name).next()
    }

    pub fn start_func_addr(&self) -> &Option<FuncAddr> {
        &self.start_func
    }
}

// struct BaseModule {
//     name: String,
//     exports: Vec<Export>,
//     export_bindings: HashMap<String, Index>,
// }

// pub enum Module {
//     Defined(DefinedModule),
// }

// impl Module {
//     fn get_base_module(&self) -> &BaseModule {
//         match self {
//             Module::Defined(defined_module) => &defined_module.base_module,
//         }
//     }
//     fn get_func_export(
//         &self,
//         env: &Environment,
//         name: String,
//         func_type: &FunctionType,
//     ) -> Option<&Export> {
//         let module = &self.get_base_module();
//         for export in &module.exports {
//             if export.name == name && export.kind == ExternalKind::Func {
//                 let func = env.get_func(export.index);
//                 if func.func_type() == func_type {
//                     Some(export);
//                 }
//             }
//         }
//         // TODO: unknown
//         None
//     }

//     fn get_export(&self, name: &String) -> Option<&Export> {
//         let module = self.get_base_module();
//         let index = module.export_bindings[name];
//         Some(&module.exports[index.0 as usize])
//     }

//     pub fn name(&self) -> &String {
//         &self.get_base_module().name
//     }
// }

// pub struct DefinedModule {
//     base_module: BaseModule,
//     pmodule: parity_wasm::elements::Module,
//     start_func: Option<u32>,
//     funcs: Vec<FuncAddr>,
// }

// impl DefinedModule {
//     pub fn read_from_parity_wasm<'a, 'b>(module: PModule, env: &'a mut Environment) -> Self {
//         let module_name = module
//             .names_section()
//             .and_then(|sec| sec.module())
//             .map(|module| module.name())
//             .unwrap_or("wasminspect_main");
//         let reader = &mut ModuleReader::new(env);
//         reader.walk(&module);
//         let start_func = module.start_section();
//         Self {
//             base_module: BaseModule {
//                 name: module_name.to_string(),
//                 exports: vec![],
//                 export_bindings: HashMap::new(),
//             },
//             pmodule: module,
//             start_func: start_func,
//             funcs: vec![],
//         }
//     }

//     pub fn start_func_index(&self) -> Option<Index> {
//         self.start_func.map(Index)
//     }

//     pub fn globals(&self) -> &[GlobalEntry] {
//         self.pmodule
//             .global_section()
//             .map(|sec| sec.entries())
//             .unwrap_or(&[])
//     }

// }

// struct ModuleReader<'a> {
//     env: &'a mut Environment,
// }

// impl<'a> ModuleReader<'a> {
//     fn new(env: &'a mut Environment) -> Self {
//         Self { env: env }
//     }
// }

// impl<'a> ModuleReader<'a> {
//     fn walk(&mut self, module: &PModule) {
//     }

//     fn walk_imports(&mut self, module: &parity_wasm::elements::Module) -> Option<()> {
//         let import_sec = module.import_section()?;
//         for entry in import_sec.entries() {
//         };
//         Some(())
//     }

//     fn walk_import_entry(&mut self, module: &parity_wasm::elements::Module, types: &[FunctionType], type_index: Index) -> Option<()> {
//         Some(())
//     }
// }

// pub type TypeVector = Vec<Type>;
// pub type IndexVector = Vec<Index>;

// #[derive(PartialEq)]
// pub enum ExternalKind {
//     Func = 0,
//     Table = 1,
//     Memory = 2,
//     Global = 3,
//     Event = 4,
// }
// pub struct Export {
//     name: String,
//     kind: ExternalKind,
//     index: Index,
// }

// #[derive(PartialEq, Clone, Copy)]
// pub struct Index(pub u32);

// impl Index {
//     pub fn zero() -> Index {
//         Index(0)
//     }

//     pub fn inc(&mut self) {
//         self.0 += 1;
//     }
// }

// impl TryFrom<usize> for Index {
//     type Error = Box<dyn Error>;
//     fn try_from(input: usize) -> Result<Index, Box<dyn Error>> {
//         Ok(u32::try_from(input).map(Index).unwrap())
//     }
// }

// impl TryInto<usize> for Index {
//     type Error = Box<dyn Error>;
//     fn try_into(self) -> Result<usize, Box<dyn Error>> {
//         Ok(usize::try_from(self.0).unwrap())
//     }
// }

// struct Address(u32);

// enum Ref {
//     Func(Index),
//     Host(Index),
//     Null,
// }

// struct Limits {
//     initial: u64,
//     max: u64,
//     has_max: bool,
//     is_shared: bool,
// }

// pub struct Table {
//     elem_type: Type,
//     limits: Limits,
//     entries: Vec<Ref>,
// }

// struct Memory {
//     page_limits: Limits,
//     data: Vec<u8>,
// }
