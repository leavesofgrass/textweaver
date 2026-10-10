//! The preview inside edit mode (`toggle_preview`, Shift+F4 in the
//! terminal reader): the editor's Markdown source is replaced by the
//! reading view of the same document, at the same place, and back,
//! without leaving edit mode.
//!
//! - The reading view is the document as the reader opens it: the reading
//!   document kept on entering edit mode while nothing has been edited,
//!   else the edited text loaded by the same Markdown loader.
//! - The place is carried by the source map that carries it on entering
//!   and leaving edit mode ([`carry_marks`]): every text character maps
//!   exactly; a caret inside markup (between `**` marks, in a link's
//!   address) shows on the nearest text character. Coming back without
//!   moving restores the caret exactly where it was.
//! - The preview is read-only and reads with the reading keys. Typing says
//!   so; editing, file, and bookmark commands go back to the source first,
//!   then run.
//! - There is no split view in the terminal: a split would halve what a
//!   Braille display's line of the screen shows, for no gain.

use textweaver_formats::Source;
use textweaver_keymap::{ActionId, Category};
use textweaver_lexicon::args;

use super::{Marks, carry_marks};
use crate::app::{App, Mode};
use crate::command::Effect;

/// The source being edited while the reading view is shown.
pub(crate) struct Preview {
    /// The edited document, put back on return.
    source: textweaver_text::Document,
    /// The source positions on entering the preview.
    entry: Marks,
    /// The caret as first shown in the preview: unmoved, `entry` returns.
    shown: textweaver_core::CharPos,
}

/// Whether `a` goes back to the source before it runs: editing, files,
/// and bookmarks work on the text being edited.
pub(crate) fn leaves_preview(a: ActionId) -> bool {
    a != ActionId::TogglePreview
        && matches!(
            a.category(),
            Category::Editing | Category::File | Category::Bookmarks
        )
}

impl App {
    /// True while edit mode shows the reading view.
    pub(crate) fn previewing(&self) -> bool {
        self.edit.as_ref().is_some_and(|e| e.preview.is_some())
    }

    /// `toggle_preview`: shows the reading view of the document being
    /// edited at the same place, or goes back to the source.
    pub(crate) fn toggle_preview(&mut self) -> Vec<Effect> {
        if self.previewing() {
            self.end_preview(true);
            return vec![Effect::Redraw];
        }
        let key = self.key(ActionId::TogglePreview);
        let Some(edit) = self.edit.as_ref() else {
            let edit_key = self.key(ActionId::ToggleEditMode);
            let msg = self.msg_args("edit-preview-not-editing", &args!["key" => edit_key]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        if !self.authoring.structure.markdown {
            let msg = self.msg("edit-preview-plain");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let kept = (!edit.changed).then(|| edit.reading.clone());
        let path = edit.session.doc().path.clone();
        // The session's document mirrors the editor's text.
        let text = self
            .session
            .as_ref()
            .map(|s| s.doc.text().to_string())
            .unwrap_or_default();
        self.stop_speech();
        // The source's structure must match its text before the place is
        // carried across.
        self.refresh_structure(true);
        let rendered = match kept {
            Some(doc) => doc,
            None => {
                let loaded = self.registry.load(
                    &Source::Bytes {
                        data: text.into_bytes(),
                        hint: "md".into(),
                    },
                    &self.load_options(),
                );
                match loaded {
                    Ok(mut doc) => {
                        doc.meta.path = path;
                        doc
                    }
                    Err(e) => {
                        let msg =
                            self.msg_args("edit-preview-failed", &args!["error" => e.to_string()]);
                        self.error(&msg);
                        return vec![Effect::Redraw];
                    }
                }
            }
        };
        let Some(s) = self.session.as_mut() else {
            return vec![Effect::Redraw];
        };
        let entry = Marks::of(s);
        let shown = carry_marks(&entry, &s.doc, &rendered, false);
        let source = std::mem::replace(&mut s.doc, rendered);
        swap_in(s, shown);
        let preview = Preview {
            source,
            entry,
            shown: s.cursor,
        };
        if let Some(edit) = self.edit.as_mut() {
            edit.preview = Some(preview);
        }
        self.mode = Mode::Browse;
        self.return_mode = Mode::Browse;
        self.scroll_to_cursor();
        let msg = self.msg_args("edit-preview-on", &args!["key" => key]);
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    /// Goes back from the reading view to the source, at the place read
    /// to (or exactly where the caret was, when it did not move). Quiet
    /// unless `announce`. Nothing happens when no preview is shown.
    pub(crate) fn end_preview(&mut self, announce: bool) {
        let Some(p) = self.edit.as_mut().and_then(|e| e.preview.take()) else {
            return;
        };
        self.stop_speech();
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let marks = if s.cursor == p.shown {
            p.entry
        } else {
            carry_marks(&Marks::of(s), &s.doc, &p.source, true)
        };
        s.doc = p.source;
        swap_in(s, marks);
        let cursor = s.cursor;
        if let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) {
            ed.set_selection(textweaver_editor::Selection::caret(cursor));
        }
        self.mode = Mode::Edit;
        self.return_mode = Mode::Edit;
        self.scroll_to_cursor();
        if announce {
            let line = self
                .session
                .as_ref()
                .map(|s| textweaver_editor::echo::line_echo(s.doc.text(), s.line()))
                .map(|(line, cut)| {
                    if cut {
                        self.with_line_continues(line)
                    } else {
                        line
                    }
                })
                .unwrap_or_default();
            let msg = self.msg_args("edit-preview-off", &args!["line" => line]);
            self.tell(&msg);
        }
    }

    /// Typing, deleting, or pasting while the reading view is shown.
    pub(crate) fn preview_read_only(&mut self) -> Vec<Effect> {
        let key = self.key(ActionId::TogglePreview);
        let msg = self.msg_args("edit-preview-read-only", &args!["key" => key]);
        self.tell(&msg);
        vec![Effect::Redraw]
    }
}

/// Puts `marks` on the session's new document and clears what belonged
/// to the old one.
fn swap_in(s: &mut crate::app::Session, marks: Marks) {
    s.revision = crate::app::next_revision();
    marks.put(s);
    s.selection = None;
    s.selection_anchor = None;
    s.find = None;
    s.spoken = None;
    s.spoken_sentence = None;
    s.speech_cursor_line = None;
    s.goal_column = None;
}
