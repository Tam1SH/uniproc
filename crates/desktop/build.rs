fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        compile_windows_resources();
    }
}

fn compile_windows_resources() {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../slint-adapter/ui/assets/icon.ico");
    res.compile().unwrap();
}
