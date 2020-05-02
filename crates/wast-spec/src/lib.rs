/// Reference: https://github.com/bytecodealliance/wasmtime/blob/master/crates/wast/src/wast.rs
use anyhow::{anyhow, bail, Context as _, Result};
use std::collections::HashMap;
use std::path::Path;
use std::str;
mod spectest;
pub use spectest::instantiate_spectest;
use wasminspect_vm::{simple_invoke_func, FuncAddr, ModuleIndex, WasmInstance, WasmValue};
use wasmparser::{validate, ModuleReader};

pub struct WastContext {
    module_index_by_name: HashMap<String, ModuleIndex>,
    instance: WasmInstance,
    current: Option<ModuleIndex>,
}

impl WastContext {
    pub fn new() -> Self {
        let mut instance = WasmInstance::new();
        instance.load_host_module("spectest".to_string(), instantiate_spectest());
        Self {
            module_index_by_name: HashMap::new(),
            instance: instance,
            current: None,
        }
    }
    pub fn run_file(&mut self, path: &Path) -> Result<()> {
        let bytes = std::fs::read(path).unwrap();
        self.run_buffer(path.to_str().unwrap(), &bytes)
    }

    pub fn extract_start_section(bytes: &[u8]) -> Result<Option<u32>> {
        let mut reader = ModuleReader::new(bytes)?;
        while !reader.eof() {
            let section = reader.read()?;
            if section.code != wasmparser::SectionCode::Start {
                continue;
            }
            match section.get_start_section_content() {
                Ok(offset) => return Ok(Some(offset)),
                Err(_) => continue,
            }
        }
        return Ok(None);
    }
    pub fn instantiate<'a>(&self, bytes: &'a [u8]) -> Result<ModuleReader<'a>> {
        validate(bytes, None)?;
        Ok(ModuleReader::new(bytes)?)
    }
    fn module(&mut self, module_name: Option<&str>, bytes: &[u8]) -> Result<()> {
        let module = self.instantiate(&bytes)?;
        let start_section = Self::extract_start_section(bytes)?;
        let module_index = self
            .instance
            .load_module_from_module(module_name.map(|n| n.to_string()), module)
            .map_err(|e| anyhow!("Failed to instantiate: {}", e))?;
        if let Some(start_section) = start_section {
            let func_addr = FuncAddr::new_unsafe(module_index, start_section as usize);
            simple_invoke_func(func_addr, vec![], &mut self.instance.store)
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
        let context = |sp: wast::Span| {
            let (line, col) = sp.linecol_in(wast);
            format!("for directive on {}:{}:{}", filename, line + 1, col)
        };

        let buf = wast::parser::ParseBuffer::new(wast).map_err(adjust_wast)?;
        let wast = wast::parser::parse::<wast::Wast>(&buf).map_err(adjust_wast)?;

        for directive in wast.directives {
            match directive {
                Module(mut module) => {
                    let bytes = module.encode().map_err(adjust_wast)?;
                    self.module(module.name.map(|s| s.name()), &bytes)
                        .map_err(|err| anyhow!("{}, {}", err, context(module.span)))?;
                }
                Register {
                    span: _,
                    name,
                    module,
                } => {
                    let module_index = self.get_instance(module.map(|s| s.name()))?;
                    self.instance.register_name(name.to_string(), module_index);
                }
                Invoke(i) => {
                    self.invoke(i.module.map(|s| s.name()), i.name, &i.args)
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
                            if val_matches(v, &e)? {
                                continue;
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
                    Err(err) => panic!("{}", err),
                },
                AssertMalformed {
                    span,
                    module,
                    message,
                } => {
                    let mut module = match module {
                        wast::QuoteModule::Module(m) => m,
                        // this is a `*.wat` parser test which we're not
                        // interested in
                        wast::QuoteModule::Quote(_) => return Ok(()),
                    };
                    let bytes = module.encode().map_err(adjust_wast)?;
                    let err = match self.module(None, &bytes) {
                        Ok(()) => {
                            panic!("{}\nexpected module to fail to instantiate", context(span))
                        }
                        Err(e) => e,
                    };
                    let error_message = format!("{:?}", err);
                    if !error_message.contains(&message) {
                        // TODO: change to panic!
                        println!(
                            "{}\nassert_malformed: expected {}, got {}",
                            context(span),
                            message,
                            error_message
                        )
                    }
                }
                AssertUnlinkable {
                    span,
                    mut module,
                    message,
                } => {
                    let bytes = module.encode().map_err(adjust_wast)?;
                    let err = match self.module(None, &bytes) {
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
                } => match self.invoke(call.module.map(|s| s.name()), call.name, &call.args) {
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
                    mut module,
                    message,
                } => {
                    let bytes = module.encode().map_err(adjust_wast)?;
                    let err = match self.module(None, &bytes) {
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
            }
        }
        Ok(())
    }

    fn get_instance(&self, name: Option<&str>) -> Result<ModuleIndex> {
        match name {
            Some(name) => self
                .module_index_by_name
                .get(name)
                .map(|i| i.clone())
                .ok_or(anyhow!("module not found with name {}", name)),
            None => match self.current.clone() {
                Some(current) => Ok(current),
                None => panic!(),
            },
        }
    }

    /// Get the value of an exported global from an instance.
    fn get(&mut self, instance_name: Option<&str>, field: &str) -> Result<Result<Vec<WasmValue>>> {
        let module_index = self.get_instance(instance_name.as_ref().map(|x| &**x))?;
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
        module_name: Option<&str>,
        func_name: &str,
        args: &[wast::Expression],
    ) -> Result<Vec<WasmValue>> {
        let module_index = self.get_instance(module_name)?;
        let args = args.iter().map(const_expr).collect();
        let result = self
            .instance
            .run(module_index, Some(func_name.to_string()), args)
            .map_err(|e| anyhow!("{}", e))?;
        Ok(result)
    }

    fn perform_execute(&mut self, exec: wast::WastExecute<'_>) -> Result<Result<Vec<WasmValue>>> {
        match exec {
            wast::WastExecute::Invoke(i) => {
                Ok(self.invoke(i.module.map(|s| s.name()), i.name, &i.args))
            }
            wast::WastExecute::Module(mut module) => {
                let binary = module.encode()?;
                let module = self.instantiate(&binary)?;
                let start_section = Self::extract_start_section(&binary)?;
                let module_index = self
                    .instance
                    .load_module_from_module(None, module)
                    .map_err(|e| anyhow!("{}", e))?;
                if let Some(start_section) = start_section {
                    let func_addr = FuncAddr::new_unsafe(module_index, start_section as usize);
                    return Ok(
                        simple_invoke_func(func_addr, vec![], &mut self.instance.store)
                            .map_err(|e| anyhow!("Failed to exec start func: {}", e)),
                    );
                }
                Ok(Ok(vec![]))
            }
            wast::WastExecute::Get { module, global } => self.get(module.map(|s| s.name()), global),
        }
    }
}

fn val_matches(actual: &WasmValue, expected: &wast::AssertExpression) -> Result<bool> {
    Ok(match (actual, expected) {
        (WasmValue::I32(a), wast::AssertExpression::I32(x)) => a == x,
        (WasmValue::I64(a), wast::AssertExpression::I64(x)) => a == x,
        (WasmValue::F32(a), wast::AssertExpression::F32(x)) => match x {
            wast::NanPattern::CanonicalNan => is_canonical_f32_nan(a),
            wast::NanPattern::ArithmeticNan => is_arithmetic_f32_nan(a),
            wast::NanPattern::Value(expected_value) => *a == expected_value.bits,
        },
        (WasmValue::F64(a), wast::AssertExpression::F64(x)) => match x {
            wast::NanPattern::CanonicalNan => is_canonical_f64_nan(a),
            wast::NanPattern::ArithmeticNan => is_arithmetic_f64_nan(a),
            wast::NanPattern::Value(expected_value) => *a == expected_value.bits,
        },
        (_, wast::AssertExpression::V128(_)) => bail!("V128 is not supported yet"),
        _ => bail!("unexpected comparing for {:?} and {:?}", actual, expected),
    })
}

fn const_expr(expr: &wast::Expression) -> WasmValue {
    match &expr.instrs[0] {
        wast::Instruction::I32Const(x) => WasmValue::I32(*x),
        wast::Instruction::I64Const(x) => WasmValue::I64(*x),
        wast::Instruction::F32Const(x) => WasmValue::F32(x.bits),
        wast::Instruction::F64Const(x) => WasmValue::F64(x.bits),
        wast::Instruction::V128Const(_) => panic!(),
        _ => panic!(),
    }
}

fn is_canonical_f32_nan(f: &u32) -> bool {
    return (f & 0x7fffffff) == 0x7fc00000;
}

fn is_canonical_f64_nan(f: &u64) -> bool {
    return (f & 0x7fffffffffffffff) == 0x7ff8000000000000;
}

fn is_arithmetic_f32_nan(f: &u32) -> bool {
    return (f & 0x00400000) == 0x00400000;
}

fn is_arithmetic_f64_nan(f: &u64) -> bool {
    return (f & 0x0008000000000000) == 0x0008000000000000;
}
