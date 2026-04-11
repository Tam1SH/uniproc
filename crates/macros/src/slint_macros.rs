use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{
    parse_quote, FnArg, ImplItem, ImplItemFn, ItemEnum, ItemImpl, ItemStruct, ItemTrait, Pat,
};

use crate::schema::load_schema;

const HELPER_ATTRS: &[&str] = &["manual", "tracing", "slint"];

pub fn strip_helper_attrs(attrs: &mut Vec<syn::Attribute>) {
    attrs.retain(|attr| {
        !HELPER_ATTRS
            .iter()
            .any(|helper| attr.path().is_ident(helper))
    });
}

/// `slint_port` and `slint_bindings` do exactly the same thing.
pub fn strip_trait_helper_attrs(mut trait_item: ItemTrait) -> TokenStream {
    for item in &mut trait_item.items {
        if let syn::TraitItem::Fn(method) = item {
            strip_helper_attrs(&mut method.attrs);
        }
    }
    quote!(#trait_item).into()
}

pub fn slint_dto_impl(item: TokenStream) -> TokenStream {
    if let Ok(mut item_struct) = syn::parse::<ItemStruct>(item.clone()) {
        for field in &mut item_struct.fields {
            strip_helper_attrs(&mut field.attrs);
        }
        return quote!(#item_struct).into();
    }

    if let Ok(mut item_enum) = syn::parse::<ItemEnum>(item) {
        for variant in &mut item_enum.variants {
            strip_helper_attrs(&mut variant.attrs);
        }
        return quote!(#item_enum).into();
    }

    panic!("#[slint_dto] can only be applied to structs and enums");
}

pub fn slint_port_adapter_impl(attr: TokenStream, mut impl_block: ItemImpl) -> TokenStream {
    let window_type = extract_window_type(attr);
    let trait_name = get_trait_name(&impl_block);
    let schema = load_schema();

    let port_def = schema
        .ports
        .iter()
        .find(|p| p.name == trait_name)
        .unwrap_or_else(|| panic!("Trait {} not found in schema", trait_name));

    let existing = get_existing_methods(&impl_block);

    for method in port_def
        .methods
        .iter()
        .filter(|m| !m.is_manual && !existing.contains(&m.name))
    {
        let fn_name = format_ident!("{}", method.name);
        let slint_fn_name = format_ident!(
            "{}",
            method.slint_name.as_deref().unwrap_or(&method.name)
        );
        let global_name = format_ident!(
            "{}",
            method.global_override.as_ref().unwrap_or(&port_def.global)
        );

        let sig_args: Vec<_> = method
            .args
            .iter()
            .map(|a| {
                let name = format_ident!("{}", a.name);
                let ty: syn::Type = syn::parse_str(&qualify_known_type_str(&a.ty)).unwrap();
                quote!(#name: #ty)
            })
            .collect();

        let call_args: Vec<_> = method
            .args
            .iter()
            .map(|a| {
                let name = format_ident!("{}", a.name);
                convert_expr_to_slint(&name, &a.ty)
            })
            .collect();

        impl_block.items.push(syn::ImplItem::Fn(syn::parse_quote! {
            fn #fn_name(&self, ui: &#window_type, #(#sig_args),*) {
                use slint::ComponentHandle;
                ui.global::<crate::#global_name>().#slint_fn_name(#(#call_args),*);
            }
        }));
    }

    apply_adapter_transform(&mut impl_block, None);
    quote!(#impl_block).into()
}

pub fn slint_bindings_adapter_impl(attr: TokenStream, mut impl_block: ItemImpl) -> TokenStream {
    let window_type = extract_window_type(attr);
    let trait_name = get_trait_name(&impl_block);
    let schema = load_schema();

    let binding_def = schema
        .bindings
        .iter()
        .find(|b| b.name == trait_name)
        .unwrap_or_else(|| panic!("Trait {} not found in schema", trait_name));

    let existing = get_existing_methods(&impl_block);

    for method in binding_def
        .methods
        .iter()
        .filter(|m| !m.is_manual && !existing.contains(&m.name))
    {
        let fn_name = format_ident!("{}", method.name);
        let slint_fn_name = format_ident!(
            "{}",
            method.slint_name.as_deref().unwrap_or(&method.name)
        );
        let global_name = format_ident!("{}", binding_def.global);

        let arg_idents: Vec<_> = (0..method.handler_args.len())
            .map(|i| format_ident!("arg{}", i + 1))
            .collect();
        let handler_args: Vec<_> = method
            .handler_args
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let ident = format_ident!("arg{}", i + 1);
                convert_expr_from_slint(&ident, &a.ty)
            })
            .collect();
        let handler_types: Vec<syn::Type> = method
            .handler_args
            .iter()
            .map(|a| syn::parse_str(&qualify_known_type_str(&a.ty)).unwrap())
            .collect();

        impl_block.items.push(syn::ImplItem::Fn(syn::parse_quote! {
            fn #fn_name<F>(&self, ui: &#window_type, handler: F)
            where F: Fn(#(#handler_types),*) + 'static
            {
                use slint::ComponentHandle;
                ui.global::<crate::#global_name>().#slint_fn_name(move |#(#arg_idents),*| {
                    handler(#(#handler_args),*);
                });
            }
        }));
    }

    apply_adapter_transform(&mut impl_block, Some(binding_def));
    quote!(#impl_block).into()
}

// ── Shared helpers ────────────────────────────────────────────────────────────

pub fn get_trait_name(impl_block: &ItemImpl) -> String {
    match &impl_block.trait_ {
        Some((_, path, _)) => path.segments.last().unwrap().ident.to_string(),
        None => panic!("This macro can only be applied to trait implementations"),
    }
}

pub fn get_existing_methods(impl_block: &ItemImpl) -> Vec<String> {
    impl_block
        .items
        .iter()
        .filter_map(|item| match item {
            syn::ImplItem::Fn(m) => Some(m.sig.ident.to_string()),
            _ => None,
        })
        .collect()
}

pub fn apply_adapter_transform(
    impl_block: &mut ItemImpl,
    binding_def: Option<&crate::schema::BindingDef>,
) {
    let self_ty = (*impl_block.self_ty).clone();

    for item in &mut impl_block.items {
        if let ImplItem::Fn(method) = item {
            let tracing = binding_def.and_then(|def| {
                def.methods
                    .iter()
                    .find(|candidate| candidate.name == method.sig.ident.to_string())
                    .map(|method_def| BindingTracingSpec {
                        scope: build_binding_scope(&def.name, &method_def.name),
                        target: method_def.tracing_target.clone(),
                        enabled: !method_def.tracing_skip,
                        handler_arity: method_def.handler_args.len(),
                    })
            });
            transform_method(&self_ty, method, tracing.as_ref());
        }
    }
}

pub fn extract_window_type(attr: TokenStream) -> syn::Type {
    let s = attr.to_string();
    s.split('=')
        .nth(1)
        .and_then(|ty_str| syn::parse_str(ty_str.trim()).ok())
        .unwrap_or_else(|| syn::parse_str("AppWindow").unwrap())
}

fn transform_method(
    self_ty: &syn::Type,
    method: &mut ImplItemFn,
    binding_tracing: Option<&BindingTracingSpec>,
) {
    let ui_arg_idx = find_ui_arg_index(method);

    if let Some(idx) = ui_arg_idx {
        remove_ui_arg(method, idx);
        let handler_wrap = binding_tracing
            .filter(|spec| spec.enabled)
            .map(|spec| build_binding_tracing_wrapper(method, spec));
        let ui_port_wrap = binding_tracing
            .is_none()
            .then(|| build_ui_port_wrapper(self_ty, method));
        let ui_upgrade_failure = build_ui_upgrade_failure(self_ty, method);
        let block = &method.block;

        method.block = parse_quote!({
            let Some(ui) = self.ui.upgrade() else { #ui_upgrade_failure };
            #handler_wrap
            #ui_port_wrap
            #block
        });
    } else if let Some(spec) = binding_tracing.filter(|spec| spec.enabled) {
        let handler_wrap = build_binding_tracing_wrapper(method, spec);
        let block = &method.block;
        method.block = parse_quote!({
            #handler_wrap
            #block
        });
    }
}

fn find_ui_arg_index(method: &ImplItemFn) -> Option<usize> {
    method.sig.inputs.iter().enumerate().find_map(|(i, arg)| {
        if let FnArg::Typed(pat_type) = arg {
            if let Pat::Ident(ref id) = *pat_type.pat {
                if id.ident == "ui" {
                    return Some(i);
                }
            }
        }
        None
    })
}

fn remove_ui_arg(method: &mut ImplItemFn, idx: usize) {
    let mut inputs = syn::punctuated::Punctuated::<syn::FnArg, syn::token::Comma>::new();
    for (i, arg) in method.sig.inputs.clone().into_iter().enumerate() {
        if i != idx {
            inputs.push(arg);
        }
    }
    method.sig.inputs = inputs;
}

struct BindingTracingSpec {
    scope: String,
    target: Option<String>,
    enabled: bool,
    handler_arity: usize,
}

fn build_binding_tracing_wrapper(
    method: &ImplItemFn,
    spec: &BindingTracingSpec,
) -> proc_macro2::TokenStream {
    let handler_ident =
        find_handler_ident(method).unwrap_or_else(|| panic!("binding tracing requires a handler parameter"));
    let scope = &spec.scope;
    let target_fields = spec
        .target
        .as_ref()
        .map(|v| quote! { Some(#v) })
        .unwrap_or_else(|| quote! { None });

    let arity = spec.handler_arity;

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
        _ => panic!("binding tracing currently supports handlers with up to 2 arguments"),
    }
}

fn build_binding_scope(trait_name: &str, method_name: &str) -> String {
    format!("Ui.{}.{}", binding_feature_name(trait_name), method_name)
}

fn binding_feature_name(trait_name: &str) -> String {
    let trimmed = trait_name
        .strip_prefix("Ui")
        .unwrap_or(trait_name)
        .strip_suffix("UiBindings")
        .or_else(|| trait_name.strip_suffix("Bindings"))
        .unwrap_or_else(|| trait_name.strip_prefix("Ui").unwrap_or(trait_name));

    trimmed.to_string()
}

fn build_ui_port_wrapper(
    self_ty: &syn::Type,
    method: &ImplItemFn,
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

fn build_ui_upgrade_failure(
    self_ty: &syn::Type,
    method: &ImplItemFn,
) -> proc_macro2::TokenStream {
    let method_name = method.sig.ident.to_string();
    let adapter_name = quote! { stringify!(#self_ty) };

    quote! {
        let __ui_port_target_value = format!("{}::{}", #adapter_name, #method_name);
        let __ui_port_scope_target = Some(__ui_port_target_value.clone());

        if app_core::trace::is_scope_enabled("ui.adapter.call")
            && app_core::trace::is_target_enabled(&__ui_port_target_value)
        {
            app_core::trace::in_named_scope(
                "ui.adapter.call",
                Some("adapter,method"),
                __ui_port_scope_target,
                || {
                    tracing::error!(
                        adapter = #adapter_name,
                        method = #method_name,
                        "ui.adapter.upgrade_failed"
                    );
                },
            );
        }

        panic!("ui handle is dropped in {}::{}", #adapter_name, #method_name);
    }
}

fn find_handler_ident(method: &ImplItemFn) -> Option<Ident> {
    method.sig.inputs.iter().find_map(|arg| {
        if let FnArg::Typed(pat_type) = arg
            && let Pat::Ident(pat_ident) = pat_type.pat.as_ref()
            && pat_ident.ident == "handler"
        {
            Some(pat_ident.ident.clone())
        } else {
            None
        }
    })
}

fn qualify_known_type_str(ty: &str) -> String {
    match ty {
        "SharedString" => "slint::SharedString".to_string(),
        "Image" => "slint::Image".to_string(),
        "Color" => "slint::Color".to_string(),
        _ => ty.to_string(),
    }
}

fn convert_expr_to_slint(name: &proc_macro2::Ident, ty: &str) -> proc_macro2::TokenStream {
    if is_trivial_numeric(ty) {
        quote!(#name as _)
    } else if ty == "bool" {
        quote!(#name)
    } else {
        quote!(#name.into())
    }
}

fn convert_expr_from_slint(name: &proc_macro2::Ident, ty: &str) -> proc_macro2::TokenStream {
    if is_trivial_numeric(ty) {
        quote!(#name as _)
    } else if ty == "bool" {
        quote!(#name)
    } else {
        quote!(#name.into())
    }
}

fn is_trivial_numeric(ty: &str) -> bool {
    matches!(
        ty,
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize"
            | "i8" | "i16" | "i32" | "i64" | "i128" | "isize"
            | "f32" | "f64"
    )
}
