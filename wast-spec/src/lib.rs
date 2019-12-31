pub fn spectest() {}

use anyhow::Result;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::str;
use wasminspect_core::interpreter::{WasmInstance, WasmValue};

pub struct WastContext {
    instances: HashMap<String, Rc<RefCell<WasmInstance>>>,
    current: Option<Rc<RefCell<WasmInstance>>>,
}

impl WastContext {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            current: None,
        }
    }
    pub fn run_file(&mut self, path: &Path) -> Result<()> {
        let bytes = std::fs::read(path).unwrap();
        self.run_buffer(path.to_str().unwrap(), &bytes)
    }

    pub fn instantiate(&self, bytes: &[u8]) -> Rc<RefCell<WasmInstance>> {
        let parity_module: parity_wasm::elements::Module =
            parity_wasm::deserialize_buffer(&bytes).unwrap();
        return Rc::new(RefCell::new(WasmInstance::new_from_parity_module(
            parity_module,
        )));
    }
    fn module(&mut self, module_name: Option<&str>, bytes: &[u8]) -> Result<()> {
        let instance = self.instantiate(&bytes);
        if let Some(module_name) = module_name {
            self.instances
                .insert(module_name.to_string(), instance.clone());
        }
        self.current = Some(instance);
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
                    self.module(module.name.map(|s| s.name()), &bytes)?;
                }
                Register { span, name, module } => {
                    let instance = self.get_instance(module.map(|s| s.name()));
                    self.instances.insert(name.to_string(), instance);
                }
                Invoke(i) => {
                    self.invoke(i.module.map(|s| s.name()), i.name, &i.args);
                }
                AssertReturn {
                    span,
                    exec,
                    results,
                } => match self.perform_execute(exec) {
                    Ok(values) => {
                        for (v, e) in values.iter().zip(results.iter().map(const_expr)) {
                            let e = e;
                            if v == &e {
                                continue;
                            }
                            panic!("expected {:?}, got {:?}", e, v)
                        }
                    }
                    Err(e) => {
                        panic!("unexpected err: {}", e);
                    }
                },
                AssertInvalid {
                    span,
                    mut module,
                    message,
                } => {
                    let bytes = module.encode().map_err(adjust_wast)?;
                    // TODO Fix type-check
                    let err = match self.module(None, &bytes) {
                        Ok(()) => {
                            println!("{}\nexpected module to fail to build", context(span));
                            break;
                        }
                        Err(e) => e,
                    };
                    let error_message = format!("{:?}", err);
                    if !error_message.contains(&message) {
                        // TODO: change to bail!
                        println!(
                            "{}\nassert_invalid: expected {}, got {}",
                            context(span),
                            message,
                            error_message
                        )
                    }
                }
                other => panic!("unsupported"),
            }
        }
        Ok(())
    }

    fn get_instance(&self, name: Option<&str>) -> Rc<RefCell<WasmInstance>> {
        match name {
            Some(name) => self.instances.get(name).unwrap().clone(),
            None => match self.current.clone() {
                Some(current) => current,
                None => panic!(),
            },
        }
    }

    /// Get the value of an exported global from an instance.
    fn get(&mut self, instance_name: Option<&str>, field: &str) -> Result<Vec<WasmValue>> {
        let instance = self.get_instance(instance_name.as_ref().map(|x| &**x));
        let instance = instance.borrow();
        panic!();
    }

    fn invoke(
        &mut self,
        module_name: Option<&str>,
        func_name: &str,
        args: &[wast::Expression],
    ) -> Vec<WasmValue> {
        println!(
            "Invoking \"{}.{}\"",
            module_name.unwrap_or("unknown"),
            func_name
        );
        let instance = self.get_instance(module_name).clone();
        let args = args.iter().map(const_expr).collect();
        return instance
            .borrow_mut()
            .run(Some(func_name.to_string()), args)
            .ok()
            .expect("func invocation");
    }

    fn perform_execute(&mut self, exec: wast::WastExecute<'_>) -> Result<Vec<WasmValue>> {
        match exec {
            wast::WastExecute::Invoke(i) => {
                Ok(self.invoke(i.module.map(|s| s.name()), i.name, &i.args))
            }
            wast::WastExecute::Module(mut module) => {
                let binary = module.encode()?;
                let result = self.instantiate(&binary);
                Ok(Vec::new())
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
