//! Fonts for the reading control: the bundled families registered for this
//! process, the installed families, and the choice saved in `[reading_aids.font]`.
//!
//! The pure parts (the family list, resolving a saved choice, turning a
//! choice into what wx needs) are here and unit-tested; the dialog is in
//! `font_dialog.rs`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use textweaver_aids::fonts::{from_store, to_store};
use textweaver_app::store::reading_aids::FontSettings as SavedFont;
use textweaver_fonts::{BUNDLED, FontFamily, FontSettings, Platform, system};

/// Suffix for bundled families in the family list, so a listener knows
/// they need nothing installed.
pub const BUILT_IN: &str = " (built in)";

/// A family the chooser offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    /// The family name wx creates the font with.
    pub family: String,
    /// The text shown and read in the list.
    pub label: String,
    /// Bundled with textweaver.
    pub bundled: bool,
}

/// The families to offer: bundled ones first, then installed ones in
/// alphabetical order, each once (an installed copy of a bundled family is
/// listed as bundled).
pub fn choices(installed: &[String]) -> Vec<Choice> {
    let mut out: Vec<Choice> = BUNDLED
        .iter()
        .map(|f| Choice {
            family: f.name.to_owned(),
            label: format!("{}{BUILT_IN}", f.name),
            bundled: true,
        })
        .collect();
    let mut seen: BTreeSet<String> = out.iter().map(|c| c.family.to_lowercase()).collect();
    let mut rest: Vec<&String> = installed.iter().collect();
    rest.sort_by_key(|n| n.to_lowercase());
    for name in rest {
        if seen.insert(name.to_lowercase()) {
            out.push(Choice {
                family: name.clone(),
                label: name.clone(),
                bundled: false,
            });
        }
    }
    out
}

/// Installed family names (typographic and older names, for matching),
/// scanned once on a background thread so the chooser opens at once.
pub struct Installed {
    names: Arc<OnceLock<Vec<String>>>,
    all: Arc<OnceLock<BTreeSet<String>>>,
}

impl Installed {
    /// Starts scanning the system font folders.
    pub fn scan_in_background() -> Installed {
        let names: Arc<OnceLock<Vec<String>>> = Arc::new(OnceLock::new());
        let all: Arc<OnceLock<BTreeSet<String>>> = Arc::new(OnceLock::new());
        let (n, a) = (Arc::clone(&names), Arc::clone(&all));
        std::thread::spawn(move || {
            let faces = system::scan();
            let mut every = BTreeSet::new();
            for f in &faces {
                every.insert(f.info.family.to_lowercase());
                every.insert(f.info.legacy_family.to_lowercase());
            }
            let _ = a.set(every);
            let _ = n.set(system::family_names(&faces));
        });
        Installed { names, all }
    }

    /// The family names to list; waits for the scan if it is still
    /// running (well under a second on a warm cache).
    pub fn names(&self) -> &[String] {
        self.wait();
        self.names.get().map_or(&[], Vec::as_slice)
    }

    /// Whether a family by this name is installed, without waiting: `None`
    /// while the scan is still running.
    pub fn has_now(&self, name: &str) -> Option<bool> {
        self.all.get().map(|a| a.contains(&name.to_lowercase()))
    }

    /// True when a family by this name is installed (ignoring case).
    pub fn has(&self, name: &str) -> bool {
        self.wait();
        self.all
            .get()
            .is_some_and(|a| a.contains(&name.to_lowercase()))
    }

    fn wait(&self) {
        let started = std::time::Instant::now();
        while self.names.get().is_none() && started.elapsed().as_secs() < 30 {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

/// Writes the bundled font files under `dir` (one folder per family, each
/// with its licence) and returns their paths, for registering with the
/// toolkit. Files already there are reused.
pub fn extract_bundled(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for f in BUNDLED {
        paths.extend(f.extract_to(&dir.join(f.key))?);
    }
    Ok(paths)
}

/// What wx needs to build the font: a face name, a size, and bold.
#[derive(Clone, Debug, PartialEq)]
pub struct Applied {
    /// The face name to create (`None`: the toolkit's default font).
    pub face: Option<String>,
    /// Size in whole points.
    pub size: i32,
    /// Bold (weight 600 and up).
    pub bold: bool,
    /// Something to say when the chosen family is not available.
    pub note: String,
}

/// Resolves the saved choice against what is installed (bundled families
/// count as installed) into what the reading control should use.
pub fn applied(settings: &SavedFont, installed: impl Fn(&str) -> bool) -> Applied {
    let s = from_store(settings).clamped();
    let r = s.resolve(Platform::current(), installed);
    Applied {
        note: r.message(&s.family),
        face: r.family,
        size: s.size_pt.round() as i32,
        bold: s.weight >= 600,
    }
}

/// The settings for a choice made in the chooser: `family` as listed, a
/// size, and bold or not. A weight other than bold or regular is kept when
/// bold is unchanged.
pub fn chosen(previous: &SavedFont, family: &str, size: i32, bold: bool) -> SavedFont {
    let weight = match (bold, previous.weight >= 600) {
        (true, true) | (false, false) => previous.weight,
        (true, false) => 700,
        (false, true) => 400,
    };
    to_store(
        &FontSettings {
            family: FontFamily::Named(family.to_owned()),
            size_pt: size as f32,
            weight,
            fetch_missing: previous.fetch_missing,
        }
        .clamped(),
    )
}

/// macOS: what to say when the reading font is a built-in family that
/// macOS cannot load, so it draws a system font instead. macOS loads an
/// application's own fonts only from its `.app` package
/// (`bundle_has_fonts`); elsewhere nothing is registered, and the family
/// shows only if it is installed (`installed`; `None` while the scan runs,
/// when nothing is said yet). `None` when there is nothing to say.
pub fn mac_bundled_fallback(
    face: Option<&str>,
    bundle_has_fonts: bool,
    installed: Option<bool>,
) -> Option<String> {
    let name = face?;
    let bundled = textweaver_fonts::bundled::family(name)?;
    if bundle_has_fonts || installed != Some(false) {
        return None;
    }
    Some(format!(
        "{} is built in, but macOS can load it only from the textweaver app package, so a system font is used.",
        bundled.name
    ))
}

/// The sentence announced when a font is applied: "Font: OpenDyslexic, 16
/// points, bold."
pub fn announcement(family: &str, size: i32, bold: bool) -> String {
    let weight = if bold { "bold" } else { "regular" };
    format!("Font: {family}, {size} points, {weight}.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_fallback_is_said_for_a_missing_built_in_family() {
        if BUNDLED.is_empty() {
            return;
        }
        let name = BUNDLED[0].name;
        let said = mac_bundled_fallback(Some(name), false, Some(false)).unwrap();
        assert!(said.starts_with(name), "{said}");
        assert!(said.ends_with("a system font is used."));
        // Loaded from the package, installed, still scanning, or not built
        // in: nothing to say.
        assert_eq!(mac_bundled_fallback(Some(name), true, Some(false)), None);
        assert_eq!(mac_bundled_fallback(Some(name), false, Some(true)), None);
        assert_eq!(mac_bundled_fallback(Some(name), false, None), None);
        assert_eq!(
            mac_bundled_fallback(Some("Verdana"), false, Some(false)),
            None
        );
        assert_eq!(mac_bundled_fallback(None, false, Some(false)), None);
    }

    #[test]
    fn bundled_families_come_first_and_once() {
        let installed = vec![
            "Verdana".to_owned(),
            "arial".to_owned(),
            "OpenDyslexic".to_owned(),
        ];
        let c = choices(&installed);
        let labels: Vec<&str> = c.iter().map(|c| c.label.as_str()).collect();
        if BUNDLED.is_empty() {
            assert_eq!(labels, ["arial", "OpenDyslexic", "Verdana"]);
            return;
        }
        assert_eq!(
            labels,
            [
                "Atkinson Hyperlegible Next (built in)",
                "Atkinson Hyperlegible Mono (built in)",
                "OpenDyslexic (built in)",
                "arial",
                "Verdana",
            ]
        );
        assert!(c[0].bundled && !c[3].bundled);
        assert_eq!(c[2].family, "OpenDyslexic");
    }

    #[test]
    fn saved_choices_resolve_to_a_face() {
        let s = SavedFont {
            family: "OpenDyslexic".into(),
            size_pt: 15.6,
            weight: 700,
            ..SavedFont::default()
        };
        let a = applied(&s, |_| false);
        assert_eq!(a.size, 16);
        assert!(a.bold);
        if !BUNDLED.is_empty() {
            assert_eq!(a.face.as_deref(), Some("OpenDyslexic"));
            assert_eq!(a.note, "");
        }
        // A family that is gone falls back and says so.
        let gone = SavedFont {
            family: "Gone Sans".into(),
            ..SavedFont::default()
        };
        let a = applied(&gone, |n| n == "Arial");
        assert!(
            a.note.starts_with("Gone Sans is not installed."),
            "{}",
            a.note
        );
    }

    #[test]
    fn choices_become_settings() {
        let prev = SavedFont {
            weight: 300,
            ..SavedFont::default()
        };
        let s = chosen(&prev, "Verdana", 18, false);
        assert_eq!(s.family, "Verdana");
        assert_eq!((s.size_pt, s.weight), (18.0, 300));
        assert_eq!(chosen(&prev, "Verdana", 18, true).weight, 700);
        let bold = SavedFont {
            weight: 800,
            ..SavedFont::default()
        };
        assert_eq!(chosen(&bold, "Verdana", 18, true).weight, 800);
        assert_eq!(chosen(&bold, "Verdana", 18, false).weight, 400);
        // Sizes are kept in range.
        assert_eq!(chosen(&prev, "Verdana", 500, false).size_pt, 144.0);
        assert_eq!(
            announcement("OpenDyslexic", 16, true),
            "Font: OpenDyslexic, 16 points, bold."
        );
    }

    #[test]
    fn bundled_files_are_extracted_with_licences() {
        let dir = std::env::temp_dir().join(format!("tw-gui-fonts-{}", std::process::id()));
        let paths = extract_bundled(&dir).unwrap();
        assert_eq!(paths.len(), BUNDLED.len() * 4);
        for f in BUNDLED {
            assert!(dir.join(f.key).join("OFL.txt").is_file());
        }
        let _ = std::fs::remove_dir_all(dir);
    }
}
