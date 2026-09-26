//! Typing and movement echo: what to speak as the user edits.
//!
//! [`for_edit`] turns one edit into echo events: the typed character, the
//! word a space or punctuation mark completes, and deleted text.
//! [`for_move`] announces the new line when the cursor moves to another
//! line, saying "blank" for an empty one (Star's Speech Cursor wording).
//! Star had no typing echo at all.

use ropey::Rope;
use serde::{Deserialize, Serialize};
use textweaver_core::{CapsIndication, CharPos, Edit};

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

/// What an empty or whitespace-only line is called.
pub const BLANK: &str = "blank";

/// Characters that belong inside a word for echo purposes: letters, digits,
/// underscore, apostrophes (`don't`), and hyphens (`well-known`).
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '\'' | '\u{2019}' | '-')
}

/// The word ending just before `at` in `text`: the run of word characters,
/// trimmed of leading and trailing apostrophes and hyphens. `None` unless
/// it contains a letter or digit.
pub fn word_before(text: &Rope, at: CharPos) -> Option<String> {
    let at = at.0.min(text.len_chars());
    let start = (0..at)
        .rev()
        .take_while(|&i| is_word_char(text.char(i)))
        .last()?;
    let word: String = text.slice(start..at).to_string();
    let word = word.trim_matches(|c| matches!(c, '\'' | '\u{2019}' | '-'));
    word.chars()
        .any(char::is_alphanumeric)
        .then(|| word.to_owned())
}

/// Echo events for `edit`, given the text as it was before the edit.
///
/// - A pure deletion gives `Deleted` with exactly the removed text.
/// - Typing one character gives `Typed`, and when that character ends a
///   word (anything but a word character) also `WordCompleted` with the
///   word before it. Typing over a selection does not speak the selection.
/// - Longer insertions (pastes) give nothing; the app announces them.
pub fn for_edit(policy: &EchoPolicy, before: &Rope, edit: &Edit) -> Vec<EchoEvent> {
    let mut out = Vec::new();
    let len = before.len_chars();
    let r = edit.range.clamp_to(len);
    if edit.text.is_empty() {
        if policy.deletions && !r.is_empty() {
            out.push(EchoEvent::Deleted(before.slice(r.to_range()).to_string()));
        }
        return out;
    }
    let mut chars = edit.text.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        if policy.characters {
            out.push(EchoEvent::Typed(c));
        }
        if policy.words && !is_word_char(c) {
            if let Some(w) = word_before(before, r.start) {
                out.push(EchoEvent::WordCompleted(w));
            }
        }
    }
    out
}

/// The text of `line` as spoken on a cursor move: without its line ending,
/// or "blank" when empty or only whitespace.
pub fn line_for_echo(text: &Rope, line: usize) -> String {
    if line >= text.len_lines() {
        return BLANK.to_owned();
    }
    let s = text.line(line).to_string();
    let s = s.trim_end_matches(['\n', '\r']);
    if s.trim().is_empty() {
        BLANK.to_owned()
    } else {
        s.to_owned()
    }
}

/// Echo events for a cursor move from `from` to `to`: the new line when the
/// line changed and `lines_on_move` is set.
pub fn for_move(policy: &EchoPolicy, text: &Rope, from: CharPos, to: CharPos) -> Vec<EchoEvent> {
    let len = text.len_chars();
    let (a, b) = (
        text.char_to_line(from.0.min(len)),
        text.char_to_line(to.0.min(len)),
    );
    if policy.lines_on_move && a != b {
        vec![EchoEvent::CursorMoved(line_for_echo(text, b))]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rope(s: &str) -> Rope {
        Rope::from_str(s)
    }

    #[test]
    fn typing_and_deleting() {
        let p = EchoPolicy::default();
        let text = rope("abc");
        assert_eq!(
            for_edit(&p, &text, &Edit::insert(3, "d")),
            vec![EchoEvent::Typed('d')]
        );
        assert_eq!(
            for_edit(&p, &text, &Edit::delete(1..2)),
            vec![EchoEvent::Deleted("b".into())]
        );
    }

    #[test]
    fn words_complete_on_space_and_punctuation() {
        let p = EchoPolicy::default();
        let text = rope("hello");
        assert_eq!(
            for_edit(&p, &text, &Edit::insert(5, " ")),
            vec![
                EchoEvent::Typed(' '),
                EchoEvent::WordCompleted("hello".into())
            ]
        );
        assert_eq!(
            for_edit(&p, &rope("don't"), &Edit::insert(5, ".")),
            vec![
                EchoEvent::Typed('.'),
                EchoEvent::WordCompleted("don't".into())
            ]
        );
        // An apostrophe or hyphen does not end a word.
        assert_eq!(
            for_edit(&p, &rope("don"), &Edit::insert(3, "'")),
            vec![EchoEvent::Typed('\'')]
        );
        // Nothing before the space, or only punctuation.
        assert_eq!(
            for_edit(&p, &rope("hi, "), &Edit::insert(4, " ")),
            vec![EchoEvent::Typed(' ')]
        );
        assert_eq!(
            for_edit(&p, &rope("-- "), &Edit::insert(2, " ")),
            vec![EchoEvent::Typed(' ')]
        );
        // Newline completes too; the word is before the insertion point.
        assert_eq!(
            for_edit(&p, &rope("one two"), &Edit::insert(3, "\n")),
            vec![
                EchoEvent::Typed('\n'),
                EchoEvent::WordCompleted("one".into())
            ]
        );
    }

    #[test]
    fn policy_switches() {
        let quiet = EchoPolicy {
            characters: false,
            words: false,
            deletions: false,
            lines_on_move: false,
            ..EchoPolicy::default()
        };
        let text = rope("word");
        assert!(for_edit(&quiet, &text, &Edit::insert(4, " ")).is_empty());
        assert!(for_edit(&quiet, &text, &Edit::delete(0..1)).is_empty());
        assert!(for_move(&quiet, &rope("a\nb"), CharPos(0), CharPos(2)).is_empty());
        let words_only = EchoPolicy {
            characters: false,
            ..EchoPolicy::default()
        };
        assert_eq!(
            for_edit(&words_only, &text, &Edit::insert(4, " ")),
            vec![EchoEvent::WordCompleted("word".into())]
        );
    }

    #[test]
    fn typing_over_a_selection_and_pasting() {
        let p = EchoPolicy::default();
        assert_eq!(
            for_edit(&p, &rope("abc"), &Edit::replace(0..3, "x")),
            vec![EchoEvent::Typed('x')]
        );
        assert!(for_edit(&p, &rope("abc"), &Edit::insert(3, "pasted text")).is_empty());
    }

    #[test]
    fn moves_between_lines_say_the_line_or_blank() {
        let p = EchoPolicy::default();
        let text = rope("first\n\n  \nlast");
        assert_eq!(
            for_move(&p, &text, CharPos(0), CharPos(6)),
            vec![EchoEvent::CursorMoved("blank".into())]
        );
        assert_eq!(
            for_move(&p, &text, CharPos(6), CharPos(8)),
            vec![EchoEvent::CursorMoved("blank".into())]
        );
        assert_eq!(
            for_move(&p, &text, CharPos(8), CharPos(12)),
            vec![EchoEvent::CursorMoved("last".into())]
        );
        assert!(for_move(&p, &text, CharPos(0), CharPos(3)).is_empty());
        assert_eq!(
            for_move(&p, &text, CharPos(12), CharPos(0)),
            vec![EchoEvent::CursorMoved("first".into())]
        );
        assert_eq!(line_for_echo(&text, 99), "blank");
    }
}

#[cfg(test)]
mod props {
    use proptest::prelude::*;
    use textweaver_core::CharRange;

    use super::*;

    #[derive(Clone, Debug)]
    enum Op {
        Insert(usize, String),
        Delete(usize, usize),
    }

    fn op() -> impl Strategy<Value = Op> {
        prop_oneof![
            (0usize..40, "[a-zA-Z0-9 .,'\\-\n]{1,3}").prop_map(|(a, s)| Op::Insert(a, s)),
            (0usize..40, 0usize..40).prop_map(|(a, b)| Op::Delete(a, b)),
        ]
    }

    proptest! {
        /// Over random insert and delete sequences: deletions echo exactly
        /// the removed text; only single-char insertions echo, once; a
        /// completed word is the word right before the insertion point and
        /// contains a letter or digit; nothing panics.
        #[test]
        fn echo_events_describe_the_edit(
            start in "[a-z .\n']{0,20}",
            ops in proptest::collection::vec(op(), 1..30),
        ) {
            let p = EchoPolicy::default();
            let mut text = Rope::from_str(&start);
            for op in ops {
                let len = text.len_chars();
                let edit = match op {
                    Op::Insert(a, s) => Edit::insert(a.min(len), s),
                    Op::Delete(a, b) => Edit::delete(CharRange::new(a.min(len), b.min(len))),
                };
                let events = for_edit(&p, &text, &edit);
                let typed = events.iter().filter(|e| matches!(e, EchoEvent::Typed(_))).count();
                let single = edit.text.chars().count() == 1;
                prop_assert_eq!(typed, usize::from(single));
                for e in &events {
                    match e {
                        EchoEvent::Deleted(d) => {
                            prop_assert!(edit.text.is_empty());
                            prop_assert_eq!(d, &text.slice(edit.range.to_range()).to_string());
                        }
                        EchoEvent::Typed(c) => {
                            prop_assert_eq!(Some(*c), edit.text.chars().next());
                        }
                        EchoEvent::WordCompleted(w) => {
                            prop_assert!(single);
                            prop_assert!(w.chars().any(char::is_alphanumeric));
                            let before = text.slice(..edit.range.start.0).to_string();
                            prop_assert!(before.trim_end_matches(['\'', '-']).ends_with(w.as_str()));
                        }
                        EchoEvent::CursorMoved(_) => prop_assert!(false, "no moves from edits"),
                    }
                }
                if edit.text.is_empty() && edit.range.is_empty() {
                    prop_assert!(events.is_empty());
                }
                edit.apply_to_rope(&mut text).unwrap();
                // Moving anywhere never panics and says at most one line.
                let n = text.len_chars();
                let moved = for_move(&p, &text, CharPos(0), CharPos(n));
                prop_assert!(moved.len() <= 1);
                if let Some(EchoEvent::CursorMoved(l)) = moved.first() {
                    prop_assert!(!l.is_empty() && !l.contains('\n'));
                }
            }
        }
    }
}
