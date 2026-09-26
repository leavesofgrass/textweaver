//! The built-in themes: Star's 23 palettes, ported and embedded as TOML
//! files (`crates/textweaver-theme/themes/*.toml`). The files are generated
//! from [`crate::star`] and a test keeps them in step.

use std::sync::OnceLock;

use crate::file::ThemeFile;
use crate::model::Theme;
use crate::resolve::{Repair, resolve};

macro_rules! sources {
    ($($name:literal),+ $(,)?) => {
        /// Built-in theme names in cycle order (Star's order; galaxy first).
        pub const NAMES: [&str; 23] = [$($name),+];
        const SOURCES: [(&str, &str); 23] = [
            $(($name, include_str!(concat!("../themes/", $name, ".toml")))),+
        ];
    };
}

sources!(
    "galaxy",
    "galaxy-light",
    "one-dark",
    "one-light",
    "dark",
    "light",
    "contrast",
    "high-contrast",
    "phosphor",
    "dracula",
    "nord",
    "solarized-dark",
    "solarized-light",
    "gruvbox-dark",
    "tokyo-night",
    "catppuccin-mocha",
    "monokai",
    "sepia",
    "amber",
    "everforest-dark",
    "rose-pine",
    "kanagawa",
    "gruvbox-light",
);

/// The TOML text of a built-in theme, for exporting or copying.
pub fn source(name: &str) -> Option<&'static str> {
    let name = canonical_name(name)?;
    SOURCES.iter().find(|(n, _)| *n == name).map(|(_, s)| *s)
}

/// The built-in name for `name`: case-insensitive, with Star's old names
/// (`obsidian` → `galaxy`) and alternate spellings (`high_contrast`).
pub fn canonical_name(name: &str) -> Option<&'static str> {
    let n = name.trim().to_ascii_lowercase();
    let n = crate::star::ALIASES
        .iter()
        .find(|(old, _)| *old == n)
        .map_or(n.as_str(), |(_, new)| *new);
    NAMES.iter().copied().find(|b| *b == n)
}

/// Every built-in theme, parsed once, in cycle order.
pub fn all() -> &'static [Theme] {
    static ALL: OnceLock<Vec<Theme>> = OnceLock::new();
    ALL.get_or_init(|| {
        SOURCES
            .iter()
            .filter_map(|(name, src)| {
                let file = ThemeFile::parse(src).ok()?;
                resolve(&file, None, Some(name), Repair::DerivedOnly)
                    .ok()
                    .map(|(t, _)| t)
            })
            .collect()
    })
}

/// A built-in theme by name (aliases and any case accepted).
pub fn get(name: &str) -> Option<&'static Theme> {
    let name = canonical_name(name)?;
    all().iter().find(|t| t.meta.name == name)
}

/// The default theme, `galaxy`.
pub fn default_theme() -> &'static Theme {
    // The embedded files are constants checked by this crate's tests.
    get(crate::DEFAULT_THEME)
        .or_else(|| all().first())
        .expect("built-in themes are embedded constants and always parse")
}
