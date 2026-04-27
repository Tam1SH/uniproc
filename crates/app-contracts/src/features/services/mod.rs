use macros::capability;

mod bindings;
mod model;
mod ports;

pub use bindings::*;
pub use model::*;
pub use ports::*;

#[capability("services")]
pub struct ServicesCapability;
