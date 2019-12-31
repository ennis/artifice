//! Derive macro for the `Data` trait and lenses
#![recursion_limit = "256"]
#![feature(proc_macro_diagnostic)]
extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use quote::TokenStreamExt;
use syn::parse_macro_input;

mod data;

//--------------------------------------------------------------------------------------------------
struct CrateName;
const CRATE: CrateName = CrateName;

impl ToTokens for CrateName {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.append(syn::Ident::new("veda", Span::call_site()))
    }
}

//--------------------------------------------------------------------------------------------------
#[proc_macro_derive(Data, attributes(argument))]
pub fn data_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    data::derive(&input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
