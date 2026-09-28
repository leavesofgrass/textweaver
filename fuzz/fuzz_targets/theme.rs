//! Fuzz target: the theme file reader (`textweaver-theme`'s `file.rs`). A
//! bad file is an error naming the key, never a panic. A file that reads
//! writes back as TOML that reads again and writes the same text. Resolved
//! into a whole theme (on its own, and inheriting from a built-in theme),
//! it is checked for contrast and turned into CSS and terminal styles without
//! panicking, and the contrast report agrees with itself.

#![no_main]

use libfuzzer_sys::fuzz_target;
use textweaver_theme::file::MAX_FILE_BYTES;
use textweaver_theme::resolve::resolve;
use textweaver_theme::terminal::ColorSupport;
use textweaver_theme::{Repair, TerminalTheme, ThemeFile, builtin, check};

fuzz_target!(|data: &[u8]| {
    // The registry refuses larger files before it reads them.
    if data.len() as u64 > MAX_FILE_BYTES {
        return;
    }
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(file) = ThemeFile::parse(text) else {
        return;
    };
    let written = file.to_toml_string();
    let again = ThemeFile::parse(&written)
        .unwrap_or_else(|e| panic!("a theme file writes TOML that reads again: {e}\n{written}"));
    assert_eq!(
        again.to_toml_string(),
        written,
        "writing a theme file is stable"
    );

    let base = file.inherits.as_deref().and_then(builtin::get);
    let bases = [base, builtin::all().first()];
    for base in bases {
        for repair in [Repair::DerivedOnly, Repair::Explicit] {
            let Ok((theme, _adjustments)) = resolve(&file, base, Some("fuzz"), repair) else {
                continue;
            };
            let report = check(&theme);
            assert_eq!(
                report.passed(),
                report.failures().next().is_none(),
                "a contrast report passes exactly when nothing fails"
            );
            let _ = report.summary();
            let _ = textweaver_theme::css::single_stylesheet(&theme);
            for support in [
                ColorSupport::NoColor,
                ColorSupport::Ansi16,
                ColorSupport::Ansi256,
                ColorSupport::TrueColor,
            ] {
                let _ = TerminalTheme::new(&theme, support);
            }
        }
    }
});
