use std::path::PathBuf;

use textweaver_core::CharRange;
use textweaver_keymap::ActionId;
use textweaver_text::GoTo;

/// Input to the application, from any frontend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// A bound action (from the keymap or the command palette).
    Action(ActionId),
    /// Open a document.
    Open(PathBuf),
    /// Text typed in edit mode or into a prompt.
    Insert(String),
    /// Run a search with this pattern (from the find prompt).
    Find(String),
    /// Jump to a target (from the go-to prompt).
    GoTo(GoTo),
    /// Set the selection (mouse, shift-arrows in the frontend).
    Select(CharRange),
    /// The viewport changed size, in columns and rows.
    Resize {
        /// Columns.
        width: u16,
        /// Rows.
        height: u16,
    },
    /// Cancel the current prompt or mode.
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
