//! The document text's font and size (the owner's session 2, ADR-0033).
//!
//! The font list (Ctrl+D, or the Font button) offers the families, the
//! bundled ones first, marked "built in", then Lexend ("downloaded", or
//! "download, 206 KB": choosing it asks before downloading, W7l), then the
//! installed ones; the choice applies at once. Ctrl+Plus, Ctrl+Minus, and Ctrl+0 step the size
//! through [`SIZES`]. Each change is saved in `[reading_aids.font]` and
//! announced: "Font: Atkinson Hyperlegible Next.", "Text size 18 points.".
//! Bold is kept as it was.
//!
//! Installed families are scanned once on a background thread, so the
//! chooser opens at once.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use textweaver_app::aids::fonts::{FontFamily, FontSettings, from_store, to_store};
use textweaver_app::lexicon::args;
use textweaver_app::lexicon::i18n::Catalog;
use textweaver_app::store::reading_aids::FontSettings as SavedFont;
use textweaver_fonts::{BUNDLED, system};

/// The text sizes Ctrl+Plus and Ctrl+Minus step through, in points: one
/// point at a time around the usual reading sizes, then larger steps up to
/// 72 points for low vision. The store accepts 6 to 144 points; a size set
/// in `settings.toml` between two steps goes to the next one.
pub const SIZES: [u16; 22] = [
    8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 20, 22, 24, 26, 28, 32, 36, 40, 48, 56, 64, 72,
];

/// A text size key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Ctrl+Plus.
    Larger,
    /// Ctrl+Minus.
    Smaller,
    /// Ctrl+0: the standard size (the store's default).
    Reset,
}

/// Where a size is among [`SIZES`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limit {
    /// Neither end.
    None,
    /// The largest step.
    Largest,
    /// The smallest step.
    Smallest,
}

/// The size after `step` from `current`, and whether it is at an end.
pub fn stepped(current: f32, step: Step) -> (f32, Limit) {
    const EPS: f32 = 0.01;
    let first = f32::from(SIZES[0]);
    let last = f32::from(SIZES[SIZES.len() - 1]);
    let size = match step {
        Step::Larger => SIZES
            .iter()
            .map(|s| f32::from(*s))
            .find(|s| *s > current + EPS)
            .unwrap_or(last.max(current)),
        Step::Smaller => SIZES
            .iter()
            .rev()
            .map(|s| f32::from(*s))
            .find(|s| *s < current - EPS)
            .unwrap_or(first.min(current)),
        Step::Reset => SavedFont::default().size_pt,
    };
    let limit = if step == Step::Reset {
        Limit::None
    } else if size >= last - EPS {
        Limit::Largest
    } else if size <= first + EPS {
        Limit::Smallest
    } else {
        Limit::None
    };
    (size, limit)
}

/// The saved font with a new size, keeping the family and weight.
pub fn with_size(previous: &SavedFont, size_pt: f32) -> SavedFont {
    SavedFont {
        size_pt,
        ..previous.clone()
    }
}

/// What is said when the size changes: "Text size 18 points."
pub fn size_message(c: &Catalog, size_pt: f32, limit: Limit) -> String {
    // Whole points: the steps are whole, and a size typed in the settings
    // is said to the nearest point.
    let size = size_pt.round() as i64;
    let id = match limit {
        Limit::None => "gui-text-size",
        Limit::Largest => "gui-text-size-largest",
        Limit::Smallest => "gui-text-size-smallest",
    };
    c.fmt(id, &args!["size" => size])
}

/// What is said when the font changes: "Font: Atkinson Hyperlegible
/// Next."
pub fn font_message(c: &Catalog, family: &str) -> String {
    c.fmt("gui-font", &args!["family" => family])
}

/// A family the chooser offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    /// The family name Parley looks up.
    pub family: String,
    /// The text shown and read in the list.
    pub label: String,
}

/// The families to offer: bundled first, then the reading fonts
/// textweaver downloads (Lexend) unless installed, then installed ones
/// alphabetically, each once.
///
/// A bundled family's label says so ("Atkinson Hyperlegible Next (built
/// in)", `gui-font-built-in`), so a listener knows it needs nothing
/// installed; a downloadable one says whether it is downloaded in
/// `folder` ("Lexend (downloaded)") or what choosing it downloads
/// ("Lexend (download, 206 KB)").
pub fn choices(c: &Catalog, installed: &[String], folder: Option<&Path>) -> Vec<Choice> {
    let mut out: Vec<Choice> = BUNDLED
        .iter()
        .map(|f| Choice {
            family: f.name.to_owned(),
            label: c.fmt("gui-font-built-in", &args!["family" => f.name]),
        })
        .collect();
    let mut seen: BTreeSet<String> = out.iter().map(|c| c.family.to_lowercase()).collect();
    for f in textweaver_fonts::downloaded::DOWNLOADABLE {
        if installed.iter().any(|n| n.eq_ignore_ascii_case(f.name)) {
            continue; // listed with the installed families
        }
        let label = if folder.is_some_and(|d| f.is_installed_in(d)) {
            c.fmt("gui-font-downloaded", &args!["family" => f.name])
        } else {
            c.fmt(
                "gui-font-to-download",
                &args!["family" => f.name, "kb" => f.kilobytes()],
            )
        };
        seen.insert(f.name.to_lowercase());
        out.push(Choice {
            family: f.name.to_owned(),
            label,
        });
    }
    let mut rest: Vec<&String> = installed.iter().collect();
    rest.sort_by_key(|n| n.to_lowercase());
    for name in rest {
        if seen.insert(name.to_lowercase()) {
            out.push(Choice {
                family: name.clone(),
                label: name.clone(),
            });
        }
    }
    out
}

/// The saved font with a new family, keeping the size and weight.
pub fn with_family(previous: &SavedFont, family: &str) -> SavedFont {
    to_store(&family_font(&from_store(previous), family))
}

/// [`with_family`] for settings already parsed.
pub fn family_font(previous: &FontSettings, family: &str) -> FontSettings {
    FontSettings {
        family: FontFamily::Named(family.to_owned()),
        ..previous.clone()
    }
    .clamped()
}
/// Installed family names, scanned in the background.
#[derive(Clone, Default)]
pub struct Installed {
    names: Arc<OnceLock<Vec<String>>>,
}

impl Installed {
    /// Starts the scan.
    pub fn scan_in_background() -> Installed {
        let names: Arc<OnceLock<Vec<String>>> = Arc::new(OnceLock::new());
        let n = Arc::clone(&names);
        std::thread::spawn(move || {
            let faces = system::scan();
            let _ = n.set(system::family_names(&faces));
        });
        Installed { names }
    }

    /// The names found so far (none while the scan runs).
    pub fn names(&self) -> &[String] {
        self.names.get().map_or(&[], Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_families_come_first_and_once() {
        let installed = vec![
            "Zapf".to_owned(),
            "arial".to_owned(),
            "Atkinson Hyperlegible Next".to_owned(),
        ];
        let c = choices(&Catalog::english(), &installed, None);
        let n = BUNDLED.len();
        assert!(c[..n].iter().all(|x| x.label.ends_with(" (built in)")));
        // Lexend next, saying what choosing it downloads.
        assert_eq!(c[n].label, "Lexend (download, 206 KB)");
        let names: Vec<&str> = c[n + 1..].iter().map(|x| x.family.as_str()).collect();
        assert_eq!(names, vec!["arial", "Zapf"]);
    }

    #[test]
    fn a_downloaded_or_installed_lexend_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let lexend = dir.path().join("lexend");
        std::fs::create_dir_all(&lexend).unwrap();
        let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/w7l");
        for f in ["Lexend-Regular.ttf", "Lexend-Bold.ttf"] {
            std::fs::copy(fixtures.join(f), lexend.join(f)).unwrap();
        }
        let c = choices(&Catalog::english(), &[], Some(dir.path()));
        assert!(
            c.iter()
                .any(|x| x.label == "Lexend (downloaded)" && x.family == "Lexend")
        );
        // Installed on the system: listed once, as an installed family.
        let c = choices(&Catalog::english(), &["Lexend".to_owned()], None);
        let lexends: Vec<&str> = c
            .iter()
            .filter(|x| x.family == "Lexend")
            .map(|x| x.label.as_str())
            .collect();
        assert_eq!(lexends, ["Lexend"]);
    }

    #[test]
    fn size_keys_step_through_the_sizes() {
        assert_eq!(stepped(14.0, Step::Larger), (15.0, Limit::None));
        assert_eq!(stepped(14.0, Step::Smaller), (13.0, Limit::None));
        // Between two steps: to the next one either way.
        assert_eq!(stepped(14.5, Step::Larger), (15.0, Limit::None));
        assert_eq!(stepped(14.5, Step::Smaller), (14.0, Limit::None));
        assert_eq!(stepped(64.0, Step::Larger), (72.0, Limit::Largest));
        assert_eq!(stepped(72.0, Step::Larger), (72.0, Limit::Largest));
        assert_eq!(stepped(9.0, Step::Smaller), (8.0, Limit::Smallest));
        // Beyond the steps (set in settings.toml): kept, and said as an end.
        assert_eq!(stepped(100.0, Step::Larger), (100.0, Limit::Largest));
        assert_eq!(stepped(6.0, Step::Smaller), (6.0, Limit::Smallest));
        assert_eq!(
            stepped(40.0, Step::Reset),
            (SavedFont::default().size_pt, Limit::None)
        );
    }

    #[test]
    fn size_and_font_changes_are_said_in_words() {
        let c = textweaver_app::lexicon::i18n::Catalog::english();
        assert_eq!(size_message(&c, 18.0, Limit::None), "Text size 18 points.");
        assert_eq!(
            size_message(&c, 72.0, Limit::Largest),
            "Text size 72 points, the largest."
        );
        assert_eq!(
            size_message(&c, 8.0, Limit::Smallest),
            "Text size 8 points, the smallest."
        );
        assert_eq!(
            font_message(&c, "Atkinson Hyperlegible Next"),
            "Font: Atkinson Hyperlegible Next."
        );
    }

    #[test]
    fn a_family_keeps_the_size_and_weight() {
        let prev = FontSettings {
            weight: 700,
            size_pt: 18.0,
            ..FontSettings::default()
        };
        let s = family_font(&prev, "Verdana");
        assert_eq!(s.weight, 700);
        assert!((s.size_pt - 18.0).abs() < 0.01);
        assert_eq!(s.family, FontFamily::Named("Verdana".into()));
        let saved = with_size(&SavedFont::default(), 20.0);
        assert!((saved.size_pt - 20.0).abs() < 0.01);
        assert_eq!(saved.family, SavedFont::default().family);
    }
}
