//! Language and study aids (Agent W3e): define word, settings profiles,
//! reading statistics, and the interface's message catalog.
//!
//! - **Define word** (Ctrl+Shift+D in the GUI, Alt+E in the terminal, or
//!   `define word` in the palette): the selected words, or the word at the
//!   cursor, looked up in the user's glossary and then Open English
//!   WordNet (`textweaver-lexicon`). The senses show in a list; Enter
//!   copies one. With no word to look up, it asks for one. The dictionary
//!   file (about 10 MB) opens on a helper thread the first time (Wave 5,
//!   W5y): "Dictionary still loading." is said once, and the list opens
//!   when it is ready ([`App::define_tick`]).
//! - **Settings profiles** (Ctrl+Shift+U, Alt+U): a list of the profiles in
//!   `profiles.toml`; Enter switches, F2 renames, Delete deletes after a
//!   yes or no, and the last items save, import, and export.
//! - **Reading statistics** (Ctrl+Shift+Y, Alt+Y): time is counted while
//!   textweaver reads aloud, with the furthest point and the sessions,
//!   and added to `stats.json` through the writer every 30 seconds, when a
//!   document closes, and on quitting. `[stats] enabled = false` stops it.
//! - **Messages**: every string these features say or show comes from the
//!   catalog for `[interface] language` ([`App::catalog`]).
//!
//! These lists are shown on the app's list model (`crate::list_model`,
//! Agent W3a), so every frontend moves through them, announces "k of n",
//! and jumps by letter the same way; `StudyList` says what Enter, Delete,
//! and F2 do on each item. Since Wave 5 (W5y), Enter on the statistics
//! list's information rows or on its on-and-off row keeps the list open,
//! on the same row, with the row's new text, instead of closing it.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant, SystemTime};

use textweaver_lexicon::i18n::{Arg, Catalog, duration};
use textweaver_lexicon::{Dictionary, Glossary, Lexicon, args};
use textweaver_store::profiles::{self, ProfileError};
use textweaver_store::{Profiles, ReadingStats, StatsDelta};

use crate::app::{App, ListKind};
use crate::command::{Confirm, Effect, PromptPurpose};
use crate::synced_library::{CombinedStats, SyncedLibrary, computer_line, stats_title};
use crate::text_util;

/// How often reading time is added to `stats.json` while reading.
const STATS_FLUSH: Duration = Duration::from_secs(30);
/// The longest gap between ticks counted as reading (a suspended laptop
/// does not count).
const MAX_TICK_GAP: Duration = Duration::from_secs(5);
/// Documents in the most-read part of the statistics list.
const MOST_READ: usize = 10;
/// How long the statistics list waits for the sync folder to be read.
const SYNC_STATS_WAIT: Duration = Duration::from_millis(1500);

/// What a study list's items do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StudyList {
    /// Define word's senses; Enter copies the item.
    Definition(Vec<String>),
    /// The profiles list.
    Profiles(Vec<ProfileEntry>),
    /// The statistics list.
    Statistics(Vec<StatsEntry>),
}

/// One item of the profiles list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ProfileEntry {
    /// A profile: Enter switches to it.
    Profile(String),
    /// Save the current settings as a new profile.
    SaveNew,
    /// Save the current settings into the profile in use.
    Update(String),
    /// Import profiles from a file.
    Import,
    /// Export every profile to a file.
    Export,
}

/// One item of the statistics list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StatsEntry {
    /// Information only.
    Info,
    /// A document: Enter opens it.
    Document(PathBuf),
    /// Turn statistics on or off.
    Toggle,
    /// Show or hide each computer's share of each document (S6).
    ByComputer,
    /// Remove this computer's statistics, after a question (W9a-c, QW13).
    Clear,
}

/// Reading time not yet added to `stats.json`.
#[derive(Debug, Default)]
struct StatsTracker {
    /// The open document: key, title, and file.
    doc: Option<(String, String, Option<PathBuf>)>,
    last_tick: Option<Instant>,
    last_flush: Option<Instant>,
    seconds: f64,
    /// This opening of the document has been counted as a session.
    counted: bool,
    /// A new session starts with the next reading.
    session_pending: bool,
    furthest_percent: u8,
    furthest_char: u64,
}

/// What opening the dictionary file on the helper thread found.
enum LexiconFound {
    /// The file, opened.
    Opened(Arc<Lexicon>),
    /// No dictionary file is installed.
    Missing,
    /// The file could not be read: why.
    Damaged(String),
}

/// The dictionary file opening on a helper thread, and the word to define
/// when it is ready (none when it opens quietly, for difficult-word
/// definitions).
struct LexiconLoad {
    found: Receiver<LexiconFound>,
    word: Option<String>,
}

/// The study features' state in the app.
pub(crate) struct Study {
    /// The interface messages.
    pub(crate) catalog: Arc<Catalog>,
    dictionary: Option<Dictionary>,
    /// The glossary file and its modification time when loaded, so an
    /// edited glossary is read again.
    glossary_stamp: Option<(PathBuf, Option<SystemTime>)>,
    lexicon_missing: bool,
    /// The dictionary file opening on a helper thread.
    lexicon_load: Option<LexiconLoad>,
    profiles: Option<Profiles>,
    /// The profile a rename or delete is for.
    pending_profile: Option<String>,
    /// A yes-or-no question about deleting this profile.
    pub(crate) question: Option<String>,
    /// A yes-or-no question about removing the reading statistics.
    pub(crate) stats_question: bool,
    stats: StatsTracker,
    /// The statistics list shows each computer's share of each document.
    stats_by_computer: bool,
}

impl std::fmt::Debug for Study {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Study")
            .field("lang", &self.catalog.lang())
            .field("question", &self.question)
            .finish_non_exhaustive()
    }
}

impl Study {
    /// The study state for `settings`: the catalog for its interface
    /// language, from `locales_dir` when it is not built in. A language
    /// that cannot be used is reported in the returned message.
    pub(crate) fn new(language: &str, locales_dir: Option<&Path>) -> (Study, Option<String>) {
        let (catalog, warning) = Catalog::for_language(language, locales_dir);
        (
            Study {
                catalog,
                dictionary: None,
                glossary_stamp: None,
                lexicon_missing: false,
                lexicon_load: None,
                profiles: None,
                pending_profile: None,
                question: None,
                stats_question: false,
                stats: StatsTracker::default(),
                stats_by_computer: false,
            },
            warning,
        )
    }
}

/// A prompt's label, from the catalog: shown, and said when it opens.
pub(crate) fn prompt_label(c: &Catalog, purpose: PromptPurpose) -> String {
    use PromptPurpose as P;
    c.tr(match purpose {
        P::Find => "prompt-find",
        P::GoTo => "prompt-go-to",
        P::Open => "prompt-open",
        P::CommandPalette => "prompt-command",
        P::SaveAs => "prompt-save-as",
        P::TableSize => "prompt-table-size",
        P::ImagePath => "prompt-image-path",
        P::ReplaceFind => "prompt-replace-find",
        P::ReplaceWith => "prompt-replace-with",
        P::NoteText => "prompt-note",
        P::EditNote => "prompt-edit-note",
        P::RenameBookmark => "prompt-rename-bookmark",
        P::ExportSettings => "prompt-export-settings",
        P::ImportSettings => "prompt-import-settings",
        P::CitationLocator => "prompt-citation-locator",
        P::ReferenceIdentifier => "prompt-reference-identifier",
        P::ImportReferences => "prompt-import-references",
        P::TemplateTitle => "prompt-template-title",
        P::DefineWord => "prompt-define-word",
        P::ProfileName => "prompt-profile-name",
        P::RenameProfile => "prompt-profile-rename",
        P::ImportProfiles => "prompt-profiles-import",
        P::ExportProfiles => "prompt-profiles-export",
        P::SettingValue => "prompt-setting-value",
        P::SyncComputerName => "prompt-sync-computer-name",
        P::DocumentDetails => "prompt-document-details",
        P::CommentReply => "prompt-comment-reply",
        P::CommentText => "prompt-comment-text",
    })
}

impl App {
    /// The interface's message catalog, for frontends' own strings.
    pub fn catalog(&self) -> Arc<Catalog> {
        self.study.catalog.clone()
    }

    /// The catalog, borrowed: for helpers that take a `&Catalog`.
    pub(crate) fn cat(&self) -> &Catalog {
        &self.study.catalog
    }

    /// The yes-or-no question asked before `a` runs, in the interface's
    /// language; `None` for actions that run at once.
    pub(crate) fn confirmation_question(&self, a: textweaver_keymap::ActionId) -> Option<String> {
        use textweaver_keymap::ActionId as A;
        a.confirmation_prompt()?;
        Some(self.msg(match a {
            A::Quit => "confirm-quit",
            _ => "confirm-delete-note",
        }))
    }

    /// A message from the catalog.
    pub(crate) fn msg(&self, id: &str) -> String {
        self.study.catalog.tr(id)
    }

    /// A message with values from the catalog.
    pub(crate) fn msg_args(&self, id: &str, args: &[(&str, Arg)]) -> String {
        self.study.catalog.fmt(id, args)
    }

    // ----- Define word -----------------------------------------------------

    /// `define_word`: the selection (up to a few words) or the word at the
    /// cursor, else a prompt.
    pub(crate) fn define_word(&mut self) -> Vec<Effect> {
        let word = self.session.as_ref().and_then(|s| {
            let range = s
                .selection
                .filter(|r| !r.is_empty() && r.len() <= 60)
                .or_else(|| text_util::word_containing(&s.doc, s.cursor))?;
            let text = s.doc.slice(range);
            (!text.trim().is_empty()).then_some(text)
        });
        match word {
            Some(w) => self.define(&w),
            None => {
                if self.session.is_some() {
                    self.tell(&self.msg("define-nothing-here"));
                }
                self.prompt(PromptPurpose::DefineWord)
            }
        }
    }

    /// Starts opening the dictionary file on a helper thread, to define
    /// `word` when it is ready ([`define_tick`](Self::define_tick)).
    /// "Dictionary still loading." is said once, when a word first waits;
    /// a word asked for meanwhile replaces the waiting one. With no word,
    /// the file opens quietly (for difficult-word definitions).
    fn load_lexicon(&mut self, word: Option<&str>) -> Vec<Effect> {
        if let Some(load) = self.study.lexicon_load.as_mut() {
            let Some(word) = word else {
                return Vec::new();
            };
            let first = load.word.replace(word.to_owned()).is_none();
            if first {
                let msg = self.msg("define-still-loading");
                self.tell(&msg);
            }
            return vec![Effect::Redraw];
        }
        let explicit = self.settings.lexicon.data_file.clone();
        let data_dir = self.paths.as_ref().map(|p| p.data_dir.clone());
        let (tx, rx) = mpsc::channel();
        let wake = self.waker_slot();
        let spawned = std::thread::Builder::new()
            .name("textweaver-lexicon".into())
            .spawn(move || {
                let found = match textweaver_lexicon::find_lexicon(
                    explicit.as_deref(),
                    data_dir.as_deref(),
                ) {
                    Ok(Some((path, l))) => {
                        log::info!("dictionary: {} ({} bytes)", path.display(), l.size());
                        LexiconFound::Opened(Arc::new(l))
                    }
                    Ok(None) => LexiconFound::Missing,
                    Err(e) => LexiconFound::Damaged(e.to_string()),
                };
                let _ = tx.send(found);
                wake.wake();
            });
        if let Err(e) = spawned {
            // No thread: open it here, as before.
            log::warn!("cannot start the dictionary thread: {e}");
            self.study.dictionary = Some(Dictionary::default());
            self.study.lexicon_missing = true;
            return match word {
                Some(w) => self.define(w),
                None => Vec::new(),
            };
        }
        self.study.lexicon_load = Some(LexiconLoad {
            found: rx,
            word: word.map(str::to_owned),
        });
        if word.is_none() {
            return Vec::new();
        }
        let msg = self.msg("define-still-loading");
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    /// True while the dictionary file is opening on its helper thread.
    #[cfg(test)]
    pub(crate) fn lexicon_loading(&self) -> bool {
        self.study.lexicon_load.is_some()
    }

    /// The glossary and dictionary for a difficult word's definition
    /// (ADR-0037), once the dictionary file is open. The first call starts
    /// opening it quietly and returns `None`; so do calls while it opens.
    pub(crate) fn definitions_dictionary(&mut self) -> Option<Dictionary> {
        if self.study.lexicon_load.is_some() {
            return None;
        }
        if !self.lexicon_tried() {
            self.load_lexicon(None);
            return None;
        }
        Some(self.dictionary())
    }

    /// Takes the dictionary file from the helper thread once it is open,
    /// and defines the word that was waiting (from [`App::tick`]).
    pub(crate) fn define_tick(&mut self) -> Vec<Effect> {
        let Some(load) = &self.study.lexicon_load else {
            return Vec::new();
        };
        let found = match load.found.try_recv() {
            Ok(found) => found,
            Err(TryRecvError::Empty) => return Vec::new(),
            Err(TryRecvError::Disconnected) => {
                LexiconFound::Damaged("the dictionary thread stopped without an answer".into())
            }
        };
        let Some(load) = self.study.lexicon_load.take() else {
            return Vec::new();
        };
        self.install_lexicon(found);
        match load.word {
            Some(w) => self.define(&w),
            None => vec![Effect::Redraw],
        }
    }

    /// Keeps what the dictionary thread found; a damaged file is said.
    fn install_lexicon(&mut self, found: LexiconFound) {
        let lexicon = match found {
            LexiconFound::Opened(l) => Some(l),
            LexiconFound::Missing => {
                self.study.lexicon_missing = true;
                None
            }
            LexiconFound::Damaged(error) => {
                self.study.lexicon_missing = true;
                let m = self.msg_args("define-dictionary-damaged", &args!["error" => error]);
                self.error(&m);
                None
            }
        };
        let glossary = self.study.dictionary.take().and_then(|d| d.glossary);
        self.study.dictionary = Some(Dictionary { glossary, lexicon });
    }

    /// The glossary and dictionary: the glossary loaded on first use and
    /// when its file changes, the dictionary file as the helper thread
    /// left it (none before it is open). Problems are announced once.
    fn dictionary(&mut self) -> Dictionary {
        let glossary_path = self.settings.lexicon.glossary.clone().or_else(|| {
            self.paths
                .as_ref()
                .and_then(textweaver_store::Paths::default_glossary)
        });
        let stamp = glossary_path.as_ref().map(|p| {
            let t = std::fs::metadata(p).and_then(|m| m.modified()).ok();
            (p.clone(), t)
        });
        if self.study.dictionary.is_none() {
            self.study.dictionary = Some(Dictionary::default());
        }
        if stamp != self.study.glossary_stamp {
            self.study.glossary_stamp = stamp.clone();
            let glossary = match &stamp {
                Some((path, _)) => match Glossary::load(path) {
                    Ok(g) => {
                        if !g.skipped.is_empty() {
                            let m = self.msg_args(
                                "define-glossary-skipped",
                                &args!["n" => g.skipped.len()],
                            );
                            self.note(&m);
                        }
                        Some(Arc::new(g))
                    }
                    Err(e) => {
                        let m = self
                            .msg_args("define-glossary-problem", &args!["error" => e.to_string()]);
                        self.error(&m);
                        None
                    }
                },
                None => None,
            };
            if let Some(d) = self.study.dictionary.as_mut() {
                d.glossary = glossary;
            }
        }
        self.study.dictionary.clone().unwrap_or_default()
    }

    /// Looks `word` up and shows the senses. The first time, the
    /// dictionary file opens on a helper thread first
    /// ([`load_lexicon`](Self::load_lexicon)).
    pub(crate) fn define(&mut self, word: &str) -> Vec<Effect> {
        if self.study.lexicon_load.is_some() || !self.lexicon_tried() {
            return self.load_lexicon(Some(word));
        }
        let dict = self.dictionary();
        let shown = textweaver_lexicon::normalize(word);
        match dict.define(word) {
            Ok(Some(d)) => {
                let c = self.study.catalog.clone();
                let title = textweaver_lexicon::list_title(&c, &d);
                let items = textweaver_lexicon::list_items(&c, &d);
                let intro = self.msg_args("define-intro", &args!["title" => &title]);
                self.tell(&intro);
                self.list = Some(ListKind::Study(StudyList::Definition(items.clone())));
                vec![Effect::ShowList { title, items }]
            }
            Ok(None) => {
                let mut m = self.msg_args("define-not-found", &args!["word" => shown]);
                if self.study.lexicon_missing {
                    m.push(' ');
                    m.push_str(&self.msg("define-no-dictionary"));
                }
                self.tell(&m);
                vec![Effect::Redraw]
            }
            Err(e) => {
                let m = self.msg_args(
                    "define-dictionary-damaged",
                    &args!["error" => e.to_string()],
                );
                self.error(&m);
                vec![Effect::Redraw]
            }
        }
    }

    /// True once the dictionary file has been looked for (found or not).
    fn lexicon_tried(&self) -> bool {
        self.study
            .dictionary
            .as_ref()
            .is_some_and(|d| d.lexicon.is_some())
            || self.study.lexicon_missing
    }

    // ----- Profiles --------------------------------------------------------

    /// The profiles, loaded from `profiles.toml` on first use (a damaged
    /// file is announced and treated as empty; it is replaced on the next
    /// save).
    fn profiles(&mut self) -> Profiles {
        if self.study.profiles.is_none() {
            let loaded = match &self.paths {
                Some(p) => Profiles::load(p),
                None => Ok(Profiles::default()),
            };
            let p = loaded.unwrap_or_else(|e| {
                let m = self.msg_args("profiles-read-failed", &args!["error" => e.to_string()]);
                self.error(&m);
                Profiles::default()
            });
            self.study.profiles = Some(p);
        }
        self.study.profiles.clone().unwrap_or_default()
    }

    /// Keeps `p` and saves it through the writer.
    fn store_profiles(&mut self, p: Profiles) {
        if let Some(paths) = self.paths.clone() {
            self.writer.send(crate::writer::Job::Profiles {
                paths,
                profiles: Box::new(p.clone()),
            });
        }
        self.study.profiles = Some(p);
    }

    fn profile_error(&self, e: &ProfileError) -> String {
        match e {
            ProfileError::NotFound(n) => self.msg_args("profile-not-found", &args!["name" => n]),
            ProfileError::EmptyName => self.msg("profile-needs-name"),
            ProfileError::Exists(n) => self.msg_args("profile-exists", &args!["name" => n]),
            ProfileError::NotAnExport(d) => {
                self.msg_args("profiles-not-an-export", &args!["detail" => d])
            }
            ProfileError::Store(s) => {
                self.msg_args("profiles-save-failed", &args!["error" => s.to_string()])
            }
        }
    }

    /// A short summary of profile `name` for its list item.
    fn profile_summary(&self, profiles: &Profiles, name: &str) -> String {
        let mut parts = Vec::new();
        let Some(p) = profiles.profiles.get(name) else {
            return self.msg("profile-summary-empty");
        };
        if let Some(v) = profiles::value_text(p, "speech.voice") {
            parts.push(self.msg_args("profile-summary-voice", &args!["voice" => v]));
        }
        if let Some(v) = profiles::value_text(p, "speech.rate") {
            parts.push(self.msg_args("profile-summary-rate", &args!["rate" => v]));
        }
        if let Some(v) = profiles::value_text(p, "display.theme") {
            parts.push(self.msg_args("profile-summary-theme", &args!["theme" => v]));
        }
        if let Some(v) = profiles::value_text(p, "accessibility.mode") {
            parts.push(self.msg_args(
                "profile-summary-access",
                &args!["mode" => v.replace('-', " ")],
            ));
        }
        if parts.is_empty() {
            self.msg("profile-summary-empty")
        } else {
            parts.join(", ")
        }
    }

    /// `settings_profiles`: the list.
    pub(crate) fn settings_profiles(&mut self) -> Vec<Effect> {
        let p = self.profiles();
        let mut entries: Vec<ProfileEntry> =
            p.names().into_iter().map(ProfileEntry::Profile).collect();
        let n = entries.len();
        entries.push(ProfileEntry::SaveNew);
        if let Some(active) = p.active.clone().filter(|a| p.profiles.contains_key(a)) {
            entries.push(ProfileEntry::Update(active));
        }
        entries.push(ProfileEntry::Import);
        if n > 0 {
            entries.push(ProfileEntry::Export);
        }
        let items: Vec<String> = entries
            .iter()
            .map(|e| match e {
                ProfileEntry::Profile(name) => {
                    let summary = self.profile_summary(&p, name);
                    let id = if p.active.as_deref() == Some(name) {
                        "profiles-item-active"
                    } else {
                        "profiles-item"
                    };
                    self.msg_args(id, &args!["name" => name, "summary" => summary])
                }
                ProfileEntry::SaveNew => self.msg("profiles-save-new"),
                ProfileEntry::Update(name) => {
                    self.msg_args("profiles-update", &args!["name" => name])
                }
                ProfileEntry::Import => self.msg("profiles-import"),
                ProfileEntry::Export => self.msg("profiles-export"),
            })
            .collect();
        let title = self.msg_args("profiles-title", &args!["n" => n]);
        let intro = self.msg_args("profiles-intro", &args!["title" => &title]);
        self.tell(&intro);
        self.list = Some(ListKind::Study(StudyList::Profiles(entries)));
        vec![Effect::ShowList { title, items }]
    }

    /// Switches to profile `name`: its settings take effect at once and are
    /// saved.
    pub(crate) fn switch_profile(&mut self, name: &str) {
        let mut p = self.profiles();
        let backend_before = self.settings.speech.backend.clone();
        let (settings, dropped) = match p.apply(name, &self.settings) {
            Ok(x) => x,
            Err(e) => {
                let m = self.profile_error(&e);
                self.error(&m);
                return;
            }
        };
        self.store_profiles(p);
        let old = std::mem::replace(&mut self.settings, settings);
        self.settings_dirty = true;
        // What the settings screen does after a change: speech, keys,
        // access mode, theme, and highlight colours.
        self.settings_changed(&old, "speech");
        self.settings_changed(&old, "highlight");
        let id = if self.settings.speech.backend == backend_before {
            "profile-switched"
        } else {
            "profile-switched-backend"
        };
        let mut m = self.msg_args(id, &args!["name" => name]);
        if !dropped.is_empty() {
            m.push(' ');
            m.push_str(&self.msg_args(
                "profile-dropped",
                &args!["n" => dropped.len(), "keys" => dropped.join(", ")],
            ));
        }
        self.tell(&m);
    }

    fn save_profile(&mut self, name: &str) {
        if self.paths.is_none() {
            self.error(&self.msg("profiles-no-persistence"));
            return;
        }
        let mut p = self.profiles();
        match p.save_current(name, &self.settings) {
            Ok(replaced) => {
                let saved = p.active.clone().unwrap_or_default();
                self.store_profiles(p);
                let id = if replaced {
                    "profile-replaced"
                } else {
                    "profile-saved"
                };
                self.tell(&self.msg_args(id, &args!["name" => saved]));
            }
            Err(e) => {
                let m = self.profile_error(&e);
                self.error(&m);
            }
        }
    }

    fn rename_profile(&mut self, from: &str, to: &str) {
        if to.trim().is_empty() || to.trim() == from {
            self.note(&self.msg("common-cancelled"));
            return;
        }
        let mut p = self.profiles();
        match p.rename(from, to) {
            Ok(()) => {
                self.store_profiles(p);
                let m = self.msg_args("profile-renamed", &args!["old" => from, "new" => to.trim()]);
                self.tell(&m);
            }
            Err(e) => {
                let m = self.profile_error(&e);
                self.error(&m);
            }
        }
    }

    fn export_profiles(&mut self, path: &Path) {
        let p = self.profiles();
        if p.profiles.is_empty() {
            self.tell(&self.msg("profiles-none-to-export"));
            return;
        }
        let toml = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("toml"));
        let written = p
            .export(None, toml)
            .map_err(|e| self.profile_error(&e))
            .and_then(|text| {
                textweaver_store::atomic_write(path, text.as_bytes()).map_err(|e| e.to_string())
            });
        match written {
            Ok(()) => {
                let m = self.msg_args(
                    "profiles-exported",
                    &args!["n" => p.profiles.len(), "file" => path.display().to_string()],
                );
                self.tell(&m);
            }
            Err(e) => {
                let m = self.msg_args("profiles-export-failed", &args!["error" => e]);
                self.error(&m);
            }
        }
    }

    fn import_profiles(&mut self, path: &Path) {
        if self.paths.is_none() {
            self.error(&self.msg("profiles-no-persistence"));
            return;
        }
        let mut p = self.profiles();
        match p.import_file(path) {
            Ok(r) => {
                let file = path.file_name().map_or_else(
                    || path.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                let mut m = self.msg_args(
                    "profiles-imported",
                    &args!["n" => r.imported.len(), "file" => file, "names" => r.imported.join(", ")],
                );
                if !r.dropped.is_empty() {
                    m.push(' ');
                    m.push_str(&self.msg_args(
                        "profile-dropped",
                        &args!["n" => r.dropped.len(), "keys" => r.dropped.join(", ")],
                    ));
                }
                if !r.imported.is_empty() {
                    self.store_profiles(p);
                }
                self.tell(&m);
            }
            Err(e) => {
                let m = self.profile_error(&e);
                self.error(&m);
            }
        }
    }

    /// Answers a yes-or-no question about deleting a profile, or about
    /// removing the reading statistics.
    pub(crate) fn confirm_study(&mut self, answer: Confirm) -> Vec<Effect> {
        if self.study.stats_question {
            return self.confirm_stats_clear(answer);
        }
        let Some(name) = self.study.question.clone() else {
            return vec![Effect::Redraw];
        };
        match answer {
            Confirm::Yes => {
                self.study.question = None;
                let mut p = self.profiles();
                match p.delete(&name) {
                    Ok(()) => {
                        self.store_profiles(p);
                        self.tell(&self.msg_args("profile-deleted", &args!["name" => &name]));
                    }
                    Err(e) => {
                        let m = self.profile_error(&e);
                        self.error(&m);
                    }
                }
                self.settings_profiles()
            }
            Confirm::No => {
                self.study.question = None;
                self.tell(&self.msg("profile-kept"));
                self.settings_profiles()
            }
            Confirm::Repeat => {
                let q = self.msg_args("profile-delete-question", &args!["name" => &name]);
                self.ask(&q);
                vec![Effect::Redraw]
            }
        }
    }

    // ----- Statistics ------------------------------------------------------

    /// This computer's statistics as saved, after the time not yet added.
    fn local_stats(&mut self) -> ReadingStats {
        self.stats_flush();
        self.writer.flush(Duration::from_secs(2));
        match &self.paths {
            Some(p) => ReadingStats::load(p).unwrap_or_default(),
            None => ReadingStats::default(),
        }
    }

    /// The question before removing the statistics, as `tw stats clear`
    /// asks it: how many documents, y or n.
    fn stats_clear_question(&mut self) -> String {
        let n = self.local_stats().documents.len();
        self.msg_args("stats-clear-question", &args!["n" => n])
    }

    /// Answers the question about removing the reading statistics: yes
    /// removes `stats.json` (the time not yet added goes with it), no
    /// keeps it.
    fn confirm_stats_clear(&mut self, answer: Confirm) -> Vec<Effect> {
        match answer {
            Confirm::Yes => {
                self.study.stats_question = false;
                self.stats_flush();
                self.writer.flush(Duration::from_secs(2));
                let removed = match &self.paths {
                    Some(p) => match std::fs::remove_file(p.stats_file()) {
                        Ok(()) => Ok(()),
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                        Err(e) => Err(e),
                    },
                    None => Ok(()),
                };
                match removed {
                    Ok(()) => self.tell(&self.msg("stats-cleared")),
                    Err(e) => {
                        let m =
                            self.msg_args("stats-clear-failed", &args!["error" => e.to_string()]);
                        self.error(&m);
                    }
                }
            }
            Confirm::No => {
                self.study.stats_question = false;
                self.tell(&self.msg("common-cancelled"));
            }
            Confirm::Repeat => {
                let q = self.stats_clear_question();
                self.ask(&q);
            }
        }
        vec![Effect::Redraw]
    }

    /// Starts counting for the document just opened.
    pub(crate) fn stats_open(&mut self) {
        let doc = self
            .session
            .as_ref()
            .map(|s| (s.key.0.clone(), s.title.clone(), s.doc.meta.path.clone()));
        self.study.stats = StatsTracker {
            doc,
            session_pending: true,
            ..StatsTracker::default()
        };
    }

    /// Counts reading time; called from [`App::tick`]. Adds it to the file
    /// every [`STATS_FLUSH`].
    pub(crate) fn stats_tick(&mut self, now: Instant) {
        if !self.settings.stats.enabled || self.study.stats.doc.is_none() {
            self.study.stats.last_tick = None;
            return;
        }
        let reading = matches!(self.playback, crate::playback::Playback::Reading);
        if reading {
            if let Some(last) = self.study.stats.last_tick {
                let gap = now.saturating_duration_since(last);
                if gap <= MAX_TICK_GAP {
                    self.study.stats.seconds += gap.as_secs_f64();
                }
            }
            self.study.stats.last_tick = Some(now);
            if let (Some(pos), Some(s)) = (self.reading_position(), self.session.as_ref()) {
                let pct = text_util::percent(&s.doc, pos);
                let t = &mut self.study.stats;
                t.furthest_percent = t.furthest_percent.max(pct);
                t.furthest_char = t.furthest_char.max(pos.0 as u64);
            }
        } else {
            self.study.stats.last_tick = None;
        }
        let due = self
            .study
            .stats
            .last_flush
            .is_none_or(|t| now.saturating_duration_since(t) >= STATS_FLUSH);
        if due && self.study.stats.seconds >= 1.0 {
            self.stats_flush();
            self.study.stats.last_flush = Some(now);
        } else if self.study.stats.last_flush.is_none() {
            self.study.stats.last_flush = Some(now);
        }
    }

    /// Adds the reading counted so far to `stats.json`, through the writer.
    pub(crate) fn stats_flush(&mut self) {
        let t = &mut self.study.stats;
        let Some((key, title, path)) = t.doc.clone() else {
            return;
        };
        if t.seconds < 0.5 && !(t.furthest_percent > 0 && t.session_pending) {
            return;
        }
        let new_session = t.session_pending && !t.counted;
        let delta = StatsDelta {
            key,
            title,
            path,
            seconds: std::mem::take(&mut t.seconds),
            new_session,
            furthest_percent: t.furthest_percent,
            furthest_char: t.furthest_char,
            at: textweaver_store::now_ts(),
        };
        if new_session {
            t.counted = true;
            t.session_pending = false;
        }
        if !self.settings.stats.enabled {
            return;
        }
        if let Some(paths) = self.paths.clone() {
            self.writer.send(crate::writer::Job::Stats {
                paths,
                deltas: vec![delta],
            });
        }
    }

    /// `reading_statistics`: the list, after its introduction.
    pub(crate) fn reading_statistics(&mut self) -> Vec<Effect> {
        let intro = self.msg(if self.settings.stats.enabled {
            "stats-intro"
        } else {
            "stats-off"
        });
        self.tell(&intro);
        self.statistics_list()
    }

    /// The statistics list, built again from `stats.json` and, with sync
    /// on and its statistics group on, the other computers' reading (S6):
    /// each document's time and sessions are summed over every computer,
    /// and each computer's share is listed under its document on request.
    fn statistics_list(&mut self) -> Vec<Effect> {
        let local = self.local_stats();
        let (synced, slow) = self.synced_for_statistics();
        let stats = CombinedStats::build(&local, synced.as_ref());
        let c = self.study.catalog.clone();
        let mut entries = Vec::new();
        let mut items = Vec::new();
        entries.push(StatsEntry::Info);
        if stats.documents.is_empty() {
            items.push(c.tr("stats-empty"));
        } else {
            items.push(c.fmt(
                "stats-total",
                &args![
                    "time" => duration(&c, stats.total_seconds()),
                    "sessions" => stats.total_sessions(),
                    "docs" => stats.documents.len()
                ],
            ));
        }
        if slow {
            entries.push(StatsEntry::Info);
            items.push(c.tr("stats-others-slow"));
        }
        if let Some(key) = self.session.as_ref().map(|s| s.key.0.clone()) {
            entries.push(StatsEntry::Info);
            items.push(match stats.by_key(&key) {
                Some(d) => c.fmt(
                    "stats-current",
                    &args![
                        "time" => duration(&c, d.seconds),
                        "pct" => d.furthest_percent,
                        "sessions" => d.sessions
                    ],
                ),
                None => c.tr("stats-current-none"),
            });
        }
        let by_computer = self.study.stats_by_computer && stats.has_others();
        for (rank, d) in stats.documents.iter().take(MOST_READ).enumerate() {
            items.push(c.fmt(
                "stats-most-read",
                &args![
                    "rank" => rank + 1,
                    "title" => stats_title(&c, d),
                    "time" => duration(&c, d.seconds),
                    "pct" => d.furthest_percent
                ],
            ));
            entries.push(match &d.path {
                Some(p) => StatsEntry::Document(p.clone()),
                None => StatsEntry::Info,
            });
            if by_computer && d.from_others() {
                for share in &d.computers {
                    items.push(computer_line(&c, share));
                    entries.push(StatsEntry::Info);
                }
            }
        }
        if stats.has_others() {
            entries.push(StatsEntry::ByComputer);
            items.push(c.tr(if by_computer {
                "stats-by-computer-on"
            } else {
                "stats-by-computer-off"
            }));
        }
        entries.push(StatsEntry::Toggle);
        items.push(c.tr(if self.settings.stats.enabled {
            "stats-toggle-on"
        } else {
            "stats-toggle-off"
        }));
        // Last, when this computer has statistics: remove them.
        if !local.documents.is_empty() {
            entries.push(StatsEntry::Clear);
            items.push(c.tr("stats-clear-item"));
        }
        let title = c.tr("stats-title");
        self.list = Some(ListKind::Study(StudyList::Statistics(entries)));
        vec![Effect::ShowList { title, items }]
    }

    /// The sync folder, read for the statistics list on a helper thread
    /// (a USB stick or a slow network folder must not hold up the keys for
    /// long), with sync and its statistics group on. The second value is
    /// true when reading took too long and the list shows only this
    /// computer's reading.
    fn synced_for_statistics(&self) -> (Option<SyncedLibrary>, bool) {
        let Some(paths) = self.paths.clone() else {
            return (None, false);
        };
        if !self.sync_enabled() || !self.settings.sync.statistics {
            return (None, false);
        }
        let settings = self.settings.clone();
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("textweaver-sync-stats".into())
            .spawn(move || {
                let _ = tx.send(SyncedLibrary::load(&paths, &settings));
            });
        if spawned.is_err() {
            return (None, true);
        }
        match rx.recv_timeout(SYNC_STATS_WAIT) {
            Ok(synced) => (synced, false),
            Err(_) => (None, true),
        }
    }
    // ----- Lists and prompts ----------------------------------------------

    /// Enter on an item of a study list.
    pub(crate) fn choose_study(&mut self, list: StudyList, n: usize) -> Vec<Effect> {
        match list {
            StudyList::Definition(items) => {
                if let Some(text) = items.get(n) {
                    self.clipboard = Some(text.clone());
                    self.tell(&self.msg("define-copied"));
                }
                vec![Effect::Redraw]
            }
            StudyList::Profiles(entries) => match entries.get(n) {
                Some(ProfileEntry::Profile(name)) => {
                    let name = name.clone();
                    self.switch_profile(&name);
                    vec![Effect::Redraw]
                }
                Some(ProfileEntry::SaveNew) => self.prompt(PromptPurpose::ProfileName),
                Some(ProfileEntry::Update(name)) => {
                    let name = name.clone();
                    self.save_profile(&name);
                    vec![Effect::Redraw]
                }
                Some(ProfileEntry::Import) => self.prompt(PromptPurpose::ImportProfiles),
                Some(ProfileEntry::Export) => self.prompt(PromptPurpose::ExportProfiles),
                None => vec![Effect::Redraw],
            },
            StudyList::Statistics(entries) => match entries.get(n) {
                Some(StatsEntry::Document(path)) => self.open_command(path.clone()),
                Some(StatsEntry::Toggle) => {
                    self.stats_flush();
                    self.settings.stats.enabled = !self.settings.stats.enabled;
                    self.settings_dirty = true;
                    let m = self.msg(if self.settings.stats.enabled {
                        "stats-turned-on"
                    } else {
                        "stats-turned-off"
                    });
                    self.tell(&m);
                    // The list stays, on the same row, which now says the
                    // other state.
                    self.pending_list_focus = Some(n);
                    self.statistics_list()
                }
                Some(StatsEntry::ByComputer) => {
                    self.study.stats_by_computer = !self.study.stats_by_computer;
                    self.pending_list_focus = Some(n);
                    let list = self.statistics_list();
                    // The row moved down past the new lines; it stays
                    // focused.
                    if let Some(ListKind::Study(StudyList::Statistics(e))) = &self.list {
                        self.pending_list_focus =
                            e.iter().position(|x| *x == StatsEntry::ByComputer);
                    }
                    list
                }
                Some(StatsEntry::Clear) => {
                    self.list = None;
                    self.study.stats_question = true;
                    let q = self.stats_clear_question();
                    self.ask(&q);
                    vec![Effect::Redraw]
                }
                // An information row: nothing to do, so the list stays.
                Some(StatsEntry::Info) => {
                    self.pending_list_focus = Some(n);
                    self.statistics_list()
                }
                None => vec![Effect::Redraw],
            },
        }
    }

    /// Delete on an item of a study list: asks before deleting a profile.
    pub(crate) fn delete_study_item(&mut self, list: StudyList, n: usize) -> Vec<Effect> {
        if let StudyList::Profiles(entries) = &list
            && let Some(ProfileEntry::Profile(name)) = entries.get(n)
        {
            self.list = None;
            self.study.question = Some(name.clone());
            let q = self.msg_args("profile-delete-question", &args!["name" => name]);
            self.ask(&q);
        } else {
            self.list = Some(ListKind::Study(list));
            self.tell(&self.msg("study-nothing-to-delete"));
        }
        vec![Effect::Redraw]
    }

    /// F2 on an item of a study list: renames a profile.
    pub(crate) fn rename_study_item(&mut self, list: StudyList, n: usize) -> Vec<Effect> {
        if let StudyList::Profiles(entries) = &list
            && let Some(ProfileEntry::Profile(name)) = entries.get(n)
        {
            self.list = None;
            self.study.pending_profile = Some(name.clone());
            let mut e = self.prompt(PromptPurpose::RenameProfile);
            e.push(Effect::Redraw);
            return e;
        }
        self.list = Some(ListKind::Study(list));
        self.tell(&self.msg("study-nothing-to-rename"));
        vec![Effect::Redraw]
    }

    /// The answers of the study prompts.
    pub(crate) fn answer_study(&mut self, purpose: PromptPurpose, text: &str) -> Vec<Effect> {
        let path = || {
            let t = text.trim().trim_matches('"');
            (!t.is_empty()).then(|| PathBuf::from(t))
        };
        match purpose {
            PromptPurpose::DefineWord => {
                if text.trim().is_empty() {
                    self.note(&self.msg("common-cancelled"));
                } else {
                    return self.define(text);
                }
            }
            PromptPurpose::ProfileName => {
                if text.trim().is_empty() {
                    self.note(&self.msg("common-cancelled"));
                } else {
                    self.save_profile(text);
                }
            }
            PromptPurpose::RenameProfile => {
                if let Some(from) = self.study.pending_profile.take() {
                    self.rename_profile(&from, text);
                }
            }
            PromptPurpose::ImportProfiles => match path() {
                Some(p) => self.import_profiles(&p),
                None => self.note(&self.msg("common-cancelled")),
            },
            PromptPurpose::ExportProfiles => match path() {
                Some(p) => self.export_profiles(&p),
                None => self.note(&self.msg("common-cancelled")),
            },
            _ => {}
        }
        vec![Effect::Redraw]
    }
}

#[cfg(test)]
mod tests;
