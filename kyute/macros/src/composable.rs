use crate::CRATE;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use quote::__private::ext::RepToTokensExt;
use syn::{parse::{ParseStream, Parser}, punctuated::Punctuated, spanned::Spanned, FnArg, Path, Pat, PatTuple};

struct ComposableArgs {
    uncached: bool,
}

impl syn::parse::Parse for ComposableArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let idents = Punctuated::<Ident, syn::Token![,]>::parse_terminated(input)?;

        let mut uncached = false;
        for ident in idents {
            if ident == "uncached" {
                uncached = true;
            } else {
                // TODO warn unrecognized attrib
            }
        }
        Ok(ComposableArgs { uncached })
    }
}

pub fn generate_composable(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // works only on trait declarations
    let mut fn_item: syn::ItemFn = syn::parse_macro_input!(item as syn::ItemFn);
    let attr_args: ComposableArgs = syn::parse_macro_input!(attr as ComposableArgs);
    let mut errors = Vec::new();

    let vis = &fn_item.vis;
    let attrs = &fn_item.attrs;
    let orig_block = &fn_item.block;

    // get name of the context arg
    let cx_arg = fn_item.sig.inputs.iter().filter(|arg| matches!(arg, FnArg::Typed(_))).next();
    let cx_pat = if let Some(cx_arg) = cx_arg {
        match cx_arg {
            FnArg::Receiver(_) => { unreachable!() }
            FnArg::Typed(pat) => { (*pat.pat).clone() }
        }
    } else{
        errors.push(syn::Error::new(fn_item.sig.span(), "missing context argument on `composable` function")
            .to_compile_error());
        syn::parse_quote!(())
    };


    let altered_fn = if attr_args.uncached {
        let sig = &fn_item.sig;
        let return_type = &fn_item.sig.output;
        //let debug_name = format!("scope for `{}`", fn_item.sig.ident);
        quote! {
            #[track_caller]
            #(#attrs)* #vis #sig {
                #(#errors)*
                let #cx_pat : ::#CRATE::cache::UiCtx = #cx_pat; // type check
                ::#CRATE::cache::scoped(#cx_pat, 0, move |#cx_pat| #return_type {
                    #orig_block
                })
            }
        }
    } else {
        // convert fn args to tuple
        let mut first_non_receiver_arg = false;
        let args: Vec<_> = fn_item.sig
            .inputs
            .iter_mut()
            .filter_map(|arg| match arg {
                FnArg::Receiver(r) => {
                    // FIXME, methods could be cached composables, we just need `self` to be any+clone
                    Some(syn::Error::new(r.span(), "methods cannot be cached `composable` functions: consider using `composable(uncached)`")
                        .to_compile_error())
                }
                FnArg::Typed(arg) => {
                    // skip Cx argument
                    if !first_non_receiver_arg {
                        first_non_receiver_arg = true;
                        return None
                    }
                    if let Some(pos) = arg.attrs.iter().position(|attr| attr.path.is_ident("uncached")) {
                        // skip uncached argument
                        arg.attrs.remove(pos);
                        return None
                    }
                    let pat = &arg.pat;
                    let val = quote! {
                        #pat.clone()
                    };
                    Some(val)
                },
            })
            .collect();


        let sig = &fn_item.sig;
        let return_type = &fn_item.sig.output;
        //let debug_name = format!("memoization wrapper for `{}`", fn_item.sig.ident);

        quote! {
            #[track_caller]
            #(#attrs)* #vis #sig {
                #(#errors)*
                let #cx_pat : ::#CRATE::cache::UiCtx = #cx_pat; // type check
                ::#CRATE::cache::memoize(#cx_pat, (#(#args,)*), move |#cx_pat| #return_type {
                    #orig_block
                })
            }
        }
    };

    altered_fn.into()
}
