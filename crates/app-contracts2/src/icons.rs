pub use guicons::{IconData, IconFamily, IconKey, IconRef, IconVariant};

// `#[allow]` on a macro invocation is ignored by rustc, hence the manual expansion.
#[allow(dead_code, unused_imports)]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/icons.rs"));
}

pub use generated::*;
