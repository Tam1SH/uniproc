use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, ItemFn, ItemImpl, ItemTrait, Meta};

mod feature_settings;
mod handler;
mod schema;
mod slint_macros;

#[proc_macro_attribute]
pub fn handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    handler::generate_standalone_handler(input)
}

#[proc_macro_attribute]
pub fn slint_port(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let trait_item = parse_macro_input!(item as ItemTrait);
    slint_macros::strip_trait_helper_attrs(trait_item)
}

#[proc_macro_attribute]
pub fn slint_bindings(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let trait_item = parse_macro_input!(item as ItemTrait);
    slint_macros::strip_trait_helper_attrs(trait_item)
}

#[proc_macro_attribute]
pub fn slint_dto(_attr: TokenStream, item: TokenStream) -> TokenStream {
    slint_macros::slint_dto_impl(item)
}

#[proc_macro_attribute]
pub fn slint_port_adapter(attr: TokenStream, item: TokenStream) -> TokenStream {
    let _ = include_bytes!("../../app-contracts/contracts-schema.json");
    let impl_block = parse_macro_input!(item as ItemImpl);
    slint_macros::slint_port_adapter_impl(attr, impl_block)
}

#[proc_macro_attribute]
pub fn slint_bindings_adapter(attr: TokenStream, item: TokenStream) -> TokenStream {
    let _ = include_bytes!("../../app-contracts/contracts-schema.json");
    let impl_block = parse_macro_input!(item as ItemImpl);
    slint_macros::slint_bindings_adapter_impl(attr, impl_block)
}

#[proc_macro_attribute]
pub fn feature_settings(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args with syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated);
    let input = parse_macro_input!(input as DeriveInput);
    feature_settings::feature_settings_impl(args, input)
}
