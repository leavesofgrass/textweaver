//! The font chooser, ported from the wxDragon spike's View, Fonts dialog:
//! a list of families (the bundled ones first, marked "built in", then the
//! installed ones), then a list of sizes. The choice is saved in
//! `[reading_aids.font]` and announced. Bold is kept as it was.
//!
//! Installed families are scanned once on a background thread, so the
//! chooser opens at once.

use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

use textweaver_app::aids::fonts::{FontFamily, FontSettings};
use textweaver_fonts::{BUNDLED, system};

/// Said after a bundled family's name, so a listener knows it needs
/// nothing installed.
pub const BUILT_IN: &str = " (built in)";

/// The sizes offered, in points.
pub const SIZES: [u16; 14] = [10, 11, 12, 13, 14, 15, 16, 18, 20, 22, 24, 28, 32, 40];

/// A family the chooser offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    /// The family name Parley looks up.
    pub family: String,
    /// The text shown and read in the list.
    pub label: String,
}

/// The families to offer: bundled first, then installed ones
/// alphabetically, each once.
pub fn choices(installed: &[String]) -> Vec<Choice> {
    let mut out: Vec<Choice> = BUNDLED
        .iter()
        .map(|f| Choice {
            family: f.name.to_owned(),
            label: format!("{}{BUILT_IN}", f.name),
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
            });
        }
    }
    out
}

/// The size list's labels, and the index of the size nearest `current`.
pub fn sizes(current: f32) -> (Vec<String>, usize) {
    let labels = SIZES.iter().map(|s| format!("{s} points")).collect();
    let nearest = SIZES
        .iter()
        .enumerate()
        .min_by(|a, b| {
            (f32::from(*a.1) - current)
                .abs()
                .total_cmp(&(f32::from(*b.1) - current).abs())
        })
        .map_or(0, |(i, _)| i);
    (labels, nearest)
}

/// The settings for a family and size, keeping the weight.
pub fn chosen(previous: &FontSettings, family: &str, size: u16) -> FontSettings {
    FontSettings {
        family: FontFamily::Named(family.to_owned()),
        size_pt: f32::from(size),
        weight: previous.weight,
        fetch_missing: previous.fetch_missing,
    }
    .clamped()
}

/// What is said when the font changes.
pub fn announcement(family: &str, size: u16, bold: bool) -> String {
    let weight = if bold { "bold" } else { "regular" };
    format!("Font: {family}, {size} points, {weight}.")
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
        let c = choices(&installed);
        let n = BUNDLED.len();
        assert!(c[..n].iter().all(|x| x.label.ends_with(BUILT_IN)));
        let names: Vec<&str> = c[n..].iter().map(|x| x.family.as_str()).collect();
        assert_eq!(names, vec!["arial", "Zapf"]);
    }

    #[test]
    fn sizes_start_near_the_current_one() {
        let (labels, i) = sizes(15.2);
        assert_eq!(labels[i], "15 points");
        assert_eq!(sizes(100.0).1, SIZES.len() - 1);
    }

    #[test]
    fn a_choice_keeps_the_weight() {
        let prev = FontSettings {
            weight: 700,
            ..FontSettings::default()
        };
        let s = chosen(&prev, "Verdana", 18);
        assert_eq!(s.weight, 700);
        assert!((s.size_pt - 18.0).abs() < 0.01);
        assert_eq!(s.family, FontFamily::Named("Verdana".into()));
        assert_eq!(
            announcement("Verdana", 18, true),
            "Font: Verdana, 18 points, bold."
        );
    }
}
