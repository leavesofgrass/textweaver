//! Fonts: the bundled families, and a Lexend downloaded into the data
//! folder, loaded straight into Parley (nothing is registered with the
//! operating system), and the reader's font setting as a Parley family
//! list.

use std::path::Path;
use std::sync::Arc;

use masonry::peniko::Blob;
use textweaver_app::aids::fonts::{FontSettings, Platform, from_store};
use textweaver_app::store::reading_aids::FontSettings as SavedFont;

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

/// Every downloaded reading font in `folder` (the data folder's
/// `fonts`) whose files match their pinned hashes, as blobs. Empty when
/// there is no folder or nothing was downloaded.
pub fn downloaded_blobs(folder: Option<&Path>) -> Vec<Blob<u8>> {
    let Some(folder) = folder else {
        return Vec::new();
    };
    textweaver_fonts::downloaded::DOWNLOADABLE
        .iter()
        .filter_map(|f| f.load_from(folder))
        .flatten()
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
pub fn doc_font(saved: &SavedFont) -> DocFont {
    font_for(&from_store(saved))
}

/// [`doc_font`] for settings already parsed.
pub fn font_for(settings: &FontSettings) -> DocFont {
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
    fn a_downloaded_lexend_is_offered_once_checked() {
        assert!(downloaded_blobs(None).is_empty());
        let dir = tempfile::tempdir().unwrap();
        assert!(downloaded_blobs(Some(dir.path())).is_empty());
        let lexend = dir.path().join("lexend");
        std::fs::create_dir_all(&lexend).unwrap();
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/w7l");
        for f in ["Lexend-Regular.ttf", "Lexend-Bold.ttf"] {
            std::fs::copy(fixtures.join(f), lexend.join(f)).unwrap();
        }
        assert_eq!(downloaded_blobs(Some(dir.path())).len(), 2);
        // A changed file is not registered.
        let mut data = std::fs::read(lexend.join("Lexend-Bold.ttf")).unwrap();
        data[64] ^= 1;
        std::fs::write(lexend.join("Lexend-Bold.ttf"), data).unwrap();
        assert!(downloaded_blobs(Some(dir.path())).is_empty());
    }

    #[test]
    fn settings_become_a_family_list_and_pixels() {
        let s = FontSettings {
            family: FontFamily::Named("Atkinson Hyperlegible Next".into()),
            size_pt: 15.0,
            weight: 700,
            ..FontSettings::default()
        };
        let f = font_for(&s);
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
