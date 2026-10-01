//! Fonts for textweaver.
//!
//! Two sources, one vocabulary:
//!
//! - **Bundled fonts** ([`bundled`]): Atkinson Hyperlegible Next (the
//!   default text font for PDF output), Atkinson Hyperlegible Mono (the code
//!   font), and OpenDyslexic, each in regular, bold, italic, and bold italic,
//!   embedded with `include_bytes!` behind the cargo feature `bundled-fonts`
//!   (on by default). All three are under the SIL Open Font License 1.1;
//!   every family carries its licence text, which must travel with the font
//!   (the EPUB writer puts it in the book beside the font files).
//! - **Downloaded fonts** ([`downloaded`]): Lexend, the one reading font
//!   that is not bundled, downloaded into the data folder the first time a
//!   reader chooses it, after asking. Its files are pinned by URL, size,
//!   and SHA-256; its licence (SIL OFL 1.1) ships with textweaver.
//! - **Installed fonts** ([`system`]): the font folders of Windows, macOS,
//!   and Linux, scanned for family and style names by reading only each
//!   file's table directory and its `name`, `OS/2`, `head`, and `post`
//!   tables ([`sfnt`]), so a family can be chosen by name, and scanned once
//!   per process ([`system::installed`]).
//!
//! Choosing and resolving a reading font is here too ([`choice`]): the
//! family a reader picked, the reading fonts Star offered
//! ([`READING_FONTS`]), each platform's fallbacks, and which family to use
//! given what is bundled and installed ([`FontSettings::resolve`]). The
//! reading aids, the writers, and the GUIs all resolve fonts through this
//! crate.
//!
//! ```
//! use textweaver_fonts::{Style, bundled};
//!
//! if let Some(f) = bundled::family("Atkinson Hyperlegible Next") {
//!     assert_eq!(f.license, "OFL-1.1");
//!     assert!(f.face(Style::Bold).data.len() > 10_000);
//! }
//! ```
//!
//! The terminal UI never uses these: a terminal shows text in its own font.

pub mod bundled;
pub mod choice;
pub mod downloaded;
pub mod sfnt;
pub mod system;

pub use bundled::{BUNDLED, BundledFace, BundledFamily};
pub use choice::{
    FontError, FontFamily, FontResolution, FontSettings, MAX_SIZE_PT, MIN_SIZE_PT, Platform,
    READING_FONTS, ReadingFont, ReadingFontId, SMALL_SIZE_PT, format_points,
};
pub use sfnt::FaceInfo;
pub use system::{FaceRef, FamilyFaces, SystemFace};

/// One of the four styles textweaver uses from a family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Style {
    /// Upright, normal weight.
    Regular,
    /// Upright, bold.
    Bold,
    /// Italic, normal weight.
    Italic,
    /// Italic, bold.
    BoldItalic,
}

impl Style {
    /// All four, in the usual order.
    pub const ALL: [Style; 4] = [
        Style::Regular,
        Style::Bold,
        Style::Italic,
        Style::BoldItalic,
    ];

    /// The style with these properties.
    pub fn new(bold: bool, italic: bool) -> Style {
        match (bold, italic) {
            (false, false) => Style::Regular,
            (true, false) => Style::Bold,
            (false, true) => Style::Italic,
            (true, true) => Style::BoldItalic,
        }
    }

    /// True for bold and bold italic.
    pub fn is_bold(self) -> bool {
        matches!(self, Style::Bold | Style::BoldItalic)
    }

    /// True for italic and bold italic.
    pub fn is_italic(self) -> bool {
        matches!(self, Style::Italic | Style::BoldItalic)
    }

    /// The name to show and speak ("bold italic").
    pub fn label(self) -> &'static str {
        match self {
            Style::Regular => "regular",
            Style::Bold => "bold",
            Style::Italic => "italic",
            Style::BoldItalic => "bold italic",
        }
    }
}

/// Where a family comes from, as [`resolve_family`] found it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FamilySource {
    /// Bundled with textweaver.
    Bundled(&'static BundledFamily),
    /// Installed on this system, or downloaded into the data folder
    /// ([`downloaded`]).
    Installed(FamilyFaces),
}

impl FamilySource {
    /// The family name.
    pub fn name(&self) -> &str {
        match self {
            FamilySource::Bundled(f) => f.name,
            FamilySource::Installed(f) => &f.name,
        }
    }
}

/// Finds a family by name: bundled families first (by name, key, or
/// alias, ignoring case), then a font downloaded into the data folder
/// ([`downloaded::find`]), then the installed fonts in `installed`
/// (usually from [`system::scan`]).
pub fn resolve_family(name: &str, installed: &[SystemFace]) -> Option<FamilySource> {
    if let Some(f) = bundled::family(name) {
        return Some(FamilySource::Bundled(f));
    }
    if let Some(f) = downloaded::find(name) {
        return Some(FamilySource::Installed(f));
    }
    system::find_family(installed, name).map(FamilySource::Installed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles() {
        for s in Style::ALL {
            assert_eq!(Style::new(s.is_bold(), s.is_italic()), s);
        }
        assert_eq!(Style::BoldItalic.label(), "bold italic");
    }

    #[test]
    fn resolve_prefers_bundled() {
        if BUNDLED.is_empty() {
            return;
        }
        let r = resolve_family("opendyslexic", &[]).unwrap();
        assert_eq!(r.name(), "OpenDyslexic");
        assert!(matches!(r, FamilySource::Bundled(_)));
        assert!(resolve_family("No Such Font 123", &[]).is_none());
    }
}
