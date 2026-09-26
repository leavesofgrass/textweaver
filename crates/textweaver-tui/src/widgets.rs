//! The prompt (minibuffer) and list state the terminal reader draws.
//!
//! Since Wave 3 they are the app's own models, shared with the GUI's
//! dialogs and JSON-RPC ([`textweaver_app::list_model`]): the focused item,
//! first-letter jumps, the "k of n" announcements, and the prompt's text,
//! caret, and history live in the app. These names keep the terminal
//! reader's API; drawing lives in `ui`.

pub use textweaver_app::{ListModel as ListView, PromptModel as Minibuffer};
