use std::collections::HashMap;

use crate::rpc::{self};
use wasminspect_debugger::{CommandContext, CommandResult, MainDebugger, Process};
use wasminspect_vm::{HostFuncBody, HostValue, WasmValue};

static VERSION: &str = "0.1.0";

pub fn handle_request(
    req: rpc::Request,
    process: &mut Process<MainDebugger>,
    context: &CommandContext,
) -> rpc::Response {
    match _handle_request(req, process, context) {
        Ok(res) => res,
        Err(err) => rpc::TextResponse::Error {
            message: err.to_string(),
        }
        .into(),
    }
}

fn to_vm_wasm_value(value: &rpc::WasmValue) -> WasmValue {
    match value {
        rpc::WasmValue::F32 { value } => WasmValue::F32(*value),
        rpc::WasmValue::F64 { value } => WasmValue::F64(*value),
        rpc::WasmValue::I32 { value } => WasmValue::I32(*value),
        rpc::WasmValue::I64 { value } => WasmValue::I64(*value),
    }
}

fn from_vm_wasm_value(value: &WasmValue) -> rpc::WasmValue {
    match value {
        WasmValue::F32(v) => rpc::WasmValue::F32 { value: *v },
        WasmValue::F64(v) => rpc::WasmValue::F64 { value: *v },
        WasmValue::I32(v) => rpc::WasmValue::I32 { value: *v },
        WasmValue::I64(v) => rpc::WasmValue::I64 { value: *v },
    }
}

fn remote_import_module(
    bytes: &[u8],
) -> anyhow::Result<HashMap<String, HashMap<String, HostValue>>> {
    let parser = wasmparser::Parser::new(0);
    let mut types = HashMap::new();
    let mut module_imports = HashMap::new();
    let mut modules: HashMap<String, HashMap<String, HostValue>> = HashMap::new();

    for payload in parser.parse_all(bytes) {
        match payload? {
            wasmparser::Payload::TypeSection(mut iter) => {
                for idx in 0..iter.get_count() {
                    let ty = iter.read()?;
                    types.insert(idx, ty);
                }
            }
            wasmparser::Payload::ImportSection(iter) => {
                for import in iter {
                    let import = import?;
                    module_imports.insert((import.module, import.field), import);

                    let ty_idx = match import.ty {
                        wasmparser::ImportSectionEntryType::Function(ty_idx) => ty_idx,
                        _ => continue,
                    };
                    let ty = match types.get(&ty_idx) {
                        Some(wasmparser::TypeDef::Func(ty)) => ty,
                        _ => continue,
                    };
                    let field_name = match import.field {
                        Some(field_name) => field_name,
                        None => continue,
                    };
                    let field_name0 = field_name.to_string().clone();
                    let field_name1 = field_name.to_string();
                    let f = HostFuncBody::new(ty.clone(), move |args, results, _, _| {
                        println!("{}", field_name0);
                        Ok(())
                    });
                    modules
                        .entry(import.module.to_string())
                        .or_default()
                        .insert(field_name1, HostValue::Func(f));
                }
            }
            _ => continue,
        }
    }
    Ok(modules)
}

fn _handle_request(
    req: rpc::Request,
    process: &mut Process<MainDebugger>,
    context: &CommandContext,
) -> Result<rpc::Response, anyhow::Error> {
    use rpc::BinaryRequestKind::*;
    use rpc::Request::*;
    use rpc::TextRequest::*;
    use rpc::*;

    match req {
        Binary(req) => match req.kind {
            Init => {
                process.debugger.reset_store();
                let imports = remote_import_module(req.bytes)?;
                for (name, module) in imports {
                    process.debugger.load_host_module(name, module);
                }
                process.debugger.load_module(req.bytes)?;
                return Ok(rpc::Response::Text(TextResponse::Init));
            }
        },
        Text(Version) => {
            return Ok(TextResponse::Version {
                value: VERSION.to_string(),
            }
            .into());
        }
        Text(CallExported { name, args }) => {
            use wasminspect_debugger::RunResult;
            let func = process.debugger.lookup_func(&name)?;
            let args = args.iter().map(to_vm_wasm_value).collect();
            match process.debugger.execute_func(func, args) {
                Ok(RunResult::Finish(values)) => {
                    let values = values.iter().map(from_vm_wasm_value).collect();
                    return Ok(TextResponse::CallResult { values }.into());
                }
                Ok(RunResult::Breakpoint) => {
                    let mut result = process.run_loop(context)?;
                    loop {
                        match result {
                            CommandResult::ProcessFinish(values) => {
                                let values = values.iter().map(from_vm_wasm_value).collect();
                                return Ok(TextResponse::CallResult { values }.into());
                            }
                            CommandResult::Exit => {
                                match process.dispatch_command("process continue", context)? {
                                    Some(r) => {
                                        result = r;
                                    }
                                    None => {
                                        result = process.run_loop(context)?;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(msg) => {
                    return Err(msg.into());
                }
            }
        }
    }
}
