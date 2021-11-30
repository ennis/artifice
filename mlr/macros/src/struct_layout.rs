use crate::CRATE;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{spanned::Spanned, Fields, Ident, ItemConst, ItemStruct};

/// Checks that the derive input has a repr(C) attribute.
pub(crate) fn has_repr_c_attr(ast: &syn::DeriveInput) -> bool {
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

pub(crate) fn ensure_repr_c_derive_input(ast: &syn::DeriveInput) -> Result<(), syn::Error> {
    if !has_repr_c_attr(ast) {
        return Err(syn::Error::new(
            ast.span(),
            format!("Cannot derive trait on a non-`repr(C)` struct"),
        ));
    } else {
        Ok(())
    }
}

/// See [generate_repr_c_struct_layout]
pub(crate) struct ReprCStructLayout {
    /// A bunch of constant items representing the offset of each field.
    /// They depend on each other and on the `field_sizes` constants,
    /// so if you use one you must paste all `field_offsets` and `field_sizes` in scope.
    ///
    /// Example:
    /// ```
    /// pub const OFFSET_0: usize = 0;
    /// pub const OFFSET_1: usize = (OFFSET_0 + SIZE_0) + (align_of::<FieldType>() - (OFFSET_0 + SIZE_0) % align_of::<FieldType>());
    /// // etc.
    /// ```
    pub(crate) field_offsets: Vec<syn::ItemConst>,

    /// A bunch of constant items with the size of each field.
    /// They are used in the `field_offsets` constants.
    ///
    /// Example:
    /// ```
    /// pub const SIZE_0: usize = size_of::<Field0Type>;
    /// pub const SIZE_1: usize = size_of::<Field1Type>;
    /// // etc.
    /// ```
    pub(crate) field_sizes: Vec<syn::ItemConst>,
    ///
    pub(crate) layout_struct: syn::ItemStruct,
    /// A constant expression of the same type as `layout_struct`.
    pub(crate) layout_expr: syn::Expr,
}

/// Utility function to generate a set of constant items containing the offsets and sizes of each
/// field of a repr(C) struct.
pub(crate) fn generate_repr_c_struct_layout(
    derive_input: &syn::DeriveInput,
    vis: &syn::Visibility,
) -> Result<ReprCStructLayout, syn::Error> {
    let fields = match derive_input.data {
        syn::Data::Struct(ref s) => &s.fields,
        _ => {
            return Err(syn::Error::new(
                derive_input.span(),
                "Expected a struct item",
            ))
        }
    };

    // --- offset and size const items ---
    // `const OFFSET_<field_index> : usize = ...;`
    // `const SIZE_<field_index> : usize = ...;`
    let (offsets, sizes) = {
        let mut offsets: Vec<ItemConst> = Vec::new();
        let mut sizes: Vec<ItemConst> = Vec::new();
        let mut offset_idents = Vec::new();
        let mut size_idents = Vec::new();

        for (i, f) in fields.iter().enumerate() {
            let field_ty = &f.ty;
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

        (offsets, sizes)
    };

    // --- field layouts ---
    // `FieldLayout { offset: OFFSET_<field_index>, size: SIZE_<field_index> }`
    let field_layouts: Vec<_> = fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let field_ty = &f.ty;
            let field_offset = &offsets[i].ident;

            quote! {
                #CRATE::utils::FieldLayout {
                    offset: #field_offset,
                    size: ::std::mem::size_of::<#field_ty>(),
                }
            }
        })
        .collect();

    // `<vis> struct __MyStruct_StructLayout { <fields...> }`
    let layout_struct_name = Ident::new(
        &format!("__{}_StructLayout", derive_input.ident.to_string()),
        Span::call_site(),
    );
    let layout_struct: ItemStruct = match fields {
        Fields::Named(_) => {
            // named fields: `struct StructLayout { field_name: FieldLayout ... }`
            let layout_fields = fields.iter().map(|f| {
                let name = f.ident.as_ref().unwrap();
                quote! { #name: #CRATE::utils::FieldLayout }
            });
            syn::parse_quote! {
                #[doc(hidden)]
                #vis struct #layout_struct_name {
                    #(#layout_fields,)*
                }
            }
        }
        Fields::Unnamed(_) => {
            // tuple-like: `struct StructLayout(FieldLayout, ...)`
            let types = fields.iter().map(|_| quote! { #CRATE::utils::FieldLayout });
            syn::parse_quote! {
                #[doc(hidden)]
                #vis struct #layout_struct_name(#(#types,)*);
            }
        }
        Fields::Unit => {
            // empty
            syn::parse_quote! {
                #[doc(hidden)]
                #vis struct #layout_struct_name {};
            }
        }
    };

    let layout_struct_init = fields.iter().enumerate().map(|(i, f)| {
        let offset = &offsets[i].ident;
        let size = &sizes[i].ident;
        if let Some(ident) = &f.ident {
            quote! { #ident: #CRATE::utils::FieldLayout { offset: #offset, size: #size } }
        } else {
            let index = syn::Index::from(i);
            quote! { #index: #CRATE::utils::FieldLayout { offset: #offset, size: #size } }
        }
    });

    //let offsets_r = &offsets[..];
    //let sizes_r = &sizes[..];

    let layout_expr = syn::parse_quote! {
        {
            #(#offsets)*
            #(#sizes)*
            #layout_struct_name { #(#layout_struct_init,)* }
        }
    };

    Ok(ReprCStructLayout {
        field_offsets: offsets,
        field_sizes: sizes,
        layout_struct,
        layout_expr,
    })
}

// Not exactly a derive, but adds an inherent impl block with a `LAYOUT` associated constant.
pub fn derive(input: proc_macro::TokenStream) -> Result<TokenStream, syn::Error> {
    let derive_input: syn::DeriveInput = syn::parse(input)?;
    // check for struct
    let fields = match derive_input.data {
        syn::Data::Struct(ref struct_data) => &struct_data.fields,
        _ => {
            return Err(syn::Error::new(
                derive_input.span(),
                "`StructLayout` can only be derived on structs",
            ));
        }
    };

    // check for `#[repr(C)]`
    ensure_repr_c_derive_input(&derive_input)?;

    // generate field offset constant items
    let struct_layout = generate_repr_c_struct_layout(&derive_input, &derive_input.vis)?;

    let struct_name = &derive_input.ident;
    let (impl_generics, ty_generics, where_clause) = derive_input.generics.split_for_impl();
    let layout_struct = &struct_layout.layout_struct;
    let layout_struct_name = &struct_layout.layout_struct.ident;
    let layout_expr = &struct_layout.layout_expr;
    let vis = &derive_input.vis;

    Ok(quote! {
        #layout_struct
        unsafe impl #impl_generics #struct_name #ty_generics #where_clause {
            #vis const LAYOUT: #layout_struct_name = #layout_expr;
        }
    })
}
