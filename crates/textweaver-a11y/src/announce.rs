//! The catalog of announcements: fixed wording, a priority, and the text
//! for each verbosity level. Frontends announce state changes through
//! [`Announcement`] so wording and verbosity rules live in one place.

use serde::{Deserialize, Serialize};
use textweaver_core::Verbosity;

use crate::Priority;

/// A user-visible state change worth announcing.
///
/// [`text`](Self::text) gives what to say at a verbosity level, or `None`
/// when nothing is said at that level. The crate docs hold the verbosity
/// table (checked against [`verbosity_table`] by a test).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Announcement {
    // ---- Playback ----
    /// Reading started.
    Playing {
        /// Words per minute.
        wpm: u16,
    },
    /// Reading paused.
    Paused,
    /// Reading resumed after a pause.
    Resumed,
    /// Reading stopped.
    Stopped,
    /// Reading reached the end of the document.
    FinishedReading,

    // ---- Voice ----
    /// The rate changed.
    Rate {
        /// Words per minute.
        wpm: u16,
    },
    /// The pitch changed.
    Pitch {
        /// Semitones from the voice's default.
        semitones: i8,
    },
    /// The volume changed.
    Volume {
        /// Percent.
        percent: u8,
    },
    /// A speed preset was selected.
    SpeedPreset {
        /// Preset name.
        name: String,
        /// Its rate.
        wpm: u16,
    },
    /// The voice changed.
    Voice {
        /// Voice name.
        name: String,
    },

    // ---- Documents ----
    /// A document opened.
    Opened {
        /// Its title.
        title: String,
    },
    /// A document could not be opened.
    OpenFailed {
        /// The title or file name.
        title: String,
        /// Why, in plain words.
        reason: String,
    },
    /// The saved reading position was restored.
    ResumedAt {
        /// Percentage through the document.
        pct: u8,
        /// When it was saved, as a date for people.
        saved: Option<String>,
    },
    /// The saved position was cleared (`had` false: there was none).
    PositionCleared {
        /// Whether a position existed.
        had: bool,
    },

    // ---- Navigation ----
    /// The cursor is on a heading.
    Heading {
        /// Level 1 to 6.
        level: u8,
        /// Heading text.
        text: String,
    },
    /// The cursor entered a table.
    Table {
        /// Rows, including the header.
        rows: usize,
        /// Columns.
        cols: usize,
    },
    /// The cursor entered a list.
    List {
        /// Number of items.
        items: usize,
    },
    /// The cursor is on a link.
    Link {
        /// Link text.
        text: String,
    },
    /// Nothing further in that direction: "No next heading".
    NoNext {
        /// What was looked for, as a noun ("heading").
        what: String,
    },
    /// Nothing earlier: "No previous table".
    NoPrevious {
        /// What was looked for, as a noun.
        what: String,
    },
    /// At the start of the document.
    StartOfDocument,
    /// At the end of the document.
    EndOfDocument,
    /// Speech Cursor read an empty line.
    BlankLine,
    /// Where the cursor is (the "say position" command).
    Position {
        /// Line number, from 1.
        line: usize,
        /// Number of lines.
        lines: usize,
        /// Percentage through the document.
        pct: u8,
        /// The enclosing heading, if any.
        heading: Option<String>,
    },
    /// Moved through the navigation history.
    History {
        /// Position in history, from 1.
        index: usize,
        /// Entries in history.
        total: usize,
    },
    /// Back at the live position after moving through history.
    HistoryPresent,
    /// No history in the requested direction.
    NoHistory {
        /// True for back, false for forward.
        back: bool,
    },

    // ---- Modes ----
    /// Speech Cursor mode turned on or off.
    SpeechCursor {
        /// On or off.
        on: bool,
    },
    /// Edit mode turned on or off.
    EditMode {
        /// On or off.
        on: bool,
    },
    /// A key hint, said only at high verbosity: "Tab leaves Speech Cursor".
    Hint {
        /// The hint.
        text: String,
    },

    // ---- Find ----
    /// A match was found.
    Match {
        /// Its number, from 1.
        index: usize,
        /// Number of matches.
        total: usize,
    },
    /// Search went past the end and continued from the other end.
    SearchWrapped {
        /// True when it wrapped to the top.
        to_top: bool,
    },
    /// Nothing matched.
    NoMatches {
        /// The search text.
        query: String,
    },

    // ---- Bookmarks ----
    /// A bookmark was set.
    BookmarkAdded {
        /// Its name.
        name: String,
        /// Percentage through the document.
        pct: u8,
    },
    /// Moved to a bookmark.
    BookmarkReached {
        /// Its name.
        name: String,
    },
    /// A bookmark was deleted.
    BookmarkDeleted {
        /// Its name.
        name: String,
    },
    /// The document has no bookmarks.
    NoBookmarks,

    // ---- Notes and highlights ----
    /// A note was added.
    NoteAdded {
        /// Percentage through the document.
        pct: u8,
    },
    /// Text was highlighted.
    HighlightAdded {
        /// The color's name ("yellow").
        color: String,
    },
    /// Moved to a note or highlight; `text` is what it says, as the store
    /// words it ("Note: check this, on \u{201c}claim\u{201d}", "Yellow
    /// highlight: key idea").
    NoteReached {
        /// The spoken description.
        text: String,
    },
    /// A note or highlight was deleted.
    NoteDeleted {
        /// True for a highlight.
        highlight: bool,
    },
    /// The document has no notes or highlights.
    NoNotes,

    // ---- Library and sync ----
    /// A folder was added to the library.
    LibraryFolderAdded {
        /// The folder's name.
        name: String,
        /// Documents found in it.
        documents: usize,
    },
    /// Reading positions from another device were merged.
    SyncMerged {
        /// Entries that differed between the devices.
        conflicts: usize,
        /// True under the `manual` policy: nothing was chosen, the local
        /// entries were kept, and the user should decide.
        manual: bool,
    },

    // ---- Editing ----
    /// A new empty document is ready.
    NewDocument,
    /// The document was saved.
    Saved {
        /// The file name.
        name: String,
    },
    /// Saving failed.
    SaveFailed {
        /// Why, in plain words.
        reason: String,
    },
    /// An edit was undone.
    Undo,
    /// An edit was redone.
    Redo,
    /// Nothing to undo or redo.
    NothingToUndo {
        /// True for redo.
        redo: bool,
    },
    /// Formatting was asked for outside edit mode.
    FormatNeedsEditMode,
    /// Find and replace replaced text.
    Replaced {
        /// Number of replacements.
        count: usize,
    },
    /// A table was inserted.
    TableInserted {
        /// Body rows.
        rows: usize,
        /// Columns.
        cols: usize,
    },
    /// An image reference was inserted.
    ImageInserted {
        /// The image file name.
        name: String,
    },
    /// Work from an autosave snapshot was recovered.
    Recovered,

    // ---- Settings and view ----
    /// The theme changed.
    Theme {
        /// Theme name.
        name: String,
    },
    /// Line numbers shown or hidden.
    LineNumbers {
        /// On or off.
        on: bool,
    },
    /// Single-key shortcuts turned on or off (`toggle_character_keys`).
    CharacterKeys {
        /// On or off.
        on: bool,
    },
    /// Something went wrong; said at every level.
    Error {
        /// The message.
        message: String,
    },
    /// Any other message, said at normal verbosity and above.
    Info {
        /// The message.
        message: String,
    },
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn on_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

fn pitch_text(s: i8) -> String {
    match s {
        0 => "normal".to_owned(),
        s if s > 0 => format!("plus {s}"),
        s => format!("minus {}", s.unsigned_abs()),
    }
}

impl Announcement {
    /// A stable id for the announcement's kind (`"rate"`, `"no_next"`).
    pub fn kind(&self) -> &'static str {
        use Announcement as A;
        match self {
            A::Playing { .. } => "playing",
            A::Paused => "paused",
            A::Resumed => "resumed",
            A::Stopped => "stopped",
            A::FinishedReading => "finished_reading",
            A::Rate { .. } => "rate",
            A::Pitch { .. } => "pitch",
            A::Volume { .. } => "volume",
            A::SpeedPreset { .. } => "speed_preset",
            A::Voice { .. } => "voice",
            A::Opened { .. } => "opened",
            A::OpenFailed { .. } => "open_failed",
            A::ResumedAt { .. } => "resumed_at",
            A::PositionCleared { .. } => "position_cleared",
            A::Heading { .. } => "heading",
            A::Table { .. } => "table",
            A::List { .. } => "list",
            A::Link { .. } => "link",
            A::NoNext { .. } => "no_next",
            A::NoPrevious { .. } => "no_previous",
            A::StartOfDocument => "start_of_document",
            A::EndOfDocument => "end_of_document",
            A::BlankLine => "blank_line",
            A::Position { .. } => "position",
            A::History { .. } => "history",
            A::HistoryPresent => "history_present",
            A::NoHistory { .. } => "no_history",
            A::SpeechCursor { .. } => "speech_cursor",
            A::EditMode { .. } => "edit_mode",
            A::Hint { .. } => "hint",
            A::Match { .. } => "match",
            A::SearchWrapped { .. } => "search_wrapped",
            A::NoMatches { .. } => "no_matches",
            A::BookmarkAdded { .. } => "bookmark_added",
            A::BookmarkReached { .. } => "bookmark_reached",
            A::BookmarkDeleted { .. } => "bookmark_deleted",
            A::NoBookmarks => "no_bookmarks",
            A::NoteAdded { .. } => "note_added",
            A::HighlightAdded { .. } => "highlight_added",
            A::NoteReached { .. } => "note_reached",
            A::NoteDeleted { .. } => "note_deleted",
            A::NoNotes => "no_notes",
            A::LibraryFolderAdded { .. } => "library_folder_added",
            A::SyncMerged { .. } => "sync_merged",
            A::NewDocument => "new_document",
            A::Saved { .. } => "saved",
            A::SaveFailed { .. } => "save_failed",
            A::Undo => "undo",
            A::Redo => "redo",
            A::NothingToUndo { .. } => "nothing_to_undo",
            A::FormatNeedsEditMode => "format_needs_edit_mode",
            A::Replaced { .. } => "replaced",
            A::TableInserted { .. } => "table_inserted",
            A::ImageInserted { .. } => "image_inserted",
            A::Recovered => "recovered",
            A::Theme { .. } => "theme",
            A::LineNumbers { .. } => "line_numbers",
            A::CharacterKeys { .. } => "character_keys",
            A::Error { .. } => "error",
            A::Info { .. } => "info",
        }
    }

    /// How urgently to deliver it. Direct answers to a key press and errors
    /// interrupt; background information waits.
    pub fn priority(&self) -> Priority {
        use Announcement as A;
        match self {
            A::Opened { .. }
            | A::ResumedAt { .. }
            | A::Saved { .. }
            | A::Recovered
            | A::FinishedReading
            | A::Hint { .. }
            | A::Info { .. }
            | A::Heading { .. }
            | A::Table { .. }
            | A::List { .. }
            | A::Link { .. }
            | A::SyncMerged { manual: false, .. } => Priority::Polite,
            _ => Priority::Assertive,
        }
    }

    /// The lowest verbosity at which anything is said.
    pub fn min_verbosity(&self) -> Verbosity {
        [Verbosity::Low, Verbosity::Normal, Verbosity::High]
            .into_iter()
            .find(|v| self.text(*v).is_some())
            .unwrap_or(Verbosity::High)
    }

    /// What to say at `verbosity`, or `None` to stay quiet.
    pub fn text(&self, verbosity: Verbosity) -> Option<String> {
        use Announcement as A;
        use Verbosity::{High, Low, Normal};
        let v = verbosity;
        let s = match self {
            A::Playing { wpm } => match v {
                High => format!("Reading at {wpm} words per minute"),
                _ => return None,
            },
            A::Paused => match v {
                Low => return None,
                _ => "Paused".to_owned(),
            },
            A::Resumed => match v {
                High => "Resuming".to_owned(),
                _ => return None,
            },
            A::Stopped => match v {
                Low => return None,
                _ => "Stopped".to_owned(),
            },
            A::FinishedReading => match v {
                Low => return None,
                _ => "End of document".to_owned(),
            },
            A::Rate { wpm } => match v {
                Low => wpm.to_string(),
                _ => format!("Rate {wpm} words per minute"),
            },
            A::Pitch { semitones } => format!("Pitch {}", pitch_text(*semitones)),
            A::Volume { percent } => match v {
                Low => format!("{percent} percent"),
                _ => format!("Volume {percent} percent"),
            },
            A::SpeedPreset { name, wpm } => match v {
                Low => name.clone(),
                _ => format!("Preset {name}, {wpm} words per minute"),
            },
            A::Voice { name } => format!("Voice {name}"),
            A::Opened { title } => match v {
                Low => title.clone(),
                _ => format!("Opened {title}"),
            },
            A::OpenFailed { title, reason } => format!("Could not open {title}: {reason}"),
            A::ResumedAt { pct, saved } => match (v, saved) {
                (Low, _) => return None,
                (High, Some(date)) => format!("Resumed at {pct} percent, saved {date}"),
                _ => format!("Resumed at {pct} percent"),
            },
            A::PositionCleared { had: true } => "Reading position cleared".to_owned(),
            A::PositionCleared { had: false } => "No saved position for this document".to_owned(),
            A::Heading { level, text } => match v {
                Low => text.clone(),
                _ => format!("Heading level {level}, {text}"),
            },
            A::Table { rows, cols } => match v {
                Low => "Table".to_owned(),
                _ => format!(
                    "Table with {} and {}",
                    plural(*rows, "row", "rows"),
                    plural(*cols, "column", "columns")
                ),
            },
            A::List { items } => match v {
                Low => "List".to_owned(),
                _ => format!("List with {}", plural(*items, "item", "items")),
            },
            A::Link { text } => match v {
                Low => text.clone(),
                _ => format!("Link, {text}"),
            },
            A::NoNext { what } => format!("No next {what}"),
            A::NoPrevious { what } => format!("No previous {what}"),
            A::StartOfDocument => "Start of document".to_owned(),
            A::EndOfDocument => "End of document".to_owned(),
            A::BlankLine => "blank".to_owned(),
            A::Position {
                line,
                lines,
                pct,
                heading,
            } => {
                let mut s = match v {
                    High => format!("Line {line} of {lines}, {pct} percent"),
                    _ => format!("Line {line}, {pct} percent"),
                };
                if let (Normal | High, Some(h)) = (v, heading) {
                    s.push_str(&format!(", in {h}"));
                }
                s
            }
            A::History { index, total } => match v {
                High => format!("History position {index} of {total}"),
                _ => return None,
            },
            A::HistoryPresent => match v {
                Low => return None,
                _ => "Back at the present position".to_owned(),
            },
            A::NoHistory { back: true } => "No earlier history".to_owned(),
            A::NoHistory { back: false } => "No forward history".to_owned(),
            A::SpeechCursor { on } => format!("Speech cursor {}", on_off(*on)),
            A::EditMode { on: true } => "Edit mode".to_owned(),
            A::EditMode { on: false } => "Read mode".to_owned(),
            A::Hint { text } => match v {
                High => text.clone(),
                _ => return None,
            },
            A::Match { index, total } => match v {
                Low => format!("{index} of {total}"),
                _ => format!("Match {index} of {total}"),
            },
            A::SearchWrapped { to_top } => match v {
                Low => return None,
                _ if *to_top => "Search wrapped to the top".to_owned(),
                _ => "Search wrapped to the bottom".to_owned(),
            },
            A::NoMatches { query } => format!("No matches for {query}"),
            A::BookmarkAdded { name, pct } => match v {
                Low => format!("Bookmark {name}"),
                _ => format!("Bookmark {name} set at {pct} percent"),
            },
            A::BookmarkReached { name } => format!("Bookmark {name}"),
            A::BookmarkDeleted { name } => format!("Bookmark {name} deleted"),
            A::NoBookmarks => "No bookmarks".to_owned(),
            A::NoteAdded { pct } => match v {
                Low => "Note added".to_owned(),
                _ => format!("Note added at {pct} percent"),
            },
            A::HighlightAdded { color } => match v {
                Low => "Highlighted".to_owned(),
                _ => format!("Highlighted in {color}"),
            },
            A::NoteReached { text } => text.clone(),
            A::NoteDeleted { highlight: false } => "Note deleted".to_owned(),
            A::NoteDeleted { highlight: true } => "Highlight removed".to_owned(),
            A::NoNotes => "No notes or highlights".to_owned(),
            A::LibraryFolderAdded { name, documents } => match v {
                Low => format!("Added {name}"),
                _ => format!(
                    "Added folder {name} with {}",
                    plural(*documents, "document", "documents")
                ),
            },
            A::SyncMerged {
                conflicts,
                manual: true,
            } => format!(
                "{} another device. Kept this device's; choose in the library",
                if *conflicts == 1 {
                    "1 reading position differs from".to_owned()
                } else {
                    format!("{conflicts} reading positions differ from")
                }
            ),
            A::SyncMerged {
                conflicts,
                manual: false,
            } => match v {
                Low => return None,
                _ => format!(
                    "Merged {} from another device",
                    plural(*conflicts, "change", "changes")
                ),
            },
            A::NewDocument => "New document, ready for editing".to_owned(),
            A::Saved { name } => match v {
                Low => "Saved".to_owned(),
                _ => format!("Saved {name}"),
            },
            A::SaveFailed { reason } => format!("Could not save: {reason}"),
            A::Undo => match v {
                Low => return None,
                _ => "Undo".to_owned(),
            },
            A::Redo => match v {
                Low => return None,
                _ => "Redo".to_owned(),
            },
            A::NothingToUndo { redo: false } => "Nothing to undo".to_owned(),
            A::NothingToUndo { redo: true } => "Nothing to redo".to_owned(),
            A::FormatNeedsEditMode => "Turn on edit mode to format text".to_owned(),
            A::Replaced { count: 0 } => "No matches".to_owned(),
            A::Replaced { count } => format!("Replaced {}", plural(*count, "match", "matches")),
            A::TableInserted { rows, cols } => format!(
                "Inserted a table with {} and {}",
                plural(*rows, "row", "rows"),
                plural(*cols, "column", "columns")
            ),
            A::ImageInserted { name } => format!("Inserted image {name}"),
            A::Recovered => "Recovered unsaved work. Remember to save.".to_owned(),
            A::Theme { name } => format!("Theme {name}"),
            A::LineNumbers { on } => format!("Line numbers {}", on_off(*on)),
            A::CharacterKeys { on } => format!("Single-key shortcuts {}", on_off(*on)),
            A::Error { message } => message.clone(),
            A::Info { message } => match v {
                Low => return None,
                _ => message.clone(),
            },
        };
        Some(s)
    }

    /// One example of every announcement, for the verbosity table and tests.
    pub fn examples() -> Vec<Announcement> {
        use Announcement as A;
        let s = |t: &str| t.to_owned();
        vec![
            A::Playing { wpm: 265 },
            A::Paused,
            A::Resumed,
            A::Stopped,
            A::FinishedReading,
            A::Rate { wpm: 285 },
            A::Pitch { semitones: -2 },
            A::Volume { percent: 80 },
            A::SpeedPreset {
                name: s("study"),
                wpm: 200,
            },
            A::Voice { name: s("Reed") },
            A::Opened { title: s("Notes") },
            A::OpenFailed {
                title: s("essay.docx"),
                reason: s("the file is damaged"),
            },
            A::ResumedAt {
                pct: 42,
                saved: Some(s("2026-09-25")),
            },
            A::PositionCleared { had: true },
            A::Heading {
                level: 2,
                text: s("Methods"),
            },
            A::Table { rows: 3, cols: 4 },
            A::List { items: 5 },
            A::Link {
                text: s("home page"),
            },
            A::NoNext { what: s("heading") },
            A::NoPrevious { what: s("table") },
            A::StartOfDocument,
            A::EndOfDocument,
            A::BlankLine,
            A::Position {
                line: 12,
                lines: 300,
                pct: 4,
                heading: Some(s("Introduction")),
            },
            A::History { index: 3, total: 5 },
            A::HistoryPresent,
            A::NoHistory { back: true },
            A::SpeechCursor { on: true },
            A::EditMode { on: true },
            A::Hint {
                text: s("Tab leaves Speech Cursor"),
            },
            A::Match { index: 3, total: 7 },
            A::SearchWrapped { to_top: true },
            A::NoMatches { query: s("cell") },
            A::BookmarkAdded {
                name: s("mark1"),
                pct: 42,
            },
            A::BookmarkReached { name: s("mark1") },
            A::BookmarkDeleted { name: s("mark1") },
            A::NoBookmarks,
            A::NoteAdded { pct: 42 },
            A::HighlightAdded { color: s("yellow") },
            A::NoteReached {
                text: s("Note: check this, on \u{201c}claim\u{201d}"),
            },
            A::NoteDeleted { highlight: false },
            A::NoNotes,
            A::LibraryFolderAdded {
                name: s("Readings"),
                documents: 12,
            },
            A::SyncMerged {
                conflicts: 2,
                manual: false,
            },
            A::NewDocument,
            A::Saved {
                name: s("notes.md"),
            },
            A::SaveFailed {
                reason: s("the disk is full"),
            },
            A::Undo,
            A::Redo,
            A::NothingToUndo { redo: false },
            A::FormatNeedsEditMode,
            A::Replaced { count: 4 },
            A::TableInserted { rows: 2, cols: 3 },
            A::ImageInserted { name: s("pic.png") },
            A::Recovered,
            A::Theme { name: s("galaxy") },
            A::LineNumbers { on: true },
            A::CharacterKeys { on: false },
            A::Error {
                message: s("Speech engine stopped"),
            },
            A::Info {
                message: s("Copied"),
            },
        ]
    }
}

/// The verbosity table in Markdown: one row per announcement kind with the
/// text said at each level ("—" for nothing). The crate docs embed it.
pub fn verbosity_table() -> String {
    let cell = |a: &Announcement, v: Verbosity| a.text(v).unwrap_or_else(|| "—".to_owned());
    let mut out =
        String::from("| Kind | Priority | Low | Normal | High |\n|---|---|---|---|---|\n");
    for a in Announcement::examples() {
        let p = match a.priority() {
            Priority::Polite => "polite",
            Priority::Assertive => "assertive",
        };
        out.push_str(&format!(
            "| `{}` | {p} | {} | {} | {} |\n",
            a.kind(),
            cell(&a, Verbosity::Low),
            cell(&a, Verbosity::Normal),
            cell(&a, Verbosity::High),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn examples_cover_every_kind_once() {
        let ex = Announcement::examples();
        let mut kinds: Vec<&str> = ex.iter().map(Announcement::kind).collect();
        let n = kinds.len();
        kinds.sort_unstable();
        kinds.dedup();
        assert_eq!(kinds.len(), n, "duplicate kinds in examples");
        // Serialization tags match `kind()`.
        for a in &ex {
            let v = serde_json::to_value(a).unwrap();
            assert_eq!(v["kind"], a.kind());
            let back: Announcement = serde_json::from_value(v).unwrap();
            assert_eq!(&back, a);
        }
    }

    #[test]
    fn verbosity_is_monotonic() {
        // Anything said at a level is also said at every higher level.
        for a in Announcement::examples() {
            let low = a.text(Verbosity::Low).is_some();
            let normal = a.text(Verbosity::Normal).is_some();
            let high = a.text(Verbosity::High).is_some();
            assert!(!low || normal, "{}", a.kind());
            assert!(!normal || high, "{}", a.kind());
            assert!(high, "{} is never said", a.kind());
        }
    }

    #[test]
    fn errors_and_command_results_are_said_at_low() {
        for a in [
            Announcement::Error {
                message: "x".into(),
            },
            Announcement::NoNext {
                what: "heading".into(),
            },
            Announcement::NoMatches { query: "q".into() },
            Announcement::FormatNeedsEditMode,
            Announcement::Rate { wpm: 300 },
            Announcement::SpeechCursor { on: false },
            Announcement::EditMode { on: true },
        ] {
            assert_eq!(a.min_verbosity(), Verbosity::Low, "{}", a.kind());
        }
        assert_eq!(
            Announcement::Error {
                message: "x".into()
            }
            .priority(),
            Priority::Assertive
        );
    }

    #[test]
    fn wording_reads_well_aloud() {
        for a in Announcement::examples() {
            for v in [Verbosity::Low, Verbosity::Normal, Verbosity::High] {
                if let Some(t) = a.text(v) {
                    // No symbols a speech engine would skip or spell out.
                    for bad in ['¶', '→', '←', '⏩', '⏪', '✏', '·', '%', '×', '/', '|']
                    {
                        assert!(!t.contains(bad), "{}: {t}", a.kind());
                    }
                    assert!(!t.contains("wpm"), "{}: {t}", a.kind());
                }
            }
        }
    }

    #[test]
    fn notes_library_and_sync_wording() {
        let manual = Announcement::SyncMerged {
            conflicts: 1,
            manual: true,
        };
        assert_eq!(manual.priority(), Priority::Assertive);
        assert_eq!(
            manual.text(Verbosity::Low).as_deref(),
            Some(
                "1 reading position differs from another device. Kept this device's; choose in the library"
            )
        );
        let two = Announcement::SyncMerged {
            conflicts: 2,
            manual: true,
        };
        assert!(
            two.text(Verbosity::Low)
                .unwrap()
                .starts_with("2 reading positions differ")
        );
        assert_eq!(
            Announcement::NoteDeleted { highlight: true }
                .text(Verbosity::Low)
                .as_deref(),
            Some("Highlight removed")
        );
        assert_eq!(
            Announcement::LibraryFolderAdded {
                name: "Readings".into(),
                documents: 1
            }
            .text(Verbosity::Normal)
            .as_deref(),
            Some("Added folder Readings with 1 document")
        );
    }

    #[test]
    fn specific_wording() {
        assert_eq!(
            Announcement::Rate { wpm: 285 }
                .text(Verbosity::Normal)
                .unwrap(),
            "Rate 285 words per minute"
        );
        assert_eq!(
            Announcement::Pitch { semitones: 3 }
                .text(Verbosity::Low)
                .unwrap(),
            "Pitch plus 3"
        );
        assert_eq!(
            Announcement::Replaced { count: 1 }
                .text(Verbosity::Low)
                .unwrap(),
            "Replaced 1 match"
        );
        assert_eq!(
            Announcement::Table { rows: 1, cols: 2 }
                .text(Verbosity::Normal)
                .unwrap(),
            "Table with 1 row and 2 columns"
        );
        assert_eq!(
            Announcement::BlankLine.text(Verbosity::Low).unwrap(),
            "blank"
        );
    }
}
