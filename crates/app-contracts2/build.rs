use guicons_build::{Emit, IconBuild};

fn main() {
    guinea_codegen::l10n::build("../../locales");

    IconBuild::auto().emit(Emit::Rust).build();
}
