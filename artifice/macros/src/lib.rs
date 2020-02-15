#![recursion_limit = "256"]
#![feature(proc_macro_diagnostic)]
extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use quote::ToTokens;
use quote::TokenStreamExt;
use syn::spanned::Spanned;

//--------------------------------------------------------------------------------------------------
struct CrateName;
const CRATE: CrateName = CrateName;

impl ToTokens for CrateName {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.append(syn::Ident::new("artifice", Span::call_site()))
    }
}

//--------------------------------------------------------------------------------------------------
#[proc_macro_attribute]
pub fn topic(attr: TokenStream, item: TokenStream) -> TokenStream {
    // works only on trait declarations
    let trait_decl: syn::ItemTrait = syn::parse_macro_input!(item as syn::ItemTrait);
    let topic: syn::Ident = syn::parse_macro_input!(attr as syn::Ident);

    let listener = &trait_decl.ident;
    let visibility = &trait_decl.vis;

    let mut publisher_methods = Vec::new();
    for item in trait_decl.items.iter() {
        if let syn::TraitItem::Method(method) = item {
            let sig = &method.sig;

            if let syn::ReturnType::Type(_, _) = &sig.output {
                let err = syn::Error::new(item.span(), "listener methods cannot have output types")
                    .to_compile_error();
                publisher_methods.push(err);
            } else {
                // Good path
                let method_name = &sig.ident;
                let args: Vec<_> = sig
                    .inputs
                    .iter()
                    .skip(1)
                    .map(|arg| match arg {
                        syn::FnArg::Typed(arg) => &arg.pat,
                        _ => unreachable!(),
                    })
                    .collect();

                let method = quote! {
                    #sig {
                        self.listeners.for_each(|l| {
                            l.borrow_mut().#method_name(#(#args),*)
                        });
                    }
                };
                publisher_methods.push(method);
            }
        } else {
            let err = syn::Error::new(item.span(), "unsupported trait item").to_compile_error();
            publisher_methods.push(err);
        }
    }

    let result = quote! {
        #trait_decl

        #visibility struct #topic {
             listeners: #CRATE::util::TopicListeners<dyn #listener>,
        }

        impl #CRATE::util::Topic for #topic {
            type Listener = dyn #listener;
        }

        impl #topic {
            pub fn listen(bus: &#CRATE::util::MessageBus, listener: std::rc::Rc<std::cell::RefCell<dyn #listener>>) {
                bus.register::<#topic>().add_listener(listener.clone())
            }

            pub fn publisher(bus: &#CRATE::util::MessageBus) -> #topic {
                #topic {
                    listeners: bus.register::<#topic>(),
                }
            }
        }

        impl #listener for #topic {
            #(#publisher_methods)*
        }

    };

    result.into()
}
