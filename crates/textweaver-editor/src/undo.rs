use ropey::Rope;
use serde::{Deserialize, Serialize};
use textweaver_core::{Bias, CharPos, CharRange, CoreError, Edit, EditOutcome};

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

/// One undo step: edits applied in order, with their inverses.
#[derive(Clone, Debug, Default)]
struct Group {
    edits: Vec<(Edit, Edit)>,
    before: Selection,
    after: Selection,
}

/// Text being edited, with undo and redo.
#[derive(Clone, Debug, Default)]
pub struct Editor {
    text: Rope,
    selection: Selection,
    undo: Vec<Group>,
    redo: Vec<Group>,
    dirty: bool,
}

impl Editor {
    /// An editor over `text`, caret at the start, clean.
    pub fn new(text: &str) -> Self {
        Editor {
            text: Rope::from_str(text),
            ..Editor::default()
        }
    }

    /// The text.
    pub fn text(&self) -> &Rope {
        &self.text
    }

    /// The selection.
    pub fn selection(&self) -> Selection {
        self.selection
    }

    /// Sets the selection (clamped to the text).
    pub fn set_selection(&mut self, sel: Selection) {
        let len = self.text.len_chars();
        self.selection = Selection {
            anchor: sel.anchor.clamp_to(len),
            head: sel.head.clamp_to(len),
        };
    }

    /// True when there are unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Marks the text as saved.
    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    /// Applies one edit as its own undo step. The selection follows the edit.
    pub fn apply(&mut self, edit: Edit) -> Result<EditOutcome, CoreError> {
        let mut out = self.apply_group(vec![edit])?;
        Ok(out.pop().unwrap_or_default())
    }

    /// Applies several edits as one undo step. Each edit's range is in the
    /// coordinates left by the previous edits.
    pub fn apply_group(&mut self, edits: Vec<Edit>) -> Result<Vec<EditOutcome>, CoreError> {
        let before = self.selection;
        let mut group = Group {
            before,
            ..Group::default()
        };
        let mut outcomes = Vec::new();
        for e in edits {
            let (o, inv) = e.apply_to_rope(&mut self.text)?;
            self.selection = self.selection.map(&o);
            group.edits.push((e, inv));
            outcomes.push(o);
        }
        group.after = self.selection;
        if !group.edits.is_empty() {
            self.undo.push(group);
            self.redo.clear();
            self.dirty = true;
        }
        Ok(outcomes)
    }

    /// Undoes the last step. Returns the outcomes of the inverse edits, in
    /// application order, so callers can shift their own positions.
    pub fn undo(&mut self) -> Option<Vec<EditOutcome>> {
        let group = self.undo.pop()?;
        let mut outs = Vec::new();
        for (_, inv) in group.edits.iter().rev() {
            // Inverse edits were produced by this rope; they always apply.
            if let Ok((o, _)) = inv.apply_to_rope(&mut self.text) {
                outs.push(o);
            }
        }
        self.selection = group.before;
        self.redo.push(group);
        self.dirty = true;
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
        self.undo.push(group);
        self.dirty = true;
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
}
