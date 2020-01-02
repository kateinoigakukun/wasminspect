use anyhow::{anyhow, bail, Context as _, Result};
use std::collections::HashMap;
use std::path::Path;
use std::str;
mod spectest;
use spectest::instantiate_spectest;
use wasminspect_core::interpreter::{ModuleIndex, Trap, WasmError, WasmInstance, WasmValue};

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

    pub fn instantiate(&self, bytes: &[u8]) -> parity_wasm::elements::Module {
        let parity_module: parity_wasm::elements::Module =
            parity_wasm::deserialize_buffer(&bytes).unwrap();
        return parity_module;
    }
    fn module(&mut self, module_name: Option<&str>, bytes: &[u8]) -> Result<()> {
        let module = self.instantiate(&bytes);
        let module_index = self
            .instance
            .load_module_from_parity_module(module_name.map(|n| n.to_string()), module);
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
                        .with_context(|| context(module.span))?;
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
                        .map_err(|err| anyhow!("{}", err))
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
                            bail!("expected {:?}, got {:?}", e, v)
                        }
                    }
                    Ok(Err(e)) => bail!("unexpected err: {}", e),
                    Err(e) => bail!("unexpected err: {}", e),
                },
                AssertTrap {
                    span,
                    exec,
                    message,
                } => match self.perform_execute(exec).with_context(|| context(span)) {
                    Ok(Ok(values)) => bail!("{}\nexpected trap, got {:?}", context(span), values),
                    Ok(Err(t)) => {
                        let result = format!("{}", t);
                        if result.contains(message) {
                            continue;
                        }
                        bail!("{}\nexpected {}, got {}", context(span), message, result,)
                    }
                    Err(err) => bail!("{}", err),
                },
                AssertMalformed {
                    span: _,
                    module: _,
                    message: _,
                } => {
                    println!("assert_malformed is unsupported");
                }
                AssertUnlinkable {
                    span: _,
                    module: _,
                    message: _,
                } => {
                    println!("assert_unlinkable is unsupported");
                }
                AssertExhaustion {
                    span: _,
                    call: _,
                    message: _,
                } => {
                    println!("assert_exhaustion is unsupported");
                }
                AssertInvalid {
                    span,
                    mut module,
                    message,
                } => {
                    println!("assert_invalid is unsupported");
                    // let bytes = module.encode().map_err(adjust_wast)?;
                    // // TODO Fix type-check
                    // let err = match self.module(None, &bytes) {
                    //     Ok(()) => {
                    //         println!("{}\nexpected module to fail to build", context(span));
                    //         break;
                    //     }
                    //     Err(e) => e,
                    // };
                    // let error_message = format!("{:?}", err);
                    // if !error_message.contains(&message) {
                    //     // TODO: change to bail!
                    //     println!(
                    //         "{}\nassert_invalid: expected {}, got {}",
                    //         context(span),
                    //         message,
                    //         error_message
                    //     )
                    // }
                }
                other => panic!("unsupported"),
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
    fn get(
        &mut self,
        instance_name: Option<&str>,
        field: &str,
    ) -> Result<Result<Vec<WasmValue>, WasmError>> {
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
        println!(
            "Invoking \"{}.{}\"",
            module_name.unwrap_or("unknown"),
            func_name
        );
        let module_index = self.get_instance(module_name).clone();
        let args = args.iter().map(const_expr).collect();
        return self
            .instance
            .run(module_index, Some(func_name.to_string()), args);
    }

    fn perform_execute(
        &mut self,
        exec: wast::WastExecute<'_>,
    ) -> Result<Result<Vec<WasmValue>, WasmError>> {
        match exec {
            wast::WastExecute::Invoke(i) => {
                Ok(self.invoke(i.module.map(|s| s.name()), i.name, &i.args))
            }
            wast::WastExecute::Module(mut module) => {
                let binary = module.encode()?;
                let module = self.instantiate(&binary);
                self.instance.load_module_from_parity_module(None, module);
                Ok(Ok(Vec::new()))
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
        wast::Instruction::V128Const(x) => panic!(),
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
