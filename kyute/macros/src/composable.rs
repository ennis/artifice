use crate::CRATE;
use proc_macro::{Diagnostic, Level};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
    parse::ParseStream, parse_quote, punctuated::Punctuated, spanned::Spanned, visit_mut::*, Abi,
    AngleBracketedGenericArguments, Arm, AttrStyle, Attribute, BareFnArg, BinOp, Binding, Block,
    BoundLifetimes, ConstParam, Constraint, Data, DataEnum, DataStruct, DataUnion, DeriveInput,
    Expr, ExprArray, ExprAssign, ExprAssignOp, ExprAsync, ExprAwait, ExprBinary, ExprBlock,
    ExprBox, ExprBreak, ExprCall, ExprCast, ExprClosure, ExprContinue, ExprField, ExprForLoop,
    ExprGroup, ExprIf, ExprIndex, ExprLet, ExprLit, ExprLoop, ExprMacro, ExprMatch, ExprMethodCall,
    ExprParen, ExprPath, ExprRange, ExprReference, ExprRepeat, ExprReturn, ExprStruct, ExprTry,
    ExprTryBlock, ExprTuple, ExprType, ExprUnary, ExprUnsafe, ExprWhile, ExprYield, Field,
    FieldPat, FieldValue, Fields, FieldsNamed, FieldsUnnamed, File, FnArg, ForeignItem,
    ForeignItemFn, ForeignItemMacro, ForeignItemStatic, ForeignItemType, GenericArgument,
    GenericMethodArgument, GenericParam, Generics, ImplItem, ImplItemConst, ImplItemMacro,
    ImplItemMethod, ImplItemType, Index, Item, ItemConst, ItemEnum, ItemExternCrate, ItemFn,
    ItemForeignMod, ItemImpl, ItemMacro, ItemMacro2, ItemMod, ItemStatic, ItemStruct, ItemTrait,
    ItemTraitAlias, ItemType, ItemUnion, ItemUse, Label, Lifetime, LifetimeDef, Lit, LitBool,
    LitByte, LitByteStr, LitChar, LitFloat, LitInt, LitStr, Local, Macro, MacroDelimiter, Member,
    Meta, MetaList, MetaNameValue, MethodTurbofish, NestedMeta, ParenthesizedGenericArguments, Pat,
    PatBox, PatIdent, PatLit, PatMacro, PatOr, PatPath, PatRange, PatReference, PatRest, PatSlice,
    PatStruct, PatTuple, PatTupleStruct, PatType, PatWild, Path, PathArguments, PathSegment,
    PredicateEq, PredicateLifetime, PredicateType, QSelf, RangeLimits, Receiver, ReturnType,
    Signature, Stmt, TraitBound, TraitBoundModifier, TraitItem, TraitItemConst, TraitItemMacro,
    TraitItemMethod, TraitItemType, Type, TypeArray, TypeBareFn, TypeGroup, TypeImplTrait,
    TypeInfer, TypeMacro, TypeNever, TypeParam, TypeParamBound, TypeParen, TypePath, TypePtr,
    TypeReference, TypeSlice, TypeTraitObject, TypeTuple, UnOp, UseGlob, UseGroup, UseName,
    UsePath, UseRename, UseTree, Variadic, Variant, VisCrate, VisPublic, VisRestricted, Visibility,
    WhereClause, WherePredicate,
};

struct ComposableArgs {
    cached: bool,
}

impl syn::parse::Parse for ComposableArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let idents = Punctuated::<Ident, syn::Token![,]>::parse_terminated(input)?;

        let mut cached = false;
        for ident in idents {
            if ident == "cached" {
                cached = true;
            } else {
                // TODO warn unrecognized attrib
            }
        }
        Ok(ComposableArgs { cached })
    }
}

pub fn extract_compose_attr(attrs: &mut Vec<Attribute>) -> bool {
    if let Some(pos) = attrs.iter().position(|attr| attr.path.is_ident("compose")) {
        if !attrs[pos].tokens.is_empty() {
            Diagnostic::spanned(
                attrs[pos].tokens.span().unwrap(),
                Level::Warning,
                "unknown tokens on `compose` attribute",
            )
            .emit();
        }
        // remove the attr
        attrs.remove(pos);
        true
    } else {
        false
    }
}

struct ComposeRewriter;

impl VisitMut for ComposeRewriter {
    fn visit_expr_call_mut(&mut self, node: &mut ExprCall) {
        if extract_compose_attr(&mut node.attrs) {
            node.args.insert(0, parse_quote! { __cx });
        }
        visit_expr_call_mut(self, node);
    }

    fn visit_expr_method_call_mut(&mut self, node: &mut ExprMethodCall) {
        if extract_compose_attr(&mut node.attrs) {
            node.args.insert(0, parse_quote! { __cx });
        }
        visit_expr_method_call_mut(self, node);
    }
}

fn rewrite_signature(sig: &mut Signature) {
    let pos = if let Some(FnArg::Receiver(_)) = sig.inputs.first_mut() {
        1
    } else {
        0
    };
    sig.inputs.insert(
        pos,
        parse_quote! {__cx: &mut ::#CRATE::cache::CompositionContext},
    );
}

pub fn generate_composable_context(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut fn_item: syn::ItemFn = syn::parse_macro_input!(item as syn::ItemFn);

    // --- rewrite function signature to add the cx parameter ---
    rewrite_signature(&mut fn_item.sig);

    let vis = &fn_item.vis;
    let attrs = &fn_item.attrs;
    let sig = &fn_item.sig;
    let block = &fn_item.block;
    let altered_fn = quote! {
        #[track_caller]
        #(#attrs)* #vis #sig {
            #block
        }
    };
    altered_fn.into()
}

pub fn generate_composable(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut fn_item: syn::ItemFn = syn::parse_macro_input!(item as syn::ItemFn);
    let attr_args: ComposableArgs = syn::parse_macro_input!(attr as ComposableArgs);
    let vis = &fn_item.vis;
    let attrs = &fn_item.attrs;

    // --- rewrite function signature to add the cx parameter ---
    rewrite_signature(&mut fn_item.sig);

    // --- rewrite all #[compose] function calls in the body ---
    ComposeRewriter.visit_block_mut(&mut fn_item.block);

    let altered_fn = if !attr_args.cached {
        let sig = &fn_item.sig;
        let block = &fn_item.block;
        quote! {
            #[track_caller]
            #(#attrs)* #vis #sig {
                __cx.scoped(0, move |__cx| {
                    #block
                })
            }
        }
    } else {
        // convert fn args to tuple
        let args: Vec<_> = fn_item.sig
            .inputs
            .iter_mut()
            .filter_map(|arg| match arg {
                FnArg::Receiver(r) => {
                    // FIXME, methods could be cached composables, we just need `self` to be any+clone
                    Diagnostic::spanned(r.span().unwrap(), Level::Error, "methods cannot be `composable(cached)` functions: consider using `composable` instead").emit();
                    Some(quote! { self })
                }
                FnArg::Typed(arg) => {
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

        {
            let sig = &fn_item.sig;
            let block = &fn_item.block;
            quote! {
                #[track_caller]
                #(#attrs)* #vis #sig {
                    __cx.memoize((#(#args,)*), move |__cx| {
                        #block
                    })
                }
            }
        }
    };

    eprintln!("{}", altered_fn);
    altered_fn.into()
}
