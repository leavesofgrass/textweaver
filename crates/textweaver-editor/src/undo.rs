use ropey::Rope;
use serde::{Deserialize, Serialize};
use textweaver_core::{Bias, CharPos, CharRange, CoreError, Edit, EditOutcome};
use unicode_segmentation::UnicodeSegmentation;

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

/// How much undo history an [`Editor`] keeps. When a new step would go
/// over either limit, the oldest steps are forgotten (the newest step is
/// always kept, however large).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UndoLimits {
    /// Most undo steps kept.
    pub steps: usize,
    /// Most bytes the undo steps may hold: the inserted and removed text of
    /// every step, plus a small allowance per edit.
    pub bytes: usize,
}

impl UndoLimits {
    /// The default: 1,000 steps or 50 MB.
    pub const DEFAULT: UndoLimits = UndoLimits {
        steps: 1000,
        bytes: 50 * 1024 * 1024,
    };
}

impl Default for UndoLimits {
    fn default() -> Self {
        UndoLimits::DEFAULT
    }
}

/// Bytes counted per edit on top of its text, for [`UndoLimits::bytes`].
const EDIT_OVERHEAD: usize = 64;

/// How far [`Editor::backspace`] and [`Editor::delete_forward`] look for
/// the edge of the grapheme next to the caret. A grapheme longer than this
/// (hundreds of combining marks) is deleted in pieces.
const GRAPHEME_WINDOW: usize = 256;

/// The saved state can no longer be reached by undo (the history cap
/// forgot the steps leading to it).
const UNREACHABLE: u64 = u64::MAX;

fn edit_bytes(e: &Edit, inv: &Edit) -> usize {
    e.text.len() + inv.text.len() + EDIT_OVERHEAD
}

/// A point in an [`Editor`]'s undo history: the text as it was when
/// [`Editor::save_point`] was called.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SavePoint(u64);

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

impl Group {
    fn bytes(&self) -> usize {
        self.edits.iter().map(|(e, i)| edit_bytes(e, i)).sum()
    }
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
///
/// Backspace and Delete remove one grapheme (what reads as one character:
/// an emoji with its modifiers, a flag, a letter with its combining
/// accents), not one code point.
///
/// History is capped by [`UndoLimits`] (by default 1,000 steps or 50 MB);
/// the oldest steps are forgotten first.
#[derive(Clone, Debug, Default)]
pub struct Editor {
    text: Rope,
    selection: Selection,
    undo: Vec<Group>,
    redo: Vec<Group>,
    /// Id of the step on top of the undo stack when last saved (0: none,
    /// [`UNREACHABLE`]: forgotten by the history cap).
    saved_id: u64,
    /// Forces dirty until the next save (recovered text).
    modified: bool,
    next_id: u64,
    limits: UndoLimits,
    /// Bytes the undo stack holds, for [`UndoLimits::bytes`].
    undo_bytes: usize,
    /// Undo steps forgotten because of the limits.
    forgotten: usize,
}

impl Editor {
    /// An editor over `text`, caret at the start, clean, with the default
    /// [`UndoLimits`].
    pub fn new(text: &str) -> Self {
        Editor {
            text: Rope::from_str(text),
            next_id: 1,
            limits: UndoLimits::DEFAULT,
            ..Editor::default()
        }
    }

    /// Replaces the whole text and clears undo history, as when a new
    /// editing session starts (undo cannot cross back into a previous
    /// session). The caret goes to the start and the editor is clean. The
    /// undo limits stay.
    pub fn set_text(&mut self, text: &str) {
        let limits = self.limits;
        *self = Editor::new(text);
        self.limits = limits;
    }

    /// Sets how much undo history is kept, forgetting the oldest steps at
    /// once when there are too many. A step limit of 0 counts as 1.
    pub fn set_undo_limits(&mut self, limits: UndoLimits) {
        self.limits = UndoLimits {
            steps: limits.steps.max(1),
            bytes: limits.bytes,
        };
        self.trim();
    }

    /// The undo history limits.
    pub fn undo_limits(&self) -> UndoLimits {
        self.limits
    }

    /// Bytes the undo history holds (see [`UndoLimits::bytes`]).
    pub fn undo_bytes(&self) -> usize {
        self.undo_bytes
    }

    /// Undo steps forgotten because of the limits since the editor was
    /// created.
    pub fn forgotten_steps(&self) -> usize {
        self.forgotten
    }

    /// Forgets the oldest undo steps while over a limit, keeping at least
    /// the newest one.
    fn trim(&mut self) {
        let steps = self.limits.steps.max(1);
        let mut drop = 0;
        let mut bytes = self.undo_bytes;
        let mut len = self.undo.len();
        while len > 1 && (len > steps || bytes > self.limits.bytes) {
            bytes = bytes.saturating_sub(self.undo[drop].bytes());
            drop += 1;
            len -= 1;
        }
        if drop == 0 {
            return;
        }
        for g in self.undo.drain(..drop) {
            // After forgetting step g, an empty stack (top id 0) means "g
            // applied": a save right after g is still reachable, a save
            // before it is not.
            if self.saved_id == g.id {
                self.saved_id = 0;
            } else if self.saved_id == 0 {
                self.saved_id = UNREACHABLE;
            }
        }
        self.undo_bytes = bytes;
        self.forgotten += drop;
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
        let p = self.save_point();
        self.mark_saved_at(p);
    }

    /// The text as it is now, as a point in the undo history, for a save
    /// that finishes later (on a background writer): typing goes on in a
    /// new undo step, and [`mark_saved_at`](Self::mark_saved_at) marks this
    /// point saved once the file is written. Edits made in between keep
    /// the editor dirty.
    pub fn save_point(&mut self) -> SavePoint {
        self.break_undo_group();
        SavePoint(self.top_id())
    }

    /// Marks the text at `point` as the saved text (see
    /// [`save_point`](Self::save_point)).
    pub fn mark_saved_at(&mut self, point: SavePoint) {
        self.saved_id = point.0;
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
        self.undo_bytes += group.bytes();
        self.undo.push(group);
        self.redo.clear();
        self.trim();
    }

    /// Adds a coalesced keystroke to the open step on top of the stack.
    fn extend_top(&mut self, edit: Edit, inv: Edit, cls: Class) {
        let bytes = edit_bytes(&edit, &inv);
        let after = self.selection;
        if let Some(g) = self.undo.last_mut() {
            g.edits.push((edit, inv));
            g.after = after;
            g.last_class = Some(cls);
            self.undo_bytes += bytes;
        }
        self.redo.clear();
        if self.undo_bytes > self.limits.bytes {
            self.trim();
        }
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
        if let Some(g) = self.undo.last_mut()
            && !outs.is_empty()
        {
            g.after = self.selection;
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
            self.extend_top(edit, inv, cls);
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

    /// The grapheme next to `caret`: after it (`forward`) or before it.
    /// Empty at the edge of the text.
    pub fn grapheme_at(&self, caret: CharPos, forward: bool) -> CharRange {
        let len = self.text.len_chars();
        let at = caret.0.min(len);
        if forward {
            if at >= len {
                return CharRange::empty(CharPos(at));
            }
            let end = (at + GRAPHEME_WINDOW).min(len);
            let window = self.text.slice(at..end).to_string();
            let n = window
                .graphemes(true)
                .next()
                .map_or(1, |g| g.chars().count().max(1));
            CharRange::new(at, at + n)
        } else {
            if at == 0 {
                return CharRange::empty(CharPos(0));
            }
            // Start the window at the line start when it is near, so a run
            // of regional indicators (flags) pairs up as it does on screen.
            let floor = at.saturating_sub(GRAPHEME_WINDOW);
            let line_start = self.text.line_to_char(self.text.char_to_line(at - 1));
            let start = line_start.max(floor);
            let window = self.text.slice(start..at).to_string();
            let n = window
                .graphemes(true)
                .next_back()
                .map_or(1, |g| g.chars().count().max(1));
            CharRange::new(at - n, at)
        }
    }

    fn delete_one(&mut self, forward: bool) -> Result<Option<EditOutcome>, CoreError> {
        let r = self.selection.range();
        if !r.is_empty() {
            return self.apply(Edit::delete(r)).map(Some);
        }
        let range = self.grapheme_at(self.selection.head, forward);
        if range.is_empty() {
            return Ok(None);
        }
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
            self.extend_top(edit, inv, cls);
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

    /// Deletes the selection, or the character (grapheme) before the
    /// caret. Returns `None` at the start of the text. Consecutive
    /// deletions coalesce like typing.
    pub fn backspace(&mut self) -> Result<Option<EditOutcome>, CoreError> {
        self.delete_one(false)
    }

    /// Deletes the selection, or the character (grapheme) after the caret.
    /// Returns `None` at the end of the text.
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
        self.undo_bytes = self.undo_bytes.saturating_sub(group.bytes());
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
        self.undo_bytes += group.bytes();
        self.undo.push(group);
        self.trim();
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

    /// Backspaces from the end of `text` until it is empty; returns what
    /// each keystroke removed.
    fn backspace_all(text: &str) -> Vec<String> {
        let mut ed = Editor::new(text);
        ed.set_selection(Selection::caret(CharPos(text.chars().count())));
        let mut removed = Vec::new();
        let mut before = ed.text().to_string();
        while ed.backspace().unwrap().is_some() {
            let now = ed.text().to_string();
            removed.push(before[now.len()..].to_owned());
            before = now;
        }
        removed
    }

    /// Deletes forward from the start until empty; returns each removal.
    fn delete_all(text: &str) -> Vec<String> {
        let mut ed = Editor::new(text);
        let mut removed = Vec::new();
        let mut before = ed.text().to_string();
        while ed.delete_forward().unwrap().is_some() {
            let now = ed.text().to_string();
            removed.push(before[..before.len() - now.len()].to_owned());
            before = now;
        }
        removed
    }

    #[test]
    fn backspace_and_delete_remove_whole_graphemes() {
        // A family emoji (zero-width joiners), a thumbs-up with a skin
        // tone, two flags, "e" with a combining acute, and "n" with a
        // combining tilde.
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
        let thumbs = "\u{1F44D}\u{1F3FD}";
        let flags = "\u{1F1FA}\u{1F1F8}\u{1F1EC}\u{1F1E7}";
        let accents = "e\u{301}n\u{303}";
        let text = format!("a{family}{thumbs}{flags}{accents}");
        let back = backspace_all(&text);
        assert_eq!(
            back,
            vec![
                "n\u{303}",
                "e\u{301}",
                "\u{1F1EC}\u{1F1E7}",
                "\u{1F1FA}\u{1F1F8}",
                thumbs,
                family,
                "a"
            ]
        );
        let fwd = delete_all(&text);
        assert_eq!(
            fwd,
            vec![
                "a",
                family,
                thumbs,
                "\u{1F1FA}\u{1F1F8}",
                "\u{1F1EC}\u{1F1E7}",
                "e\u{301}",
                "n\u{303}"
            ]
        );
    }

    #[test]
    fn a_grapheme_delete_is_undone_whole() {
        let text = "x\u{1F1FA}\u{1F1F8}";
        let mut ed = Editor::new(text);
        ed.set_selection(Selection::caret(CharPos(3)));
        let out = ed.backspace().unwrap().unwrap();
        assert_eq!(out.removed, CharRange::new(1, 3));
        assert_eq!(ed.text().to_string(), "x");
        ed.undo().unwrap();
        assert_eq!(ed.text().to_string(), text);
        assert_eq!(ed.selection(), Selection::caret(CharPos(3)));
        // Line breaks: CRLF is one grapheme; a flag at a line start pairs
        // from there.
        assert_eq!(backspace_all("a\r\nb"), vec!["b", "\r\n", "a"]);
        assert_eq!(
            backspace_all("\u{1F1FA}\n\u{1F1FA}\u{1F1F8}"),
            vec!["\u{1F1FA}\u{1F1F8}", "\n", "\u{1F1FA}"]
        );
    }

    #[test]
    fn undo_history_is_capped_by_steps() {
        let mut ed = Editor::new("");
        ed.set_undo_limits(UndoLimits {
            steps: 3,
            bytes: usize::MAX,
        });
        for w in ["one ", "two ", "three ", "four ", "five "] {
            ed.insert_text(w).unwrap();
        }
        assert_eq!(ed.undo_depth(), 3);
        assert_eq!(ed.forgotten_steps(), 2);
        while ed.undo().is_some() {}
        assert_eq!(
            ed.text().to_string(),
            "one two ",
            "the two oldest steps are kept in the text, not undoable"
        );
        assert!(ed.is_dirty(), "the original text cannot be reached");
    }

    #[test]
    fn undo_history_is_capped_by_bytes() {
        let mut ed = Editor::new("");
        let limit = 10_000;
        ed.set_undo_limits(UndoLimits {
            steps: usize::MAX,
            bytes: limit,
        });
        let chunk = "x".repeat(1000);
        for _ in 0..50 {
            ed.insert_text(&chunk).unwrap();
            assert!(ed.undo_bytes() <= limit, "{}", ed.undo_bytes());
        }
        assert!(ed.undo_depth() < 50 && ed.undo_depth() >= 5);
        // One step bigger than the limit is still kept.
        ed.insert_text(&"y".repeat(20_000)).unwrap();
        assert_eq!(ed.undo_depth(), 1);
        ed.undo().unwrap();
        assert_eq!(ed.text().len_chars(), 50_000);
        assert_eq!(ed.undo_bytes(), 0);
        ed.redo().unwrap();
        assert!(ed.undo_bytes() > limit);
    }

    #[test]
    fn a_save_inside_forgotten_history_stays_reachable_only_at_the_edge() {
        let mut ed = Editor::new("");
        ed.set_undo_limits(UndoLimits {
            steps: 2,
            bytes: usize::MAX,
        });
        ed.insert_text("a").unwrap();
        ed.mark_saved();
        ed.insert_text("b").unwrap();
        ed.insert_text("c").unwrap();
        // "a" was forgotten; the saved state ("a") is the bottom of the
        // stack now.
        assert_eq!(ed.undo_depth(), 2);
        ed.undo().unwrap();
        ed.undo().unwrap();
        assert_eq!(ed.text().to_string(), "a");
        assert!(!ed.is_dirty());
        ed.redo().unwrap();
        assert!(ed.is_dirty());
    }

    #[test]
    fn typing_many_words_keeps_memory_bounded() {
        let mut ed = Editor::new("");
        ed.set_undo_limits(UndoLimits {
            steps: 100,
            bytes: usize::MAX,
        });
        for _ in 0..500 {
            ed.type_text("word ").unwrap();
        }
        assert_eq!(ed.undo_depth(), 100);
        assert_eq!(ed.text().len_chars(), 2500);
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
