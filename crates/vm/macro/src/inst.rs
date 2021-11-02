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
