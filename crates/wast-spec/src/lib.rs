/// Reference: https://github.com/bytecodealliance/wasmtime/blob/master/crates/wast/src/wast.rs
use anyhow::{anyhow, bail, Context as _, Result};
use std::collections::HashMap;
use std::path::Path;
use std::str;
mod spectest;
pub use spectest::instantiate_spectest;
use wasmi_validation::{validate_module, PlainValidator};
use wasminspect_vm::{
    simple_invoke_func, FuncAddr, ModuleIndex, WasmError, WasmInstance, WasmValue,
};
use wasmparser::ModuleReader;

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

    pub fn extract_start_section(bytes: &[u8]) -> Option<u32> {
        let module =
            parity_wasm::deserialize_buffer::<parity_wasm::elements::Module>(&bytes).unwrap();
        return module.start_section();
    }
    pub fn instantiate<'a>(
        &self,
        bytes: &'a [u8],
        ignore_validation: bool,
    ) -> Result<ModuleReader<'a>> {
        let module = parity_wasm::deserialize_buffer(&bytes)
            .with_context(|| anyhow!("Failed to parse wasm"))?;
        let reader = ModuleReader::new(bytes)?;
        match validate_module::<PlainValidator>(&module)
            .map_err(|e| anyhow!("validation error: {}", e))
        {
            Err(err) => {
                if ignore_validation {
                    Ok(reader)
                } else {
                    if format!("{}", err).contains("trying to import mutable global glob") {
                        Ok(reader)
                    } else {
                        Err(err)
                    }
                }
            }
            Ok(_) => Ok(reader),
        }
    }
    fn module(
        &mut self,
        module_name: Option<&str>,
        bytes: &[u8],
        ignore_validation: bool,
    ) -> Result<()> {
        let module = self.instantiate(&bytes, ignore_validation)?;
        let start_section = Self::extract_start_section(bytes);
        let module_index = self
            .instance
            .load_module_from_parity_module(module_name.map(|n| n.to_string()), module)
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
                    self.module(module.name.map(|s| s.name()), &bytes, true)
                        .map_err(|err| anyhow!("{}, {}", err, context(module.span)))?;
                }
                Register {
                    span: _,
                    name,
                    module,
                } => {
                    let module_index = self.get_instance(module.map(|s| s.name()));
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
                        for (v, e) in values.iter().zip(results.iter().map(const_expr)) {
                            let e = e;
                            if is_equal_value(*v, e) {
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
                    let err = match self.module(None, &bytes, false) {
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
                    let err = match self.module(None, &bytes, false) {
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
                    let err = match self.module(None, &bytes, false) {
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
                AssertReturnCanonicalNan { span, invoke } => {
                    match self.invoke(invoke.module.map(|s| s.name()), invoke.name, &invoke.args) {
                        Ok(values) => {
                            for v in values.iter() {
                                match v {
                                    WasmValue::F32(x) => {
                                        if !is_canonical_f32_nan(x) {
                                            println!("{}\nexpected canonical NaN", context(span))
                                        }
                                    }
                                    WasmValue::F64(x) => {
                                        if !is_canonical_f64_nan(x) {
                                            println!("{}\nexpected canonical NaN", context(span))
                                        }
                                    }
                                    other => bail!("expected float, got {:?}", other),
                                };
                            }
                        }
                        Err(t) => bail!("{}\nunexpected trap: {}", context(span), t),
                    }
                }
                AssertReturnArithmeticNan { span, invoke } => {
                    match self.invoke(invoke.module.map(|s| s.name()), invoke.name, &invoke.args) {
                        Ok(values) => {
                            for v in values.iter() {
                                match v {
                                    WasmValue::F32(x) => {
                                        if !is_arithmetic_f32_nan(x) {
                                            println!("{}\nexpected arithmetic NaN", context(span))
                                        }
                                    }
                                    WasmValue::F64(x) => {
                                        if !is_arithmetic_f64_nan(x) {
                                            println!("{}\nexpected arithmetic NaN", context(span))
                                        }
                                    }
                                    other => bail!("expected float, got {:?}", other),
                                };
                            }
                        }
                        Err(t) => bail!("{}\nunexpected trap: {}", context(span), t),
                    }
                }
                _ => panic!("unsupported"),
            }
        }
        Ok(())
    }

    fn get_instance(&self, name: Option<&str>) -> ModuleIndex {
        match name {
            Some(name) => self.module_index_by_name.get(name).unwrap().clone(),
            None => match self.current.clone() {
                Some(current) => current,
                None => panic!(),
            },
        }
    }

    /// Get the value of an exported global from an instance.
    fn get(&mut self, instance_name: Option<&str>, field: &str) -> Result<Result<Vec<WasmValue>>> {
        let module_index = self.get_instance(instance_name.as_ref().map(|x| &**x));
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
    ) -> Result<Vec<WasmValue>, WasmError> {
        let module_index = self.get_instance(module_name).clone();
        let args = args.iter().map(const_expr).collect();
        return self
            .instance
            .run(module_index, Some(func_name.to_string()), args);
    }

    fn perform_execute(&mut self, exec: wast::WastExecute<'_>) -> Result<Result<Vec<WasmValue>>> {
        match exec {
            wast::WastExecute::Invoke(i) => Ok(self
                .invoke(i.module.map(|s| s.name()), i.name, &i.args)
                .map_err(|e| anyhow!("{}", e))),
            wast::WastExecute::Module(mut module) => {
                let binary = module.encode()?;
                let module = self.instantiate(&binary, false)?;
                let start_section = Self::extract_start_section(&binary);
                let module_index = self
                    .instance
                    .load_module_from_parity_module(None, module)
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

fn const_expr(expr: &wast::Expression) -> WasmValue {
    match &expr.instrs[0] {
        wast::Instruction::I32Const(x) => WasmValue::I32(*x),
        wast::Instruction::I64Const(x) => WasmValue::I64(*x),
        wast::Instruction::F32Const(x) => WasmValue::F32(f32::from_bits(x.bits)),
        wast::Instruction::F64Const(x) => WasmValue::F64(f64::from_bits(x.bits)),
        wast::Instruction::V128Const(_) => panic!(),
        _ => panic!(),
    }
}

fn is_equal_value(lhs: WasmValue, rhs: WasmValue) -> bool {
    match (lhs, rhs) {
        (WasmValue::I32(lhs), WasmValue::I32(rhs)) => (lhs == rhs),
        (WasmValue::I64(lhs), WasmValue::I64(rhs)) => (lhs == rhs),
        (WasmValue::F32(lhs), WasmValue::F32(rhs)) => {
            (lhs == rhs) || (lhs.is_nan() && rhs.is_nan())
        }
        (WasmValue::F64(lhs), WasmValue::F64(rhs)) => {
            (lhs == rhs) || (lhs.is_nan() && rhs.is_nan())
        }
        (_, _) => false,
    }
}

fn is_canonical_f32_nan(f: &f32) -> bool {
    return (f.to_bits() & 0x7fffffff) == 0x7fc00000;
}

fn is_canonical_f64_nan(f: &f64) -> bool {
    return (f.to_bits() & 0x7fffffffffffffff) == 0x7ff8000000000000;
}

fn is_arithmetic_f32_nan(f: &f32) -> bool {
    return (f.to_bits() & 0x00400000) == 0x00400000;
}

fn is_arithmetic_f64_nan(f: &f64) -> bool {
    return (f.to_bits() & 0x0008000000000000) == 0x0008000000000000;
}
