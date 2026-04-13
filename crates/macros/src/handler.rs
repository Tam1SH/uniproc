use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, GenericArgument, GenericParam, ItemFn, PatType, PathArguments, Type, TypeParam};

pub fn generate_standalone_handler(mut item: ItemFn) -> TokenStream {
    let fn_name = &item.sig.ident;
    let has_context = item.sig.inputs.len() >= 3;

    let (existing_window_ident, already_has_window) = find_window_generic(&item);

    let window_ident = existing_window_ident.unwrap_or_else(|| quote::format_ident!("TWindow"));

    if has_context && !already_has_window {
        let window_param: GenericParam = syn::parse_quote!(#window_ident: app_core::app::Window);
        item.sig.generics.params.push(window_param);
    }

    if has_context {
        if let FnArg::Typed(PatType { ty, .. }) = &mut item.sig.inputs[2] {
            modify_context_signature(ty, &window_ident);
        }
    }

    let actor_ty = extract_actor_ty(&item.sig.inputs[0]);
    let msg_ty = extract_msg_ty(&item.sig.inputs[1]);

    let (actor_ty, msg_ty) = match (actor_ty, msg_ty) {
        (Some(a), Some(m)) => (a, m),
        _ => return TokenStream::from(quote!(#item)),
    };

    let mut trait_generics = item.sig.generics.clone();
    if !already_has_window && !has_context {
        let window_param: GenericParam = syn::parse_quote!(#window_ident: app_core::app::Window);
        trait_generics.params.push(window_param);
    }
    let (impl_generics, _, where_clause) = trait_generics.split_for_impl();

    let call_args = if has_context {
        quote! { self, msg, ctx }
    } else {
        quote! { self, msg }
    };

    TokenStream::from(quote! {
        #item

        impl #impl_generics app_core::actor::traits::Handler<#msg_ty, #window_ident> for #actor_ty #where_clause {
            fn handle(&mut self, msg: #msg_ty, ctx: &Context<Self, #window_ident>) {
                #fn_name(#call_args);
            }
        }
    })
}

fn find_window_generic(item: &ItemFn) -> (Option<syn::Ident>, bool) {
    for param in &item.sig.generics.params {
        if let GenericParam::Type(TypeParam { ident, bounds, .. }) = param {
            for bound in bounds {
                if let syn::TypeParamBound::Trait(tb) = bound {
                    if let Some(seg) = tb.path.segments.last() {
                        if seg.ident == "Window" {
                            return (Some(ident.clone()), true);
                        }
                    }
                }
            }
        }
    }
    (None, false)
}

fn modify_context_signature(ty: &mut Type, window_ident: &syn::Ident) {
    if let Type::Reference(tr) = ty {
        modify_context_signature(tr.elem.as_mut(), window_ident);
    } else if let Type::Path(tp) = ty {
        if let Some(last_seg) = tp.path.segments.last_mut() {
            if last_seg.ident == "Context" {
                if let PathArguments::AngleBracketed(args) = &mut last_seg.arguments {
                    if args.args.len() == 1 {
                        let t_arg: GenericArgument = syn::parse_quote!(#window_ident);
                        args.args.push(t_arg);
                    }
                }
            }
        }
    }
}

fn extract_actor_ty(arg: &FnArg) -> Option<&Type> {
    if let FnArg::Typed(PatType { ty, .. }) = arg {
        if let Type::Reference(tr) = ty.as_ref() {
            return Some(tr.elem.as_ref());
        }
    }
    None
}

fn extract_msg_ty(arg: &FnArg) -> Option<&Type> {
    if let FnArg::Typed(PatType { ty, .. }) = arg {
        return Some(ty.as_ref());
    }
    None
}
