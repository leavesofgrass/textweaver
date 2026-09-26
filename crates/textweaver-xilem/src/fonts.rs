//! Fonts: the bundled families loaded straight into Parley (nothing is
//! registered with the operating system), and the reader's font setting
//! as a Parley family list.

use std::sync::Arc;

use masonry::peniko::Blob;
use textweaver_app::aids::fonts::{FontSettings, Platform};

use crate::document::DocFont;

/// The interface and default document font stack: Atkinson Hyperlegible
/// Next (bundled), then the system's interface font.
pub const DEFAULT_STACK: &str = "\"Atkinson Hyperlegible Next\", system-ui, sans-serif";

/// Code: Atkinson Hyperlegible Mono (bundled), then the system's.
pub const MONO_STACK: &str = "\"Atkinson Hyperlegible Mono\", ui-monospace, monospace";

/// Every bundled font file, as blobs for Parley's font collection.
pub fn bundled_blobs() -> Vec<Blob<u8>> {
    textweaver_fonts::BUNDLED
        .iter()
        .flat_map(|family| family.faces.iter())
        .map(|face| Blob::new(Arc::new(face.data)))
        .collect()
}

/// Quotes a family name for a CSS-style list.
fn quoted(name: &str) -> String {
    if name.contains(',') || name.contains('"') {
        // Names with separators cannot be listed; skip them.
        String::new()
    } else {
        format!("\"{name}\"")
    }
}

/// The document font for the reader's `[reading_aids.font]` setting:
/// its family's fallback chain (which ends with a platform sans-serif and
/// the bundled Atkinson Hyperlegible Next), its size in points as
/// logical pixels, and bold from a weight of 600 or more.
pub fn doc_font(settings: &FontSettings) -> DocFont {
    let s = settings.clamped();
    let mut names: Vec<String> = s
        .family
        .fallback_chain(Platform::current())
        .iter()
        .map(|n| quoted(n))
        .filter(|n| !n.is_empty())
        .collect();
    names.push(s.family.css_generic().to_owned());
    DocFont {
        family: names.join(", "),
        // Points to CSS pixels: 96 per inch over 72.
        size: s.size_pt * 96.0 / 72.0,
        bold: s.weight >= 600,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use textweaver_app::aids::fonts::FontFamily;

    #[test]
    fn every_bundled_face_is_offered() {
        let n: usize = textweaver_fonts::BUNDLED
            .iter()
            .map(|f| f.faces.len())
            .sum();
        assert_eq!(bundled_blobs().len(), n);
        assert!(n >= 12, "{n}");
    }

    #[test]
    fn settings_become_a_family_list_and_pixels() {
        let s = FontSettings {
            family: FontFamily::Named("Atkinson Hyperlegible Next".into()),
            size_pt: 15.0,
            weight: 700,
            ..FontSettings::default()
        };
        let f = doc_font(&s);
        assert!(
            f.family.starts_with("\"Atkinson Hyperlegible Next\""),
            "{}",
            f.family
        );
        assert!(f.family.ends_with("sans-serif"), "{}", f.family);
        assert!((f.size - 20.0).abs() < 0.01);
        assert!(f.bold);
    }
}
