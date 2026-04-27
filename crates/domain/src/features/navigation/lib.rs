#[path = "mod.rs"]
pub mod navigation_impl;

pub mod features {
    pub use crate::navigation_impl as navigation;
}
