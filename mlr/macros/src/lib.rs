//! Proc-macro for auto-deriving shader interfaces:
//! - `BufferLayout`
//! - `VertexLayout`
//! - `VertexInputInterface`
//! - `DescriptorSetInterface`
//! - `PushConstantInterface`
//! - `FragmentOutputInterface`
#![recursion_limit = "256"]
#![feature(proc_macro_diagnostic)]

extern crate darling;
extern crate proc_macro;
extern crate quote;
extern crate syn;

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, TokenStreamExt};
use syn::spanned::Spanned;

//--------------------------------------------------------------------------------------------------
struct CrateName;
const CRATE: CrateName = CrateName;

impl ToTokens for CrateName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(syn::Ident::new("mlr", Span::call_site()))
    }
}

type FieldList = syn::punctuated::Punctuated<syn::Field, syn::Token![,]>;

//--------------------------------------------------------------------------------------------------
mod vertex_data;
mod descriptor_set_interface;
//mod fragment_output_interface;
mod struct_layout;
//mod vertex_input_interface;

#[proc_macro_derive(ShaderArguments, attributes(argument))]
pub fn shader_arguments_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    descriptor_set_interface::derive(input).unwrap_or_else(|e| e.into_compile_error()).into()
}

#[proc_macro_derive(VertexData)]
pub fn vertex_data_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    vertex_data::derive(input).unwrap_or_else(|e| e.into_compile_error()).into()
}

#[proc_macro_derive(StructLayout)]
pub fn struct_layout_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    struct_layout::derive(input).unwrap_or_else(|e| e.into_compile_error()).into()
}

/*#[proc_macro_derive(VertexInputInterface, attributes(layout))]
pub fn vertex_input_interface_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_struct(
        "VertexInputInterface",
        input,
        vertex_input_interface::generate,
    )
}

#[proc_macro_derive(FragmentOutputInterface, attributes(attachment))]
pub fn fragment_output_interface_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_struct(
        "FragmentOutputInterface",
        input,
        fragment_output_interface::generate,
    )
}*/

/*
#[proc_macro_derive(PipelineInterface, attributes(descriptor_set))]
pub fn pipeline_interface_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_struct(
        "PipelineInterface",
        input,
        pipeline_interface::generate,
    )
}
*/

/*#[proc_macro_derive(StructuredBufferData)]
pub fn structured_buffer_data_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_struct(
        "StructuredBufferData",
        input,
        vertex_data::generate_structured_buffer_data,
    )
}*/

/*
#[proc_macro_derive(VertexData)]
pub fn vertex_data_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).expect("Couldn't parse item");

    let result = match ast.data {
        syn::Data::Struct(ref s) => layout::generate_vertex_data(&ast, &s.fields),
        _ => panic!("BufferLayout trait can only be automatically derived on structs."),
    };

    result.into()
}

#[proc_macro_derive(Arguments, attributes(argument))]
pub fn arguments_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).expect("Couldn't parse item");

    let result = match ast.data {
        syn::Data::Struct(ref s) => arguments::generate(&ast, &s.fields),
        _ => panic!("PipelineInterface trait can only be derived on structs"),
    };

    result.into()
}
*/
