//! Mostly stolen from druid-derive
use proc_macro2::{TokenStream, Span};
use quote::quote;
use syn::spanned::Spanned;
use crate::CRATE;

pub fn derive(input: &syn::DeriveInput) -> Result<TokenStream, syn::Error> {
    match &input.data {
        syn::Data::Struct(s) => derive_struct(input, s),
        syn::Data::Enum(e) => Err(syn::Error::new(
            e.enum_token.span(),
            "Lens implementations cannot be derived from enums",
        )),
        syn::Data::Union(u) => Err(syn::Error::new(
            u.union_token.span(),
            "Lens implementations cannot be derived from unions",
        )),
    }
}

fn derive_struct(input: &syn::DeriveInput, s: &syn::DataStruct) -> Result<TokenStream, syn::Error> {

    let ty = &input.ident;
    let vis = &input.vis;
    //let internal_mod_name = syn::Ident::new(&format!("{}_lenses", ty), Span::call_site());

    let fields = match &s.fields {
        syn::Fields::Named(fields_named) => &fields_named.named,
        syn::Fields::Unnamed(fields_unnamed) => &fields_unnamed.unnamed,
        syn::Fields::Unit => return Err(syn::Error::new(input.ident.span(), "Lens implementations cannot be derived from unit structs"))
    };

    let mut decls = Vec::new();
    let mut impls = Vec::new();
    let mut associated_items = Vec::new();

    for (i, f) in fields.iter().enumerate() {
        let name = f.ident.clone().unwrap_or_else(|| syn::Ident::new(&format!("elem_{}", i), Span::call_site()));
        let lens_ty_name = syn::Ident::new(&format!("{}Lens_{}", ty, name), Span::call_site());
        let lty = &f.ty;
        let access  = match &f.ident {
            Some(ident) => {
                quote!{ #ident }
            },
            None => {
                let index = syn::Index::from(i);
                quote!{ #index }
            }
        };

        let decl = quote!{
            #[allow(non_camel_case_types)]
            #[derive(Copy,Clone)]
            #vis struct #lens_ty_name;
        };
        decls.push(decl);

        let lens_impl = quote!{
            impl #CRATE::lens::Lens for #lens_ty_name {
                type Root = #ty;
                type Leaf = #lty;

                fn path(&self) -> #CRATE::lens::Path<#ty, #lty> {
                    #CRATE::lens::Path::field(#i)
                }

                fn get<'a>(&self, data: &'a #ty) -> &'a #lty {
                    &data.#access
                }

                fn get_mut<'a>(&self, data: &'a mut #ty) -> &'a mut #lty {
                    &mut data.#access
                }

                fn try_get<'a>(&self, data: &'a #ty) -> Option<&'a #lty> {
                    Some(self.get(data))
                }

                fn try_get_mut<'a>(&self, data: &'a mut #ty) -> Option<&'a mut #lty> {
                    Some(self.get_mut(data))
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

    let expanded = quote!{
        #(#decls)*
        #(#impls)*

        #[allow(non_upper_case_globals)]
        impl #impl_generics #ty #ty_generics #where_clause {
            #(#associated_items)*
        }
    };

    Ok(expanded)
}
