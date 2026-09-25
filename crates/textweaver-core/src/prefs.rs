//! Small preference enums shared by settings (`store`), speech, and
//! accessibility. They live in core so `store` does not depend on `speech` or
//! `a11y`.

use serde::{Deserialize, Serialize};

/// How much punctuation the speech layer pronounces.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PunctuationLevel {
    /// Speak no punctuation; it only shapes prosody.
    None,
    /// Speak punctuation that carries meaning in prose (for example `@`, `#`, `/`).
    #[default]
    Some,
    /// Speak every punctuation character.
    All,
}

/// Which unit the reading highlight follows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HighlightGranularity {
    /// Highlight the word being spoken.
    #[default]
    Word,
    /// Highlight the sentence being spoken.
    Sentence,
    /// Highlight the sentence and, within it, the word.
    Both,
}

/// How much the application says about state changes and structure.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Verbosity {
    /// Only essential announcements.
    Low,
    /// Announcements with structure names ("heading level 2").
    #[default]
    Normal,
    /// Everything, including positions and counts.
    High,
}

/// How capital letters are indicated when speaking characters and echoing typing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapsIndication {
    /// No indication.
    None,
    /// A short tone before the character.
    Tone,
    /// Raise the pitch for the character.
    #[default]
    Pitch,
    /// Say "cap" before the character.
    SayCap,
}
