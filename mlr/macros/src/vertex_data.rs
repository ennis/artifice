use crate::{
    struct_layout::{ensure_repr_c_derive_input, generate_repr_c_struct_layout},
    CRATE,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{__private::str, spanned::Spanned};
use crate::struct_layout::has_repr_c_attr;

/*pub fn generate_structured_buffer_data(
    derive_input: &syn::DeriveInput,
    fields: &FieldList,
) -> TokenStream {
    if let Err(e) = ensure_repr_c("StructuredBufferData", derive_input) {
        return e;
    }

    let struct_name = &derive_input.ident;
    let field_offsets_sizes = generate_repr_c_struct_layout(derive_input);

    let mut struct_fields = Vec::new();
    let mut layouts = Vec::new();
    let mut offsets = Vec::new();

    for (i, f) in fields.iter().enumerate() {
        let field_ty = &f.ty;
        let offset = &field_offsets_sizes.offsets[i].ident;

        // skip padding fields (with an underscore)
        if f.ident.as_ref().unwrap().to_string().starts_with('_') {
            continue;
        }

        struct_fields.push(quote! {
            #G::typedesc::StructField {
                ty: &<#field_ty as #G::StructuredBufferData>::TYPE,
                .. #G::typedesc::StructField::new()
            }
        });

        offsets.push(quote! { Self::#offset });
        layouts.push(quote! { <#field_ty as #G::StructuredBufferData>::LAYOUT });
    }

    let field_offsets_sizes_impl = field_offsets_sizes.impl_block;

    quote! {
        #field_offsets_sizes_impl

        unsafe impl #G::StructuredBufferData for #struct_name {
            const TYPE: #G::typedesc::TypeDesc<'static> = #G::typedesc::TypeDesc::Struct(
                #G::typedesc::StructType {
                    fields: &[#(#struct_fields),*],
                    .. #G::typedesc::StructType::new()
                }
            );
            const LAYOUT: #G::layout::Layout<'static> = #G::layout::Layout {
                align: std::mem::align_of::<#struct_name>(),
                size: std::mem::size_of::<#struct_name>(),
                inner: #G::layout::InnerLayout::Struct(#G::layout::FieldsLayout {
                    offsets: &[#(#offsets),*],
                    layouts: &[#(&#layouts),*]
                })
            };
        }
    }
}*/

pub fn derive(input: proc_macro::TokenStream) -> TokenStream {
    let derive_input: syn::DeriveInput = match syn::parse(input) {
        Ok(input) => input,
        Err(e) => return e.into_compile_error(),
    };

    // check for struct
    let fields = match derive_input.data {
        syn::Data::Struct(ref struct_data) => &struct_data.fields,
        _ => {
            return syn::Error::new(
                derive_input.span(),
                "`VertexData` can only be derived on structs",
            )
            .into_compile_error();
        }
    };

    // check for `#[repr(C)]`
    let repr_c_check = if !has_repr_c_attr(&derive_input) {
        syn::Error::new(
            derive_input.span(),
            format!("`VertexData` can only be derived on `repr(C)` structs"),
        ).into_compile_error()
    } else {
        quote! {}
    };

    // generate field offset constant items
    let struct_layout = match generate_repr_c_struct_layout(&derive_input, &derive_input.vis) {
        Ok(struct_layout) => struct_layout,
        Err(e) => return e.into_compile_error(),
    };

    // `VertexAttribute { format: <FieldType as VertexAttributeType>::FORMAT, offset: OFFSET_field_index }`
    let attribs = fields.iter().enumerate().map(|(i, f)| {
        let field_ty = &f.ty;
        match f.ident {
            Some(ref ident) => {
                quote! {
                    #CRATE::VertexAttribute {
                        format: <#field_ty as #CRATE::VertexAttributeType>::FORMAT,
                        offset: Self::layout().#ident.offset as u32,
                    }
                }
            }
            None => {
                let index = syn::Index::from(i);
                quote! {
                    #CRATE::VertexAttribute {
                        format: <#field_ty as #CRATE::VertexAttributeType>::FORMAT,
                        offset: Self::layout().#index.offset as u32,
                    }
                }
            }
        }
    });

    let struct_name = &derive_input.ident;
    let (impl_generics, ty_generics, where_clause) = derive_input.generics.split_for_impl();
    let layout_struct = &struct_layout.layout_struct;
    let layout_const_fn = &struct_layout.layout_const_fn;

    quote! {
        #repr_c_check
        #layout_struct
        impl #impl_generics #struct_name #ty_generics #where_clause {
            #layout_const_fn
        }
        unsafe impl #impl_generics #CRATE::VertexData for #struct_name #ty_generics #where_clause {
            const ATTRIBUTES: &'static [#CRATE::VertexAttribute] = {
                &[#(#attribs,)*]
            };
        }
    }
}
