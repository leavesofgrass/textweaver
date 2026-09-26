//! Embeds the Windows application manifest (Common Controls v6, DPI
//! awareness) with the MSVC linker; no build dependencies needed.

fn main() {
    println!("cargo::rerun-if-changed=textweaver-gui.manifest");
    let windows = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows");
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|env| env == "msvc");
    if windows && msvc {
        let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
        let manifest = std::path::Path::new(&dir).join("textweaver-gui.manifest");
        println!("cargo::rustc-link-arg-bins=/MANIFEST:EMBED");
        println!(
            "cargo::rustc-link-arg-bins=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
}
