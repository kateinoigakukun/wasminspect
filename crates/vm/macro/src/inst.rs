use anyhow::{anyhow, Result};
use quote::quote;
use syn::{Data, DeriveInput, Variant};

pub fn try_from_wasmparser_operator(ast: DeriveInput) -> Result<proc_macro2::TokenStream> {
    let variants = match &ast.data {
        Data::Enum(v) => &v.variants,
        _ => return Err(anyhow!("unexpected non enum type")),
    };
    let name = &ast.ident;
    let translate_arms = variants.into_iter().map(|v| build_translate_arm(name, v));

    Ok(quote! {
        impl TryFrom<wasmparser::Operator<'_>> for #name {
            type Error = wasmparser::BinaryReaderError;
            fn try_from(op: wasmparser::Operator<'_>) -> Result<Self, Self::Error> {
                Ok(match op {
                    #(#translate_arms),*
                })
            }
        }
    })
}

pub fn define_instr_kind(ast: proc_macro2::TokenStream) -> Result<proc_macro2::TokenStream> {
    // Accept ($( @$proposal:ident $op:ident $({ $($arg:ident: $argty:ty),* })? => $visit:ident)*)

    let mut tokens = proc_macro2::TokenStream::new();
    let mut iter = ast.into_iter();

    loop {
        let at = match iter.next() {
            Some(t) => t,
            None => break,
        };
        assert_eq!(at.to_string(), "@");

        let _proposal = iter.next().expect("unexpected end of input");

        let op = iter.next().expect("unexpected end of input");
        let op = match op {
            proc_macro2::TokenTree::Ident(i) => i,
            _ => panic!("unexpected token: {}", op),
        };

        let mut payload = None;
        if let Some(proc_macro2::TokenTree::Group(g)) = iter.clone().next() {
            iter.next();
            payload = Some(g.stream());
        }

        assert_eq!(
            iter.next().expect("unexpected end of input").to_string(),
            "="
        );
        assert_eq!(
            iter.next().expect("unexpected end of input").to_string(),
            ">"
        );
        iter.next().expect("unexpected end of input");

        tokens.extend(build_instr_kind_case(op, payload));
    }

    Ok(quote! {
        #[derive(Debug, Clone, TryFromWasmParserOperator)]
        pub enum InstructionKind {
            #tokens
        }
    })
}

fn build_instr_kind_case(
    op: proc_macro2::Ident,
    payload: Option<proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    if let Some(payload) = payload {
        // BrTable is a special case because it has lifetime in its payload
        if op == "BrTable" {
            return quote! {
                #op {
                    targets: BrTableData
                },
            };
        }
        quote! {
            #op { #payload },
        }
    } else {
        quote! {
            #op,
        }
    }
}

fn build_translate_arm(
    enum_name: &proc_macro2::Ident,
    variant: &Variant,
) -> proc_macro2::TokenStream {
    let variant_name = variant.ident.clone();
    match &variant.fields {
        syn::Fields::Named(fields) => {
            let fields = fields
                .named
                .iter()
                .filter_map(|f| f.ident.as_ref())
                .collect::<Vec<_>>();
            let fields_and_values = fields.iter().map(|field| {
                quote! {
                    #field: WasmInstPayloadFrom::from_payload(#field)?
                }
            });
            quote! {
                wasmparser::Operator::#variant_name { #(#fields),* } => #enum_name::#variant_name { #(#fields_and_values),* }
            }
        }
        syn::Fields::Unnamed(fields) => {
            let fields = fields
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, _)| proc_macro2::Ident::new(&format!("field{}", i), variant.ident.span()))
                .collect::<Vec<_>>();
            quote! {
                wasmparser::Operator::#variant_name ( #(#fields),* ) => #enum_name::#variant_name
            }
        }
        syn::Fields::Unit => {
            quote! {
                wasmparser::Operator::#variant_name => #enum_name::#variant_name
            }
        }
    }
}
