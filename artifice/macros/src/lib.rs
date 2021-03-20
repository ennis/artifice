#![recursion_limit = "256"]
#![feature(proc_macro_diagnostic)]
extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::spanned::Spanned;

mod data;
mod topic;

//--------------------------------------------------------------------------------------------------
struct CrateName;
const CRATE: CrateName = CrateName;

impl ToTokens for CrateName {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.append(syn::Ident::new("artifice", Span::call_site()))
    }
}

//--------------------------------------------------------------------------------------------------
#[proc_macro_derive(Data, attributes(argument))]
pub fn data_derive(input: TokenStream) -> TokenStream {
    data::derive(input)
}

//--------------------------------------------------------------------------------------------------
#[proc_macro_attribute]
pub fn topic(attr: TokenStream, item: TokenStream) -> TokenStream {
    topic::topic(attr, item)
}
