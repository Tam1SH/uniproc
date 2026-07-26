#![cfg_attr(coverage, feature(coverage_attribute))]
#![cfg_attr(coverage, coverage(off))]

use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemImpl};

#[proc_macro_attribute]
pub fn actor_manifest(attr: TokenStream, item: TokenStream) -> TokenStream {
    let impl_block = parse_macro_input!(item as ItemImpl);

    let _ = std::hint::black_box(app_contracts::__force_link_anchor as fn());

    forsl_codegen::actor_manifest::actor_manifest_impl(
        attr.into(),
        impl_block,
        forsl_core::contracts::bindings(),
    )
    .into()
}
