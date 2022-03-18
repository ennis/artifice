use crate::CRATE;
use darling::{
    util::{Flag, SpannedValue},
    FromDeriveInput, FromField, FromMeta,
};
use proc_macro::{Diagnostic, Level};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::ops::Deref;
use syn::{spanned::Spanned, TraitItem};

#[derive(FromDeriveInput, Debug)]
#[darling(forward_attrs(allow, doc, cfg, repr))]
struct PipelineInterfaceStruct {
    ident: syn::Ident,
    generics: syn::Generics,
    vis: syn::Visibility,
    attrs: Vec<syn::Attribute>,
}

/*
#[derive(Debug, FromField)]
#[darling(attributes(attachment))]
struct AttachmentAttr {
    format: syn::Ident,
    #[darling(default)]
    load_op: Option<syn::Ident>,
    #[darling(default)]
    store_op: Option<syn::Ident>,
    layout: syn::Ident,
    #[darling(default)]
    samples: SpannedValue<Option<u32>>,
    #[darling(default)]
    color: SpannedValue<Flag>,
    #[darling(default)]
    depth: SpannedValue<Flag>,
    ty: syn::Type,
}*/

pub fn generate_pipeline_interface(derive_input: &syn::DeriveInput, fields: &FieldList) -> TokenStream {
    // parse the trait item
    let item_trait = syn::parse_macro_input!(item as syn::ItemTrait);

    // ensure that the trait has a `draw` and `new` method.

    let mut draw_method = item_trait.items.iter_mut().find(|item| match item {
        TraitItem::Const(_) => {}
        TraitItem::Method(_) => {}
        TraitItem::Type(_) => {}
        _ => {}
    })

    q
}
