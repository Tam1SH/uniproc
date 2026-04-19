use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn, PatType, Type};

pub fn generate_standalone_handler(item: ItemFn) -> TokenStream {
    let fn_name = &item.sig.ident;
    let has_context = item.sig.inputs.len() >= 3;

    let actor_ty = extract_actor_ty(&item.sig.inputs[0]);
    let msg_ty = extract_msg_ty(&item.sig.inputs[1]);

    let (actor_ty, msg_ty) = match (actor_ty, msg_ty) {
        (Some(a), Some(m)) => (a, m),
        _ => return TokenStream::from(quote!(#item)),
    };

    let trait_generics = item.sig.generics.clone();

    let (impl_generics, _, where_clause) = trait_generics.split_for_impl();

    let call_args = if has_context {
        quote! { self, msg, ctx }
    } else {
        quote! { self, msg }
    };

    TokenStream::from(quote! {
        #item

        impl #impl_generics app_core::actor::traits::Handler<#msg_ty> for #actor_ty #where_clause {
            fn handle(&mut self, msg: #msg_ty, ctx: &app_core::actor::traits::Context<Self>) {
                #fn_name(#call_args);
            }
        }
    })
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
