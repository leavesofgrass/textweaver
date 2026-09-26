//! The community pronunciation lexicon: the plain-text entries of the IBMTTS
//! community dictionaries (`third_party/ibmtts-dictionaries/`, CC0; see its
//! README and ADR-0007), applied as a normalization transform for engines
//! that do not normalize text themselves (espeak-ng, SAPI voices, Omnivox).
//! ETI-Eloquence loads the same files natively through ECI instead.
//!
//! Off by default ([`CommunityLexiconConfig::enabled`]); a normalization
//! option.
//!
//! **What is used.** Each dictionary line is `word<TAB>respelling`, in
//! Windows-1252 with CRLF line ends. Entries whose respelling starts with a
//! backquote are ECI phoneme strings (`` `[.1In.0It] ``), meaningless to
//! other engines, and are skipped. ECI stress annotations inside a
//! respelling (`` em `0 box ``) are removed, and an entry that still has a
//! backquote after that is skipped. The main (`<LANG>main.dic`) and
//! abbreviation (`<LANG>abbr.dic`) dictionaries are read; the root
//! dictionary is not, because it holds phoneme strings only (68,413 of its
//! 68,414 US English entries) and its entries apply inside inflected words,
//! which a whole-word lexicon cannot reproduce.
//!
//! **Matching.** Whole words, case-sensitive, as ECI matches them (the
//! dictionary lists "Airbnb" and "airbnb" separately). A capitalized word not
//! found as written is looked up in lower case too, so a sentence-initial
//! "Omg" finds "omg". A hyphenated word not found whole is looked up part by
//! part ("JPEG-compressed" finds "JPEG"). Each replacement is an `Expanded`
//! span over its source word, so highlighting stays exact.
//!
//! **Where the files are** ([`find_dir`]): the configured directory, else
//! `TEXTWEAVER_ECI_DICTIONARIES` (the variable the Eloquence backend reads,
//! so one setting serves both; `off` is ignored here, since the lexicon has
//! its own switch), else an `ibmtts-dictionaries` folder beside the
//! executable, else the repository's `third_party/ibmtts-dictionaries` when
//! running from a checkout.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use regex::Regex;
use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange, OffsetMap, SpokenBuilder};

use super::Transform;

/// Environment variable naming the dictionary directory (shared with the
/// Eloquence backend).
pub const DICTIONARIES_ENV: &str = "TEXTWEAVER_ECI_DICTIONARIES";

/// The folder name beside the executable.
pub const DIR_NAME: &str = "ibmtts-dictionaries";

/// Settings for the community lexicon (`[normalization.community_lexicon]`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CommunityLexiconConfig {
    /// Apply the lexicon (off by default).
    pub enabled: bool,
    /// Directory holding the `.dic` files; `None` searches ([`find_dir`]).
    pub dir: Option<PathBuf>,
    /// ECI's three-letter language code in the file names: `ENU` (US
    /// English, the default) or `DEU` (German).
    pub language: String,
}

impl Default for CommunityLexiconConfig {
    fn default() -> Self {
        CommunityLexiconConfig {
            enabled: false,
            dir: None,
            language: "ENU".to_owned(),
        }
    }
}

/// The dictionary directory: `configured` if given, else the search
/// described in the module docs. `None` when no directory with `.dic` files
/// is found.
pub fn find_dir(configured: Option<&Path>) -> Option<PathBuf> {
    if let Some(d) = configured {
        return Some(d.to_owned());
    }
    let has_dics = |p: &Path| {
        std::fs::read_dir(p).is_ok_and(|mut it| {
            it.any(|e| {
                e.is_ok_and(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .to_ascii_lowercase()
                        .ends_with(".dic")
                })
            })
        })
    };
    if let Some(v) = std::env::var_os(DICTIONARIES_ENV) {
        let p = PathBuf::from(&v);
        if !v.eq_ignore_ascii_case("off") && has_dics(&p) {
            return Some(p);
        }
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        // Beside the executable, or beside its parent (test binaries live
        // in target/<profile>/deps).
        for d in [Some(dir), dir.parent()].into_iter().flatten() {
            let p = d.join(DIR_NAME);
            if has_dics(&p) {
                return Some(p);
            }
        }
    }
    let checkout =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../third_party/ibmtts-dictionaries");
    has_dics(&checkout).then_some(checkout)
}

/// Removes ECI stress annotations (`` `0 `` to `` `9 ``) and extra spaces.
/// `None` when the respelling is a phoneme string or keeps a backquote.
fn respelling(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.starts_with('`') {
        return None;
    }
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '`' && chars.peek().is_some_and(char::is_ascii_digit) {
            chars.next();
            continue;
        }
        out.push(c);
    }
    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    (!out.is_empty() && !out.contains('`')).then_some(out)
}

/// Parses dictionary text (already decoded): `word<TAB>respelling` lines.
/// The first entry for a word wins.
pub fn parse_entries(text: &str, into: &mut HashMap<String, String>) {
    for line in text.lines() {
        let Some((word, rest)) = line.split_once('\t') else {
            continue;
        };
        let word = word.trim();
        if word.is_empty() || word.contains(char::is_whitespace) {
            continue;
        }
        if let Some(r) = respelling(rest) {
            into.entry(word.to_owned()).or_insert(r);
        }
    }
}

/// Reads the main and abbreviation dictionaries for `language` from `dir`.
/// Missing files are skipped; the result may be empty.
pub fn load(dir: &Path, language: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return map;
    };
    let files: Vec<(String, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|e| (e.file_name().to_string_lossy().to_lowercase(), e.path()))
        .collect();
    let lang = language.to_lowercase();
    for suffix in ["main.dic", "abbr.dic"] {
        let want = format!("{lang}{suffix}");
        let Some((_, path)) = files.iter().find(|(n, _)| *n == want) else {
            continue;
        };
        match std::fs::read(path) {
            Ok(bytes) => {
                let (text, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
                parse_entries(&text, &mut map);
            }
            Err(e) => log::warn!("cannot read {}: {e}", path.display()),
        }
    }
    map
}

type Cache = Mutex<HashMap<(PathBuf, String), Arc<HashMap<String, String>>>>;

/// [`load`], cached per directory and language for the process (pipelines
/// are rebuilt whenever a speech setting changes).
pub fn load_cached(dir: &Path, language: &str) -> Arc<HashMap<String, String>> {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let cache = CACHE.get_or_init(Cache::default);
    let key = (dir.to_owned(), language.to_ascii_uppercase());
    let mut c = cache.lock().unwrap_or_else(|p| p.into_inner());
    Arc::clone(
        c.entry(key)
            .or_insert_with(|| Arc::new(load(dir, language))),
    )
}

/// The community lexicon transform.
#[derive(Clone)]
pub struct CommunityLexicon {
    entries: Arc<HashMap<String, String>>,
}

impl std::fmt::Debug for CommunityLexicon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommunityLexicon")
            .field("entries", &self.entries.len())
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

impl CommunityLexicon {
    /// A lexicon over `entries` (`word -> respelling`).
    pub fn new(entries: Arc<HashMap<String, String>>) -> Self {
        CommunityLexicon { entries }
    }

    /// The lexicon for `config`, or `None` when it is off, no directory is
    /// found, or the directory has no usable entries.
    pub fn from_config(config: &CommunityLexiconConfig) -> Option<Self> {
        if !config.enabled {
            return None;
        }
        let dir = find_dir(config.dir.as_deref())?;
        let entries = load_cached(&dir, &config.language);
        (!entries.is_empty()).then(|| Self::new(entries))
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when there are no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The respelling for `word` as written, or for a capitalized word in
    /// lower case.
    fn lookup(&self, word: &str) -> Option<&str> {
        if let Some(r) = self.entries.get(word) {
            return Some(r);
        }
        let mut chars = word.chars();
        let first = chars.next()?;
        if first.is_uppercase() && chars.clone().all(|c| !c.is_uppercase()) {
            let lower: String = first.to_lowercase().chain(chars).collect();
            return self.entries.get(&lower).map(String::as_str);
        }
        None
    }
}

impl Transform for CommunityLexicon {
    fn name(&self) -> &'static str {
        "community_lexicon"
    }

    fn apply(&self, input: &str) -> (String, OffsetMap) {
        let mut b = SpokenBuilder::new();
        let mut done_byte = 0usize;
        let mut done_char = 0usize;
        let mut any = false;
        // Literal text from `done` up to byte `to`.
        let flush =
            |b: &mut SpokenBuilder, done_byte: &mut usize, done_char: &mut usize, to: usize| {
                if to > *done_byte {
                    let s = &input[*done_byte..to];
                    b.push_literal(s, CharPos(*done_char));
                    *done_char += s.chars().count();
                    *done_byte = to;
                }
            };
        for m in word_re().find_iter(input) {
            let word = m.as_str();
            if let Some(r) = self.lookup(word) {
                flush(&mut b, &mut done_byte, &mut done_char, m.start());
                let n = word.chars().count();
                b.push_expanded(r, CharRange::new(done_char, done_char + n));
                done_char += n;
                done_byte = m.end();
                any = true;
                continue;
            }
            if !word.contains('-') {
                continue;
            }
            // Part by part: "JPEG-compressed".
            let mut part_start = m.start();
            for part in word.split('-') {
                let part_end = part_start + part.len();
                if let Some(r) = self.lookup(part) {
                    flush(&mut b, &mut done_byte, &mut done_char, part_start);
                    let n = part.chars().count();
                    b.push_expanded(r, CharRange::new(done_char, done_char + n));
                    done_char += n;
                    done_byte = part_end;
                    any = true;
                }
                part_start = part_end + 1;
            }
        }
        if !any {
            return super::identity(input);
        }
        flush(&mut b, &mut done_byte, &mut done_char, input.len());
        b.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lexicon(pairs: &[(&str, &str)]) -> CommunityLexicon {
        let mut m = HashMap::new();
        for (k, v) in pairs {
            m.insert((*k).to_owned(), (*v).to_owned());
        }
        CommunityLexicon::new(Arc::new(m))
    }

    fn apply(l: &CommunityLexicon, s: &str) -> String {
        let (out, map) = l.apply(s);
        map.check_invariants(&out).unwrap();
        out
    }

    #[test]
    fn parsing_skips_phonemes_and_strips_stress_marks() {
        let mut m = HashMap::new();
        parse_entries(
            "tabindex\tTab Index\r\ninit\t`[.1In.0It]\r\nmbox\tem `0 box\r\nbad\tx `[y]\r\n\
             no tab here\r\nomg\toe em jee\r\nomg\tsecond\r\n",
            &mut m,
        );
        assert_eq!(m.get("tabindex").map(String::as_str), Some("Tab Index"));
        assert_eq!(m.get("mbox").map(String::as_str), Some("em box"));
        assert_eq!(m.get("omg").map(String::as_str), Some("oe em jee"));
        assert!(!m.contains_key("init"));
        assert!(!m.contains_key("bad"));
        assert_eq!(m.len(), 3);
    }

    #[test]
    fn whole_words_case_sensitive_with_capitalized_fallback() {
        let l = lexicon(&[("omg", "oe em jee"), ("IV", "four"), ("JPEG", "jeigh peg")]);
        assert_eq!(apply(&l, "omg, that is IV."), "oe em jee, that is four.");
        assert_eq!(apply(&l, "Omg!"), "oe em jee!");
        assert_eq!(apply(&l, "OMG and iv and omgs"), "OMG and iv and omgs");
        assert_eq!(apply(&l, "JPEG-compressed"), "jeigh peg-compressed");
        assert_eq!(apply(&l, "nothing here"), "nothing here");
    }

    #[test]
    fn replacements_map_to_their_words() {
        let l = lexicon(&[("tabindex", "Tab Index")]);
        let (out, map) = l.apply("Set tabindex now.");
        assert_eq!(out, "Set Tab Index now.");
        let r = map.to_source(&out, 4..7).unwrap();
        assert_eq!(
            r,
            CharRange::new(4, 12),
            "any part of the respelling is the word"
        );
        assert_eq!(map.to_source(&out, 14..17), Some(CharRange::new(13, 16)));
    }

    #[test]
    fn the_vendored_dictionaries_load() {
        let dir = find_dir(None).expect("third_party/ibmtts-dictionaries in the checkout");
        let en = load(&dir, "ENU");
        assert!(en.len() > 500, "{}", en.len());
        assert_eq!(en.get("tabindex").map(String::as_str), Some("Tab Index"));
        assert_eq!(en.get("omg").map(String::as_str), Some("oe em jee"));
        assert_eq!(en.get("WWII").map(String::as_str), Some("world war two"));
        assert!(en.values().all(|v| !v.contains('`')));
        let de = load(&dir, "deu");
        assert!(!de.is_empty());
        let on = CommunityLexiconConfig {
            enabled: true,
            ..CommunityLexiconConfig::default()
        };
        let l = CommunityLexicon::from_config(&on).unwrap();
        assert_eq!(
            apply(&l, "Set the tabindex, omg."),
            "Set the Tab Index, oe em jee."
        );
        assert!(CommunityLexicon::from_config(&CommunityLexiconConfig::default()).is_none());
    }

    #[test]
    fn the_pipeline_uses_it_only_when_on_and_not_for_native_engines() {
        use crate::normalize::{NormalizeConfig, Pipeline};
        use textweaver_core::PunctuationLevel;
        let mut config = NormalizeConfig::default();
        let names = |c: &NormalizeConfig, native| {
            Pipeline::for_settings(c, PunctuationLevel::default(), false, native).names()
        };
        assert!(!names(&config, false).contains(&"community_lexicon"));
        config.community_lexicon.enabled = true;
        assert!(names(&config, false).contains(&"community_lexicon"));
        assert!(!names(&config, true).contains(&"community_lexicon"));
        config.community_lexicon.dir = Some(PathBuf::from("/no/such/dictionaries"));
        assert!(!names(&config, false).contains(&"community_lexicon"));
        // Settings files: the section is optional and partial.
        let c: CommunityLexiconConfig = serde_json::from_str(r#"{"enabled":true}"#).unwrap();
        assert_eq!(c.language, "ENU");
    }
}
