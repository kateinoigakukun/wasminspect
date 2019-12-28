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
    pmodule: parity_wasm::elements::Module,
    start_func: Option<u32>,
    funcs: Vec<Func>,
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
            funcs: vec![],
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

    pub fn exported_func_by_name(&self, name: String) -> Option<Index> {
        let export_sec: &ExportSection = match self.pmodule.export_section() {
            Some(export_sec) => export_sec,
            None => return None,
        };
        export_sec
            .entries()
            .iter()
            .filter_map(|entry| match entry.internal() {
                Internal::Function(func_index) => {
                    if entry.field().to_string() == name {
                        Some(Index(func_index.clone()))
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .next()
    }
}

struct ModuleReader<'a> {
    env: &'a mut Environment,
}

impl<'a> ModuleReader<'a> {
    fn new(env: &'a mut Environment) -> Self {
        Self { env: env }
    }
}

impl<'a> ModuleReader<'a> {
    fn walk(&mut self, module: &PModule) {
        let types = self.walk_types(module);
        self.walk_functions(module, &types);
    }

    fn walk_types(&mut self, module: &PModule) -> Vec<FunctionType> {
        let type_sec = match module.type_section() {
            Some(type_sec) => type_sec,
            None => return vec![],
        };

        for type_ in type_sec.types() {
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

    fn walk_functions(&mut self, module: &PModule, types: &[FunctionType]) {
        let function_sec = match module.function_section() {
            Some(function_sec) => function_sec,
            None => return,
        };
        let code_sec = match module.code_section() {
            Some(code_sec) => code_sec,
            None => return,
        };
        for (entry, body) in function_sec.entries().into_iter().zip(code_sec.bodies()) {
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
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Value {
    I32(i32),
    I64(i64),
	F32(f32),
	F64(f64),
}

impl Value {
    pub fn value_type(&self) -> ValueType {
        match self {
            Value::I32(_) => ValueType::I32,
            Value::I64(_) => ValueType::I64,
            Value::F32(_) => ValueType::F32,
            Value::F64(_) => ValueType::F64,
        }
    }
}

pub enum ValueConversionError {
    InvalidType(String)
}

impl TryFrom<Value> for i32 {
    type Error = ValueConversionError;
    fn try_from(input: Value) -> Result<i32, ValueConversionError> {
        match input {
            Value::I32(val) => Ok(val),
            _ => Err(ValueConversionError::InvalidType("i32".to_string()))
        }
    }
}

impl TryFrom<Value> for i64 {
    type Error = ValueConversionError;
    fn try_from(input: Value) -> Result<i64, ValueConversionError> {
        match input {
            Value::I64(val) => Ok(val),
            _ => Err(ValueConversionError::InvalidType("i64".to_string()))
        }
    }
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
