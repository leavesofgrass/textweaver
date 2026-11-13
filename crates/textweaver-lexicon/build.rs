//! Gathers the message catalogs: each language's folder under `locales`
//! holds one `.ftl` file per area of the interface. This writes
//! `catalogs.rs`, which embeds every file with `include_str!`, in sorted
//! order, joined into one text per language (`EN_TEXT`), so the parser
//! reads a language as one catalog, and listed file by file (`EN_FILES`)
//! for the tests that check each file.

use std::fmt::Write as _;
use std::path::Path;

/// The languages built in, English first, as `i18n::LANGUAGES` lists them.
const LANGS: [&str; 6] = ["en", "es", "fr", "de", "pt", "ar"];

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    // A folder is watched whole: a file added, changed, or removed reruns this.
    println!("cargo:rerun-if-changed=locales");
    let mut out =
        String::from("// Written by build.rs from the folders in locales. Do not edit.\n");
    for lang in LANGS {
        let folder = dir.join(lang);
        let mut files: Vec<String> = std::fs::read_dir(&folder)
            .unwrap_or_else(|e| panic!("{}: {e}", folder.display()))
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".ftl"))
            .collect();
        files.sort();
        assert!(!files.is_empty(), "{} has no .ftl files", folder.display());
        let name = lang.to_ascii_uppercase();
        writeln!(
            out,
            "/// The `{lang}` catalog's files: (name, text), sorted by name."
        )
        .unwrap();
        writeln!(out, "#[cfg(test)]").unwrap();
        writeln!(out, "const {name}_FILES: &[(&str, &str)] = &[").unwrap();
        for f in &files {
            let path = folder.join(f).display().to_string();
            writeln!(out, "    ({f:?}, include_str!({path:?})),").unwrap();
        }
        writeln!(out, "];").unwrap();
        writeln!(
            out,
            "/// The `{lang}` catalog, its files joined in sorted order."
        )
        .unwrap();
        write!(out, "const {name}_TEXT: &str = concat!(").unwrap();
        for f in &files {
            let path = folder.join(f).display().to_string();
            write!(out, "include_str!({path:?}), \"\\n\", ").unwrap();
        }
        writeln!(out, ");").unwrap();
    }
    let dest = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("catalogs.rs");
    std::fs::write(&dest, out).unwrap_or_else(|e| panic!("{}: {e}", dest.display()));
}
