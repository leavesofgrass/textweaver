//! The lists a frontend shows in a panel beside the document (Wave 8d):
//! the Contents (the headings, as the outline lists them with Alt+O) and
//! the Notes (as the notes list shows them).
//!
//! Both come from the same models as the app's own lists, so the panel and
//! the list dialog never disagree: [`App::panel_entries`] builds the rows
//! with the outline's and the notes list's own labels, and
//! [`App::go_to_panel_entry`] goes where choosing that row in the list
//! would, with the same announcement.
//!
//! A frontend rebuilds the rows only when [`App::panel_key`] changes (a new
//! document, an edit, headings parsed again, a note added or changed, the
//! interface language), never on a caret or highlight move; the row at the
//! cursor comes from [`current_entry`], a binary search.

use std::hash::{DefaultHasher, Hash, Hasher};

use textweaver_core::CharPos;

use crate::app::App;
use crate::command::Effect;

/// A panel beside the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Panel {
    /// The headings (or a paged document's pages).
    Contents,
    /// The notes.
    Notes,
}

/// One row of a panel: where it goes, and what it says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PanelEntry {
    /// The position the row goes to.
    pub pos: CharPos,
    /// The row's text, meaning first ("Methods, level 2").
    pub label: String,
}

/// The row a cursor at `cursor` is in: the last one starting at or before
/// it. `entries` are in document order.
pub fn current_entry(entries: &[PanelEntry], cursor: CharPos) -> Option<usize> {
    entries.partition_point(|e| e.pos <= cursor).checked_sub(1)
}

impl App {
    /// A number that changes whenever `panel`'s rows may have changed: the
    /// document, its text revision, its parsed structure, the notes, and
    /// the interface language. `None` with no document. Cheap: the
    /// Contents' key reads a few numbers; the Notes' key hashes the notes.
    pub fn panel_key(&self, panel: Panel) -> Option<u64> {
        let s = self.session()?;
        let mut h = DefaultHasher::new();
        panel.hash(&mut h);
        s.key.hash(&mut h);
        s.revision.hash(&mut h);
        self.cat().lang().hash(&mut h);
        match panel {
            Panel::Contents => s.doc.markers().len().hash(&mut h),
            Panel::Notes => {
                s.notes.len().hash(&mut h);
                for n in &s.notes {
                    n.id.hash(&mut h);
                    n.note.hash(&mut h);
                    n.range.start.0.hash(&mut h);
                    n.range.end.0.hash(&mut h);
                }
            }
        }
        Some(h.finish())
    }

    /// The rows of `panel`, in document order: the outline's headings (or
    /// pages) with its labels, or the notes with the notes list's labels.
    /// Empty with no document, or none to list.
    pub fn panel_entries(&self, panel: Panel) -> Vec<PanelEntry> {
        match panel {
            Panel::Contents => {
                let (items, _) = self.outline_items();
                items
                    .iter()
                    .map(|h| PanelEntry {
                        pos: h.pos,
                        label: crate::lists::outline_label(self.cat(), h),
                    })
                    .collect()
            }
            Panel::Notes => {
                let Some(s) = self.session() else {
                    return Vec::new();
                };
                s.notes
                    .iter()
                    .enumerate()
                    .filter_map(|(i, n)| {
                        Some(PanelEntry {
                            pos: n.range.start,
                            label: self.note_item(i)?,
                        })
                    })
                    .collect()
            }
        }
    }

    /// Goes to row `index` of `panel`, as choosing it in the outline or the
    /// notes list does: recorded in history, announced, and reading follows
    /// when it was reading. Nothing happens for a row that is no longer
    /// there.
    pub fn go_to_panel_entry(&mut self, panel: Panel, index: usize) -> Vec<Effect> {
        self.entry(|app| {
            match panel {
                Panel::Contents => {
                    let (items, _) = app.outline_items();
                    if let Some(h) = items.get(index) {
                        app.go_to_heading(h);
                    }
                }
                Panel::Notes => app.go_to_note(index, false),
            }
            if app.edit.is_some() {
                // In edit mode the caret follows, as after any jump.
                app.sync_editor_caret();
            }
            vec![Effect::Redraw]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(pos: usize) -> PanelEntry {
        PanelEntry {
            pos: CharPos(pos),
            label: String::new(),
        }
    }

    #[test]
    fn the_current_row_is_the_last_one_started() {
        let rows = [e(0), e(10), e(20)];
        assert_eq!(current_entry(&rows, CharPos(0)), Some(0));
        assert_eq!(current_entry(&rows, CharPos(15)), Some(1));
        assert_eq!(current_entry(&rows, CharPos(99)), Some(2));
        let later = [e(5)];
        assert_eq!(current_entry(&later, CharPos(2)), None);
        assert_eq!(current_entry(&[], CharPos(2)), None);
    }
}
