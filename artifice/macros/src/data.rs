//! Mostly stolen from druid-derive
use crate::CRATE;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;

pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    let result = match &input.data {
        syn::Data::Struct(s) => derive_struct(&input, s),
        syn::Data::Enum(e) => Err(syn::Error::new(
            e.enum_token.span(),
            "Lens implementations cannot be derived from enums",
        )),
        syn::Data::Union(u) => Err(syn::Error::new(
            u.union_token.span(),
            "Lens implementations cannot be derived from unions",
        )),
    };

    result.unwrap_or_else(|err| err.to_compile_error()).into()
}

fn derive_struct(input: &syn::DeriveInput, s: &syn::DataStruct) -> syn::Result<TokenStream> {
    let ty = &input.ident;
    let vis = &input.vis;
    //let internal_mod_name = syn::Ident::new(&format!("{}_lenses", ty), Span::call_site());

    let fields = match &s.fields {
        syn::Fields::Named(fields_named) => &fields_named.named,
        syn::Fields::Unnamed(fields_unnamed) => &fields_unnamed.unnamed,
        syn::Fields::Unit => {
            return Err(syn::Error::new(
                input.ident.span(),
                "`Data` implementations cannot be derived from unit structs",
            ))
        }
    };

    let addr_enum = syn::Ident::new(&format!("DataAddress_{}", ty), Span::call_site());

    let mut decls = Vec::new();
    let mut impls = Vec::new();
    let mut associated_items = Vec::new();
    let mut addr_variants = Vec::new();
    let mut addr_variant_debug_arms = Vec::new();

    for (i, f) in fields.iter().enumerate() {
        let name = f
            .ident
            .clone()
            .unwrap_or_else(|| syn::Ident::new(&format!("elem_{}", i), Span::call_site()));
        let lens_ty_name = syn::Ident::new(&format!("{}Lens_{}", ty, name), Span::call_site());
        let lty = &f.ty;
        let access = match &f.ident {
            Some(ident) => {
                quote! { #ident }
            }
            None => {
                let index = syn::Index::from(i);
                quote! { #index }
            }
        };

        let addr_variant = quote! {
            #name ( Option<<#lty as #CRATE::util::model::Data>::Address> )
        };
        addr_variants.push(addr_variant);

        let addr_variant_debug = quote! {
            #addr_enum::#name ( addr ) => {
                write!(f, stringify!(#name))?;
                if let Some(addr) = addr {
                    write!(f, ".{:?}", addr)?;
                }
            }
        };
        addr_variant_debug_arms.push(addr_variant_debug);

        let decl = quote! {
            #[allow(non_camel_case_types)]
            #[derive(Copy,Clone)]
            #vis struct #lens_ty_name;
        };
        decls.push(decl);

        // that's an awful lot of code for just one field
        let lens_impl = quote! {
            impl #CRATE::util::model::Lens<#ty,#lty> for #lens_ty_name {

                fn with<R, F: FnOnce(&#lty) -> R>(&self, data: &#ty, f: F) -> R {
                    f(&data.#access)
                }

                fn with_mut<R, F: FnOnce(&mut #lty) -> R>(&self, data: &mut #ty, f: F) -> R {
                    f(&mut data.#access)
                }

                fn try_with<R, F: FnOnce(&#lty) -> R>(&self, data: &#ty, f: F) -> Option<R> {
                    Some(f(&data.#access))
                }

                fn try_with_mut<R, F: FnOnce(&mut #lty) -> R>(&self, data: &mut #ty, f: F) -> Option<R> {
                    Some(f(&mut data.#access))
                }

                fn address(&self) -> Option<#addr_enum> {
                    Some(#addr_enum::#name(None))
                }

                fn concat<K, C: Data>(&self, rhs: &K) -> Option<#addr_enum>
                    where
                        K: #CRATE::util::model::Lens<#lty, C>
                        {
                    Some(#addr_enum::#name(rhs.address()))
                }

                fn unprefix(&self, addr: <#ty as #CRATE::util::model::Data>::Address) -> Option<Option<<#lty as #CRATE::util::model::Data>::Address>>
                {
                    if let #addr_enum::#name(rest) = addr {
                        Some(rest)
                    } else {
                        None
                    }
                }
            }
        };
        impls.push(lens_impl);

        let assoc_item = quote! {
            #vis const #name : #lens_ty_name = #lens_ty_name;
        };
        associated_items.push(assoc_item);
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        #(#decls)*
        #(#impls)*

        #[derive(Clone,Eq,PartialEq)]
        pub enum #addr_enum {
            #(#addr_variants),*
        }

        impl std::fmt::Debug for #addr_enum {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match self {
                    #(#addr_variant_debug_arms),*
                }
                Ok(())
            }
        }


        #[allow(non_upper_case_globals)]
        impl #impl_generics #CRATE::util::model::Data for #ty #ty_generics #where_clause {
            type Address = #addr_enum;
        }

        #[allow(non_upper_case_globals)]
        impl #impl_generics #ty #ty_generics #where_clause {
            #(#associated_items)*
        }
    };

    Ok(expanded)
}
