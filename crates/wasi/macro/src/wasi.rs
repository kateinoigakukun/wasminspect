use crate::utils;
use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;
use utils::witx_target_module_map_ident;
use witx::WasmType;

fn emit_func_extern(
    name: &str,
    params: &[WasmType],
    returns: &[WasmType],
    module_map_id: &Ident,
    module_id: &Ident,
) -> TokenStream {
    let to_wasmparser_ty = |abi_ty: &WasmType| match abi_ty {
        WasmType::I32 => quote! { ::wasmparser::ValType::I32 },
        WasmType::I64 => quote! { ::wasmparser::ValType::I64 },
        WasmType::F32 => quote! { ::wasmparser::ValType::F32 },
        WasmType::F64 => quote! { ::wasmparser::ValType::F64 },
    };

    let mut param_types = Vec::new();
    for param in params {
        let param = to_wasmparser_ty(param);
        param_types.push(quote! { #param });
    }

    let mut arg_values = Vec::new();
    for (idx, param) in params.iter().enumerate() {
        let cast_fn = match param {
            WasmType::I32 => quote! { as_i32 },
            WasmType::I64 => quote! { as_i64 },
            WasmType::F32 => quote! { as_f32 },
            WasmType::F64 => quote! { as_f64 },
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
            WasmType::I32 => (quote! { i32 }, quote! { WasmValue::I32 }),
            WasmType::I64 => (quote! { i64 }, quote! { WasmValue::I64 }),
            WasmType::F32 => (quote! { f32 }, quote! { WasmValue::F32 }),
            WasmType::F64 => (quote! { f64 }, quote! { WasmValue::F64 }),
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
            let result = match wasi_common::snapshots::preview_1::#module_id::#name_id(
                &*wasi_ctx,
                &mem,
                #(#arg_values),*
            ) {
                Ok(result) => result,
                Err(e) => return Err(Trap::HostFunctionError(Box::new(WasiError(format!("{:?}", e))))),
            };
            #ret_value
        }
    };
    quote! {
        let ty = ::wasmparser::FuncType {
            params: vec![#(#param_types),*].into_boxed_slice(),
            returns: vec![#(#return_types),*].into_boxed_slice(),
        };
        let func = HostValue::Func(HostFuncBody::new(ty, move |args, ret, ctx, store| {
            log::debug!("{}({:?})", #name, args);
            let wasi_ctx = store.get_embed_context::<WasiContext>().unwrap();
            let mut wasi_ctx = wasi_ctx.ctx.borrow_mut();
            let bc = unsafe { borrow::BorrowChecker::new() };
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
    let module_map_id = witx_target_module_map_ident(args.next().expect("module map id"));
    args.next(); // consume ","
    let module_map_id = Ident::new(&module_map_id, Span::call_site());
    let path = utils::witx_path_from_arg(args.next().expect("witx path"));
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
            let (params, returns) = func.wasm_signature();

            ctor_externs.push(emit_func_extern(
                name,
                &params,
                &returns,
                &module_map_id,
                &module_id,
            ));
        }
    }
    quote! {
        struct WasiMemory {
            mem: *mut u8,
            mem_size: u32,
            bc: borrow::BorrowChecker,
        }
        unsafe impl ::wiggle::GuestMemory for WasiMemory {
            fn base(&self) -> (*mut u8, u32) {
                return (self.mem, self.mem_size);
            }
            fn has_outstanding_borrows(&self) -> bool {
                self.bc.has_outstanding_borrows()
            }
            fn is_shared_borrowed(&self, r: ::wiggle::Region) -> bool {
                self.bc.is_shared_borrowed(r)
            }
            fn is_mut_borrowed(&self, r: ::wiggle::Region) -> bool {
                self.bc.is_mut_borrowed(r)
            }
            fn shared_borrow(&self, r: ::wiggle::Region) -> Result<::wiggle::BorrowHandle, ::wiggle::GuestError> {
                self.bc.shared_borrow(r)
            }
            fn mut_borrow(&self, r: ::wiggle::Region) -> Result<::wiggle::BorrowHandle, ::wiggle::GuestError> {
                self.bc.mut_borrow(r)
            }
            fn shared_unborrow(&self, h: ::wiggle::BorrowHandle) {
                self.bc.shared_unborrow(h)
            }
            fn mut_unborrow(&self, h: ::wiggle::BorrowHandle) {
                self.bc.mut_unborrow(h)
            }
        }
        #(#ctor_externs)*
    }
}
