use guicons_build::{Emit, IconBuild};
use std::path::Path;

fn main() {
    guinea_codegen::l10n::build("../../locales");

    // Not `IconBuild::auto()`: that stops at the nearest ancestor `Cargo.toml`
    // (this crate's own), but uniproc keeps one shared `icons.gui.toml` at the
    // repo root, two levels up from `crates/app-contracts2`.
    IconBuild::new(Path::new("../../icons.gui.toml"))
        .emit(Emit::Rust)
        .build();
}
