//! Font settings: family, size, and weight, with fallbacks and CSS.
//!
//! Star let the reader pick any family and offered three reading fonts it
//! downloaded on first use (`star/fonts.py`, `gui/mixin_fontspacing.py`):
//! OpenDyslexic, Atkinson Hyperlegible, and Lexend, all under the SIL Open
//! Font License 1.1. textweaver keeps that list and that approach: font
//! files are never bundled; [`READING_FONTS`] records each font's licence,
//! home page, and the pinned download URLs Star used, and the GUI fetches
//! a chosen font into its cache after asking (see `docs/reading-aids.md`).
//!
//! This module is pure: it never enumerates or loads fonts. The GUI passes
//! a predicate that says whether a family is installed, and gets back the
//! family to use, whether it fell back, and whether a download would help
//! ([`FontSettings::resolve`]). HTML views get a CSS font stack whose
//! fallbacks do the same job in the browser ([`FontSettings::to_css`]).
//!
//! Line height lives in [`TextSpacing`]; the two meet
//! in [`FontSettings::describe`] and [`FontSettings::to_css`].

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use crate::spacing::TextSpacing;

/// One of the reading fonts in [`READING_FONTS`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReadingFontId {
    /// OpenDyslexic.
    OpenDyslexic,
    /// Atkinson Hyperlegible.
    AtkinsonHyperlegible,
    /// Lexend.
    Lexend,
}

impl ReadingFontId {
    /// The font's record.
    pub fn font(self) -> &'static ReadingFont {
        match self {
            ReadingFontId::OpenDyslexic => &READING_FONTS[0],
            ReadingFontId::AtkinsonHyperlegible => &READING_FONTS[1],
            ReadingFontId::Lexend => &READING_FONTS[2],
        }
    }
}

/// A reading font textweaver knows how to fetch. All are SIL OFL 1.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadingFont {
    /// Which font.
    pub id: ReadingFontId,
    /// Settings key, as in Star (`qt_reading_font`).
    pub key: &'static str,
    /// The family name the files register under.
    pub family: &'static str,
    /// Other family names that are the same design (older or variant
    /// builds a reader may have installed).
    pub alternates: &'static [&'static str],
    /// Who made it and why it helps, in one sentence.
    pub about: &'static str,
    /// SPDX licence identifier.
    pub license: &'static str,
    /// Home page for the font and its licence.
    pub homepage: &'static str,
    /// Font files, pinned to immutable commits (Star's URLs).
    pub files: &'static [&'static str],
}

impl ReadingFont {
    /// The file whose presence in the font cache means "fetched" (the
    /// regular face).
    pub fn probe_file(&self) -> &'static str {
        self.files
            .first()
            .and_then(|u| u.rsplit('/').next())
            .unwrap_or("")
    }

    /// The font with settings key `key`.
    pub fn by_key(key: &str) -> Option<&'static ReadingFont> {
        READING_FONTS.iter().find(|f| f.key == key)
    }
}

macro_rules! urls {
    ($base:expr, [$($f:literal),* $(,)?]) => {
        &[$(concat!($base, "/", $f)),*]
    };
}

/// The reading fonts Star offered, in Star's order.
pub const READING_FONTS: [ReadingFont; 3] = [
    ReadingFont {
        id: ReadingFontId::OpenDyslexic,
        key: "opendyslexic",
        family: "OpenDyslexic",
        alternates: &["OpenDyslexic3", "OpenDyslexicAlta"],
        about: "Designed for readers with dyslexia: heavier letter bottoms and distinct letter shapes.",
        license: "OFL-1.1",
        homepage: "https://opendyslexic.org/",
        files: urls!(
            "https://raw.githubusercontent.com/antijingoist/opendyslexic/1824da5c0e41dc3e13ffc7f3a636dcaf695d61b7/compiled",
            [
                "OpenDyslexic-Regular.otf",
                "OpenDyslexic-Bold.otf",
                "OpenDyslexic-Italic.otf",
                "OpenDyslexic-BoldItalic.otf",
            ]
        ),
    },
    ReadingFont {
        id: ReadingFontId::AtkinsonHyperlegible,
        key: "atkinson",
        family: "Atkinson Hyperlegible",
        alternates: &["Atkinson Hyperlegible Next"],
        about: "From the Braille Institute: letters that are hard to confuse, for low-vision readers.",
        license: "OFL-1.1",
        homepage: "https://www.brailleinstitute.org/freefont/",
        files: urls!(
            "https://raw.githubusercontent.com/googlefonts/atkinson-hyperlegible/c14451f32cd7d15b8fae441338f41c3bcebc74c4/fonts/ttf",
            [
                "AtkinsonHyperlegible-Regular.ttf",
                "AtkinsonHyperlegible-Bold.ttf",
                "AtkinsonHyperlegible-Italic.ttf",
                "AtkinsonHyperlegible-BoldItalic.ttf",
            ]
        ),
    },
    ReadingFont {
        id: ReadingFontId::Lexend,
        key: "lexend",
        family: "Lexend",
        alternates: &["Lexend Deca"],
        about: "Widely spaced letters, designed to reduce visual stress and improve reading fluency.",
        license: "OFL-1.1",
        homepage: "https://www.lexend.com/",
        files: urls!(
            "https://raw.githubusercontent.com/googlefonts/lexend/cd26b9c2538d758138c20c3d2f10362ed613854b/fonts/lexend/ttf",
            ["Lexend-Regular.ttf", "Lexend-Bold.ttf"]
        ),
    },
];

/// Which operating system's installed fonts to prefer in fallbacks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Platform {
    /// Windows.
    Windows,
    /// macOS.
    MacOs,
    /// Linux and other Unix desktops.
    Linux,
}

impl Platform {
    /// The platform this build targets.
    pub fn current() -> Self {
        if cfg!(windows) {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::MacOs
        } else {
            Platform::Linux
        }
    }
}

/// The family the reader chose.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum FontFamily {
    /// The system's interface font (Segoe UI, San Francisco, and so on).
    #[default]
    SystemUi,
    /// A plain sans-serif (Star's default: Segoe UI, Helvetica Neue, or
    /// DejaVu Sans).
    Sans,
    /// A serif.
    Serif,
    /// A monospace font.
    Monospace,
    /// One of [`READING_FONTS`].
    Reading(ReadingFontId),
    /// Any installed family, by name.
    Named(String),
}

impl From<String> for FontFamily {
    fn from(s: String) -> Self {
        match s.trim() {
            "" | "system-ui" | "default" => FontFamily::SystemUi,
            "sans" | "sans-serif" => FontFamily::Sans,
            "serif" => FontFamily::Serif,
            "monospace" | "mono" => FontFamily::Monospace,
            other => match ReadingFont::by_key(other) {
                Some(f) => FontFamily::Reading(f.id),
                None => FontFamily::Named(other.to_owned()),
            },
        }
    }
}

impl From<FontFamily> for String {
    fn from(f: FontFamily) -> Self {
        f.key()
    }
}

fn platform_sans(p: Platform) -> &'static [&'static str] {
    match p {
        Platform::Windows => &["Segoe UI", "Arial"],
        Platform::MacOs => &["Helvetica Neue", "Helvetica"],
        Platform::Linux => &["DejaVu Sans", "Noto Sans", "Liberation Sans"],
    }
}

fn platform_serif(p: Platform) -> &'static [&'static str] {
    match p {
        Platform::Windows => &["Cambria", "Georgia", "Times New Roman"],
        Platform::MacOs => &["Iowan Old Style", "Georgia", "Times"],
        Platform::Linux => &["DejaVu Serif", "Noto Serif", "Liberation Serif"],
    }
}

fn platform_mono(p: Platform) -> &'static [&'static str] {
    match p {
        Platform::Windows => &["Cascadia Mono", "Consolas", "Courier New"],
        Platform::MacOs => &["SF Mono", "Menlo", "Courier"],
        Platform::Linux => &["DejaVu Sans Mono", "Noto Sans Mono", "Liberation Mono"],
    }
}

fn platform_ui(p: Platform) -> &'static [&'static str] {
    match p {
        Platform::Windows => &["Segoe UI"],
        Platform::MacOs => &[".AppleSystemUIFont", "Helvetica Neue"],
        Platform::Linux => &["Cantarell", "Ubuntu", "DejaVu Sans"],
    }
}

impl FontFamily {
    /// The settings value: `system-ui`, `sans`, `serif`, `monospace`, a
    /// reading font key, or a family name.
    pub fn key(&self) -> String {
        match self {
            FontFamily::SystemUi => "system-ui".into(),
            FontFamily::Sans => "sans".into(),
            FontFamily::Serif => "serif".into(),
            FontFamily::Monospace => "monospace".into(),
            FontFamily::Reading(id) => id.font().key.into(),
            FontFamily::Named(n) => n.clone(),
        }
    }

    /// The name to show and speak.
    pub fn label(&self) -> String {
        match self {
            FontFamily::SystemUi => "System font".into(),
            FontFamily::Sans => "Sans serif".into(),
            FontFamily::Serif => "Serif".into(),
            FontFamily::Monospace => "Monospace".into(),
            FontFamily::Reading(id) => id.font().family.into(),
            FontFamily::Named(n) => n.clone(),
        }
    }

    /// The choices a font picker offers first, before installed families.
    pub fn choices() -> Vec<FontFamily> {
        let mut v = vec![
            FontFamily::SystemUi,
            FontFamily::Sans,
            FontFamily::Serif,
            FontFamily::Monospace,
        ];
        v.extend(READING_FONTS.iter().map(|f| FontFamily::Reading(f.id)));
        v
    }

    /// The reading font behind this choice, if it is one.
    pub fn reading_font(&self) -> Option<&'static ReadingFont> {
        match self {
            FontFamily::Reading(id) => Some(id.font()),
            _ => None,
        }
    }

    /// Family names to try, in order, ending with a platform sans-serif.
    /// A reading font falls back to the other reading fonts first (Star's
    /// order), then to sans-serif.
    pub fn fallback_chain(&self, platform: Platform) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut add = |names: &[&str]| {
            for n in names {
                if !out.iter().any(|o| o.eq_ignore_ascii_case(n)) {
                    out.push((*n).to_owned());
                }
            }
        };
        match self {
            FontFamily::SystemUi => add(platform_ui(platform)),
            FontFamily::Sans => {}
            FontFamily::Serif => add(platform_serif(platform)),
            FontFamily::Monospace => add(platform_mono(platform)),
            FontFamily::Reading(id) => {
                let f = id.font();
                add(&[f.family]);
                add(f.alternates);
                for f in &READING_FONTS {
                    add(&[f.family]);
                    add(f.alternates);
                }
            }
            FontFamily::Named(n) => add(&[n.as_str()]),
        }
        add(platform_sans(platform));
        out
    }

    /// The CSS generic family that ends a stack for this choice.
    pub fn css_generic(&self) -> &'static str {
        match self {
            FontFamily::SystemUi => "system-ui, sans-serif",
            FontFamily::Serif => "serif",
            FontFamily::Monospace => "monospace",
            _ => "sans-serif",
        }
    }
}

/// Smallest font size textweaver accepts, in points.
pub const MIN_SIZE_PT: f32 = 6.0;
/// Largest font size textweaver accepts, in points.
pub const MAX_SIZE_PT: f32 = 144.0;
/// Below this size, [`FontSettings::warnings`] suggests a larger one.
pub const SMALL_SIZE_PT: f32 = 12.0;

/// Font settings for reading views.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FontSettings {
    /// The family.
    pub family: FontFamily,
    /// Size in points (Star's `qt_font_size`, default 14).
    pub size_pt: f32,
    /// Weight, 100 (thin) to 900 (black); 400 is regular, 700 bold.
    pub weight: u16,
    /// Offer to download a missing reading font (after asking the reader).
    pub fetch_missing: bool,
}

impl Default for FontSettings {
    fn default() -> Self {
        FontSettings {
            family: FontFamily::Sans,
            size_pt: 14.0,
            weight: 400,
            fetch_missing: true,
        }
    }
}

/// A font setting outside the supported range.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum FontError {
    /// Size is not a number or out of range.
    #[error("font size must be between {MIN_SIZE_PT} and {MAX_SIZE_PT} points, not {0}")]
    Size(f32),
    /// Weight out of range.
    #[error("font weight must be between 100 and 900, not {0}")]
    Weight(u16),
    /// A named family with no name.
    #[error("the font family name is empty")]
    EmptyFamily,
}

/// Which family to use, after checking what is installed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontResolution {
    /// The family to apply, or `None` to keep the toolkit's default.
    pub family: Option<String>,
    /// True when the chosen family is missing and a fallback is used.
    pub fell_back: bool,
    /// A reading font that is missing and can be downloaded.
    pub download: Option<&'static ReadingFont>,
}

impl FontResolution {
    /// A sentence to announce when the reader picks a font: empty when
    /// the chosen font is in use.
    pub fn message(&self, chosen: &FontFamily) -> String {
        if !self.fell_back {
            return String::new();
        }
        let using = self.family.as_deref().unwrap_or("the default font");
        match self.download {
            Some(f) => format!(
                "{} is not installed. Using {using}. It can be downloaded, free, under the Open Font License.",
                f.family
            ),
            None => format!("{} is not installed. Using {using}.", chosen.label()),
        }
    }
}

/// Everything a GUI needs to set a font on a text control, in points.
#[derive(Clone, Debug, PartialEq)]
pub struct FontDescription {
    /// Families to try, in order.
    pub families: Vec<String>,
    /// Font size in points.
    pub size_pt: f32,
    /// Weight, 100 to 900.
    pub weight: u16,
    /// Line height in points (size × spacing line height).
    pub line_height_pt: f32,
    /// Extra space between letters, in points.
    pub letter_spacing_pt: f32,
    /// Extra space between words, in points.
    pub word_spacing_pt: f32,
    /// Space after each paragraph, in points.
    pub paragraph_spacing_pt: f32,
}

fn css_quote(name: &str) -> String {
    let clean: String = name
        .chars()
        .filter(|c| !matches!(c, '"' | '\\' | ';' | '{' | '}' | '<' | '>'))
        .collect();
    format!("\"{clean}\"")
}

fn trim(v: f32) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() { "0".into() } else { s.into() }
}

impl FontSettings {
    /// Checks size, weight, and family.
    pub fn validate(&self) -> Result<(), FontError> {
        if !self.size_pt.is_finite() || !(MIN_SIZE_PT..=MAX_SIZE_PT).contains(&self.size_pt) {
            return Err(FontError::Size(self.size_pt));
        }
        if !(100..=900).contains(&self.weight) {
            return Err(FontError::Weight(self.weight));
        }
        if matches!(&self.family, FontFamily::Named(n) if n.trim().is_empty()) {
            return Err(FontError::EmptyFamily);
        }
        Ok(())
    }

    /// The settings with size and weight clamped into range, and an empty
    /// family name replaced by sans-serif.
    pub fn clamped(&self) -> Self {
        let size = if self.size_pt.is_finite() {
            self.size_pt.clamp(MIN_SIZE_PT, MAX_SIZE_PT)
        } else {
            FontSettings::default().size_pt
        };
        let family = match &self.family {
            FontFamily::Named(n) if n.trim().is_empty() => FontFamily::Sans,
            f => f.clone(),
        };
        FontSettings {
            family,
            size_pt: size,
            weight: self.weight.clamp(100, 900),
            fetch_missing: self.fetch_missing,
        }
    }

    /// Advice to show and speak, such as a small size. Empty when none.
    pub fn warnings(&self) -> Vec<String> {
        let mut v = Vec::new();
        let s = self.clamped();
        if s.size_pt < SMALL_SIZE_PT {
            v.push(format!(
                "{} points is small for reading; {SMALL_SIZE_PT} or more is easier.",
                trim(s.size_pt)
            ));
        }
        if s.weight < 300 {
            v.push("Very light text can be hard to see; regular weight is 400.".into());
        }
        v
    }

    /// Picks the family to use. `installed` says whether a family is
    /// installed (the GUI asks its toolkit; matching should ignore case).
    pub fn resolve(&self, platform: Platform, installed: impl Fn(&str) -> bool) -> FontResolution {
        let s = self.clamped();
        if s.family == FontFamily::SystemUi && platform == Platform::MacOs {
            // The toolkit's default is the system font.
            return FontResolution {
                family: None,
                fell_back: false,
                download: None,
            };
        }
        let chain = s.family.fallback_chain(platform);
        let wanted: Vec<&str> = match &s.family {
            FontFamily::Reading(id) => {
                let f = id.font();
                let mut w = vec![f.family];
                w.extend_from_slice(f.alternates);
                w
            }
            FontFamily::Named(n) => vec![n.as_str()],
            FontFamily::SystemUi => platform_ui(platform).to_vec(),
            FontFamily::Serif => platform_serif(platform).to_vec(),
            FontFamily::Monospace => platform_mono(platform).to_vec(),
            FontFamily::Sans => platform_sans(platform).to_vec(),
        };
        let found = chain.iter().find(|n| installed(n)).cloned();
        let fell_back = found
            .as_deref()
            .is_none_or(|f| !wanted.iter().any(|w| w.eq_ignore_ascii_case(f)));
        let download = if fell_back && s.fetch_missing {
            s.family.reading_font()
        } else {
            None
        };
        FontResolution {
            family: found,
            fell_back,
            download,
        }
    }

    /// A description the GUI can apply directly, with the spacing settings
    /// converted to points for this size.
    pub fn describe(&self, spacing: &TextSpacing, platform: Platform) -> FontDescription {
        let s = self.clamped();
        let sp = spacing.clamped();
        FontDescription {
            families: s.family.fallback_chain(platform),
            size_pt: s.size_pt,
            weight: s.weight,
            line_height_pt: s.size_pt * sp.line_height,
            letter_spacing_pt: s.size_pt * sp.letter_spacing,
            word_spacing_pt: s.size_pt * sp.word_spacing,
            paragraph_spacing_pt: s.size_pt * sp.paragraph_spacing,
        }
    }

    /// The CSS `font-family` value: every fallback, quoted, then the
    /// generic family. Fallbacks for all platforms are included, since an
    /// HTML file may be opened anywhere.
    pub fn css_font_family(&self) -> String {
        let s = self.clamped();
        let mut names: Vec<String> = Vec::new();
        for p in [Platform::Windows, Platform::MacOs, Platform::Linux] {
            for n in s.family.fallback_chain(p) {
                if n.starts_with('.') {
                    continue; // private macOS names are not usable in CSS
                }
                if !names.iter().any(|o| o.eq_ignore_ascii_case(&n)) {
                    names.push(n);
                }
            }
        }
        let mut out: Vec<String> = Vec::new();
        if s.family == FontFamily::SystemUi {
            out.push("system-ui".into());
        }
        out.extend(names.iter().map(|n| css_quote(n)));
        out.push(s.family.css_generic().into());
        out.join(", ")
    }

    /// CSS for `selector`: the font stack, size, and weight, plus the text
    /// spacing rules from [`TextSpacing::to_css`].
    pub fn to_css(&self, spacing: &TextSpacing, selector: &str) -> String {
        let s = self.clamped();
        let mut out = String::new();
        let _ = writeln!(out, "{selector} {{");
        let _ = writeln!(out, "  font-family: {};", s.css_font_family());
        let _ = writeln!(out, "  font-size: {}pt;", trim(s.size_pt));
        let _ = writeln!(out, "  font-weight: {};", s.weight);
        let _ = writeln!(out, "}}");
        out.push_str(&spacing.to_css(selector));
        out
    }

    /// A summary to show and speak: "Atkinson Hyperlegible, 16 points,
    /// regular."
    pub fn summary(&self) -> String {
        let s = self.clamped();
        let weight = match s.weight {
            0..=249 => "light",
            250..=349 => "semi-light",
            350..=449 => "regular",
            450..=649 => "medium",
            650..=749 => "bold",
            _ => "heavy",
        };
        format!(
            "{}, {} points, {weight}.",
            s.family.label(),
            trim(s.size_pt)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reading_fonts_are_recorded() {
        assert_eq!(READING_FONTS.len(), 3);
        for f in &READING_FONTS {
            assert_eq!(f.license, "OFL-1.1");
            assert!(f.homepage.starts_with("https://"));
            assert!(!f.files.is_empty());
            for u in f.files {
                assert!(u.starts_with("https://raw.githubusercontent.com/"), "{u}");
                assert!(u.ends_with(".otf") || u.ends_with(".ttf"), "{u}");
                // Pinned to a 40-hex commit, never a branch.
                assert!(
                    u.split('/')
                        .any(|p| p.len() == 40 && p.chars().all(|c| c.is_ascii_hexdigit())),
                    "{u}"
                );
            }
            assert!(f.probe_file().contains("Regular"));
            assert_eq!(ReadingFont::by_key(f.key), Some(f));
            assert_eq!(f.id.font(), f);
        }
        assert!(READING_FONTS[0].files[0].contains("/antijingoist/opendyslexic/"));
        assert!(READING_FONTS[1].files[0].contains("/googlefonts/atkinson-hyperlegible/"));
        assert!(READING_FONTS[2].files[0].contains("/googlefonts/lexend/"));
    }

    #[test]
    fn families_round_trip_as_strings() {
        for f in FontFamily::choices() {
            assert_eq!(FontFamily::from(f.key()), f);
        }
        assert_eq!(FontFamily::from("default".to_owned()), FontFamily::SystemUi);
        assert_eq!(
            FontFamily::from("Comic Neue".to_owned()),
            FontFamily::Named("Comic Neue".into())
        );
        let s = FontSettings {
            family: FontFamily::Reading(ReadingFontId::Lexend),
            ..FontSettings::default()
        };
        let t = toml::to_string(&s).unwrap();
        assert!(t.contains("family = \"lexend\""), "{t}");
        assert_eq!(toml::from_str::<FontSettings>(&t).unwrap(), s);
        let partial: FontSettings = toml::from_str("size_pt = 18.0").unwrap();
        assert_eq!(partial.family, FontFamily::Sans);
        assert_eq!(
            FontFamily::Reading(ReadingFontId::AtkinsonHyperlegible).label(),
            "Atkinson Hyperlegible"
        );
    }

    #[test]
    fn fallback_chains() {
        let c = FontFamily::Reading(ReadingFontId::AtkinsonHyperlegible)
            .fallback_chain(Platform::Windows);
        assert_eq!(c[0], "Atkinson Hyperlegible");
        assert!(c.contains(&"OpenDyslexic".to_owned()));
        assert_eq!(c.last().unwrap(), "Arial");
        let m = FontFamily::Monospace.fallback_chain(Platform::Linux);
        assert_eq!(m[0], "DejaVu Sans Mono");
        assert_eq!(
            FontFamily::Sans.fallback_chain(Platform::MacOs)[0],
            "Helvetica Neue"
        );
    }

    #[test]
    fn resolve_uses_installed_fonts_and_offers_downloads() {
        let s = FontSettings {
            family: FontFamily::Reading(ReadingFontId::OpenDyslexic),
            ..FontSettings::default()
        };
        // Installed: exact family.
        let r = s.resolve(Platform::Windows, |n| {
            n.eq_ignore_ascii_case("opendyslexic")
        });
        assert_eq!(r.family.as_deref(), Some("OpenDyslexic"));
        assert!(!r.fell_back && r.download.is_none());
        assert_eq!(r.message(&s.family), "");
        // An alternate build counts as the same font.
        let r = s.resolve(Platform::Windows, |n| n == "OpenDyslexic3");
        assert!(!r.fell_back);
        // Missing: falls back to Lexend, offers the download.
        let r = s.resolve(Platform::Windows, |n| n == "Lexend" || n == "Segoe UI");
        assert_eq!(r.family.as_deref(), Some("Lexend"));
        assert!(r.fell_back);
        assert_eq!(r.download.map(|f| f.key), Some("opendyslexic"));
        assert_eq!(
            r.message(&s.family),
            "OpenDyslexic is not installed. Using Lexend. It can be downloaded, free, under the Open Font License."
        );
        // Nothing installed at all.
        let r = s.resolve(Platform::Linux, |_| false);
        assert_eq!(r.family, None);
        // Downloads off: no offer.
        let off = FontSettings {
            fetch_missing: false,
            ..s.clone()
        };
        assert_eq!(off.resolve(Platform::Linux, |_| false).download, None);
        // A named family that is missing.
        let named = FontSettings {
            family: FontFamily::Named("Comic Neue".into()),
            ..FontSettings::default()
        };
        let r = named.resolve(Platform::Windows, |n| n == "Segoe UI");
        assert_eq!(r.family.as_deref(), Some("Segoe UI"));
        assert_eq!(
            r.message(&named.family),
            "Comic Neue is not installed. Using Segoe UI."
        );
        // System UI on macOS is the toolkit default.
        let ui = FontSettings {
            family: FontFamily::SystemUi,
            ..FontSettings::default()
        };
        assert_eq!(ui.resolve(Platform::MacOs, |_| false).family, None);
    }

    #[test]
    fn validation_and_warnings() {
        assert!(FontSettings::default().validate().is_ok());
        let tiny = FontSettings {
            size_pt: 3.0,
            ..FontSettings::default()
        };
        assert_eq!(tiny.validate(), Err(FontError::Size(3.0)));
        assert_eq!(
            tiny.validate().unwrap_err().to_string(),
            "font size must be between 6 and 144 points, not 3"
        );
        assert_eq!(tiny.clamped().size_pt, MIN_SIZE_PT);
        assert_eq!(tiny.warnings().len(), 1);
        let heavy = FontSettings {
            weight: 1000,
            ..FontSettings::default()
        };
        assert_eq!(heavy.validate(), Err(FontError::Weight(1000)));
        let empty = FontSettings {
            family: FontFamily::Named(" ".into()),
            ..FontSettings::default()
        };
        assert_eq!(empty.validate(), Err(FontError::EmptyFamily));
        assert_eq!(empty.clamped().family, FontFamily::Sans);
        let nan = FontSettings {
            size_pt: f32::NAN,
            weight: 200,
            ..FontSettings::default()
        };
        assert_eq!(nan.clamped().size_pt, 14.0);
        assert_eq!(nan.warnings().len(), 1);
    }

    #[test]
    fn css_and_description() {
        let s = FontSettings {
            family: FontFamily::Reading(ReadingFontId::AtkinsonHyperlegible),
            size_pt: 16.0,
            weight: 400,
            fetch_missing: true,
        };
        let fam = s.css_font_family();
        assert!(fam.starts_with("\"Atkinson Hyperlegible\", "), "{fam}");
        assert!(fam.ends_with(", sans-serif"), "{fam}");
        let css = s.to_css(&TextSpacing::wcag(), "main");
        assert!(css.contains("font-size: 16pt;"), "{css}");
        assert!(css.contains("line-height: 1.5;"), "{css}");
        let ui = FontSettings {
            family: FontFamily::SystemUi,
            ..FontSettings::default()
        };
        let ui_fam = ui.css_font_family();
        assert!(ui_fam.starts_with("system-ui, \"Segoe UI\""), "{ui_fam}");
        assert!(!ui_fam.contains(".AppleSystemUIFont"));
        let hostile = FontSettings {
            family: FontFamily::Named("x\"; } body { color: red".into()),
            ..FontSettings::default()
        };
        assert!(!hostile.css_font_family().contains('}'));
        let d = s.describe(&TextSpacing::wcag(), Platform::Windows);
        assert_eq!(d.families[0], "Atkinson Hyperlegible");
        assert_eq!(d.line_height_pt, 24.0);
        assert!((d.letter_spacing_pt - 1.92).abs() < 1e-4);
        assert_eq!(d.paragraph_spacing_pt, 32.0);
        assert_eq!(s.summary(), "Atkinson Hyperlegible, 16 points, regular.");
    }
}
