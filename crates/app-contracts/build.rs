fn main() {
    build_utils::collector::main();
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR missing"));
    build_utils::slint_parser::generate_navigation_routes(
        std::path::Path::new("../slint-adapter/ui/pages"),
        &out_dir.join("navigation_routes.rs"),
    );
}
