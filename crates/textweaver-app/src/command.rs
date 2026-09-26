use std::path::PathBuf;

use textweaver_core::{CharPos, CharRange, Direction, Unit};
use textweaver_keymap::ActionId;
use textweaver_text::GoTo;

/// Input to the application, from any frontend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// A bound action (from the keymap or the command palette).
    Action(ActionId),
    /// Open a document.
    Open(PathBuf),
    /// Text typed in edit mode (wave 2). Prompts send their whole answer
    /// with [`Command::Answer`] instead.
    Insert(String),
    /// Run a search with this pattern (from the find prompt).
    Find(String),
    /// Jump to a target (from the go-to prompt).
    GoTo(GoTo),
    /// Set the selection (mouse, shift-arrows in the frontend).
    Select(CharRange),
    /// Move the cursor quietly (a GUI caret click): no history entry, no
    /// announcement, no reading; while paused, reading resumes from there.
    /// See [`App::set_cursor`](crate::App::set_cursor).
    SetCursor(CharPos),
    /// Grow or shrink the selection by one unit from its moving end
    /// (Shift+arrows). Only `Grapheme`, `Word`, and `Line` are meaningful.
    ExtendSelection(Unit, Direction),
    /// The answer to the open prompt, interpreted according to its
    /// [`PromptPurpose`]: a search pattern, a go-to target, a path, or a
    /// command name.
    Answer(String),
    /// The user chose item `n` (0-based) of the list shown by the last
    /// [`Effect::ShowList`].
    Choose(usize),
    /// The viewport changed size, in columns and rows of document text.
    Resize {
        /// Columns.
        width: u16,
        /// Rows.
        height: u16,
    },
    /// Cancel the current prompt, list, or mode.
    Cancel,
    /// Edit mode: delete the selection or the character before the caret
    /// (Backspace).
    DeleteBack,
    /// Edit mode: delete the selection or the character after the caret
    /// (Delete).
    DeleteForward,
    /// Edit mode: move the caret, extending the selection when `extend` is
    /// set (Shift). The new character, word, or line is echoed.
    MoveCaret {
        /// How far.
        by: CaretMove,
        /// Which way.
        direction: Direction,
        /// Extend the selection instead of collapsing it.
        extend: bool,
    },
    /// Notes, highlights, and bookmark management (until the keymap has
    /// actions for them; see [`NoteCommand`]).
    Notes(NoteCommand),
    /// The answer to a pending confirmation ([`App::pending_confirmation`]):
    /// the frontend sends it for `y` ([`Confirm::Yes`]), for `n`, `a`, or
    /// Escape ([`Confirm::No`]), and for any other key ([`Confirm::Repeat`]).
    ///
    /// [`App::pending_confirmation`]: crate::App::pending_confirmation
    Confirm(Confirm),
    /// Delete item `n` (0-based) of the list shown by the last
    /// [`Effect::ShowList`] (bookmarks, notes, highlights). Other lists
    /// ignore it and say so.
    DeleteItem(usize),
    /// Rename or edit item `n` of the shown list: opens a prompt for the new
    /// bookmark name or note text.
    RenameItem(usize),
    /// Mark or unmark item `n` of the shown list (Space): in the voice
    /// list, a favourite voice. Other lists ignore it and say so.
    MarkItem(usize),
    /// Periodic housekeeping from the frontend's event loop: autosave
    /// snapshots while editing and periodic position saves. The same as
    /// [`App::tick`](crate::App::tick) with the current time.
    Tick,
}

/// How far a [`Command::MoveCaret`] moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CaretMove {
    /// One character.
    Char,
    /// One word.
    Word,
    /// One line, keeping the column.
    Line,
    /// To the start or end of the line.
    LineEdge,
    /// One screen of lines.
    Page,
    /// To the start or end of the document.
    DocumentEdge,
}

/// Notes, highlights, and bookmark management commands.
///
/// The keymap (Agent C) has no actions for these yet; frontends bind them
/// through [`extra_bindings`](crate::extra_bindings), and the command
/// palette accepts their [`name`](NoteCommand::name)s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NoteCommand {
    /// Add a note to the selection, or to the sentence at the cursor
    /// (prompts for the text).
    Add,
    /// List the notes; Enter jumps, Delete deletes, F2 edits.
    List,
    /// Move to the next note.
    Next,
    /// Move to the previous note.
    Previous,
    /// Highlight the selection, or the sentence at the cursor; on an
    /// existing highlight, remove it.
    ToggleHighlight,
    /// List the highlights; Enter jumps, Delete removes.
    ListHighlights,
    /// Rename a bookmark: the bookmark at the cursor, else one chosen from
    /// the list.
    RenameBookmark,
    /// Delete the bookmark at the cursor, else one chosen from the list.
    DeleteBookmark,
}

impl NoteCommand {
    /// Every command, in help order.
    pub const ALL: [NoteCommand; 8] = [
        NoteCommand::Add,
        NoteCommand::List,
        NoteCommand::Next,
        NoteCommand::Previous,
        NoteCommand::ToggleHighlight,
        NoteCommand::ListHighlights,
        NoteCommand::RenameBookmark,
        NoteCommand::DeleteBookmark,
    ];

    /// The command palette name (snake_case, like action ids).
    pub fn name(self) -> &'static str {
        match self {
            NoteCommand::Add => "add_note",
            NoteCommand::List => "list_notes",
            NoteCommand::Next => "next_note",
            NoteCommand::Previous => "previous_note",
            NoteCommand::ToggleHighlight => "toggle_highlight",
            NoteCommand::ListHighlights => "list_highlights",
            NoteCommand::RenameBookmark => "rename_bookmark",
            NoteCommand::DeleteBookmark => "delete_bookmark",
        }
    }

    /// One-line help.
    pub fn help(self) -> &'static str {
        match self {
            NoteCommand::Add => "Add a note to the selection or the current sentence",
            NoteCommand::List => "List notes",
            NoteCommand::Next => "Move to the next note",
            NoteCommand::Previous => "Move to the previous note",
            NoteCommand::ToggleHighlight => {
                "Highlight the selection or the current sentence, or remove a highlight"
            }
            NoteCommand::ListHighlights => "List highlights",
            NoteCommand::RenameBookmark => "Rename a bookmark",
            NoteCommand::DeleteBookmark => "Delete a bookmark",
        }
    }

    /// The command for a palette name (`add_note` or `add note`).
    pub fn from_name(name: &str) -> Option<Self> {
        let n = name.trim().to_lowercase().replace([' ', '-'], "_");
        NoteCommand::ALL.into_iter().find(|c| c.name() == n)
    }
}

/// What the frontend must do after a dispatch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    /// State changed; redraw.
    Redraw,
    /// Ask the user for text (find, go to, open); the answer comes back as a
    /// `Command`.
    Prompt {
        /// Prompt label, also spoken.
        label: String,
        /// What the answer is for.
        purpose: PromptPurpose,
    },
    /// Show a list (bookmarks, help); the frontend renders it accessibly.
    ShowList {
        /// List title.
        title: String,
        /// Items.
        items: Vec<String>,
    },
    /// Exit the application.
    Quit,
}

/// Why a prompt was opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PromptPurpose {
    /// Answer becomes `Command::Find`.
    Find,
    /// Answer is parsed into `Command::GoTo`.
    GoTo,
    /// Answer becomes `Command::Open`.
    Open,
    /// Answer runs a command by id.
    CommandPalette,
    /// Answer is the file to save to; empty accepts the suggestion in the
    /// label.
    SaveAs,
    /// Answer is a table size such as `3 by 2` (columns by rows); empty is
    /// 2 by 2.
    TableSize,
    /// Answer is the path of an image to insert.
    ImagePath,
    /// Answer is the text to find (the replacement is asked next).
    ReplaceFind,
    /// Answer replaces every match.
    ReplaceWith,
    /// Answer is the text of a new note.
    NoteText,
    /// Answer is the new text of a note; empty keeps it.
    EditNote,
    /// Answer is the new name of a bookmark; empty keeps it.
    RenameBookmark,
    /// Answer is the file to export settings to (`.toml` for TOML).
    ExportSettings,
    /// Answer is the settings file to import (JSON or TOML).
    ImportSettings,
}

impl PromptPurpose {
    /// The prompt label, which is also what is spoken when it opens.
    pub fn label(self) -> &'static str {
        match self {
            PromptPurpose::Find => "Find",
            PromptPurpose::GoTo => "Go to line, percent, start, or end",
            PromptPurpose::Open => "Open file",
            PromptPurpose::CommandPalette => "Command",
            PromptPurpose::SaveAs => "Save as",
            PromptPurpose::TableSize => "Table size, columns by rows, for example 3 by 2",
            PromptPurpose::ImagePath => "Image file",
            PromptPurpose::ReplaceFind => "Replace, find what",
            PromptPurpose::ReplaceWith => "Replace with",
            PromptPurpose::NoteText => "Note",
            PromptPurpose::EditNote => "Edit note, Enter keeps it",
            PromptPurpose::RenameBookmark => "New bookmark name, Enter keeps it",
            PromptPurpose::ExportSettings => {
                "Export settings to file, for example textweaver-settings.json"
            }
            PromptPurpose::ImportSettings => "Import settings from file",
        }
    }
}

/// An answer to a confirmation question such as "Quit textweaver? y or n".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confirm {
    /// Go ahead.
    Yes,
    /// Abort; nothing happens.
    No,
    /// Say the question again (any key other than yes or no).
    Repeat,
}

impl Confirm {
    /// The answer a typed character gives: `y` is yes; `n` and `a` (abort)
    /// are no; anything else repeats the question.
    pub fn from_char(c: char) -> Confirm {
        match c.to_ascii_lowercase() {
            'y' => Confirm::Yes,
            'n' | 'a' => Confirm::No,
            _ => Confirm::Repeat,
        }
    }
}
