//! State for the Phase 2 authoring and navigation features (Agent P2b):
//! structure while editing, the outline, citations, export and preview,
//! spell checking, links, find and replace, and templates. It lives in one
//! field of [`App`](crate::App) so the features stay out of the app's core
//! state.

// Without the `publish` feature the export and lookup jobs are never made.
#![cfg_attr(not(feature = "publish"), allow(dead_code))]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use textweaver_text::Marker;

/// Opens a file or a web address with the system's default program.
pub type Launcher = Arc<dyn Fn(&str) -> std::io::Result<()> + Send + Sync>;

/// Makes the HTTP client for reference lookups (DOI, ISBN); called on the
/// lookup thread.
pub type ClientFactory = Arc<dyn Fn() -> Box<dyn crate::citations::HttpClient> + Send + Sync>;

/// Re-parsing the Markdown being edited.
#[derive(Debug, Default)]
pub(crate) struct Structure {
    /// The text being edited is Markdown (its structure is parsed).
    pub(crate) markdown: bool,
    /// Bumped by every edit.
    pub(crate) version: u64,
    /// The version the document's markers were parsed at.
    pub(crate) parsed: u64,
    /// When the text last changed.
    pub(crate) last_edit: Option<Instant>,
    /// A parse running on another thread: the version it parses, and where
    /// its markers arrive.
    pub(crate) pending: Option<(u64, Receiver<Vec<Marker>>)>,
}

impl Structure {
    /// True when the markers match the text.
    pub(crate) fn fresh(&self) -> bool {
        self.parsed == self.version
    }
}

/// Work running off the UI thread whose result is announced when it
/// arrives (exports, reference lookups).
pub(crate) enum Job {
    /// An export or preview: what it was, and the file written (with the
    /// writer's warnings) or the error.
    Export {
        what: String,
        kind: ExportKind,
        rx: Receiver<Result<ExportDone, String>>,
        /// When it started, and when progress was last said.
        progress: Progress,
    },
    /// A DOI or ISBN lookup.
    Lookup {
        input: String,
        rx: Receiver<Result<crate::citations::Reference, String>>,
    },
}

/// One heading in the outline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OutlineItem {
    pub(crate) pos: textweaver_core::CharPos,
    pub(crate) level: u8,
    pub(crate) text: String,
}

/// One choice in the spelling list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SpellChoice {
    /// Replace the word with this one.
    Replace(String),
    /// Add the word to the personal word list.
    Add,
    /// Leave it.
    Ignore,
}

/// The lists the authoring features show (see [`crate::lists`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AuthoringList {
    /// The headings; `shown` holds the indexes that match the filter.
    Outline {
        items: Vec<OutlineItem>,
        shown: Vec<usize>,
    },
    /// The citation picker; `shown` holds the indexes that match.
    Citations {
        entries: Vec<crate::citations::PickerEntry>,
        shown: Vec<usize>,
    },
    /// Spelling choices for the word at `range`.
    Spelling {
        word: String,
        range: textweaver_core::CharRange,
        choices: Vec<SpellChoice>,
    },
    /// Fixes for the grammar problem at `range` (Agent W4g), then "Leave
    /// it as it is".
    Grammar {
        words: String,
        range: textweaver_core::CharRange,
        fixes: Vec<String>,
    },
    /// What to do with the current match of find and replace.
    Replace,
    /// Templates for a new document.
    Templates(Vec<crate::templates::Template>),
}

impl AuthoringList {
    /// True for lists that type-to-filter.
    pub(crate) fn filterable(&self) -> bool {
        matches!(
            self,
            AuthoringList::Outline { .. } | AuthoringList::Citations { .. }
        )
    }
}

/// What an export job is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExportKind {
    /// An export next to the document: say where, offer to open it.
    Export,
    /// The first preview: open it in the browser.
    PreviewOpen,
    /// The preview rewritten after a save.
    PreviewRefresh,
    /// The preview rewritten after typing paused (`[preview] live`).
    PreviewLive,
}

/// A finished export or preview.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ExportDone {
    /// The file written.
    pub(crate) path: PathBuf,
    /// The writer's warnings.
    pub(crate) warnings: Vec<String>,
    /// For a preview: its headings' ids and texts, in order, so a reload
    /// can land on the heading nearest the caret.
    pub(crate) headings: Vec<(String, String)>,
}

/// Progress announcements for a long export: the first after
/// [`Progress::FIRST`], then every [`Progress::EVERY`], so a slow export
/// is heard without flooding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Progress {
    pub(crate) started: Instant,
    pub(crate) next: Instant,
}

impl Progress {
    /// When the first "still exporting" is said.
    pub(crate) const FIRST: std::time::Duration = std::time::Duration::from_secs(2);
    /// How often it is said after that.
    pub(crate) const EVERY: std::time::Duration = std::time::Duration::from_secs(10);

    /// Progress from `now`.
    pub(crate) fn new(now: Instant) -> Self {
        Progress {
            started: now,
            next: now + Self::FIRST,
        }
    }

    /// The whole seconds since the start when a progress announcement is
    /// due at `now` (and schedules the next one); `None` otherwise.
    pub(crate) fn due(&mut self, now: Instant) -> Option<u64> {
        if now < self.next {
            return None;
        }
        self.next = now + Self::EVERY;
        Some(now.saturating_duration_since(self.started).as_secs())
    }
}

/// A yes or no question this module asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Question {
    /// Open this file or address with the default program?
    Open(String),
}

/// The authoring state.
#[derive(Default)]
pub(crate) struct Authoring {
    pub(crate) structure: Structure,
    pub(crate) prefetch: crate::structure::Prefetch,
    /// How files and addresses are opened; `None` is the system's default
    /// program, and only in a session that keeps files (not in tests).
    pub(crate) launcher: Option<Launcher>,
    pub(crate) client: Option<ClientFactory>,
    pub(crate) jobs: Vec<Job>,
    pub(crate) question: Option<Question>,
    /// Filter typed into a filterable list, and the list's full items.
    pub(crate) filter: String,
    /// The HTML preview file, rewritten on each save while set.
    pub(crate) preview: Option<PathBuf>,
    /// The text last copied or cut in textweaver, for Paste.
    pub(crate) copied: Option<String>,
    /// Listening to the rendered text: which reading, and how its
    /// positions map back to the source being edited.
    pub(crate) listening: Option<Listening>,
    /// Links followed into other files, for going back.
    pub(crate) link_back: Vec<LinkBack>,
    /// The citation being inserted: its key, waiting for the locator.
    pub(crate) citing: Option<String>,
    /// The personal word list, loaded on first use.
    pub(crate) words: Option<std::collections::BTreeSet<String>>,
    /// Harper's dictionary and rules, loaded on the first grammar check.
    #[cfg(feature = "grammar")]
    pub(crate) grammar: Option<crate::grammar::GrammarChecker>,
    /// Find and replace, one match at a time.
    pub(crate) replace: Option<crate::replace::ReplaceSession>,
    /// The template chosen for a new document, waiting for its title.
    pub(crate) template: Option<crate::templates::Template>,
    /// The note last signalled while reading, so it is signalled once.
    pub(crate) note_signalled: Option<String>,
    /// Libraries loaded for reading citations aloud.
    pub(crate) cite_cache: Option<crate::citations::CachedLibraries>,
    /// The edit version the preview last showed (`[preview] live`).
    pub(crate) preview_version: u64,
    /// The document folder of the preview, for its images.
    pub(crate) preview_folder: Option<PathBuf>,
}

/// Listening to the rendered text while editing.
pub(crate) struct Listening {
    pub(crate) generation: textweaver_speech::ReadingGeneration,
    pub(crate) rendered: textweaver_text::Document,
    pub(crate) map: crate::structure::SourceMap,
}

/// A link followed into another file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LinkBack {
    /// The file the link was in, and where.
    pub(crate) from: PathBuf,
    pub(crate) pos: textweaver_core::CharPos,
    /// The file it opened, and its history length on arrival.
    pub(crate) to: PathBuf,
    pub(crate) history_len: usize,
}

impl std::fmt::Debug for Authoring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Authoring")
            .field("structure", &self.structure)
            .field("jobs", &self.jobs.len())
            .field("question", &self.question)
            .field("preview", &self.preview)
            .finish()
    }
}

/// Opens `target` (a file path or an address) with the default program:
/// `start` on Windows, `open` on macOS, `xdg-open` elsewhere.
pub fn open_with_system(target: &str) -> std::io::Result<()> {
    use std::process::{Command, Stdio};
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        // The empty string is `start`'s window title.
        c.args(["/C", "start", ""]).arg(target);
        c
    } else if cfg!(target_os = "macos") {
        let mut c = Command::new("open");
        c.arg(target);
        c
    } else {
        let mut c = Command::new("xdg-open");
        c.arg(target);
        c
    };
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
}

/// A file name for speech: the name alone, else the path.
pub(crate) fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}
