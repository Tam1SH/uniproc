fn main() {
    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        let out_dir = std::env::var("OUT_DIR").unwrap();

        std::process::Command::new("zig")
            .args(&[
                "build-lib",
                "native/hyperv.zig",
                "-lc",
                &format!("-femit-bin={}/hyperv_fix.lib", out_dir),
                "-target",
                "x86_64-windows-msvc",
            ])
            .status()
            .expect("Zig is not installed or not in PATH");

        println!("cargo:rustc-link-search=native={}", out_dir);
        println!("cargo:rustc-link-lib=static=hyperv_fix");

        println!("cargo:rustc-link-lib=advapi32");
        println!("cargo:rustc-link-lib=netapi32");
    }
}
