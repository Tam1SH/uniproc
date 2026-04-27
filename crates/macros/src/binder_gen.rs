use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemTrait, PathArguments, TraitItem, Type, TypeParamBound, WherePredicate};

pub fn generate_binder(trait_item: &ItemTrait) -> TokenStream {
    let trait_ident = &trait_item.ident;
    let binder_name = format_ident!(
        "{}",
        trait_ident
            .to_string()
            .replace("Ui", "")
            .replace("Bindings", "Binder")
    );

    let methods = trait_item.items.iter().filter_map(|item| {
        if let TraitItem::Fn(m) = item {
            Some(generate_method(m, trait_ident))
        } else {
            None
        }
    });

    quote! {
        pub struct #binder_name<'p, A: 'static, P> {
            inner: app_core::actor::binder::UiBinder<'p, A, P>,
        }

        impl<'p, A: 'static, P: #trait_ident> #binder_name<'p, A, P> {
            pub fn new(addr: &app_core::actor::Addr<A>, port: &'p P) -> Self {
                Self { inner: app_core::actor::binder::UiBinder::new(addr, port) }
            }

            #(#methods)*

            pub fn raw(self, f: impl FnOnce(&app_core::actor::Addr<A>, &P)) -> Self {
                Self { inner: self.inner.raw(f) }
            }
        }
    }
}

fn generate_method(method: &syn::TraitItemFn, _trait_ident: &syn::Ident) -> TokenStream {
    let method_ident = &method.sig.ident;
    let (arity, types) = extract_handler_types(method);

    match arity {
        0 => quote! {
            pub fn #method_ident<M>(self, msg: M) -> Self
            where M: app_core::actor::Message + Clone, A: app_core::actor::Handler<M> {
                Self { inner: self.inner.on0(|p, f| p.#method_ident(f), msg) }
            }
        },
        1 => {
            let ty = &types[0];
            quote! {
                pub fn #method_ident<M>(self, ctor: impl Fn(#ty) -> M + 'static) -> Self
                where M: app_core::actor::Message, A: app_core::actor::Handler<M> {
                    Self { inner: self.inner.on1(|p, f| p.#method_ident(f), ctor) }
                }
            }
        }
        2 => {
            let ty1 = &types[0];
            let ty2 = &types[1];
            quote! {
                pub fn #method_ident<M>(self, ctor: impl Fn(#ty1, #ty2) -> M + 'static) -> Self
                where M: app_core::actor::Message, A: app_core::actor::Handler<M> {
                    Self { inner: self.inner.on2(|p, f| p.#method_ident(f), ctor) }
                }
            }
        }
        _ => quote! {},
    }
}

fn extract_handler_types(method: &syn::TraitItemFn) -> (usize, Vec<Type>) {
    let mut types = Vec::new();
    let Some(where_clause) = &method.sig.generics.where_clause else {
        return (0, types);
    };

    for predicate in &where_clause.predicates {
        if let WherePredicate::Type(pred) = predicate {
            if let Type::Path(path) = &pred.bounded_ty {
                if !path.path.is_ident("F") {
                    continue;
                }
            }

            for bound in &pred.bounds {
                if let TypeParamBound::Trait(tr) = bound {
                    let segment = tr.path.segments.last().unwrap();
                    if segment.ident == "Fn"
                        || segment.ident == "FnMut"
                        || segment.ident == "FnOnce"
                    {
                        if let PathArguments::Parenthesized(args) = &segment.arguments {
                            for input_ty in &args.inputs {
                                types.push(input_ty.clone());
                            }
                            return (types.len(), types);
                        }
                    }
                }
            }
        }
    }
    (0, types)
}
