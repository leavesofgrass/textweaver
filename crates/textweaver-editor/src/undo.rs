use ropey::Rope;
use serde::{Deserialize, Serialize};
use textweaver_core::{Bias, CharPos, CharRange, CoreError, Edit, EditOutcome};

use crate::markdown::Formatted;

/// A selection: `anchor` stays put, `head` moves. Empty when they are equal
/// (a caret).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    /// Where the selection started.
    pub anchor: CharPos,
    /// Where the caret is.
    pub head: CharPos,
}

impl Selection {
    /// A caret at `pos`.
    pub fn caret(pos: CharPos) -> Self {
        Selection {
            anchor: pos,
            head: pos,
        }
    }

    /// A selection from `anchor` to `head`.
    pub fn new(anchor: impl Into<CharPos>, head: impl Into<CharPos>) -> Self {
        Selection {
            anchor: anchor.into(),
            head: head.into(),
        }
    }

    /// The selected range.
    pub fn range(&self) -> CharRange {
        CharRange::new(self.anchor, self.head)
    }

    /// True when nothing is selected.
    pub fn is_caret(&self) -> bool {
        self.anchor == self.head
    }

    fn map(&self, o: &EditOutcome) -> Selection {
        Selection {
            anchor: o.map_pos(self.anchor, Bias::After),
            head: o.map_pos(self.head, Bias::After),
        }
    }
}

/// What kind of change an undo step holds; typing and deleting coalesce.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Kind {
    #[default]
    Other,
    Typing,
    DeletingBack,
    DeletingForward,
}

/// Character classes for word-sized undo steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    Word,
    Space,
    Newline,
    Other,
}

fn class(c: char) -> Class {
    if c == '\n' {
        Class::Newline
    } else if c.is_alphanumeric() || c == '_' || c == '\'' || c == '\u{2019}' {
        Class::Word
    } else if c.is_whitespace() {
        Class::Space
    } else {
        Class::Other
    }
}

/// One undo step: edits applied in order, with their inverses.
#[derive(Clone, Debug, Default)]
struct Group {
    edits: Vec<(Edit, Edit)>,
    before: Selection,
    after: Selection,
    kind: Kind,
    /// Still accepting coalesced keystrokes.
    open: bool,
    /// Class of the last char typed or deleted, for word boundaries.
    last_class: Option<Class>,
    id: u64,
}

/// Text being edited, with undo and redo.
///
/// Undo steps: every [`apply`](Self::apply), [`apply_group`](Self::apply_group),
/// and [`apply_formatted`](Self::apply_formatted) call is one step (so each
/// Markdown command is one step). Keystrokes from
/// [`type_char`](Self::type_char), [`backspace`](Self::backspace), and
/// [`delete_forward`](Self::delete_forward) coalesce into word-sized steps:
/// typing `hello world` gives two steps, `hello ` and `world`; a new line,
/// moving the caret, any other edit, or saving starts a new step.
#[derive(Clone, Debug, Default)]
pub struct Editor {
    text: Rope,
    selection: Selection,
    undo: Vec<Group>,
    redo: Vec<Group>,
    /// Id of the step on top of the undo stack when last saved (0: none).
    saved_id: u64,
    /// Forces dirty until the next save (recovered text).
    modified: bool,
    next_id: u64,
}

impl Editor {
    /// An editor over `text`, caret at the start, clean.
    pub fn new(text: &str) -> Self {
        Editor {
            text: Rope::from_str(text),
            next_id: 1,
            ..Editor::default()
        }
    }

    /// Replaces the whole text and clears undo history, as when a new
    /// editing session starts (undo cannot cross back into a previous
    /// session). The caret goes to the start and the editor is clean.
    pub fn set_text(&mut self, text: &str) {
        *self = Editor::new(text);
    }

    /// The text.
    pub fn text(&self) -> &Rope {
        &self.text
    }

    /// The selection.
    pub fn selection(&self) -> Selection {
        self.selection
    }

    /// Sets the selection (clamped to the text). Ends the current typing
    /// step.
    pub fn set_selection(&mut self, sel: Selection) {
        let len = self.text.len_chars();
        let sel = Selection {
            anchor: sel.anchor.clamp_to(len),
            head: sel.head.clamp_to(len),
        };
        if sel != self.selection {
            self.break_undo_group();
        }
        self.selection = sel;
    }

    /// Ends the current typing or deleting step, so the next keystroke
    /// starts a new undo step.
    pub fn break_undo_group(&mut self) {
        if let Some(g) = self.undo.last_mut() {
            g.open = false;
        }
    }

    fn top_id(&self) -> u64 {
        self.undo.last().map_or(0, |g| g.id)
    }

    /// True when there are unsaved changes: the text differs from the last
    /// save by at least one step (undoing back to the saved state makes the
    /// editor clean again).
    pub fn is_dirty(&self) -> bool {
        self.modified || self.top_id() != self.saved_id
    }

    /// Marks the text as saved.
    pub fn mark_saved(&mut self) {
        self.break_undo_group();
        self.saved_id = self.top_id();
        self.modified = false;
    }

    /// Marks the text as unsaved without an edit (for example text
    /// recovered from an autosave snapshot).
    pub fn mark_modified(&mut self) {
        self.modified = true;
    }

    /// True when there is a step to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// True when there is a step to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Number of undo steps.
    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    fn push_group(&mut self, mut group: Group) {
        if let Some(g) = self.undo.last_mut() {
            g.open = false;
        }
        group.id = self.next_id.max(1);
        self.next_id = group.id + 1;
        self.undo.push(group);
        self.redo.clear();
    }

    /// Applies one edit as its own undo step. The selection follows the edit.
    pub fn apply(&mut self, edit: Edit) -> Result<EditOutcome, CoreError> {
        let mut out = self.apply_group(vec![edit])?;
        Ok(out.pop().unwrap_or_default())
    }

    /// Applies several edits as one undo step. Each edit's range is in the
    /// coordinates left by the previous edits. On an error, edits already
    /// applied in this call are rolled back.
    pub fn apply_group(&mut self, edits: Vec<Edit>) -> Result<Vec<EditOutcome>, CoreError> {
        let before = self.selection;
        let mut group = Group {
            before,
            ..Group::default()
        };
        let mut outcomes = Vec::new();
        for e in edits {
            match e.apply_to_rope(&mut self.text) {
                Ok((o, inv)) => {
                    self.selection = self.selection.map(&o);
                    group.edits.push((e, inv));
                    outcomes.push(o);
                }
                Err(err) => {
                    for (_, inv) in group.edits.iter().rev() {
                        let _ = inv.apply_to_rope(&mut self.text);
                    }
                    self.selection = before;
                    return Err(err);
                }
            }
        }
        group.after = self.selection;
        if !group.edits.is_empty() {
            self.push_group(group);
        }
        Ok(outcomes)
    }

    /// Applies a formatting command's edits as one undo step and sets the
    /// selection it asks for.
    pub fn apply_formatted(&mut self, f: &Formatted) -> Result<Vec<EditOutcome>, CoreError> {
        let outs = self.apply_group(f.edits.clone())?;
        let len = self.text.len_chars();
        self.selection = Selection {
            anchor: f.selection.anchor.clamp_to(len),
            head: f.selection.head.clamp_to(len),
        };
        if let Some(g) = self.undo.last_mut() {
            if !outs.is_empty() {
                g.after = self.selection;
            }
        }
        Ok(outs)
    }

    /// Replaces the selection with `text` as one undo step (a paste).
    pub fn insert_text(&mut self, text: &str) -> Result<EditOutcome, CoreError> {
        let r = self.selection.range();
        let out = self.apply(Edit::replace(r, text))?;
        self.selection = Selection::caret(out.inserted.end);
        if let Some(g) = self.undo.last_mut() {
            g.after = self.selection;
        }
        Ok(out)
    }

    /// Types one character, replacing the selection. Consecutive typing
    /// coalesces into word-sized undo steps.
    pub fn type_char(&mut self, c: char) -> Result<EditOutcome, CoreError> {
        let r = self.selection.range();
        let cls = class(c);
        let joins = r.is_empty()
            && self.undo.last().is_some_and(|g| {
                g.open
                    && g.kind == Kind::Typing
                    && g.after == self.selection
                    && cls != Class::Newline
                    && !(cls == Class::Word && g.last_class != Some(Class::Word))
            });
        let edit = Edit::replace(r, c.to_string());
        let (out, inv) = edit.apply_to_rope(&mut self.text)?;
        let before = self.selection;
        self.selection = Selection::caret(out.inserted.end);
        if joins {
            if let Some(g) = self.undo.last_mut() {
                g.edits.push((edit, inv));
                g.after = self.selection;
                g.last_class = Some(cls);
            }
            self.redo.clear();
        } else {
            self.push_group(Group {
                edits: vec![(edit, inv)],
                before,
                after: self.selection,
                kind: Kind::Typing,
                open: cls != Class::Newline,
                last_class: Some(cls),
                id: 0,
            });
        }
        Ok(out)
    }

    /// Types each character of `s` in turn (keystrokes, not a paste).
    pub fn type_text(&mut self, s: &str) -> Result<(), CoreError> {
        for c in s.chars() {
            self.type_char(c)?;
        }
        Ok(())
    }

    fn delete_one(&mut self, forward: bool) -> Result<Option<EditOutcome>, CoreError> {
        let r = self.selection.range();
        if !r.is_empty() {
            return self.apply(Edit::delete(r)).map(Some);
        }
        let caret = self.selection.head;
        let len = self.text.len_chars();
        let range = if forward {
            if caret.0 >= len {
                return Ok(None);
            }
            CharRange::new(caret.0, caret.0 + 1)
        } else {
            if caret.0 == 0 {
                return Ok(None);
            }
            CharRange::new(caret.0 - 1, caret.0)
        };
        let deleted = self.text.char(range.start.0);
        let cls = class(deleted);
        let kind = if forward {
            Kind::DeletingForward
        } else {
            Kind::DeletingBack
        };
        let joins = self.undo.last().is_some_and(|g| {
            g.open
                && g.kind == kind
                && g.after == self.selection
                && cls != Class::Newline
                && !(cls == Class::Word && g.last_class.is_some_and(|c| c != Class::Word))
        });
        let edit = Edit::delete(range);
        let (out, inv) = edit.apply_to_rope(&mut self.text)?;
        let before = self.selection;
        self.selection = Selection::caret(range.start);
        if joins {
            if let Some(g) = self.undo.last_mut() {
                g.edits.push((edit, inv));
                g.after = self.selection;
                g.last_class = Some(cls);
            }
            self.redo.clear();
        } else {
            self.push_group(Group {
                edits: vec![(edit, inv)],
                before,
                after: self.selection,
                kind,
                open: cls != Class::Newline,
                last_class: Some(cls),
                id: 0,
            });
        }
        Ok(Some(out))
    }

    /// Deletes the selection, or the character before the caret. Returns
    /// `None` at the start of the text. Consecutive deletions coalesce like
    /// typing.
    pub fn backspace(&mut self) -> Result<Option<EditOutcome>, CoreError> {
        self.delete_one(false)
    }

    /// Deletes the selection, or the character after the caret. Returns
    /// `None` at the end of the text.
    pub fn delete_forward(&mut self) -> Result<Option<EditOutcome>, CoreError> {
        self.delete_one(true)
    }

    /// Undoes the last step. Returns the outcomes of the inverse edits, in
    /// application order, so callers can shift their own positions.
    pub fn undo(&mut self) -> Option<Vec<EditOutcome>> {
        let mut group = self.undo.pop()?;
        group.open = false;
        let mut outs = Vec::new();
        for (_, inv) in group.edits.iter().rev() {
            // Inverse edits were produced by this rope; they always apply.
            if let Ok((o, _)) = inv.apply_to_rope(&mut self.text) {
                outs.push(o);
            }
        }
        self.selection = group.before;
        self.redo.push(group);
        Some(outs)
    }

    /// Redoes the last undone step.
    pub fn redo(&mut self) -> Option<Vec<EditOutcome>> {
        let group = self.redo.pop()?;
        let mut outs = Vec::new();
        for (e, _) in &group.edits {
            if let Ok((o, _)) = e.apply_to_rope(&mut self.text) {
                outs.push(o);
            }
        }
        self.selection = group.after;
        if let Some(g) = self.undo.last_mut() {
            g.open = false;
        }
        self.undo.push(group);
        Some(outs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_is_one_undo_step() {
        let mut ed = Editor::new("word");
        ed.apply_group(vec![Edit::insert(0, "**"), Edit::insert(6, "**")])
            .unwrap();
        assert_eq!(ed.text().to_string(), "**word**");
        ed.undo().unwrap();
        assert_eq!(ed.text().to_string(), "word");
        ed.redo().unwrap();
        assert_eq!(ed.text().to_string(), "**word**");
        assert!(ed.is_dirty());
    }

    #[test]
    fn typing_coalesces_into_word_steps() {
        let mut ed = Editor::new("");
        ed.type_text("hello world, again").unwrap();
        let steps: Vec<String> = std::iter::from_fn(|| {
            ed.undo()?;
            Some(ed.text().to_string())
        })
        .collect();
        assert_eq!(steps, vec!["hello world, ", "hello ", ""]);
    }

    #[test]
    fn newline_and_caret_moves_break_steps() {
        let mut ed = Editor::new("");
        ed.type_text("ab\ncd").unwrap();
        ed.set_selection(Selection::caret(CharPos(0)));
        ed.type_char('x').unwrap();
        assert_eq!(ed.text().to_string(), "xab\ncd");
        ed.undo().unwrap();
        assert_eq!(ed.text().to_string(), "ab\ncd");
        ed.undo().unwrap();
        assert_eq!(ed.text().to_string(), "ab\n");
        ed.undo().unwrap();
        assert_eq!(ed.text().to_string(), "ab");
        ed.undo().unwrap();
        assert_eq!(ed.text().to_string(), "");
        assert!(!ed.can_undo());
    }

    #[test]
    fn backspace_coalesces_by_word() {
        let mut ed = Editor::new("hello world");
        ed.set_selection(Selection::caret(CharPos(11)));
        for _ in 0..8 {
            ed.backspace().unwrap();
        }
        assert_eq!(ed.text().to_string(), "hel");
        ed.undo().unwrap();
        assert_eq!(ed.text().to_string(), "hello");
        ed.undo().unwrap();
        assert_eq!(ed.text().to_string(), "hello world");
        assert_eq!(ed.selection(), Selection::caret(CharPos(11)));
        let mut at_start = Editor::new("a");
        assert_eq!(at_start.backspace().unwrap(), None);
        at_start.set_selection(Selection::caret(CharPos(1)));
        assert_eq!(at_start.delete_forward().unwrap(), None);
    }

    #[test]
    fn delete_forward_and_selection_delete() {
        let mut ed = Editor::new("abc def");
        ed.delete_forward().unwrap();
        ed.delete_forward().unwrap();
        assert_eq!(ed.text().to_string(), "c def");
        ed.set_selection(Selection::new(1, 3));
        ed.backspace().unwrap();
        assert_eq!(ed.text().to_string(), "cef");
        ed.undo().unwrap();
        ed.undo().unwrap();
        assert_eq!(ed.text().to_string(), "abc def");
    }

    #[test]
    fn dirty_tracks_the_saved_step() {
        let mut ed = Editor::new("x");
        assert!(!ed.is_dirty());
        ed.type_char('a').unwrap();
        assert!(ed.is_dirty());
        ed.mark_saved();
        assert!(!ed.is_dirty());
        ed.type_char('b').unwrap();
        assert!(ed.is_dirty());
        ed.undo();
        assert!(!ed.is_dirty(), "back at the saved text");
        ed.undo();
        assert!(ed.is_dirty());
        ed.redo();
        assert!(!ed.is_dirty());
        ed.mark_modified();
        assert!(ed.is_dirty());
        ed.mark_saved();
        assert!(!ed.is_dirty());
    }

    #[test]
    fn failed_group_rolls_back() {
        let mut ed = Editor::new("abc");
        let err = ed.apply_group(vec![Edit::insert(0, "x"), Edit::delete(2..99)]);
        assert!(err.is_err());
        assert_eq!(ed.text().to_string(), "abc");
        assert!(!ed.can_undo());
    }

    #[test]
    fn paste_is_one_step_and_set_text_resets() {
        let mut ed = Editor::new("ab");
        ed.set_selection(Selection::caret(CharPos(1)));
        ed.insert_text("XYZ W").unwrap();
        assert_eq!(ed.text().to_string(), "aXYZ Wb");
        assert_eq!(ed.selection(), Selection::caret(CharPos(6)));
        ed.undo().unwrap();
        assert_eq!(ed.text().to_string(), "ab");
        ed.set_text("new");
        assert!(!ed.can_undo() && !ed.can_redo() && !ed.is_dirty());
    }
}
