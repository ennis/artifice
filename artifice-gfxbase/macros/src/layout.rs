use crate::G;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

/// Checks that the derive input has a repr(C) attribute.
fn has_repr_c_attr(ast: &syn::DeriveInput) -> bool {
    ast.attrs.iter().any(|attr| match attr.parse_meta() {
        Ok(meta) => match meta {
            syn::Meta::List(list) => {
                (list
                    .path
                    .get_ident()
                    .map_or(false, |i| i.to_string() == "repr"))
                    && list.nested.iter().next().map_or(false, |n| match n {
                        syn::NestedMeta::Meta(syn::Meta::Path(ref path)) => {
                            path.get_ident().map_or(false, |i| i.to_string() == "C")
                        }
                        _ => false,
                    })
            }
            _ => false,
        },
        Err(_) => false,
    })
}

/// See [generate_struct_layout]
struct StructLayout {
    offsets: Vec<syn::ItemConst>,
    sizes: Vec<syn::ItemConst>,
}

/// Utility function to generate a set of constant items containing the offsets and sizes of each
/// field of a repr(C) struct.
fn generate_struct_layout(fields: &syn::Fields) -> StructLayout {
    let fields = match *fields {
        syn::Fields::Named(ref fields_named) => &fields_named.named,
        syn::Fields::Unnamed(ref fields_unnamed) => &fields_unnamed.unnamed,
        syn::Fields::Unit => panic!("cannot generate struct layout of unit structs"),
    };

    let mut offsets = Vec::new();
    let mut sizes = Vec::new();
    let mut offset_idents = Vec::new();
    let mut size_idents = Vec::new();

    for (i, f) in fields.iter().enumerate() {
        let field_ty = &f.ty;

        // field offset item
        if i == 0 {
            offsets.push(syn::parse_quote! { pub const OFFSET_0: usize = 0; });
            sizes.push(
                syn::parse_quote! { pub const SIZE_0: usize = ::std::mem::size_of::<#field_ty>(); },
            );
            offset_idents.push(Ident::new("OFFSET_0", Span::call_site()));
            size_idents.push(Ident::new("SIZE_0", Span::call_site()));
        } else {
            let offset0 = &offset_idents[i - 1];
            let offset1 = Ident::new(&format!("OFFSET_{}", i), Span::call_site());
            let size0 = &size_idents[i - 1];
            let size1 = Ident::new(&format!("SIZE_{}", i), Span::call_site());

            offsets.push(syn::parse_quote! {
                pub const #offset1: usize =
                    (#offset0+#size0)
                    + (::std::mem::align_of::<#field_ty>() -
                            (#offset0+#size0)
                                % ::std::mem::align_of::<#field_ty>())
                      % ::std::mem::align_of::<#field_ty>();
            });
            sizes.push(syn::parse_quote! {
                 pub const #size1: usize = ::std::mem::size_of::<#field_ty>();
            });

            offset_idents.push(offset1);
            size_idents.push(size1);
        };
    }

    StructLayout { offsets, sizes }
}

pub fn generate_structured_buffer_data(
    ast: &syn::DeriveInput,
    fields: &syn::Fields,
) -> TokenStream {
    if !has_repr_c_attr(ast) {
        panic!("derive(StructuredBufferData) can only be used on repr(C) structs");
    }

    let struct_name = &ast.ident;
    let privmod = syn::Ident::new(
        &format!("__StructuredBufferData_{}", struct_name),
        Span::call_site(),
    );

    let layout = generate_struct_layout(fields);

    let fields = match *fields {
        syn::Fields::Named(ref fields_named) => &fields_named.named,
        syn::Fields::Unnamed(ref fields_unnamed) => &fields_unnamed.unnamed,
        syn::Fields::Unit => panic!("cannot generate struct layout of unit structs"),
    };

    let mut field_tys = Vec::new();
    let mut layouts = Vec::new();
    let mut offsets = Vec::new();

    for (i, f) in fields.iter().enumerate() {
        let field_ty = &f.ty;
        let offset = &layout.offsets[i].ident;

        // skip padding fields (with an underscore)
        if f.ident.as_ref().unwrap().to_string().starts_with('_') {
            continue;
        }

        field_tys.push(quote! { <#field_ty as #G::buffer::StructuredBufferData>::TYPE });

        offsets.push(quote! { #privmod::#offset });

        layouts.push(quote! { <#field_ty as #G::buffer::StructuredBufferData>::LAYOUT });
    }

    let offset_consts = &layout.offsets;
    let size_consts = &layout.sizes;

    quote! {
        #[allow(non_snake_case)]
        mod #privmod {
            use super::*;
            #(#offset_consts)*
            #(#size_consts)*
        }

        unsafe impl #G::buffer::StructuredBufferData for #struct_name {
            const TYPE: #G::typedesc::TypeDesc<'static> = #G::typedesc::TypeDesc::Struct {
                fields: &[#(&#field_tys),*],
            };
            const LAYOUT: #G::typedesc::Layout<'static> = #G::typedesc::Layout {
                align: std::mem::align_of::<#struct_name>(),
                size: std::mem::size_of::<#struct_name>(),
                details: #G::typedesc::LayoutDetails::Struct(#G::typedesc::FieldsLayout {
                    offsets: &[#(#offsets),*],
                    layouts: &[#(&#layouts),*]
                })
            };
        }
    }
}

pub fn generate_vertex_data(ast: &syn::DeriveInput, fields: &syn::Fields) -> TokenStream {
    if !has_repr_c_attr(ast) {
        panic!("derive(VertexData) can only be used on repr(C) structs");
    }

    let struct_name = &ast.ident;
    let privmod = syn::Ident::new(&format!("__vertex_data_{}", struct_name), Span::call_site());

    let layout = generate_struct_layout(fields);

    let fields = match *fields {
        syn::Fields::Named(ref fields_named) => &fields_named.named,
        syn::Fields::Unnamed(ref fields_unnamed) => &fields_unnamed.unnamed,
        syn::Fields::Unit => panic!("cannot generate struct layout of unit structs"),
    };

    let mut attribs = Vec::new();

    for (i, f) in fields.iter().enumerate() {
        let field_ty = &f.ty;
        let offset = &layout.offsets[i];
        let offset = &offset.ident;

        attribs.push(quote! {
            #G::vertex::VertexLayoutElement {
                //ty: &<#field_ty as #vertex::VertexAttributeType>::EQUIVALENT_TYPE,
                //location: #i as u32,
                format: <#field_ty as #G::vertex::VertexAttributeType>::FORMAT,
                offset: #privmod::#offset as u32,
                semantic: None  // TODO
            }
        });
    }

    let offsets = &layout.offsets;
    let sizes = &layout.sizes;

    quote! {
        #[allow(non_snake_case)]
        mod #privmod {
            use super::*;
            #(#offsets)*
            #(#sizes)*
        }

        unsafe impl #G::vertex::VertexData for #struct_name {
            const LAYOUT: #G::vertex::VertexLayout<'static> =
                #G::vertex::VertexLayout {
                    elements: &[#(#attribs,)*],
                    stride: ::std::mem::size_of::<#struct_name>()
                };
        }
    }
}
