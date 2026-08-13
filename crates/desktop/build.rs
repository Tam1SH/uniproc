fn main() {
    windows_reactor_setup::as_self_contained();
    guinea_meta_build::generate("../../app.toml");
}
