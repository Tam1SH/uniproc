use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Item, ItemImpl, ItemStruct};

pub fn window_feature_impl(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);

    match item {
        Item::Struct(item_struct) => process_struct(item_struct),
        Item::Impl(item_impl) => process_impl(item_impl),
        _ => syn::Error::new_spanned(
            item,
            "#[window_feature] can only be applied to structs or impl blocks",
        )
        .to_compile_error()
        .into(),
    }
}

fn process_struct(item_struct: ItemStruct) -> TokenStream {
    let vis = &item_struct.vis;
    let ident = &item_struct.ident;

    let expanded = quote! {
        #[derive(Clone)]
        #vis struct #ident<F: Clone> {
            make_port: F,
            tracker: framework::lifecycle_tracker::FeatureLifecycle,
        }

        impl<F: Clone> #ident<F> {
            pub fn new(make_port: F) -> Self {
                Self {
                    make_port,
                    tracker: framework::lifecycle_tracker::FeatureLifecycle::new(),
                }
            }
        }
    };

    expanded.into()
}

fn process_impl(mut item_impl: ItemImpl) -> TokenStream {
    if let Some((_, trait_path, _)) = &item_impl.trait_ {
        let has_uninstall = item_impl.items.iter().any(|item| {
            if let syn::ImplItem::Fn(method) = item {
                method.sig.ident == "uninstall"
            } else {
                false
            }
        });

        if !has_uninstall {
            let last_segment = trait_path.segments.last().unwrap();
            let window_ty =
                if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments {
                    if let Some(syn::GenericArgument::Type(ty)) = args.args.first() {
                        quote! { #ty }
                    } else {
                        quote! { TWindow }
                    }
                } else {
                    quote! { TWindow }
                };

            let uninstall_method: syn::ImplItemFn = syn::parse_quote! {
                fn uninstall(self: Box<Self>, ui: &#window_ty) -> anyhow::Result<()> {
                    let token = ui.new_token();
                    let tracker = self.tracker.clone();

                    drop(self);

                    tracker.shutdown(&token);
                    Ok(())
                }
            };

            item_impl.items.push(syn::ImplItem::Fn(uninstall_method));
        }
    }

    quote! { #item_impl }.into()
}
