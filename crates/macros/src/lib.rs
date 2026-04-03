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
    let self_ty = (*item_impl.self_ty).clone();

    for item in &mut item_impl.items {
        if let ImplItem::Fn(method) = item {
            let ui_action = take_ui_action_attr(method);
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

                let handler_wrap = ui_action
                    .as_ref()
                    .map(|spec| build_ui_action_wrapper(method, spec));
                let ui_port_wrap = if ui_action.is_none() {
                    Some(build_ui_port_wrapper(&self_ty, method))
                } else {
                    None
                };
                let block = &method.block;

                method.block = parse_quote! ({
                    let Some(ui) = self.ui.upgrade() else { #return_stmt };
                    #handler_wrap
                    #ui_port_wrap
                    #block
                });
            } else if let Some(spec) = ui_action.as_ref() {
                let handler_wrap = build_ui_action_wrapper(method, spec);
                let block = &method.block;
                method.block = parse_quote! ({
                    #handler_wrap
                    #block
                });
            }
        }
    }

    TokenStream::from(quote!(#item_impl))
}

struct UiActionAttr {
    scope: syn::LitStr,
    target: Option<syn::LitStr>,
}

fn take_ui_action_attr(method: &mut syn::ImplItemFn) -> Option<UiActionAttr> {
    let pos = method
        .attrs
        .iter()
        .position(|attr| attr.path().is_ident("ui_action"))?;
    let attr = method.attrs.remove(pos);
    parse_ui_action_attr(&attr)
}

fn parse_ui_action_attr(attr: &Attribute) -> Option<UiActionAttr> {
    let mut scope = None;
    let mut target = None;

    if let Ok(meta) =
        attr.parse_args_with(syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated)
    {
        for item in meta {
            if let Meta::NameValue(nv) = item
                && let Expr::Lit(lit) = &nv.value
                && let Lit::Str(value) = &lit.lit
            {
                if nv.path.is_ident("scope") {
                    scope = Some(value.clone());
                } else if nv.path.is_ident("target") {
                    target = Some(value.clone());
                }
            }
        }
    }

    scope.map(|scope| UiActionAttr { scope, target })
}

fn build_ui_action_wrapper(
    method: &syn::ImplItemFn,
    spec: &UiActionAttr,
) -> proc_macro2::TokenStream {
    let handler_ident = find_handler_ident(method)
        .unwrap_or_else(|| panic!("ui_action requires a handler parameter"));
    let scope = &spec.scope;
    let target_fields = spec
        .target
        .as_ref()
        .map(|value| quote! { Some(#value) })
        .unwrap_or_else(|| quote! { None });
    let arity = spec
        .target
        .as_ref()
        .map(|value| value.value().split(',').filter(|part| !part.trim().is_empty()).count())
        .unwrap_or(0);

    match arity {
        0 => quote! {
            let handler = {
                let handler = #handler_ident;
                move || {
                    app_core::trace::in_ui_action_scope(#scope, #target_fields, None, || handler())
                }
            };
        },
        1 => quote! {
            let handler = {
                let handler = #handler_ident;
                move |__ui_arg0| {
                    let __ui_target = app_core::trace::format_ui_target_1(&__ui_arg0);
                    app_core::trace::in_ui_action_scope(
                        #scope,
                        #target_fields,
                        __ui_target,
                        || handler(__ui_arg0),
                    )
                }
            };
        },
        2 => quote! {
            let handler = {
                let handler = #handler_ident;
                move |__ui_arg0, __ui_arg1| {
                    let __ui_target = app_core::trace::format_ui_target_2(&__ui_arg0, &__ui_arg1);
                    app_core::trace::in_ui_action_scope(
                        #scope,
                        #target_fields,
                        __ui_target,
                        || handler(__ui_arg0, __ui_arg1),
                    )
                }
            };
        },
        _ => panic!("ui_action currently supports handlers with up to 2 arguments"),
    }
}

fn build_ui_port_wrapper(
    self_ty: &syn::Type,
    method: &syn::ImplItemFn,
) -> proc_macro2::TokenStream {
    let method_name = method.sig.ident.to_string();
    let adapter_name = quote! { stringify!(#self_ty) };

    quote! {
        if app_core::trace::is_scope_enabled("ui.adapter.call") {
            let __ui_port_target_value = format!("{}::{}", #adapter_name, #method_name);
            let __ui_port_scope_target = Some(__ui_port_target_value.clone());
            let __ui_port_call = || {
                tracing::debug!(
                    adapter = #adapter_name,
                    method = #method_name,
                    "ui.adapter.call"
                );
            };
            if app_core::trace::is_target_enabled(&__ui_port_target_value) {
                app_core::trace::in_named_scope(
                    "ui.adapter.call",
                    Some("adapter,method"),
                    __ui_port_scope_target,
                    __ui_port_call,
                );
            }
        }
    }
}

fn find_handler_ident(method: &syn::ImplItemFn) -> Option<Ident> {
    for arg in &method.sig.inputs {
        if let FnArg::Typed(pat_type) = arg
            && let Pat::Ident(pat_ident) = pat_type.pat.as_ref()
            && pat_ident.ident == "handler"
        {
            return Some(pat_ident.ident.clone());
        }
    }

    None
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
