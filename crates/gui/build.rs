fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .with_style("fluent".into());

    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("ui/assets/icon.ico");
        res.compile().unwrap();
    }

    slint_build::compile_with_config("ui/app-window.slint", config).expect("Slint build failed");
}