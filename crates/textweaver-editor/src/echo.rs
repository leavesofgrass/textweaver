//! Typing and movement echo: what to speak as the user edits.

use ropey::Rope;
use serde::{Deserialize, Serialize};
use textweaver_core::{CapsIndication, Edit};

/// Which echo events are produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EchoPolicy {
    /// Speak each typed character.
    pub characters: bool,
    /// Speak a word when it is completed by a space or punctuation.
    pub words: bool,
    /// Speak the new line when the cursor moves to another line.
    pub lines_on_move: bool,
    /// Speak deleted text.
    pub deletions: bool,
    /// How capitals are indicated.
    pub caps: CapsIndication,
}

impl Default for EchoPolicy {
    fn default() -> Self {
        EchoPolicy {
            characters: true,
            words: true,
            lines_on_move: true,
            deletions: true,
            caps: CapsIndication::default(),
        }
    }
}

/// Something worth speaking after an edit or a move.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EchoEvent {
    /// A character was typed.
    Typed(char),
    /// A word was completed.
    WordCompleted(String),
    /// Text was deleted.
    Deleted(String),
    /// The cursor moved to a line with this text (empty lines say "blank").
    CursorMoved(String),
}

/// Echo events for `edit`, given the text as it was before the edit.
///
/// Phase 0: typed characters and deletions only. Agent C adds word
/// completion and proptests over insert/delete sequences.
pub fn for_edit(policy: &EchoPolicy, before: &Rope, edit: &Edit) -> Vec<EchoEvent> {
    let mut out = Vec::new();
    if !edit.range.is_empty() && policy.deletions {
        let len = before.len_chars();
        let r = edit.range.clamp_to(len);
        out.push(EchoEvent::Deleted(before.slice(r.to_range()).to_string()));
    }
    let mut chars = edit.text.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        if policy.characters {
            out.push(EchoEvent::Typed(c));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_and_deleting() {
        let p = EchoPolicy::default();
        let text = Rope::from_str("abc");
        assert_eq!(
            for_edit(&p, &text, &Edit::insert(3, "d")),
            vec![EchoEvent::Typed('d')]
        );
        assert_eq!(
            for_edit(&p, &text, &Edit::delete(1..2)),
            vec![EchoEvent::Deleted("b".into())]
        );
    }
}
