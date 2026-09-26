//! State for the minibuffer (prompts) and the list overlay (help,
//! bookmarks). Drawing lives in `ui`.

use textweaver_app::PromptPurpose;
use textweaver_app::keymap::ActionId;

/// A one-line prompt with a caret, history, and (for the command palette)
/// completion candidates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Minibuffer {
    /// Prompt label.
    pub label: String,
    /// What the answer is for.
    pub purpose: PromptPurpose,
    text: Vec<char>,
    caret: usize,
    /// Position in the prompt's history while recalling (`None` when
    /// editing fresh text).
    pub history_index: Option<usize>,
    /// Palette candidates for the current text.
    pub candidates: Vec<(ActionId, String)>,
    /// Which candidate Up and Down are on.
    pub candidate: Option<usize>,
}

impl Minibuffer {
    /// An empty prompt.
    pub fn new(label: impl Into<String>, purpose: PromptPurpose) -> Self {
        Minibuffer {
            label: label.into(),
            purpose,
            text: Vec::new(),
            caret: 0,
            history_index: None,
            candidates: Vec::new(),
            candidate: None,
        }
    }

    /// The text typed so far.
    pub fn text(&self) -> String {
        self.text.iter().collect()
    }

    /// The caret, in chars from the start of the text.
    pub fn caret(&self) -> usize {
        self.caret
    }

    /// Replaces the text and puts the caret at its end.
    pub fn set_text(&mut self, s: &str) {
        self.text = s.chars().collect();
        self.caret = self.text.len();
    }

    /// Inserts a char at the caret.
    pub fn insert(&mut self, c: char) {
        self.text.insert(self.caret, c);
        self.caret += 1;
    }

    /// Deletes the char before the caret.
    pub fn backspace(&mut self) -> Option<char> {
        if self.caret == 0 {
            return None;
        }
        self.caret -= 1;
        Some(self.text.remove(self.caret))
    }

    /// Deletes the char at the caret.
    pub fn delete(&mut self) -> Option<char> {
        (self.caret < self.text.len()).then(|| self.text.remove(self.caret))
    }

    /// Moves the caret one char left; false at the start.
    pub fn left(&mut self) -> bool {
        let moved = self.caret > 0;
        self.caret = self.caret.saturating_sub(1);
        moved
    }

    /// Moves the caret one char right; false at the end.
    pub fn right(&mut self) -> bool {
        let moved = self.caret < self.text.len();
        self.caret = (self.caret + 1).min(self.text.len());
        moved
    }

    /// Caret to the start.
    pub fn home(&mut self) {
        self.caret = 0;
    }

    /// Caret to the end.
    pub fn end(&mut self) {
        self.caret = self.text.len();
    }

    /// Deletes from the start to the caret; returns what was deleted.
    pub fn kill_to_start(&mut self) -> String {
        let gone: String = self.text.drain(..self.caret).collect();
        self.caret = 0;
        gone
    }

    /// Deletes from the caret to the end; returns what was deleted.
    pub fn kill_to_end(&mut self) -> String {
        self.text.drain(self.caret..).collect()
    }

    /// Deletes the word before the caret; returns it.
    pub fn delete_word_back(&mut self) -> String {
        let mut start = self.caret;
        while start > 0 && self.text[start - 1].is_whitespace() {
            start -= 1;
        }
        while start > 0 && !self.text[start - 1].is_whitespace() {
            start -= 1;
        }
        let gone: String = self.text.drain(start..self.caret).collect();
        self.caret = start;
        gone
    }

    /// The char at the caret, if any.
    pub fn char_at_caret(&self) -> Option<char> {
        self.text.get(self.caret).copied()
    }
}

/// A list shown over the document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListView {
    /// Title.
    pub title: String,
    /// Items.
    pub items: Vec<String>,
    /// Focused item.
    pub selected: usize,
}

impl ListView {
    /// A list focused on its first item.
    pub fn new(title: impl Into<String>, items: Vec<String>) -> Self {
        ListView {
            title: title.into(),
            items,
            selected: 0,
        }
    }

    /// Moves the focus by `delta`, clamped; returns true when it moved.
    pub fn step(&mut self, delta: isize) -> bool {
        let last = self.items.len().saturating_sub(1);
        let next = self.selected.saturating_add_signed(delta).min(last);
        let moved = next != self.selected;
        self.selected = next;
        moved
    }

    /// The focused item's text.
    pub fn current(&self) -> Option<&str> {
        self.items.get(self.selected).map(String::as_str)
    }

    /// The focused item as spoken: its text and where it is in the list
    /// ("Chapter two, 2 of 5").
    pub fn spoken_item(&self) -> Option<String> {
        let item = self.current()?;
        Some(format!(
            "{item}, {} of {}",
            self.selected + 1,
            self.items.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minibuffer_editing() {
        let mut m = Minibuffer::new("Find", PromptPurpose::Find);
        for c in "hello world".chars() {
            m.insert(c);
        }
        assert_eq!(m.delete_word_back(), "world");
        assert_eq!(m.text(), "hello ");
        m.home();
        m.right();
        assert_eq!(m.kill_to_end(), "ello ");
        assert_eq!(m.backspace(), Some('h'));
        assert_eq!(m.backspace(), None);
        m.set_text("abc");
        m.left();
        assert_eq!(m.kill_to_start(), "ab");
        assert_eq!(m.text(), "c");
    }

    #[test]
    fn list_steps_clamp() {
        let mut l = ListView::new("t", vec!["a".into(), "b".into()]);
        assert!(!l.step(-1));
        assert!(l.step(5));
        assert_eq!(l.current(), Some("b"));
        assert_eq!(l.spoken_item().as_deref(), Some("b, 2 of 2"));
        assert_eq!(ListView::new("t", Vec::new()).spoken_item(), None);
    }
}
