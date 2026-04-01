use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput};
use syn::{
    parse_quote, Attribute, Data, DataStruct, Expr, Fields, FnArg, ImplItem, ItemImpl, Lit, Meta,
    Pat, ReturnType,
};

#[proc_macro_attribute]
pub fn ui_adapter(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item_impl = parse_macro_input!(input as ItemImpl);

    for item in &mut item_impl.items {
        if let ImplItem::Fn(method) = item {
            let mut ui_arg_idx = None;

            for (i, arg) in method.sig.inputs.iter().enumerate() {
                if let FnArg::Typed(pat_type) = arg {
                    if let Pat::Ident(ref id) = *pat_type.pat {
                        if id.ident == "ui" {
                            ui_arg_idx = Some(i);
                            break;
                        }
                    }
                }
            }

            if let Some(idx) = ui_arg_idx {
                let mut inputs =
                    syn::punctuated::Punctuated::<syn::FnArg, syn::token::Comma>::new();
                for (i, arg) in method.sig.inputs.clone().into_iter().enumerate() {
                    if i != idx {
                        inputs.push(arg);
                    }
                }
                method.sig.inputs = inputs;

                let mut default_value = quote! { Default::default() };

                if let Some(pos) = method
                    .attrs
                    .iter()
                    .position(|attr| attr.path().is_ident("default"))
                {
                    let attr = &method.attrs[pos];
                    if let Ok(val) = attr.parse_args::<syn::Expr>() {
                        default_value = quote! { #val };
                    }
                    method.attrs.remove(pos);
                }

                let sig = &method.sig;
                let return_stmt = match &sig.output {
                    ReturnType::Default => quote! { return },
                    ReturnType::Type(_, _) => quote! { return #default_value },
                };

                let block = &method.block;

                method.block = parse_quote! ({
                    let Some(ui) = self.ui.upgrade() else { #return_stmt };
                    #block
                });
            }
        }
    }

    TokenStream::from(quote!(#item_impl))
}

#[proc_macro_attribute]
pub fn feature_settings(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args with syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated);
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let struct_vis = &input.vis;

    let original_attrs: Vec<Attribute> = input
        .attrs
        .iter()
        .filter(|attr| !attr.path().is_ident("feature_settings"))
        .cloned()
        .collect();

    let prefix = parse_prefix_arg(&args);
    let is_root = prefix.is_some();

    let struct_name_str = struct_name.to_string();
    let wrapper_name = if struct_name_str.ends_with("Settings") {
        struct_name.clone()
    } else {
        format_ident!("{}Settings", struct_name)
    };

    let fields = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("feature_settings can only be used on structs with named fields"),
    };

    let mut settings_entries = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        let setting_attr = field
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("setting"));

        if let Some(attr) = setting_attr {
            let (key, default_expr, is_json, is_nested) = parse_setting_attr(attr, field_name);

            if !is_nested && default_expr.is_none() {
                panic!(
                    "Field '{}' is not nested and must have a default value",
                    field_name
                );
            }

            settings_entries.push((
                field_name,
                key,
                default_expr,
                is_json,
                is_nested,
                field_type,
            ));
        } else {
            panic!("Field {} must have #[setting] attribute", field_name);
        }
    }

    let patch_methods = settings_entries
        .iter()
        .filter_map(|(field_name, _, _, _is_json, is_nested, ty)| {
            if *is_nested {
                return None;
            }

            if is_trivial_type(ty) {
                let set_name = format_ident!("set_{}", field_name);
                Some(quote! {
                    pub fn #set_name(&self, value: #ty) -> anyhow::Result<()> {
                        self.#field_name.set(value)
                    }
                })
            } else {
                let patch_name = format_ident!("patch_{}", field_name);
                Some(quote! {
                    pub fn #patch_name<F>(&self, f: F) -> anyhow::Result<()>
                    where F: FnOnce(&mut std::sync::Arc<#ty>)
                    {
                        let mut data = self.#field_name.get_arc();
                        f(&mut data);
                        let json = serde_json::to_value(data.as_ref())?;
                        self.#field_name.get_store_subscription().settings.set(&self.#field_name.get_path(), json)?;

                        Ok(())
                    }
                })
            }
        })
        .collect::<Vec<_>>();

    let expanded = if is_root {
        let prefix_str = prefix.unwrap();
        generate_root_settings(
            struct_vis,
            &wrapper_name,
            &original_attrs,
            &prefix_str,
            &settings_entries,
            &patch_methods,
        )
    } else {
        generate_nested_settings(
            struct_vis,
            &wrapper_name,
            &original_attrs,
            &settings_entries,
            &patch_methods,
        )
    };

    TokenStream::from(expanded)
}

fn generate_root_settings(
    struct_vis: &syn::Visibility,
    wrapper_name: &Ident,
    original_attrs: &[Attribute],
    prefix_str: &str,
    settings_entries: &[(&Ident, String, Option<Expr>, bool, bool, &syn::Type)],
    patch_methods: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    let struct_fields = settings_entries
        .iter()
        .map(|(field_name, _, _, _, is_nested, ty)| {
            if *is_nested {
                quote! {
                    #field_name: std::sync::Arc<#ty>,
                }
            } else {
                quote! { #field_name: context::settings::ReactiveSetting<#ty>, }
            }
        });

    let init_fields = settings_entries.iter().map(|(field_name, key, default_expr, is_json, is_nested, ty)| {
        if *is_nested {
            quote! {
                #field_name: std::sync::Arc::new(
                    #ty::new::<Self>(&store, #key)?
                ),
            }
        } else {
            let def = default_expr.as_ref().unwrap();
            let default_value = if *is_json {
                quote! { serde_json::from_value::<#ty>(#def).expect(concat!("Failed to deserialize default JSON for setting '", #key, "'")) }
            } else {
                quote! { #def }
            };
            quote! {
                #field_name: context::settings::setting_or::<Self, #ty>(&store, #key, #default_value)?,
            }
        }
    });

    let getters = settings_entries
        .iter()
        .map(|(field_name, _, _, _, is_nested, ty)| {
            if *is_nested {
                quote! {
                    pub fn #field_name(&self) -> std::sync::Arc<#ty> {
                        self.#field_name.clone()
                    }
                }
            } else {
                quote! {
                    pub fn #field_name(&self) -> context::settings::ReactiveSetting<#ty> {
                        self.#field_name.clone()
                    }
                }
            }
        });

    let ensure_calls = settings_entries.iter().map(|(_, key, default_expr, is_json, is_nested, ty)| {
        if *is_nested {
            quote! {
                #ty::ensure_defaults::<Self>(settings, #key)?;
            }
        } else {
            let def = default_expr.as_ref().unwrap();
            let default_value = if *is_json {
                quote! { serde_json::from_value::<#ty>(#def).expect(concat!("Failed to deserialize default JSON for setting '", #key, "'")) }
            } else {
                quote! { #def }
            };
            quote! {
                <Self as context::settings::FeatureSettings>::ensure_default(settings, #key, #default_value)?;
            }
        }
    });

    quote! {
        #[derive(Debug, Clone)]
        #(#original_attrs)*
        #struct_vis struct #wrapper_name {
            store: std::sync::Arc<context::settings::store::SettingsStore>,
            #(#struct_fields)*
        }

        impl context::settings::SettingsScope for #wrapper_name {
            const PREFIX: &'static str = #prefix_str;
        }

        impl context::settings::FeatureSettings for #wrapper_name {
            fn ensure_defaults(settings: &context::settings::store::SettingsStore) -> anyhow::Result<()> {
                #(#ensure_calls)*
                Ok(())
            }
        }

        impl #wrapper_name {
            pub fn new(shared: &app_core::shared_state::SharedState) -> anyhow::Result<Self> {
                let store = context::settings::settings_from(shared);
                <Self as context::settings::FeatureSettings>::ensure_defaults(&store)?;
                Ok(Self {
                    #(#init_fields)*
                    store,
                })
            }

            #(#getters)*
            #(#patch_methods)*

            pub fn store(&self) -> &std::sync::Arc<context::settings::store::SettingsStore> {
                &self.store
            }
        }
    }
}

fn generate_nested_settings(
    struct_vis: &syn::Visibility,
    wrapper_name: &Ident,
    original_attrs: &[Attribute],
    settings_entries: &[(&Ident, String, Option<Expr>, bool, bool, &syn::Type)],
    patch_methods: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    let struct_fields = settings_entries
        .iter()
        .map(|(field_name, _, _, _, is_nested, ty)| {
            if *is_nested {
                quote! {
                    #field_name: std::sync::Arc<#ty>,
                }
            } else {
                quote! {
                    #field_name: std::sync::Arc<context::settings::ReactiveSetting<#ty>>,
                }
            }
        });

    let init_fields = settings_entries.iter().map(|(field_name, key, default_expr, is_json, is_nested, ty)| {
        if *is_nested {
            quote! {
                #field_name: std::sync::Arc::new(
                    #ty::new::<TScope>(store, &format!("{}.{}", namespace, #key))?
                ),
            }
        } else {
            let def = default_expr.as_ref().unwrap();
            let default_value = if *is_json {
                quote! { serde_json::from_value::<#ty>(#def).expect(concat!("Failed to deserialize default JSON for setting '", #key, "'")) }
            } else {
                quote! { #def }
            };
            quote! {
                #field_name: std::sync::Arc::new(
                    context::settings::setting_or::<TScope, #ty>(
                        store,
                        &format!("{}.{}", namespace, #key),
                        #default_value,
                    )?
                ),
            }
        }
    });

    let getters = settings_entries.iter().map(|(field_name, _, _, _, is_nested, ty)| {
        if *is_nested {
            quote! {
                pub fn #field_name(&self) -> std::sync::Arc<#ty> {
                    self.#field_name.clone()
                }
            }
        } else {
            quote! {
                pub fn #field_name(&self) -> std::sync::Arc<context::settings::ReactiveSetting<#ty>> {
                    self.#field_name.clone()
                }
            }
        }
    });

    let ensure_calls = settings_entries.iter().map(|(_, key, default_expr, is_json, is_nested, ty)| {
        if *is_nested {
            quote! {
                #ty::ensure_defaults::<TScope>(settings, &format!("{}.{}", namespace, #key))?;
            }
        } else {
            let def = default_expr.as_ref().unwrap();
            let default_value = if *is_json {
                quote! { serde_json::from_value::<#ty>(#def).expect(concat!("Failed to deserialize default JSON for setting '", #key, "'")) }
            } else {
                quote! { #def }
            };
            quote! {
                TScope::ensure_default(settings, &format!("{}.{}", namespace, #key), #default_value)?;
            }
        }
    });

    quote! {
        #[derive(Debug, Clone)]
        #(#original_attrs)*
        #struct_vis struct #wrapper_name {
            #(#struct_fields)*
        }

        impl #wrapper_name {
            pub fn new<TScope: context::settings::FeatureSettings>(
                store: &std::sync::Arc<context::settings::store::SettingsStore>,
                namespace: &str,
            ) -> anyhow::Result<Self> {
                Ok(Self {
                    #(#init_fields)*
                })
            }

            pub fn ensure_defaults<TScope: context::settings::FeatureSettings>(
                settings: &context::settings::store::SettingsStore,
                namespace: &str,
            ) -> anyhow::Result<()> {
                #(#ensure_calls)*
                Ok(())
            }

            #(#getters)*
            #(#patch_methods)*
        }
    }
}

fn is_trivial_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(tp) = ty {
        if tp.qself.is_none() && tp.path.segments.len() == 1 {
            let ident = tp.path.segments[0].ident.to_string();
            matches!(
                ident.as_str(),
                "u8" | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "f32"
                    | "f64"
                    | "bool"
                    | "String"
                    | "usize"
                    | "isize"
            )
        } else {
            false
        }
    } else {
        false
    }
}

fn parse_prefix_arg(args: &syn::punctuated::Punctuated<Meta, syn::Token![,]>) -> Option<String> {
    for meta in args {
        if let Meta::NameValue(nv) = meta
            && nv.path.is_ident("prefix")
            && let Expr::Lit(lit) = &nv.value
            && let Lit::Str(s) = &lit.lit
        {
            return Some(s.value());
        }
    }
    None
}

fn parse_setting_attr(attr: &Attribute, field_name: &Ident) -> (String, Option<Expr>, bool, bool) {
    let mut explicit_key = None;
    let mut default: Option<Expr> = None;
    let mut is_json = false;
    let mut is_nested = false;

    if let Ok(meta) =
        attr.parse_args_with(syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated)
    {
        for item in meta {
            match item {
                Meta::NameValue(nv) => {
                    if nv.path.is_ident("key") {
                        if let Expr::Lit(lit) = &nv.value
                            && let Lit::Str(s) = &lit.lit
                        {
                            explicit_key = Some(s.value());
                        }
                    } else if nv.path.is_ident("default") {
                        default = Some(nv.value.clone());
                        if let Expr::Macro(mac) = &nv.value
                            && mac.mac.path.segments.last().map(|s| s.ident.to_string())
                                == Some("json".to_string())
                        {
                            is_json = true;
                        }
                    } else if nv.path.is_ident("default_json") {
                        default = Some(nv.value.clone());
                        is_json = true;
                    }
                }
                Meta::Path(p) if p.is_ident("nested") => {
                    is_nested = true;
                }
                _ => {}
            }
        }
    }

    let key = explicit_key.unwrap_or_else(|| field_name.to_string());
    (key, default, is_json, is_nested)
}
