//! The voice manager: every voice of every engine in one list (Agent W3f).
//!
//! The list the Choose Voice command (Alt+V) shows is built here. Its
//! rows are, in order:
//!
//! - two filter rows, "Language: all" and "Engine: all": Enter on one
//!   moves to the next language or engine and shows the list again;
//! - the voices that pass the filters, favourites first: the current
//!   engine's voices, the other engines' voices (listed once per session
//!   on a helper thread, which starts each engine, asks, and closes it),
//!   the Piper voices installed in the data folder (Piper need not be
//!   running), and, once the Piper catalogue has been fetched,
//!   the Piper voices that can be downloaded, each with its size and
//!   licence;
//! - "Fetch the Piper voice list", which downloads the catalogue (about
//!   250 KB) after a yes.
//!
//! Enter on a voice uses it (switching engine if it belongs to another)
//! and speaks a sample; on a voice to download it asks first, saying the
//! size and the licence. Space marks a favourite. Delete removes an
//! installed Piper voice, after a yes.
//!
//! This module is the model: pure data, labels, and filters, tested on
//! its own. `voice.rs` wires it to the app. When W3a moves list state
//! into the app's list model, [`VoiceManager`] and [`VoiceRow`] move with
//! it unchanged; only the `ListKind::Voices` plumbing in `app.rs` changes.
//!
//! **Rate and pitch per voice** ([`remembered_params`], [`remember_params`]):
//! as screen readers do, each voice keeps its own rate and pitch. They are
//! saved in `[speech.voice_params]` (a table keyed `"engine:voice"`, kept
//! as an unknown section until the store has a typed one) when the voice
//! is left, and restored when it is chosen again.

use textweaver_core::{Pitch, Rate};
use textweaver_engines::piper::{Catalog, Licence, LicenceKind, VoiceStore};
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog as Messages;
use textweaver_speech::Voice;
use textweaver_store::Settings;

/// The Piper backend's id.
pub const PIPER: &str = "piper";

/// Whether a voice can be used now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VoiceStatus {
    /// Installed and usable.
    Ready,
    /// A Piper voice that can be downloaded.
    Downloadable {
        /// Download size in bytes.
        bytes: u64,
        /// Its licence.
        licence: Licence,
    },
}

/// One voice in the manager.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceEntry {
    /// The engine's backend id (`piper`, `sapi`, `eci`).
    pub engine: String,
    /// The engine's name for people ("Piper neural voices").
    pub engine_name: String,
    /// The voice.
    pub voice: Voice,
    /// Installed or downloadable.
    pub status: VoiceStatus,
}

impl VoiceEntry {
    /// The primary language subtag, lowercase (`en`), or empty.
    pub fn language(&self) -> String {
        self.voice
            .languages
            .first()
            .map(|l| primary_language(l))
            .unwrap_or_default()
    }

    /// The key it is remembered by: `"engine:voice id"`.
    pub fn key(&self) -> String {
        params_key(&self.engine, &self.voice.id)
    }
}

/// `en` from `en-US`, `en_GB`, or `EN`.
pub fn primary_language(tag: &str) -> String {
    tag.split(['-', '_'])
        .next()
        .unwrap_or(tag)
        .to_ascii_lowercase()
}

/// A language's English name, for the filter row ("English"), or the
/// code itself when it is not one of the common ones.
pub fn language_name(code: &str) -> String {
    language_name_in(&Messages::english(), code)
}

/// [`language_name`] in the catalog's language.
pub fn language_name_in(c: &Messages, code: &str) -> String {
    let id = match code {
        "ar" => "voices-language-ar",
        "ca" => "voices-language-ca",
        "cs" => "voices-language-cs",
        "cy" => "voices-language-cy",
        "da" => "voices-language-da",
        "de" => "voices-language-de",
        "el" => "voices-language-el",
        "en" => "voices-language-en",
        "es" => "voices-language-es",
        "fa" => "voices-language-fa",
        "fi" => "voices-language-fi",
        "fr" => "voices-language-fr",
        "hi" => "voices-language-hi",
        "hu" => "voices-language-hu",
        "is" => "voices-language-is",
        "it" => "voices-language-it",
        "ja" => "voices-language-ja",
        "ka" => "voices-language-ka",
        "kk" => "voices-language-kk",
        "ko" => "voices-language-ko",
        "lb" => "voices-language-lb",
        "lv" => "voices-language-lv",
        "nl" => "voices-language-nl",
        "no" | "nb" => "voices-language-no",
        "pl" => "voices-language-pl",
        "pt" => "voices-language-pt",
        "ro" => "voices-language-ro",
        "ru" => "voices-language-ru",
        "sk" => "voices-language-sk",
        "sl" => "voices-language-sl",
        "sr" => "voices-language-sr",
        "sv" => "voices-language-sv",
        "sw" => "voices-language-sw",
        "tr" => "voices-language-tr",
        "uk" => "voices-language-uk",
        "vi" => "voices-language-vi",
        "zh" => "voices-language-zh",
        other => return other.to_owned(),
    };
    c.tr(id)
}

/// One row of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VoiceRow {
    /// "Language: English": Enter shows the next language.
    LanguageFilter,
    /// "Engine: Piper": Enter shows the next engine.
    EngineFilter,
    /// A voice: index into the entries.
    Voice(usize),
    /// "Fetch the Piper voice list".
    FetchCatalog,
}

/// The voices of every engine, filtered by language and engine.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VoiceManager {
    entries: Vec<VoiceEntry>,
    language: Option<String>,
    engine: Option<String>,
    rows: Vec<VoiceRow>,
    /// Offer to fetch the Piper catalogue.
    pub offer_catalog: bool,
}

impl VoiceManager {
    /// A manager over `entries`, unfiltered.
    pub fn new(entries: Vec<VoiceEntry>) -> Self {
        let mut m = VoiceManager {
            entries,
            ..VoiceManager::default()
        };
        m.refresh(&[]);
        m
    }

    /// Every entry, filtered or not.
    pub fn entries(&self) -> &[VoiceEntry] {
        &self.entries
    }

    /// Replaces the entries, keeping the filters (a filter no entry
    /// matches any more is cleared).
    pub fn set_entries(&mut self, entries: Vec<VoiceEntry>, favourites: &[String]) {
        self.entries = entries;
        if let Some(l) = &self.language
            && !self.entries.iter().any(|e| e.language() == *l)
        {
            self.language = None;
        }
        if let Some(g) = &self.engine
            && !self.entries.iter().any(|e| e.engine == *g)
        {
            self.engine = None;
        }
        self.refresh(favourites);
    }

    /// The rows shown, in order.
    pub fn rows(&self) -> &[VoiceRow] {
        &self.rows
    }

    /// Row `n`.
    pub fn row(&self, n: usize) -> Option<&VoiceRow> {
        self.rows.get(n)
    }

    /// The entry on row `n`, if it is a voice.
    pub fn entry_at(&self, n: usize) -> Option<&VoiceEntry> {
        match self.rows.get(n)? {
            VoiceRow::Voice(i) => self.entries.get(*i),
            _ => None,
        }
    }

    /// The languages present, sorted by name.
    pub fn languages(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .entries
            .iter()
            .map(VoiceEntry::language)
            .filter(|l| !l.is_empty())
            .collect();
        out.sort_by_key(|l| language_name(l));
        out.dedup();
        out
    }

    /// The engines present: id and name, in the order first seen.
    pub fn engines(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();
        for e in &self.entries {
            if !out.iter().any(|(id, _)| *id == e.engine) {
                out.push((e.engine.clone(), e.engine_name.clone()));
            }
        }
        out
    }

    /// The language filter (`en`), if any.
    pub fn language_filter(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// The engine filter (a backend id), if any.
    pub fn engine_filter(&self) -> Option<&str> {
        self.engine.as_deref()
    }

    /// Shows only `language` (a primary subtag) or, with `None`, all.
    pub fn set_language(&mut self, language: Option<String>, favourites: &[String]) {
        self.language = language;
        self.refresh(favourites);
    }

    /// Shows only `engine` or, with `None`, all.
    pub fn set_engine(&mut self, engine: Option<String>, favourites: &[String]) {
        self.engine = engine;
        self.refresh(favourites);
    }

    /// Moves the language filter to the next language (after the last,
    /// back to all) and says what is shown.
    pub fn next_language(&mut self, favourites: &[String]) -> String {
        self.next_language_in(&Messages::english(), favourites)
    }

    /// [`next_language`](Self::next_language), saying it in the catalog's
    /// language.
    pub fn next_language_in(&mut self, c: &Messages, favourites: &[String]) -> String {
        let langs = self.languages();
        let next = match &self.language {
            None => langs.first().cloned(),
            Some(cur) => langs
                .iter()
                .position(|l| l == cur)
                .and_then(|i| langs.get(i + 1).cloned()),
        };
        self.set_language(next, favourites);
        self.shown_sentence_in(c)
    }

    /// Moves the engine filter to the next engine and says what is shown.
    pub fn next_engine(&mut self, favourites: &[String]) -> String {
        self.next_engine_in(&Messages::english(), favourites)
    }

    /// [`next_engine`](Self::next_engine), saying it in the catalog's
    /// language.
    pub fn next_engine_in(&mut self, c: &Messages, favourites: &[String]) -> String {
        let engines = self.engines();
        let next = match &self.engine {
            None => engines.first().map(|(id, _)| id.clone()),
            Some(cur) => engines
                .iter()
                .position(|(id, _)| id == cur)
                .and_then(|i| engines.get(i + 1).map(|(id, _)| id.clone())),
        };
        self.set_engine(next, favourites);
        self.shown_sentence_in(c)
    }

    /// "12 voices: English, all engines."
    pub fn shown_sentence(&self) -> String {
        self.shown_sentence_in(&Messages::english())
    }

    /// [`shown_sentence`](Self::shown_sentence) in the catalog's language.
    pub fn shown_sentence_in(&self, c: &Messages) -> String {
        let n = self
            .rows
            .iter()
            .filter(|r| matches!(r, VoiceRow::Voice(_)))
            .count();
        c.fmt(
            "voices-shown",
            &args![
                "n" => n,
                "language" => self.language_label(c),
                "engine" => self.engine_label(c)
            ],
        )
    }

    fn language_label(&self, c: &Messages) -> String {
        match &self.language {
            None => c.tr("voices-all-languages"),
            Some(l) => language_name_in(c, l),
        }
    }

    fn engine_label(&self, c: &Messages) -> String {
        match &self.engine {
            None => c.tr("voices-all-engines"),
            Some(id) => self
                .entries
                .iter()
                .find(|e| e.engine == *id)
                .map_or_else(|| id.clone(), |e| e.engine_name.clone()),
        }
    }

    fn passes(&self, e: &VoiceEntry) -> bool {
        self.language.as_ref().is_none_or(|l| e.language() == *l)
            && self.engine.as_ref().is_none_or(|g| e.engine == *g)
    }

    /// Rebuilds the rows: filters, then installed voices (favourites first,
    /// in favourite order), then downloadable ones.
    pub fn refresh(&mut self, favourites: &[String]) {
        let mut shown: Vec<usize> = (0..self.entries.len())
            .filter(|&i| self.passes(&self.entries[i]))
            .collect();
        let rank = |e: &VoiceEntry| {
            let fav = favourites
                .iter()
                .position(|f| *f == e.voice.id || f.eq_ignore_ascii_case(&e.voice.name));
            let ready = e.status == VoiceStatus::Ready;
            (!ready, fav.unwrap_or(usize::MAX))
        };
        shown.sort_by_key(|&i| rank(&self.entries[i]));
        let mut rows = vec![VoiceRow::LanguageFilter, VoiceRow::EngineFilter];
        rows.extend(shown.into_iter().map(VoiceRow::Voice));
        if self.offer_catalog {
            rows.push(VoiceRow::FetchCatalog);
        }
        self.rows = rows;
    }

    /// The row labels, to read and show. `current` is the engine and voice
    /// in use.
    pub fn labels(&self, favourites: &[String], current: (&str, Option<&str>)) -> Vec<String> {
        self.labels_in(&Messages::english(), favourites, current)
    }

    /// [`labels`](Self::labels) in the catalog's language.
    pub fn labels_in(
        &self,
        c: &Messages,
        favourites: &[String],
        current: (&str, Option<&str>),
    ) -> Vec<String> {
        // The engine is named only when the list holds more than one.
        let several = self.engines().len() > 1;
        self.rows
            .iter()
            .map(|r| match r {
                VoiceRow::LanguageFilter => c.fmt(
                    "voices-language-row",
                    &args!["language" => self.language_label(c)],
                ),
                VoiceRow::EngineFilter => c.fmt(
                    "voices-engine-row",
                    &args!["engine" => self.engine_label(c)],
                ),
                VoiceRow::FetchCatalog => c.tr("voices-fetch-row"),
                VoiceRow::Voice(i) => {
                    let e = &self.entries[*i];
                    let fav = favourites
                        .iter()
                        .any(|f| *f == e.voice.id || f.eq_ignore_ascii_case(&e.voice.name));
                    let is_current = e.engine == current.0
                        && current.1.is_some_and(|c| {
                            c == e.voice.id || c.eq_ignore_ascii_case(&e.voice.name)
                        });
                    entry_label_in(c, e, several, fav, is_current)
                }
            })
            .collect()
    }
}

/// "Joe (medium), en-US, Piper, medium, favorite, current", or for a
/// download "Amy (low), en-US, Piper, download 63 MB, non-commercial".
/// `engine` names the engine (when the list has several).
pub fn entry_label(e: &VoiceEntry, engine: bool, favourite: bool, current: bool) -> String {
    entry_label_in(&Messages::english(), e, engine, favourite, current)
}

/// [`entry_label`] in the catalog's language.
pub fn entry_label_in(
    c: &Messages,
    e: &VoiceEntry,
    engine: bool,
    favourite: bool,
    current: bool,
) -> String {
    let mut parts = vec![e.voice.name.clone()];
    if let Some(l) = e.voice.languages.first() {
        parts.push(l.clone());
    }
    let engine_tag = short_engine_name(&e.engine_name);
    if engine
        && !e
            .voice
            .tags
            .iter()
            .any(|t| t.eq_ignore_ascii_case(&engine_tag))
    {
        parts.push(engine_tag);
    }
    parts.extend(e.voice.tags.iter().cloned());
    if let VoiceStatus::Downloadable { bytes, licence } = &e.status {
        parts.push(c.fmt(
            "voices-download-size",
            &args!["size" => textweaver_engines::piper::catalog::megabytes(*bytes)],
        ));
        parts.push(c.tr(match licence.kind {
            LicenceKind::PublicDomain => "voices-licence-public-domain",
            LicenceKind::Attribution => "voices-licence-attribution",
            LicenceKind::ShareAlike => "voices-licence-share-alike",
            LicenceKind::NonCommercial => "voices-licence-non-commercial",
            LicenceKind::Unknown => "voices-licence-unknown",
        }));
    }
    if favourite {
        parts.push(c.tr("voices-favourite"));
    }
    if current {
        parts.push(c.tr("voices-current"));
    }
    parts.join(", ")
}

/// "Piper" from "Piper neural voices", "SAPI 5" as is.
fn short_engine_name(name: &str) -> String {
    match name {
        "Piper neural voices" => "Piper".into(),
        other => other.to_owned(),
    }
}

/// The current engine's voices as entries.
pub fn engine_entries(engine: &str, engine_name: &str, voices: &[Voice]) -> Vec<VoiceEntry> {
    voices
        .iter()
        .map(|v| VoiceEntry {
            engine: engine.to_owned(),
            engine_name: engine_name.to_owned(),
            voice: v.clone(),
            status: VoiceStatus::Ready,
        })
        .collect()
}

/// Piper's installed voices, and those in `catalog` not installed yet.
/// Downloadable voices whose licence is only known after asking have
/// `LicenceKind::Unknown` until then (the catalogue does not carry
/// licences; each voice's `MODEL_CARD` does).
pub fn piper_entries(store: &VoiceStore, catalog: Option<&Catalog>) -> Vec<VoiceEntry> {
    let installed = store.installed();
    let mut out: Vec<VoiceEntry> = installed
        .iter()
        .map(|v| VoiceEntry {
            engine: PIPER.into(),
            engine_name: "Piper neural voices".into(),
            voice: v.to_voice(),
            status: VoiceStatus::Ready,
        })
        .collect();
    if let Some(c) = catalog {
        for v in &c.voices {
            if installed.iter().any(|i| i.key == v.key) {
                continue;
            }
            let quality = v.quality.replace('_', " ");
            out.push(VoiceEntry {
                engine: PIPER.into(),
                engine_name: "Piper neural voices".into(),
                voice: Voice {
                    id: v.key.clone(),
                    name: format!(
                        "{} ({quality})",
                        textweaver_engines::piper::catalog::display_name(&v.name)
                    ),
                    languages: vec![v.bcp47()],
                    gender: None,
                    tags: vec!["Piper".into(), quality],
                },
                status: VoiceStatus::Downloadable {
                    bytes: v.size_bytes(),
                    licence: Licence::classify(""),
                },
            });
        }
    }
    out
}

/// Where the Piper catalogue is kept once fetched.
pub fn catalog_path(voices_dir: &std::path::Path) -> std::path::PathBuf {
    voices_dir.join("voices.json")
}

/// The Piper catalogue kept in `voices_dir`, if it was fetched.
pub fn cached_catalog(voices_dir: &std::path::Path) -> Option<Catalog> {
    let json = std::fs::read_to_string(catalog_path(voices_dir)).ok()?;
    Catalog::from_json(&json).ok()
}

/// `"engine:voice"`.
pub fn params_key(engine: &str, voice: &str) -> String {
    format!("{engine}:{voice}")
}

/// The rate and pitch remembered for `key`, from `[speech.voice_params]`.
pub fn remembered_params(settings: &Settings, key: &str) -> Option<(Rate, Pitch)> {
    let p = settings.speech.voice_params.get(key)?;
    Some((
        Rate::Wpm(p.rate).clamped(),
        Pitch::Semitones(p.pitch).clamped(),
    ))
}

/// Remembers `rate` and `pitch` for `key` in `[speech.voice_params]`.
pub fn remember_params(settings: &mut Settings, key: &str, rate: Rate, pitch: Pitch) {
    settings.speech.voice_params.insert(
        key.to_owned(),
        textweaver_store::RememberedVoice {
            rate: rate.wpm(),
            pitch: pitch.semitones(),
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voice(id: &str, name: &str, lang: &str) -> Voice {
        Voice {
            id: id.into(),
            name: name.into(),
            languages: vec![lang.into()],
            gender: None,
            tags: Vec::new(),
        }
    }

    fn manager() -> VoiceManager {
        let mut entries = engine_entries(
            "sapi",
            "SAPI 5",
            &[
                voice("zira", "Microsoft Zira", "en-US"),
                voice("hedda", "Microsoft Hedda", "de-DE"),
            ],
        );
        entries.push(VoiceEntry {
            engine: PIPER.into(),
            engine_name: "Piper neural voices".into(),
            voice: Voice {
                tags: vec!["Piper".into(), "medium".into()],
                ..voice("en_US-joe-medium", "Joe (medium)", "en-US")
            },
            status: VoiceStatus::Ready,
        });
        entries.push(VoiceEntry {
            engine: PIPER.into(),
            engine_name: "Piper neural voices".into(),
            voice: Voice {
                tags: vec!["Piper".into(), "low".into()],
                ..voice("en_US-amy-low", "Amy (low)", "en-US")
            },
            status: VoiceStatus::Downloadable {
                bytes: 63_000_000,
                licence: Licence::classify("CC BY-NC-SA 4.0"),
            },
        });
        VoiceManager::new(entries)
    }

    #[test]
    fn rows_have_filters_then_ready_voices_then_downloads() {
        let m = manager();
        let labels = m.labels(&["en_US-joe-medium".into()], ("sapi", Some("zira")));
        assert_eq!(
            labels,
            vec![
                "Language: all languages",
                "Engine: all engines",
                "Microsoft Zira, en-US, SAPI 5, current",
                "Microsoft Hedda, de-DE, SAPI 5",
                "Joe (medium), en-US, Piper, medium, favorite",
                "Amy (low), en-US, Piper, low, download 63 MB, non-commercial",
            ]
        );
        // Favourites first among the ready voices.
        let mut m = m;
        m.refresh(&["en_US-joe-medium".into()]);
        assert_eq!(m.entry_at(2).unwrap().voice.id, "en_US-joe-medium");
        assert_eq!(m.row(0), Some(&VoiceRow::LanguageFilter));
        assert!(m.entry_at(0).is_none());
    }

    #[test]
    fn filters_cycle_and_say_what_is_shown() {
        let mut m = manager();
        // Sorted by name: English before German.
        assert_eq!(m.languages(), vec!["en".to_owned(), "de".to_owned()]);
        assert_eq!(m.next_language(&[]), "3 voices: English, all engines.");
        assert_eq!(m.next_language(&[]), "1 voice: German, all engines.");
        assert_eq!(
            m.next_language(&[]),
            "4 voices: all languages, all engines."
        );
        assert_eq!(m.next_engine(&[]), "2 voices: all languages, SAPI 5.");
        assert_eq!(
            m.next_engine(&[]),
            "2 voices: all languages, Piper neural voices."
        );
        assert_eq!(m.engine_filter(), Some(PIPER));
        m.set_language(Some("en".into()), &[]);
        assert_eq!(
            m.shown_sentence(),
            "2 voices: English, Piper neural voices."
        );
        // Entries without the filtered engine clear the filter.
        m.set_entries(
            engine_entries("eci", "Eloquence", &[voice("r", "Reed", "en-US")]),
            &[],
        );
        assert_eq!(m.engine_filter(), None);
        assert_eq!(m.language_filter(), Some("en"));
    }

    #[test]
    fn the_catalogue_row_is_offered_last() {
        let mut m = manager();
        m.offer_catalog = true;
        m.refresh(&[]);
        assert_eq!(m.rows().last(), Some(&VoiceRow::FetchCatalog));
    }

    #[test]
    fn piper_entries_list_installed_then_downloadable() {
        let tmp = tempfile::tempdir().unwrap();
        let store = VoiceStore::new(tmp.path());
        let dir = store.voice_dir("en_US-joe-medium");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("en_US-joe-medium.onnx"), b"m").unwrap();
        std::fs::write(dir.join("en_US-joe-medium.onnx.json"), b"{}").unwrap();
        let catalog = Catalog::from_json(
            r#"{"en_US-joe-medium": {"key": "en_US-joe-medium", "name": "joe",
                 "language": {"code": "en_US"}, "quality": "medium", "files": {}},
               "de_DE-thorsten-medium": {"key": "de_DE-thorsten-medium", "name": "thorsten",
                 "language": {"code": "de_DE"}, "quality": "medium",
                 "files": {"a.onnx": {"size_bytes": 5000000}}}}"#,
        )
        .unwrap();
        let e = piper_entries(&store, Some(&catalog));
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].status, VoiceStatus::Ready);
        assert_eq!(e[1].voice.name, "Thorsten (medium)");
        assert_eq!(e[1].language(), "de");
        assert!(matches!(
            e[1].status,
            VoiceStatus::Downloadable {
                bytes: 5_000_000,
                ..
            }
        ));
        assert_eq!(piper_entries(&store, None).len(), 1);
        std::fs::write(catalog_path(tmp.path()), "{}").unwrap();
        assert_eq!(cached_catalog(tmp.path()).map(|c| c.voices.len()), Some(0));
    }

    #[test]
    fn rate_and_pitch_are_remembered_per_voice() {
        let mut s = Settings::default();
        let key = params_key("piper", "en_US-joe-medium");
        assert_eq!(remembered_params(&s, &key), None);
        remember_params(&mut s, &key, Rate::Wpm(320), Pitch::Semitones(-2));
        remember_params(&mut s, "sapi:zira", Rate::Wpm(400), Pitch::default());
        assert_eq!(
            remembered_params(&s, &key),
            Some((Rate::Wpm(320), Pitch::Semitones(-2)))
        );
        assert_eq!(
            remembered_params(&s, "sapi:zira"),
            Some((Rate::Wpm(400), Pitch::Semitones(0)))
        );
        // Survives a save and load of the settings file.
        let text = toml::to_string(&s).unwrap();
        let back: Settings = toml::from_str(&text).unwrap();
        assert_eq!(
            remembered_params(&back, &key),
            Some((Rate::Wpm(320), Pitch::Semitones(-2)))
        );
    }

    #[test]
    fn language_names() {
        assert_eq!(primary_language("en_GB"), "en");
        assert_eq!(language_name("de"), "German");
        assert_eq!(language_name("xx"), "xx");
    }
}
