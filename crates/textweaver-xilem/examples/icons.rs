//! Draws the loom logo into the crate's `assets/icons/` folder: a PNG per
//! size and variant, `textweaver.ico`, `textweaver-high-contrast.ico`, and
//! `textweaver.res` (what each Windows program links). Run it after a
//! change to the logo's drawing (`src/icon.rs`) and commit the files:
//!
//! ```text
//! cargo run -p textweaver-xilem --example icons
//! ```
//!
//! An optional argument names another folder to write into.

use std::path::PathBuf;

fn main() {
    let dir = std::env::args_os().nth(1).map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/icons"),
        PathBuf::from,
    );
    match textweaver_xilem::icon::write_assets(&dir) {
        Ok(files) => {
            for f in files {
                println!("wrote {}", f.display());
            }
        }
        Err(e) => {
            eprintln!("icons: {e}");
            std::process::exit(1);
        }
    }
}
