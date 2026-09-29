//! Font settings for reading views: family, size, and weight, together
//! with text spacing, for GUIs and CSS.
//!
//! Choosing and resolving a family is `textweaver-fonts`' job (its
//! `choice` module): the reading fonts Star offered, each platform's
//! fallbacks, and which family to use given what is bundled and installed.
//! This module re-exports those types, so `textweaver_aids::FontSettings`
//! is the same type the writers and the GUIs resolve, and adds what needs
//! the reading aids' [`TextSpacing`]: [`describe`] for a GUI text control
//! and [`to_css`] for HTML views.
//!
//! The saved form is `[reading_aids.font]`
//! (`textweaver_store::reading_aids::FontSettings`, plain data);
//! [`from_store`] and [`to_store`] convert between the two.

use std::fmt::Write as _;

pub use textweaver_fonts::choice::{
    FontError, FontFamily, FontResolution, FontSettings, MAX_SIZE_PT, MIN_SIZE_PT, Platform,
    READING_FONTS, ReadingFont, ReadingFontId, SMALL_SIZE_PT,
};
use textweaver_fonts::format_points;
use textweaver_store::reading_aids as saved;

use crate::spacing::TextSpacing;

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

/// A description the GUI can apply directly, with the spacing settings
/// converted to points for this size.
pub fn describe(font: &FontSettings, spacing: &TextSpacing, platform: Platform) -> FontDescription {
    let s = font.clamped();
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

/// CSS for `selector`: the font stack, size, and weight, plus the text
/// spacing rules from [`TextSpacing::to_css`].
pub fn to_css(font: &FontSettings, spacing: &TextSpacing, selector: &str) -> String {
    let s = font.clamped();
    let mut out = String::new();
    let _ = writeln!(out, "{selector} {{");
    let _ = writeln!(out, "  font-family: {};", s.css_font_family());
    let _ = writeln!(out, "  font-size: {}pt;", format_points(s.size_pt));
    let _ = writeln!(out, "  font-weight: {};", s.weight);
    let _ = writeln!(out, "}}");
    out.push_str(&spacing.to_css(selector));
    out
}

/// The font settings saved in `[reading_aids.font]`, ready to resolve.
pub fn from_store(saved: &saved::FontSettings) -> FontSettings {
    FontSettings {
        family: FontFamily::from(saved.family.as_str()),
        size_pt: saved.size_pt,
        weight: saved.weight,
        // textweaver never downloads fonts; a missing reading font is
        // named with where to get it (`[reading_aids.font] fetch_missing`
        // was removed in Wave 5).
        fetch_missing: true,
    }
}

/// `font` in the form `[reading_aids.font]` saves.
pub fn to_store(font: &FontSettings) -> saved::FontSettings {
    saved::FontSettings {
        family: font.family.key(),
        size_pt: font.size_pt,
        weight: font.weight,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_and_description() {
        let s = FontSettings {
            family: FontFamily::Reading(ReadingFontId::AtkinsonHyperlegible),
            size_pt: 16.0,
            weight: 400,
            fetch_missing: true,
        };
        let css = to_css(&s, &TextSpacing::wcag(), "main");
        assert!(css.contains("font-size: 16pt;"), "{css}");
        assert!(css.contains("line-height: 1.5;"), "{css}");
        let d = describe(&s, &TextSpacing::wcag(), Platform::Windows);
        assert_eq!(d.families[0], "Atkinson Hyperlegible");
        assert_eq!(d.line_height_pt, 24.0);
        assert!((d.letter_spacing_pt - 1.92).abs() < 1e-4);
        assert_eq!(d.paragraph_spacing_pt, 32.0);
    }

    #[test]
    fn saved_settings_convert_both_ways() {
        let s = FontSettings {
            family: FontFamily::Reading(ReadingFontId::Lexend),
            size_pt: 18.0,
            weight: 700,
            fetch_missing: true,
        };
        let saved = to_store(&s);
        assert_eq!(saved.family, "lexend");
        assert_eq!(from_store(&saved), s);
        // The defaults agree.
        assert_eq!(
            from_store(&saved::FontSettings::default()),
            FontSettings::default()
        );
        assert_eq!(
            to_store(&FontSettings::default()),
            saved::FontSettings::default()
        );
        // A family by name round-trips as its name.
        let named = FontSettings {
            family: FontFamily::Named("Comic Neue".into()),
            ..FontSettings::default()
        };
        assert_eq!(from_store(&to_store(&named)), named);
    }
}
