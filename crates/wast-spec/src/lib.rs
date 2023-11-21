/// Reference: https://github.com/bytecodealliance/wasmtime/blob/master/crates/wast/src/wast.rs
use anyhow::{anyhow, bail, Context as _, Result};
use std::collections::HashMap;
use std::path::Path;
use std::str;
use wast::{
    core::{HeapType, NanPattern, WastArgCore, WastRetCore},
    QuoteWat, WastArg, Wat,
};
mod spectest;
pub use spectest::instantiate_spectest;
use wasminspect_vm::{
    invoke_func_ignoring_break, FuncAddr, ModuleIndex, NumVal, RefType, RefVal, WasmInstance,
    WasmValue, F32, F64,
};

pub struct WastContext {
    module_index_by_name: HashMap<String, ModuleIndex>,
    instance: WasmInstance,
    current: Option<ModuleIndex>,
    config: wasminspect_vm::Config,
}

impl WastContext {
    pub fn new(config: wasminspect_vm::Config) -> Self {
        let mut instance = WasmInstance::new();
        instance.load_host_module("spectest".to_string(), instantiate_spectest());
        Self {
            module_index_by_name: HashMap::new(),
            instance,
            current: None,
            config,
        }
    }
    pub fn run_file(&mut self, path: &Path) -> Result<()> {
        let bytes = std::fs::read(path).unwrap();
        self.run_buffer(path.to_str().unwrap(), &bytes)
    }

    pub fn extract_start_section(bytes: &[u8]) -> Result<Option<u32>> {
        let parser = wasmparser::Parser::new(0);
        for payload in parser.parse_all(bytes) {
            match payload? {
                wasmparser::Payload::StartSection { func, .. } => {
                    return Ok(Some(func));
                }
                _ => continue,
            }
        }
        Ok(None)
    }
    fn module(&mut self, mut wat: QuoteWat<'_>) -> Result<()> {
        let module_id = match &wat {
            wast::QuoteWat::Wat(Wat::Module(m)) => m.id,
            wast::QuoteWat::QuoteModule(_, bytes) => None,
            _ => panic!(),
        };
        let module_name = module_id.map(|id| id.name());
        let mut bytes = wat.encode()?;
        self.validate(&bytes)?;
        let start_section = Self::extract_start_section(&bytes)?;
        let module_index = self
            .instance
            .load_module_from_module(module_name.map(|n| n.to_string()), &mut bytes)
            .map_err(|e| anyhow!("Failed to instantiate: {}", e))?;
        if let Some(start_section) = start_section {
            let func_addr = FuncAddr::new_unsafe(module_index, start_section as usize);
            invoke_func_ignoring_break(func_addr, vec![], &mut self.instance.store, &self.config)
                .map_err(|e| anyhow!("Failed to exec start func: {}", e))?;
        }
        self.current = Some(module_index);
        if let Some(module_name) = module_name {
            self.module_index_by_name
                .insert(module_name.to_string(), module_index);
        }
        Ok(())
    }

    pub fn run_buffer(&mut self, filename: &str, wast: &[u8]) -> Result<()> {
        use wast::WastDirective::*;

        let wast = str::from_utf8(wast)?;

        let adjust_wast = |mut err: wast::Error| {
            err.set_path(filename.as_ref());
            err.set_text(wast);
            err
        };
        let context = |sp: wast::token::Span| {
            let (line, col) = sp.linecol_in(wast);
            format!("for directive on {}:{}:{}", filename, line + 1, col)
        };

        let buf = wast::parser::ParseBuffer::new(wast).map_err(adjust_wast)?;
        let wast = wast::parser::parse::<wast::Wast>(&buf).map_err(adjust_wast)?;

        for directive in wast.directives {
            match directive {
                Register {
                    span: _,
                    name,
                    module,
                } => {
                    let module_index = self.get_instance(module)?;
                    self.instance.register_name(name.to_string(), module_index);
                }
                Invoke(i) => {
                    self.invoke(i.module, i.name, &i.args)
                        .map_err(|err| anyhow!("Failed to invoke {}", err))
                        .with_context(|| context(i.span))?;
                }
                AssertReturn {
                    span,
                    exec,
                    results,
                } => match self.perform_execute(exec).with_context(|| context(span)) {
                    Ok(Ok(values)) => {
                        for (v, e) in values.iter().zip(results) {
                            match &e {
                                wast::WastRet::Core(e) => {
                                    if val_matches(v, e)? {
                                        continue;
                                    }
                                }
                                wast::WastRet::Component(_) => todo!("component is not supported yet")
                            }
                            bail!("expected {:?}, got {:?} {}", e, v, context(span))
                        }
                    }
                    Ok(Err(e)) => panic!("unexpected err: {}, {}", e, context(span)),
                    Err(e) => panic!("unexpected err: {}", e),
                },
                AssertTrap {
                    span,
                    exec,
                    message,
                } => match self.perform_execute(exec).with_context(|| context(span)) {
                    Ok(Ok(values)) => panic!("{}\nexpected trap, got {:?}", context(span), values),
                    Ok(Err(t)) => {
                        let result = format!("{}", t);
                        if result.contains(message) {
                            continue;
                        }
                        panic!("{}\nexpected {}, got {}", context(span), message, result,)
                    }
                    Err(err) => panic!("got wast level exception: {}", err),
                },
                AssertMalformed {
                    span,
                    module,
                    message: _,
                } => {
                    if let Ok(()) = self.module(module) {
                        panic!("{}\nexpected module to fail to instantiate", context(span))
                    };
                }
                AssertUnlinkable {
                    span,
                    mut module,
                    message,
                } => {
                    let err = match self.module(QuoteWat::Wat(module)) {
                        Ok(()) => panic!("{}\nexpected module to fail to link", context(span)),
                        Err(e) => e,
                    };
                    let error_message = format!("{:?}", err);
                    if !error_message.contains(&message) {
                        panic!(
                            "{}\nassert_unlinkable: expected {}, got {}",
                            context(span),
                            message,
                            error_message
                        )
                    }
                }
                AssertExhaustion {
                    span,
                    call,
                    message,
                } => match self.invoke(call.module, call.name, &call.args) {
                    Ok(values) => panic!("{}\nexpected trap, got {:?}", context(span), values),
                    Err(t) => {
                        let result = format!("{}", t);
                        if result.contains(message) {
                            continue;
                        }
                        panic!("{}\nexpected {}, got {}", context(span), message, result)
                    }
                },
                AssertInvalid {
                    span,
                    module,
                    message,
                } => {
                    let err = match self.module(module) {
                        Ok(()) => panic!("{}\nexpected module to fail to build", context(span)),
                        Err(e) => e,
                    };
                    let error_message = format!("{:?}", err);
                    if !error_message.contains(&message) {
                        // TODO: change to panic!
                        println!(
                            "{}\nassert_invalid: expected {}, got {}",
                            context(span),
                            message,
                            error_message
                        )
                    }
                }
                Wat(wast::QuoteWat::QuoteModule(span, source)) => {
                    let mut module = String::new();
                    for (_, src) in source {
                        module.push_str(str::from_utf8(src)?);
                        module.push(' ');
                    }
                    let buf = wast::parser::ParseBuffer::new(&module).map_err(adjust_wast)?;
                    let mut wat = wast::parser::parse::<wast::Wat>(&buf).map_err(|mut e| {
                        e.set_text(&module);
                        e
                    })?;
                    self.module(QuoteWat::Wat(wat))
                        .with_context(|| context(span))?;
                }
                Wat(wat) => {
                    self.module(wat)?;
                }
                AssertException { span, exec } => {
                    match self.perform_execute(exec).with_context(|| context(span)) {
                        Ok(Ok(values)) => {
                            panic!("{}\nexpected trap, got {:?}", context(span), values)
                        }
                        Ok(Err(_)) => {
                            todo!()
                        }
                        Err(err) => panic!("{}", err),
                    }
                }
                _ => todo!("unsupported directive: "),
            }
        }
        Ok(())
    }

    fn get_instance(&self, module_id: Option<wast::token::Id>) -> Result<ModuleIndex> {
        let name = module_id.map(|s| s.name());
        match name {
            Some(name) => self
                .module_index_by_name
                .get(name)
                .copied()
                .ok_or_else(|| anyhow!("module not found with name {}", name)),
            None => match self.current {
                Some(current) => Ok(current),
                None => panic!(),
            },
        }
    }

    /// Get the value of an exported global from an instance.
    fn get(
        &mut self,
        module_id: Option<wast::token::Id>,
        field: &str,
    ) -> Result<Result<Vec<WasmValue>>> {
        let module_index = self.get_instance(module_id)?;
        match self
            .instance
            .get_global(module_index, field)
            .map(|value| vec![value])
        {
            Some(v) => Ok(Ok(v)),
            None => Err(anyhow!("no global named {}", field)),
        }
    }

    fn invoke(
        &mut self,
        module_id: Option<wast::token::Id>,
        func_name: &str,
        args: &[wast::WastArg],
    ) -> Result<Vec<WasmValue>> {
        let module_index = self.get_instance(module_id)?;
        let args = args
            .iter()
            .map(|v| match v {
                WastArg::Core(core) => Ok(const_expr(core)),
                WastArg::Component(_) => bail!("component is not supported yet"),
            })
            .collect::<Result<Vec<_>>>()?;
        let result = self
            .instance
            .run(
                module_index,
                Some(func_name.to_string()),
                args,
                &self.config,
            )
            .map_err(|e| anyhow!("{}", e))?;
        Ok(result)
    }

    fn perform_execute(&mut self, exec: wast::WastExecute<'_>) -> Result<Result<Vec<WasmValue>>> {
        match exec {
            wast::WastExecute::Invoke(i) => Ok(self.invoke(i.module, i.name, &i.args)),
            wast::WastExecute::Wat(mut module) => {
                let mut binary = module.encode()?;
                self.validate(&binary)?;
                let start_section = Self::extract_start_section(&binary)?;
                let module_index = match self.instance.load_module_from_module(None, &mut binary) {
                    Ok(idx) => idx,
                    Err(e) => return Ok(Err(anyhow!("while instntiation: {}", e))),
                };
                if let Some(start_section) = start_section {
                    let func_addr = FuncAddr::new_unsafe(module_index, start_section as usize);
                    return Ok(invoke_func_ignoring_break(
                        func_addr,
                        vec![],
                        &mut self.instance.store,
                        &self.config,
                    )
                    .map_err(|e| anyhow!("Failed to exec start func: {}", e)));
                }
                Ok(Ok(vec![]))
            }
            wast::WastExecute::Get { module, global } => self.get(module, global),
        }
    }

    fn validate(&self, bytes: &[u8]) -> wasmparser::Result<()> {
        let mut validator = wasmparser::Validator::new_with_features(self.config.features);
        validator.validate_all(bytes)?;
        Ok(())
    }
}

fn val_matches(actual: &WasmValue, expected: &WastRetCore) -> Result<bool> {
    Ok(match (actual, expected) {
        (WasmValue::Num(NumVal::I32(a)), WastRetCore::I32(x)) => a == x,
        (WasmValue::Num(NumVal::I64(a)), WastRetCore::I64(x)) => a == x,
        (WasmValue::Num(NumVal::F32(a)), WastRetCore::F32(x)) => match x {
            NanPattern::CanonicalNan => is_canonical_f32_nan(a),
            NanPattern::ArithmeticNan => is_arithmetic_f32_nan(a),
            NanPattern::Value(expected_value) => a.to_bits() == expected_value.bits,
        },
        (WasmValue::Num(NumVal::F64(a)), WastRetCore::F64(x)) => match x {
            NanPattern::CanonicalNan => is_canonical_f64_nan(a),
            NanPattern::ArithmeticNan => is_arithmetic_f64_nan(a),
            NanPattern::Value(expected_value) => a.to_bits() == expected_value.bits,
        },
        (WasmValue::Ref(RefVal::ExternRef(a)), WastRetCore::RefExtern(Some(x))) => a == x,
        (WasmValue::Ref(RefVal::NullRef(a)), WastRetCore::RefNull(Some(x))) => {
            Some(*a) == to_ref_type(x)
        }
        (_, WastRetCore::V128(_)) => bail!("V128 is not supported yet"),
        _ => bail!("unexpected comparing for {:?} and {:?}", actual, expected),
    })
}

fn to_ref_type(heap_ty: &HeapType) -> Option<RefType> {
    match heap_ty {
        HeapType::Func => Some(RefType::FuncRef),
        HeapType::Extern => Some(RefType::ExternRef),
        _ => None,
    }
}

fn const_expr(expr: &WastArgCore) -> WasmValue {
    match &expr {
        WastArgCore::I32(x) => WasmValue::I32(*x),
        WastArgCore::I64(x) => WasmValue::I64(*x),
        WastArgCore::F32(x) => WasmValue::F32(x.bits),
        WastArgCore::F64(x) => WasmValue::F64(x.bits),
        WastArgCore::V128(_) => panic!(),
        WastArgCore::RefExtern(x) => WasmValue::Ref(RefVal::ExternRef(*x)),
        WastArgCore::RefNull(ty) => WasmValue::Ref(RefVal::NullRef(to_ref_type(ty).unwrap())),
        other => panic!("unsupported const expr inst {:?}", other),
    }
}

fn is_canonical_f32_nan(f: &F32) -> bool {
    (f.to_bits() & 0x7fffffff) == 0x7fc00000
}

fn is_canonical_f64_nan(f: &F64) -> bool {
    (f.to_bits() & 0x7fffffffffffffff) == 0x7ff8000000000000
}

fn is_arithmetic_f32_nan(f: &F32) -> bool {
    (f.to_bits() & 0x00400000) == 0x00400000
}

fn is_arithmetic_f64_nan(f: &F64) -> bool {
    (f.to_bits() & 0x0008000000000000) == 0x0008000000000000
}
