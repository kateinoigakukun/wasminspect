pub fn spectest() {}

use anyhow::Result;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::str;
use wasminspect_core::interpreter::{WasmInstance, WasmValue};

pub struct WastContext {
    instances: HashMap<String, Rc<RefCell<WasmInstance>>>,
    current: Option<Rc<RefCell<WasmInstance>>>,
}

impl WastContext {
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
                    let parity_module: parity_wasm::elements::Module =
                        parity_wasm::deserialize_buffer(&bytes).unwrap();
                    let instance = Rc::new(RefCell::new(WasmInstance::new_from_parity_module(
                        parity_module,
                    )));
                    if let Some(module_name) = module.name.map(|s| s.name()) {
                        self.instances
                            .insert(module_name.to_string(), instance.clone());
                    }
                    self.current = Some(instance);
                }
                Register { span, name, module } => {
                    let instance = self.get_instance(module.map(|s| s.name()));
                    self.instances.insert(name.to_string(), instance);
                }
                Invoke(i) => {self.invoke(i.module.map(|s| s.name()), i.name, &i.args);}
                _ => panic!("unsupported"),
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

    fn invoke(
        &mut self,
        module_name: Option<&str>,
        func_name: &str,
        args: &[wast::Expression],
    ) -> WasmValue {
        let instance = self.get_instance(module_name).clone();
        let args = args.iter().map(const_expr).collect();
        return instance
            .borrow_mut()
            .run(Some(func_name.to_string()), args)
            .ok()
            .expect("func invocation")[0];
    }
}

fn const_expr(expr: &wast::Expression) -> WasmValue {
    match &expr.instrs[0] {
        wast::Instruction::I32Const(x) => WasmValue::I32(*x),
        wast::Instruction::I64Const(x) => WasmValue::I64(*x),
        wast::Instruction::F32Const(x) => panic!(),
        wast::Instruction::F64Const(x) => panic!(),
        wast::Instruction::V128Const(x) => panic!(),
        _ => panic!(),
    }
}
