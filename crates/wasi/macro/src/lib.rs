extern crate proc_macro;
mod wasi;
mod utils;

use proc_macro::TokenStream;

#[proc_macro]
pub fn define_wasi_fn_for_wasminspect(args: TokenStream) -> TokenStream {
    wasi::define_wasi_fn_for_wasminspect(args.into()).into()
}
