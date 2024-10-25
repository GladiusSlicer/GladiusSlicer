extern crate proc_macro;
use proc_macro::TokenStream;

mod settings;

#[proc_macro_derive(Settings, attributes(Optional, Combine, Recursive, AllowDefault))]
pub fn derive_macro_builder(input: TokenStream) -> TokenStream {
    settings::derive_proc_macro_impl(input)
}
