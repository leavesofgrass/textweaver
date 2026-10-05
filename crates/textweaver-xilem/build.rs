//! Links the loom icon into the window's program on Windows, so Explorer,
//! shortcuts and the taskbar show it, and the window loads it by id
//! (`src/icon.rs`). `assets/icons/textweaver.res` is a compiled resource
//! file made by `cargo run -p textweaver-xilem --example icons`; the
//! Microsoft linker takes it as an input, so no resource compiler is
//! needed. Other targets (and the GNU toolchain) build without it, and the
//! window falls back to the committed PNG.

fn main() {
    let res = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icons/textweaver.res");
    println!("cargo:rerun-if-changed={}", res.display());
    let windows = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows");
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|env| env == "msvc");
    if windows && msvc && res.is_file() {
        println!("cargo:rustc-link-arg-bins={}", res.display());
    }
}
