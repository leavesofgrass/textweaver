use std::path::PathBuf;

use textweaver_core::{CharRange, Direction, Unit};
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptPurpose {
    /// Answer becomes `Command::Find`.
    Find,
    /// Answer is parsed into `Command::GoTo`.
    GoTo,
    /// Answer becomes `Command::Open`.
    Open,
    /// Answer runs a command by id.
    CommandPalette,
}

impl PromptPurpose {
    /// The prompt label, which is also what is spoken when it opens.
    pub fn label(self) -> &'static str {
        match self {
            PromptPurpose::Find => "Find",
            PromptPurpose::GoTo => "Go to line, percent, start, or end",
            PromptPurpose::Open => "Open file",
            PromptPurpose::CommandPalette => "Command",
        }
    }
}
