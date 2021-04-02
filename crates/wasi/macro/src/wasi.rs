use crate::utils;
use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;
use utils::witx_target_module_map_ident;

enum Abi {
    I32,
    I64,
    F32,
    F64,
}

fn translate_param_for_abi(param: &witx::TypeRef, params: &mut Vec<Abi>) {
    let mut add_param = |abi_ty: Abi| {
        params.push(abi_ty);
    };
    match &*param.type_() {
        witx::Type::Int(e) => match e.repr {
            witx::IntRepr::U64 => add_param(Abi::I64),
            witx::IntRepr::U32 => add_param(Abi::I32),
            _ => add_param(Abi::I32),
        },

        witx::Type::Enum(e) => match e.repr {
            witx::IntRepr::U64 => add_param(Abi::I64),
            witx::IntRepr::U32 => add_param(Abi::I32),
            _ => add_param(Abi::I32),
        },

        witx::Type::Flags(f) => match f.repr {
            witx::IntRepr::U64 => add_param(Abi::I64),
            witx::IntRepr::U32 => add_param(Abi::I32),
            _ => add_param(Abi::I32),
        },

        witx::Type::Builtin(witx::BuiltinType::Char8)
        | witx::Type::Builtin(witx::BuiltinType::S8)
        | witx::Type::Builtin(witx::BuiltinType::U8)
        | witx::Type::Builtin(witx::BuiltinType::S16)
        | witx::Type::Builtin(witx::BuiltinType::U16)
        | witx::Type::Builtin(witx::BuiltinType::S32)
        | witx::Type::Builtin(witx::BuiltinType::U32)
        | witx::Type::Builtin(witx::BuiltinType::USize) => {
            add_param(Abi::I32);
        }

        witx::Type::Builtin(witx::BuiltinType::S64)
        | witx::Type::Builtin(witx::BuiltinType::U64) => {
            add_param(Abi::I64);
        }

        witx::Type::Builtin(witx::BuiltinType::F32) => {
            add_param(Abi::F32);
        }

        witx::Type::Builtin(witx::BuiltinType::F64) => {
            add_param(Abi::F64);
        }

        witx::Type::Builtin(witx::BuiltinType::String) | witx::Type::Array(_) => {
            add_param(Abi::I32); // ptr
            add_param(Abi::I32); // len
        }

        witx::Type::ConstPointer(_) | witx::Type::Handle(_) | witx::Type::Pointer(_) => {
            add_param(Abi::I32);
        }

        witx::Type::Struct(_) | witx::Type::Union(_) => {
            panic!("unsupported argument type")
        }
    }
}

fn emit_func_extern(
    name: &str,
    params: &[Abi],
    returns: &[Abi],
    module_map_id: &Ident,
    module_id: &Ident,
) -> TokenStream {
    let to_wasmparser_ty = |abi_ty: &Abi| match abi_ty {
        &Abi::I32 => quote! { ::wasmparser::Type::I32 },
        &Abi::I64 => quote! { ::wasmparser::Type::I64 },
        &Abi::F32 => quote! { ::wasmparser::Type::F32 },
        &Abi::F64 => quote! { ::wasmparser::Type::F64 },
    };

    let mut param_types = Vec::new();
    for param in params {
        let param = to_wasmparser_ty(param);
        param_types.push(quote! { #param });
    }

    let mut arg_values = Vec::new();
    for (idx, param) in params.iter().enumerate() {
        let cast_fn = match param {
            &Abi::I32 => quote! { as_i32 },
            &Abi::I64 => quote! { as_i64 },
            &Abi::F32 => quote! { as_f32 },
            &Abi::F64 => quote! { as_f64 },
        };
        let idx_lit = Literal::usize_unsuffixed(idx);
        arg_values.push(quote! { args[#idx_lit].#cast_fn().unwrap() });
    }

    let mut return_types = Vec::new();
    for ret in returns {
        let ret = to_wasmparser_ty(ret);
        return_types.push(quote! { #ret });
    }

    let mut ret_value = quote! {};
    if let Some(ret_ty) = returns.first() {
        assert!(returns.len() == 1);
        let (primitive_ty, ty_case) = match ret_ty {
            &Abi::I32 => (quote! { i32 }, quote! { WasmValue::I32 }),
            &Abi::I64 => (quote! { i64 }, quote! { WasmValue::I64 }),
            &Abi::F32 => (quote! { f32 }, quote! { WasmValue::F32 }),
            &Abi::F64 => (quote! { f64 }, quote! { WasmValue::F64 }),
        };
        ret_value = quote! { ret.push(#ty_case(result as #primitive_ty)); };
    }

    let name_id = Ident::new(name, Span::call_site());
    let name_str = name;
    let call_expr = if name == "proc_exit" {
        quote! {
            let result = crate::wasi_proc_exit(
                #(#arg_values),*
            );
        }
    } else {
        quote! {
            let result = wasi_common::wasi::#module_id::#name_id(
                &*wasi_ctx,
                &mem,
                #(#arg_values),*
            );
            #ret_value
        }
    };
    quote! {
        let ty = ::wasmparser::FuncType {
            params: vec![#(#param_types),*].into_boxed_slice(),
            returns: vec![#(#return_types),*].into_boxed_slice(),
        };
        let func = HostValue::Func(HostFuncBody::new(ty, move |args, ret, ctx, store| {
            let wasi_ctx = store.get_embed_context::<WasiContext>().unwrap();
            let mut wasi_ctx = wasi_ctx.ctx.borrow_mut();
            let bc = unsafe { wiggle::BorrowChecker::new() };
            let mem = WasiMemory {
                mem: ctx.mem.as_mut_ptr(),
                mem_size: ctx.mem.len() as u32,
                bc,
            };
            #call_expr
            Ok(())
        }));
        #module_map_id.insert(#name_str.to_string(), func);
    }
}

pub fn define_wasi_fn_for_wasminspect(args: TokenStream) -> TokenStream {
    let mut args = args.into_iter();
    let module_map_id = witx_target_module_map_ident(args.next().unwrap());
    args.next(); // consume ","
    let module_map_id = Ident::new(&module_map_id, Span::call_site());
    let path = utils::witx_path_from_arg(args.next().unwrap());
    let doc = match witx::load(&[&path]) {
        Ok(doc) => doc,
        Err(e) => {
            panic!("error opening file {}: {}", path.display(), e);
        }
    };

    let mut ctor_externs = Vec::new();

    for module in doc.modules() {
        let module_name = module.name.as_str();
        let module_id = Ident::new(module_name, Span::call_site());

        for func in module.funcs() {
            let name = func.name.as_str();
            let mut params = Vec::new();
            let mut returns = Vec::new();

            for param in func.params.iter() {
                translate_param_for_abi(&param.tref, &mut params);
            }
            let mut results = func.results.iter();

            // The first result is returned bare right now...
            if let Some(ret) = results.next() {
                match &*ret.tref.type_() {
                    witx::Type::Enum(e) => match e.repr {
                        witx::IntRepr::U16 => {
                            returns.push(Abi::I32);
                        }
                        other => unreachable!("unhandled type: {:?}", other),
                    },
                    other => panic!("unsupported first return {:?}", other),
                }
            }

            // ... and all remaining results are returned via out-poiners
            for _ in results {
                params.push(Abi::I32);
            }

            ctor_externs.push(emit_func_extern(name, &params, &returns, &module_map_id, &module_id));
        }
    }
    quote! {
        struct WasiMemory {
            mem: *mut u8,
            mem_size: u32,
            bc: wiggle::BorrowChecker,
        }

        unsafe impl ::wiggle::GuestMemory for WasiMemory {
            fn base(&self) -> (*mut u8, u32) {
                return (self.mem, self.mem_size);
            }
            fn borrow_checker(&self) -> &::wiggle::BorrowChecker {
                &self.bc
            }
        }

        #(#ctor_externs)*
    }
}
