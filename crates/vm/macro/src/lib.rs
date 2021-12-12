extern crate proc_macro;
mod inst;

use proc_macro::TokenStream;
use syn::DeriveInput;

#[proc_macro_derive(TryFromWasmParserOperator)]
pub fn try_from_wasmparser_operator(args: TokenStream) -> TokenStream {
    inst::try_from_wasmparser_operator(syn::parse_macro_input!(args as DeriveInput))
        .unwrap()
        .into()
}
