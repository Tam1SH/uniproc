use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Attribute, Data, DataStruct, DeriveInput, Expr, Fields, Ident, Lit, Meta, Visibility};

pub fn feature_settings_impl(
    args: syn::punctuated::Punctuated<Meta, syn::Token![,]>,
    input: DeriveInput,
) -> TokenStream {
    let struct_vis = &input.vis;
    let original_attrs: Vec<Attribute> = input
        .attrs
        .iter()
        .filter(|a| !a.path().is_ident("feature_settings"))
        .cloned()
        .collect();

    let prefix = parse_prefix_arg(&args);
    let is_root = prefix.is_some();

    let struct_name = &input.ident;
    let wrapper_name = if struct_name.to_string().ends_with("Settings") {
        struct_name.clone()
    } else {
        format_ident!("{}Settings", struct_name)
    };

    let named_fields = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(f),
            ..
        }) => &f.named,
        _ => panic!("feature_settings can only be used on structs with named fields"),
    };

    let entries: Vec<SettingEntry> = named_fields
        .iter()
        .map(|field| {
            let name = field.ident.as_ref().unwrap().clone();
            let ty = field.ty.clone();
            let attr = field
                .attrs
                .iter()
                .find(|a| a.path().is_ident("setting"))
                .unwrap_or_else(|| panic!("Field {} must have #[setting] attribute", name));
            let (key, default_expr, is_json, is_nested) = parse_setting_attr(attr, &name);
            if !is_nested && default_expr.is_none() {
                panic!(
                    "Field '{}' is not nested and must have a default value",
                    name
                );
            }
            SettingEntry {
                name,
                key,
                default_expr,
                is_json,
                is_nested,
                ty,
            }
        })
        .collect();

    let patch_methods = gen_patch_methods(&entries);

    let expanded = if is_root {
        generate_root(
            struct_vis,
            &wrapper_name,
            &original_attrs,
            &prefix.unwrap(),
            &entries,
            &patch_methods,
        )
    } else {
        generate_nested(
            struct_vis,
            &wrapper_name,
            &original_attrs,
            &entries,
            &patch_methods,
        )
    };

    TokenStream::from(expanded)
}

// ── SettingEntry ──────────────────────────────────────────────────────────────

struct SettingEntry {
    name: Ident,
    key: String,
    default_expr: Option<Expr>,
    is_json: bool,
    is_nested: bool,
    ty: syn::Type,
}

impl SettingEntry {
    /// Tokens that evaluate to the default value at compile/expand time.
    fn default_tokens(&self) -> TokenStream2 {
        let ty = &self.ty;
        let def = self.default_expr.as_ref().unwrap();
        let key = &self.key;
        if self.is_json {
            quote! {
                serde_json::from_value::<#ty>(#def)
                    .expect(concat!("Failed to deserialize default JSON for setting '", #key, "'"))
            }
        } else {
            quote! { #def }
        }
    }
}

// ── Patch-method generation ───────────────────────────────────────────────────

fn gen_patch_methods(entries: &[SettingEntry]) -> Vec<TokenStream2> {
    entries
        .iter()
        .filter(|e| !e.is_nested)
        .map(|e| {
            let name = &e.name;
            let ty = &e.ty;
            if is_trivial_type(ty) {
                let setter = format_ident!("set_{}", name);
                quote! {
                    pub fn #setter(&self, value: #ty) -> anyhow::Result<()> {
                        self.#name.set(value)
                    }
                }
            } else {
                let patcher = format_ident!("patch_{}", name);
                quote! {
                    pub fn #patcher<F>(&self, f: F) -> anyhow::Result<()>
                    where F: FnOnce(&mut std::sync::Arc<#ty>)
                    {
                        let mut data = self.#name.get_arc();
                        f(&mut data);
                        let json = serde_json::to_value(data.as_ref())?;
                        self.#name.get_store_subscription().settings.set(&self.#name.get_path(), json)?;
                        Ok(())
                    }
                }
            }
        })
        .collect()
}

// ── Code generators ───────────────────────────────────────────────────────────

fn generate_root(
    vis: &Visibility,
    name: &Ident,
    attrs: &[Attribute],
    prefix: &str,
    entries: &[SettingEntry],
    patch_methods: &[TokenStream2],
) -> TokenStream2 {
    let struct_fields = entries.iter().map(|e| {
        let fname = &e.name;
        let ty = &e.ty;
        if e.is_nested {
            quote! { #fname: std::sync::Arc<#ty>, }
        } else {
            quote! { #fname: context::settings::ReactiveSetting<#ty>, }
        }
    });

    let init_fields = entries.iter().map(|e| {
        let fname = &e.name;
        let key = &e.key;
        let ty = &e.ty;
        if e.is_nested {
            quote! {
                #fname: std::sync::Arc::new(#ty::new::<Self>(&store, #key)?),
            }
        } else {
            let def = e.default_tokens();
            quote! {
                #fname: context::settings::setting_or::<Self, #ty>(&store, #key, #def)?,
            }
        }
    });

    let getters = entries.iter().map(|e| {
        let fname = &e.name;
        let ty = &e.ty;
        if e.is_nested {
            quote! {
                pub fn #fname(&self) -> std::sync::Arc<#ty> { self.#fname.clone() }
            }
        } else {
            quote! {
                pub fn #fname(&self) -> context::settings::ReactiveSetting<#ty> { self.#fname.clone() }
            }
        }
    });

    let ensure_calls = entries.iter().map(|e| {
        let key = &e.key;
        let ty = &e.ty;
        if e.is_nested {
            quote! { #ty::ensure_defaults::<Self>(settings, #key)?; }
        } else {
            let def = e.default_tokens();
            quote! {
                <Self as context::settings::FeatureSettings>::ensure_default(settings, #key, #def)?;
            }
        }
    });

    quote! {
        #[derive(Debug, Clone)]
        #(#attrs)*
        #vis struct #name {
            store: std::sync::Arc<context::settings::store::SettingsStore>,
            #(#struct_fields)*
        }

        impl context::settings::SettingsScope for #name {
            const PREFIX: &'static str = #prefix;
        }

        impl context::settings::FeatureSettings for #name {
            fn ensure_defaults(settings: &context::settings::store::SettingsStore) -> anyhow::Result<()> {
                #(#ensure_calls)*
                Ok(())
            }
        }

        impl #name {
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

fn generate_nested(
    vis: &Visibility,
    name: &Ident,
    attrs: &[Attribute],
    entries: &[SettingEntry],
    patch_methods: &[TokenStream2],
) -> TokenStream2 {
    let struct_fields = entries.iter().map(|e| {
        let fname = &e.name;
        let ty = &e.ty;
        if e.is_nested {
            quote! { #fname: std::sync::Arc<#ty>, }
        } else {
            quote! { #fname: std::sync::Arc<context::settings::ReactiveSetting<#ty>>, }
        }
    });

    let init_fields = entries.iter().map(|e| {
        let fname = &e.name;
        let key = &e.key;
        let ty = &e.ty;
        if e.is_nested {
            quote! {
                #fname: std::sync::Arc::new(
                    #ty::new::<TScope>(store, &format!("{}.{}", namespace, #key))?
                ),
            }
        } else {
            let def = e.default_tokens();
            quote! {
                #fname: std::sync::Arc::new(
                    context::settings::setting_or::<TScope, #ty>(
                        store,
                        &format!("{}.{}", namespace, #key),
                        #def,
                    )?
                ),
            }
        }
    });

    let getters = entries.iter().map(|e| {
        let fname = &e.name;
        let ty = &e.ty;
        if e.is_nested {
            quote! {
                pub fn #fname(&self) -> std::sync::Arc<#ty> { self.#fname.clone() }
            }
        } else {
            quote! {
                pub fn #fname(&self) -> std::sync::Arc<context::settings::ReactiveSetting<#ty>> {
                    self.#fname.clone()
                }
            }
        }
    });

    let ensure_calls = entries.iter().map(|e| {
        let key = &e.key;
        let ty = &e.ty;
        if e.is_nested {
            quote! {
                #ty::ensure_defaults::<TScope>(settings, &format!("{}.{}", namespace, #key))?;
            }
        } else {
            let def = e.default_tokens();
            quote! {
                TScope::ensure_default(settings, &format!("{}.{}", namespace, #key), #def)?;
            }
        }
    });

    quote! {
        #[derive(Debug, Clone)]
        #(#attrs)*
        #vis struct #name {
            #(#struct_fields)*
        }

        impl #name {
            pub fn new<TScope: context::settings::FeatureSettings>(
                store: &std::sync::Arc<context::settings::store::SettingsStore>,
                namespace: &str,
            ) -> anyhow::Result<Self> {
                Ok(Self { #(#init_fields)* })
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

// ── Helpers ───────────────────────────────────────────────────────────────────

fn is_trivial_type(ty: &syn::Type) -> bool {
    const TRIVIAL: &[&str] = &[
        "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64", "i128", "f32", "f64",
        "bool", "String", "usize", "isize",
    ];
    if let syn::Type::Path(tp) = ty {
        tp.qself.is_none()
            && tp.path.segments.len() == 1
            && TRIVIAL.contains(&tp.path.segments[0].ident.to_string().as_str())
    } else {
        false
    }
}

fn parse_prefix_arg(args: &syn::punctuated::Punctuated<Meta, syn::Token![,]>) -> Option<String> {
    args.iter().find_map(|meta| {
        if let Meta::NameValue(nv) = meta
            && nv.path.is_ident("prefix")
            && let Expr::Lit(lit) = &nv.value
            && let Lit::Str(s) = &lit.lit
        {
            Some(s.value())
        } else {
            None
        }
    })
}

fn parse_setting_attr(attr: &Attribute, field_name: &Ident) -> (String, Option<Expr>, bool, bool) {
    let mut explicit_key = None;
    let mut default: Option<Expr> = None;
    let mut is_json = false;
    let mut is_nested = false;

    if let Ok(meta) = attr.parse_args_with(
        syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
    ) {
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
                        if let Expr::Macro(mac) = &nv.value
                            && mac.mac.path.segments.last().map(|s| s.ident.to_string())
                                == Some("json".to_string())
                        {
                            is_json = true;
                        }
                        default = Some(nv.value.clone());
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
