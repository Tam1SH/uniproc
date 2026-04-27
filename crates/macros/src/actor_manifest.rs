use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{
    Attribute, Fields, FieldsNamed, FieldsUnnamed, Ident, ImplItem, ItemImpl, ItemStruct, Token,
    Type, Visibility,
};

enum ManifestItem {
    New(ItemStruct),
    Existing(Type),
}

struct ParsedItem {
    attrs: Vec<Attribute>,
    kind: ManifestItem,
}

impl ParsedItem {
    fn parse_with_context(input: ParseStream, force_new: bool) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;

        if input.peek(Token![@]) {
            let _: Token![@] = input.parse()?;
            let ty: Type = input.parse()?;
            return Ok(ParsedItem {
                attrs,
                kind: ManifestItem::Existing(ty),
            });
        }

        if input.peek(Token![struct]) || (input.peek(Token![pub]) && input.peek2(Token![struct])) {
            let mut s: ItemStruct = input.parse()?;
            s.attrs = Vec::new();
            if let Fields::Unit = s.fields {
                s.semi_token = Some(Default::default());
            }
            return Ok(ParsedItem {
                attrs,
                kind: ManifestItem::New(s),
            });
        }

        let _: Visibility = input.parse().unwrap_or(Visibility::Inherited);

        if input.peek(Ident) && (input.peek2(syn::token::Paren) || input.peek2(syn::token::Brace)) {
            let ident: Ident = input.parse()?;
            if input.peek(syn::token::Paren) {
                let mut fields: FieldsUnnamed = input.parse()?;
                for f in &mut fields.unnamed {
                    f.vis = syn::parse_quote!(pub);
                }
                Ok(ParsedItem {
                    attrs,
                    kind: ManifestItem::New(create_struct(ident, Fields::Unnamed(fields))),
                })
            } else {
                let mut fields: FieldsNamed = input.parse()?;
                for f in &mut fields.named {
                    f.vis = syn::parse_quote!(pub);
                }
                Ok(ParsedItem {
                    attrs,
                    kind: ManifestItem::New(create_struct(ident, Fields::Named(fields))),
                })
            }
        } else {
            let ty: Type = input.parse()?;
            let maybe_ident = if let Type::Path(ref p) = ty {
                if p.qself.is_none() && p.path.segments.len() == 1 {
                    let seg = &p.path.segments[0];
                    if let syn::PathArguments::None = seg.arguments {
                        Some(seg.ident.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            if force_new {
                if let Some(ident) = maybe_ident {
                    Ok(ParsedItem {
                        attrs,
                        kind: ManifestItem::New(create_struct(ident, Fields::Unit)),
                    })
                } else {
                    Ok(ParsedItem {
                        attrs,
                        kind: ManifestItem::Existing(ty),
                    })
                }
            } else {
                Ok(ParsedItem {
                    attrs,
                    kind: ManifestItem::Existing(ty),
                })
            }
        }
    }
}

fn create_struct(ident: Ident, fields: Fields) -> ItemStruct {
    let semi = if let Fields::Named(_) = fields {
        None
    } else {
        Some(Default::default())
    };
    ItemStruct {
        attrs: Vec::new(),
        vis: syn::parse_quote!(pub),
        struct_token: Default::default(),
        ident,
        generics: Default::default(),
        fields,
        semi_token: semi,
    }
}

struct TransformResult {
    marker_type: Type,
    generated_structs: Vec<TokenStream>,
    logic_calls: Vec<TokenStream>,
}

fn transform_manifest(
    ty: &mut Type,
    mac_name: &str,
    force_new: bool,
    self_ty: &Type,
    marker_ident: Ident,
    is_bus: bool,
) -> Option<TransformResult> {
    let mut generated_structs = Vec::new();
    let mut logic_calls = Vec::new();

    let tokens = match ty {
        Type::Macro(m) if m.mac.path.is_ident(mac_name) => m.mac.tokens.clone(),
        Type::Path(p) => {
            for seg in &mut p.path.segments {
                if let syn::PathArguments::AngleBracketed(args) = &mut seg.arguments {
                    for arg in &mut args.args {
                        if let syn::GenericArgument::Type(inner) = arg {
                            if let Some(res) = transform_manifest(
                                inner,
                                mac_name,
                                force_new,
                                self_ty,
                                marker_ident.clone(),
                                is_bus,
                            ) {
                                return Some(res);
                            }
                        }
                    }
                }
            }
            return None;
        }
        _ => return None,
    };

    let items = (move |input: ParseStream| {
        let mut punctuated = Punctuated::new();
        while !input.is_empty() {
            let item = ParsedItem::parse_with_context(input, force_new)?;
            punctuated.push_value(item);
            if input.is_empty() {
                break;
            }
            let punct: Token![,] = input.parse()?;
            punctuated.push_punct(punct);
        }
        Ok(punctuated)
    })
    .parse2(tokens)
    .expect("Failed to parse manifest macro");

    for it in items {
        let attrs = it.attrs;
        let item_ty = match it.kind {
            ManifestItem::New(s) => {
                let id = &s.ident;
                generated_structs.push(quote! {
                    #(#attrs)* #[derive(Debug, Clone)] #s
                    #(#attrs)* impl app_core::actor::traits::Message for #id {}
                });
                quote!(#id)
            }
            ManifestItem::Existing(t) => quote!(#t),
        };

        if is_bus {
            logic_calls.push(quote! {
                #(#attrs)*
                <#item_ty as app_core::actor::event_bus::builder::EventSubscription<#self_ty>>::subscribe_into(addr.clone(), tracker);
            });
        } else {
            logic_calls.push(quote! {
                #(#attrs)*
                assert_handler::<#self_ty, #item_ty>();
            });
        }
    }

    *ty = syn::parse_quote!(#marker_ident);

    Some(TransformResult {
        marker_type: syn::parse_quote!(#marker_ident),
        generated_structs,
        logic_calls,
    })
}

pub fn actor_manifest_impl(
    _attr: proc_macro::TokenStream,
    mut impl_block: ItemImpl,
) -> TokenStream {
    let self_ty = &impl_block.self_ty;
    let (impl_generics, _, where_clause) = impl_block.generics.split_for_impl();

    let base_name = quote!(#self_ty)
        .to_string()
        .replace(" ", "")
        .replace("<", "_")
        .replace(">", "_")
        .replace("::", "_");

    let bus_marker_id = format_ident!("__Bus_{}", base_name);
    let handlers_marker_id = format_ident!("__Handlers_{}", base_name);

    let mut all_structs = Vec::new();
    let mut bus_logic = quote! {};
    let mut handlers_logic = quote! {};

    for item in &mut impl_block.items {
        if let ImplItem::Type(ty_item) = item {
            if let Some(res) = transform_manifest(
                &mut ty_item.ty,
                "bus",
                false,
                self_ty,
                bus_marker_id.clone(),
                true,
            ) {
                all_structs.extend(res.generated_structs);
                let calls = res.logic_calls;
                bus_logic = quote! {
                    #[doc(hidden)] pub struct #bus_marker_id;
                    impl #impl_generics app_core::actor::event_bus::builder::EventSubscription<#self_ty> for #bus_marker_id #where_clause {
                        fn subscribe_into(addr: app_core::actor::Addr<#self_ty>, tracker: &impl app_core::lifecycle_tracker::LifecycleTracker) {
                            #(#calls)*
                        }
                    }
                };
            }

            if let Some(res) = transform_manifest(
                &mut ty_item.ty,
                "handlers",
                true,
                self_ty,
                handlers_marker_id.clone(),
                false,
            ) {
                all_structs.extend(res.generated_structs);
                let checks = res.logic_calls;
                handlers_logic = quote! {
                    #[doc(hidden)] pub struct #handlers_marker_id;
                    impl #impl_generics app_core::actor::DirectHandler<#self_ty> for #handlers_marker_id #where_clause {}
                    const _: () = {
                        fn check_handlers #impl_generics () #where_clause {
                            fn assert_handler<A, M>() where
                                A: app_core::actor::Handler<M>,
                                M: app_core::actor::Message,
                            {}
                            #(#checks)*
                        }
                    };
                };
            }
        }
    }

    quote! {
        #(#all_structs)*
        #bus_logic
        #handlers_logic
        #impl_block
    }
    .into()
}
