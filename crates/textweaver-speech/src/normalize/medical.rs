//! The medical lexicon: drug names, clinical terms, eponyms and safe dosing
//! abbreviations, respelled for reading aloud (health sciences report,
//! sections 2 and 7 item 5).
//!
//! Off by default ([`MedicalLexiconConfig::enabled`]); a normalization
//! option, meant for a health sciences profile. In the chain it runs after
//! the user's pronunciation lexicon and before the community lexicon, so the
//! user's own entries always win.
//!
//! **Data.** A bundled file, `medical-en.toml` beside this module, compiled
//! into the program (a few kilobytes, so it is not packed): `[pronunciations]`
//! and `[dosing]` tables of `term = "spoken form"`, and a `[tall_man]` list
//! of drug names. A per-user overlay file
//! ([`MedicalLexiconConfig::overlay`]) in the same format, or with plain
//! `term = "spoken form"` lines at the top, wins over the bundled entries.
//!
//! **Matching.** One hash lookup per word. A key in lower case matches the
//! word in any capitalization; a key with a capital letter matches only as
//! written ("PRN", never "prn"). A hyphenated word is looked up whole, then
//! part by part; a possessive ("Raynaud's") is looked up without its "'s",
//! which stays as written. Abbreviations on the error-prone list
//! ([`ERROR_PRONE`]) are never replaced, even by an overlay entry: the
//! abbreviations step spells them.
//!
//! **Tall Man names** ("hydrOXYzine", "DOPamine") are drug names printed
//! with capitals to tell look-alikes apart. Read by their capitals they
//! come out as "hydr OX Yzine", so they are kept whole: with the lexicon on
//! they are read as their respelling, or as the plain name when there is
//! none, and split caps never cuts one ([`is_tall_man`]), lexicon on or off.
//!
//! Each replacement is an `Expanded` span over its source word, so the
//! highlight stays on the word.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use regex::Regex;
use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange, OffsetMap, SpokenBuilder};

use super::Transform;
use super::abbreviations::ERROR_PRONE;

/// The bundled data file's text.
pub const BUNDLED: &str = include_str!("medical-en.toml");

/// Settings for the medical lexicon (`[normalization.medical_lexicon]`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MedicalLexiconConfig {
    /// Apply the lexicon (off by default).
    pub enabled: bool,
    /// The user's overlay file, whose entries win over the bundled ones.
    /// `None` reads none. A file that is missing or cannot be read is
    /// logged and skipped.
    pub overlay: Option<PathBuf>,
}

/// Parsed lexicon entries.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MedicalEntries {
    /// Keys with a capital letter, matched as written.
    exact: HashMap<String, String>,
    /// Keys in lower case, matched in any capitalization.
    folded: HashMap<String, String>,
    /// Tall Man names, in lower case.
    tall_man: HashSet<String>,
}

/// True when `word` is on the error-prone list in any capitalization.
fn error_prone(word: &str) -> bool {
    ERROR_PRONE.iter().any(|e| e.eq_ignore_ascii_case(word))
}

impl MedicalEntries {
    /// Parses a lexicon file: `[pronunciations]` and `[dosing]` tables and
    /// top-level `term = "spoken form"` pairs, and a `[tall_man]` table
    /// with a `names` list. Other keys are ignored. Later entries replace
    /// earlier ones.
    ///
    /// # Errors
    ///
    /// The TOML parser's message when the text is not TOML.
    pub fn parse(text: &str) -> Result<Self, String> {
        let table: toml::Table = toml::from_str(text).map_err(|e| e.to_string())?;
        let mut out = MedicalEntries::default();
        out.add_pairs(&table);
        for section in ["pronunciations", "dosing"] {
            if let Some(toml::Value::Table(t)) = table.get(section) {
                out.add_pairs(t);
            }
        }
        if let Some(toml::Value::Table(t)) = table.get("tall_man")
            && let Some(toml::Value::Array(names)) = t.get("names")
        {
            for n in names.iter().filter_map(toml::Value::as_str) {
                let n = n.trim().to_lowercase();
                if !n.is_empty() {
                    out.tall_man.insert(n);
                }
            }
        }
        Ok(out)
    }

    fn add_pairs(&mut self, table: &toml::Table) {
        for (key, value) in table {
            let Some(spoken) = value.as_str() else {
                continue;
            };
            let (key, spoken) = (key.trim(), spoken.trim());
            if key.is_empty()
                || spoken.is_empty()
                || key.contains(char::is_whitespace)
                || error_prone(key)
            {
                continue;
            }
            if key.chars().any(char::is_uppercase) {
                self.exact.insert(key.to_owned(), spoken.to_owned());
            } else {
                self.folded.insert(key.to_owned(), spoken.to_owned());
            }
        }
    }

    /// Number of entries with a spoken form (Tall Man names not counted).
    pub fn len(&self) -> usize {
        self.exact.len() + self.folded.len()
    }

    /// True when there are no entries with a spoken form.
    pub fn is_empty(&self) -> bool {
        self.exact.is_empty() && self.folded.is_empty()
    }

    /// The spoken form for `word`: an exact entry, else a lower-case entry.
    fn spoken(&self, word: &str, lower: &str) -> Option<&str> {
        self.exact
            .get(word)
            .or_else(|| self.folded.get(lower))
            .map(String::as_str)
    }
}

/// The bundled entries, parsed once.
pub fn bundled() -> &'static Arc<MedicalEntries> {
    static BUNDLED_ENTRIES: OnceLock<Arc<MedicalEntries>> = OnceLock::new();
    BUNDLED_ENTRIES.get_or_init(|| {
        Arc::new(MedicalEntries::parse(BUNDLED).unwrap_or_else(|e| {
            log::error!("the bundled medical lexicon does not parse: {e}");
            MedicalEntries::default()
        }))
    })
}

/// True when `word` is a Tall Man spelling of a bundled drug name: mixed
/// capitals over a name on the list ("hydrOXYzine", "DOPamine"). Split
/// caps keeps such words whole.
pub fn is_tall_man(word: &str) -> bool {
    word.chars().any(char::is_uppercase)
        && word.chars().any(char::is_lowercase)
        && with_lower(word, |lower| bundled().tall_man.contains(lower))
}

/// Calls `f` with `word` in lower case. Allocates nothing for a word
/// already in lower case or an ASCII word of up to 64 bytes.
fn with_lower<R>(word: &str, f: impl FnOnce(&str) -> R) -> R {
    if !word.chars().any(char::is_uppercase) {
        return f(word);
    }
    let mut buf = [0u8; 64];
    if word.is_ascii()
        && let Some(b) = buf.get_mut(..word.len())
    {
        b.copy_from_slice(word.as_bytes());
        b.make_ascii_lowercase();
        if let Ok(s) = std::str::from_utf8(b) {
            return f(s);
        }
    }
    f(&word.to_lowercase())
}

/// Reads an overlay file. `None` (logged) when it cannot be read or parsed.
pub fn load_overlay(path: &Path) -> Option<MedicalEntries> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            log::warn!(
                "cannot read the medical lexicon overlay {}: {e}",
                path.display()
            );
            return None;
        }
    };
    match MedicalEntries::parse(&text) {
        Ok(entries) => Some(entries),
        Err(e) => {
            log::warn!(
                "the medical lexicon overlay {} is not valid: {e}",
                path.display()
            );
            None
        }
    }
}

/// The medical lexicon transform.
#[derive(Clone)]
pub struct MedicalLexicon {
    bundled: Arc<MedicalEntries>,
    overlay: Option<Arc<MedicalEntries>>,
}

impl std::fmt::Debug for MedicalLexicon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MedicalLexicon")
            .field("bundled", &self.bundled.len())
            .field("overlay", &self.overlay.as_ref().map(|o| o.len()))
            .finish()
    }
}

fn word_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"[\p{L}\p{N}]+(?:['\u{2019}-][\p{L}\p{N}]+)*").unwrap_or_else(|e| {
            panic!("bad built-in pattern: {e}");
        })
    })
}

impl MedicalLexicon {
    /// The bundled lexicon with an optional overlay that wins over it.
    pub fn new(bundled: Arc<MedicalEntries>, overlay: Option<MedicalEntries>) -> Self {
        MedicalLexicon {
            bundled,
            overlay: overlay.map(Arc::new),
        }
    }

    /// The lexicon for `config`, or `None` when it is off.
    pub fn from_config(config: &MedicalLexiconConfig) -> Option<Self> {
        if !config.enabled {
            return None;
        }
        let overlay = config.overlay.as_deref().and_then(load_overlay);
        Some(Self::new(Arc::clone(bundled()), overlay))
    }

    /// The spoken form for one word as written, if the lexicon has one.
    pub fn lookup(&self, word: &str) -> Option<&str> {
        // Error-prone abbreviations were left out when the entries were
        // read, in every capitalization, so no check is needed here.
        with_lower(word, |lower| {
            if let Some(o) = &self.overlay
                && let Some(s) = o.spoken(word, lower)
            {
                return Some(s);
            }
            if let Some(s) = self.bundled.spoken(word, lower) {
                return Some(s);
            }
            // A Tall Man spelling without a respelling: its plain name.
            if lower == word {
                return None;
            }
            self.bundled
                .tall_man
                .get(lower)
                .or_else(|| self.overlay.as_ref().and_then(|o| o.tall_man.get(lower)))
                .map(String::as_str)
        })
    }

    /// The spoken form for `word` and the bytes of `word` it replaces: the
    /// whole word, or the word without a possessive "'s".
    fn lookup_word<'a>(&'a self, word: &str) -> Option<(&'a str, usize)> {
        if let Some(s) = self.lookup(word) {
            return Some((s, word.len()));
        }
        for suffix in ["'s", "\u{2019}s"] {
            if let Some(stem) = word.strip_suffix(suffix)
                && let Some(s) = self.lookup(stem)
            {
                return Some((s, stem.len()));
            }
        }
        None
    }
}

impl Transform for MedicalLexicon {
    fn name(&self) -> &'static str {
        "medical_lexicon"
    }

    fn apply(&self, input: &str) -> (String, OffsetMap) {
        self.apply_changed(input)
            .unwrap_or_else(|| super::identity(input))
    }

    fn apply_changed(&self, input: &str) -> Option<(String, OffsetMap)> {
        let mut b = SpokenBuilder::new();
        let mut done_byte = 0usize;
        let mut done_char = 0usize;
        let mut any = false;
        let mut replace = |b: &mut SpokenBuilder, start: usize, len: usize, spoken: &str| {
            if start > done_byte {
                let s = &input[done_byte..start];
                b.push_literal(s, CharPos(done_char));
                done_char += s.chars().count();
            }
            let n = input[start..start + len].chars().count();
            b.push_expanded(spoken, CharRange::new(done_char, done_char + n));
            done_char += n;
            done_byte = start + len;
        };
        for m in word_re().find_iter(input) {
            let word = m.as_str();
            if let Some((spoken, len)) = self.lookup_word(word) {
                replace(&mut b, m.start(), len, spoken);
                any = true;
                continue;
            }
            if !word.contains('-') {
                continue;
            }
            let mut part_start = m.start();
            for part in word.split('-') {
                if let Some((spoken, len)) = self.lookup_word(part) {
                    replace(&mut b, part_start, len, spoken);
                    any = true;
                }
                part_start += part.len() + 1;
            }
        }
        if !any {
            return None;
        }
        if done_byte < input.len() {
            b.push_literal(&input[done_byte..], CharPos(done_char));
        }
        Some(b.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on() -> MedicalLexicon {
        MedicalLexicon::new(Arc::clone(bundled()), None)
    }

    fn apply(l: &MedicalLexicon, s: &str) -> String {
        let (out, map) = l.apply(s);
        map.check_invariants(&out)
            .unwrap_or_else(|e| panic!("{e} for {s:?} -> {out:?}"));
        out
    }

    #[test]
    fn the_bundled_file_parses() {
        let b = bundled();
        assert!(b.len() >= 50, "{}", b.len());
        assert!(b.tall_man.len() >= 40, "{}", b.tall_man.len());
        assert_eq!(b.exact.get("PRN").map(String::as_str), Some("as needed"));
        // Every Tall Man name is lower case and one word.
        assert!(b.tall_man.iter().all(|n| n.chars().all(char::is_lowercase)));
    }

    #[test]
    fn keys_in_lower_case_match_any_capitalization() {
        let l = on();
        assert_eq!(apply(&l, "Warfarin"), "WAR-fuh-rin");
        assert_eq!(
            apply(&l, "WARFARIN, warfarin."),
            "WAR-fuh-rin, WAR-fuh-rin."
        );
        // Keys with capitals match only as written.
        assert_eq!(apply(&l, "PRN or prn"), "as needed or prn");
        assert_eq!(apply(&l, "q6h and Q6H"), "every 6 hours and every 6 hours");
        assert_eq!(apply(&l, "nothing medical"), "nothing medical");
    }

    #[test]
    fn tall_man_names_are_kept_whole() {
        let l = on();
        assert_eq!(apply(&l, "hydrOXYzine"), "hye-DROK-sih-zeen");
        assert_eq!(apply(&l, "hydrALAZINE"), "hye-DRAL-uh-zeen");
        assert_eq!(apply(&l, "DOPamine"), "DOH-puh-meen");
        // No respelling: the plain name.
        assert_eq!(
            apply(&l, "cycloSPORINE and cycloSERINE"),
            "cyclosporine and cycloserine"
        );
        assert_eq!(apply(&l, "cyclosporine"), "cyclosporine");
        assert!(is_tall_man("hydrOXYzine"));
        assert!(is_tall_man("DOBUTamine"));
        assert!(!is_tall_man("hydroxyzine"));
        assert!(!is_tall_man("HYDROXYZINE"));
        assert!(!is_tall_man("iPhone"));
    }

    #[test]
    fn possessives_and_hyphens() {
        let l = on();
        assert_eq!(apply(&l, "Raynaud's"), "ray-NOH's");
        assert_eq!(
            apply(&l, "Guillain-Barré syndrome"),
            "ghee-YAN bah-RAY syndrome"
        );
        assert_eq!(apply(&l, "Sjögren\u{2019}s"), "SHOW-grin\u{2019}s");
        assert_eq!(apply(&l, "warfarin-induced"), "WAR-fuh-rin-induced");
    }

    #[test]
    fn error_prone_abbreviations_are_never_replaced() {
        let overlay = MedicalEntries::parse(
            "QD = \"every day\"\nqod = \"every other day\"\nMS = \"morphine\"\n",
        )
        .unwrap();
        assert!(overlay.is_empty());
        let l = MedicalLexicon::new(Arc::clone(bundled()), Some(overlay));
        assert_eq!(apply(&l, "QD QOD MS"), "QD QOD MS");
    }

    #[test]
    fn the_overlay_wins() {
        let overlay = MedicalEntries::parse(
            "warfarin = \"WAR-far-in\"\n[pronunciations]\nmetformin = \"met-FOR-min-two\"\n[dosing]\nPRN = \"when needed\"\n",
        )
        .unwrap();
        let l = MedicalLexicon::new(Arc::clone(bundled()), Some(overlay));
        assert_eq!(
            apply(&l, "warfarin, metformin PRN, ibuprofen"),
            "WAR-far-in, met-FOR-min-two when needed, eye-byoo-PROH-fen"
        );
    }

    #[test]
    fn the_overlay_file_is_read_from_the_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("medical-lexicon.toml");
        std::fs::write(&path, "warfarin = \"WAR-far-in\"\n").unwrap();
        let config = MedicalLexiconConfig {
            enabled: true,
            overlay: Some(path),
        };
        let l = MedicalLexicon::from_config(&config).unwrap();
        assert_eq!(apply(&l, "warfarin"), "WAR-far-in");
        // A missing or broken file leaves the bundled entries.
        let missing = MedicalLexiconConfig {
            enabled: true,
            overlay: Some(dir.path().join("missing.toml")),
        };
        let l = MedicalLexicon::from_config(&missing).unwrap();
        assert_eq!(apply(&l, "warfarin"), "WAR-fuh-rin");
        let broken = dir.path().join("broken.toml");
        std::fs::write(&broken, "warfarin = ").unwrap();
        assert!(load_overlay(&broken).is_none());
        assert!(MedicalLexicon::from_config(&MedicalLexiconConfig::default()).is_none());
    }

    #[test]
    fn replacements_map_to_their_words() {
        let l = on();
        let text = "Give warfarin now.";
        let (out, map) = l.apply(text);
        assert_eq!(out, "Give WAR-fuh-rin now.");
        let at = out.find("fuh").unwrap() as u32;
        assert_eq!(map.to_source(&out, at..at + 3), Some(CharRange::new(5, 13)));
        let now = out.find("now").unwrap() as u32;
        assert_eq!(
            map.to_source(&out, now..now + 3),
            Some(CharRange::new(14, 17))
        );
    }
}
