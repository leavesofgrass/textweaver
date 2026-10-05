//! Links the loom icon into `tw.exe` on Windows, so Explorer and shortcuts
//! show it. The compiled resource file is the window's
//! (`crates/textweaver-xilem/assets/icons/textweaver.res`, made by its
//! `icons` example); the Microsoft linker takes it as an input, so no
//! resource compiler is needed. Other targets build without it.

fn main() {
    let res = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../textweaver-xilem/assets/icons/textweaver.res");
    println!("cargo:rerun-if-changed={}", res.display());
    let windows = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows");
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|env| env == "msvc");
    if windows && msvc && res.is_file() {
        println!("cargo:rustc-link-arg-bins={}", res.display());
    }
}
