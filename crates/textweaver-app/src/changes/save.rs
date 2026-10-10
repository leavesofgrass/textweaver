//! Save changes to the Word file (task B1-t2): the session's review written
//! into the original `.docx` in place by `textweaver_writers::docx_update`,
//! after a copy of the original is kept beside it, the first time.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use textweaver_core::CharRange;
use textweaver_formats::DocumentComment;
use textweaver_lexicon::args;
use textweaver_text::Document;
use textweaver_writers::docx_update::{
    CommentThread, DocxUpdate, ThreadReply, UpdateReport, backup, update_docx,
};

use super::{Review, collapse, sync_comment_notes};
use crate::app::App;
use crate::command::Effect;

/// The update that writes `review` of `doc` into its Word file. Changes
/// not listed (formatting changes, paragraph marks) follow when every
/// listed change was decided the same way and none is left.
pub fn docx_update(doc: &Document, review: &Review) -> DocxUpdate {
    let decisions: HashMap<String, bool> = review
        .decisions
        .iter()
        .filter(|d| !d.change.id.is_empty())
        .map(|d| (d.change.id.clone(), d.accepted))
        .collect();
    let rest = if textweaver_formats::changes(&doc.meta).is_empty() {
        let mut all = review.decisions.iter().map(|d| d.accepted);
        all.next().filter(|&first| all.all(|a| a == first))
    } else {
        None
    };
    let threads = if review.comments_changed || !review.deleted_comments.is_empty() {
        let whole = collapse(&doc.slice(CharRange::new(0, doc.len_chars())));
        textweaver_formats::comments(&doc.meta)
            .iter()
            .map(|c| thread(doc, &whole, c))
            .collect()
    } else {
        Vec::new()
    };
    DocxUpdate {
        decisions,
        rest,
        threads,
        deleted_comments: review
            .deleted_comments
            .iter()
            .map(|c| c.id.clone())
            .collect(),
    }
}

/// A comment as a thread to write, with the text it is on and which
/// occurrence of that text it is (for a new comment's place).
fn thread(doc: &Document, whole: &str, c: &DocumentComment) -> CommentThread {
    let r = c.range.clamp_to(doc.len_chars());
    let anchor = collapse(&doc.slice(r));
    let occurrence = if anchor.is_empty() {
        0
    } else {
        let before = collapse(&doc.slice(CharRange::new(0, r.start.0))).len();
        whole
            .match_indices(&anchor)
            .take_while(|(i, _)| *i < before)
            .count()
    };
    CommentThread {
        id: c.id.clone(),
        author: c.author.clone(),
        date: c.date.clone(),
        text: c.text.clone(),
        resolved: c.resolved,
        replies: c
            .replies
            .iter()
            .map(|r| ThreadReply {
                author: r.author.clone(),
                date: r.date.clone(),
                text: r.text.clone(),
            })
            .collect(),
        anchor,
        occurrence,
    }
}

/// Reads `path`, writes `update` into it after keeping a copy of the
/// original (when `keep_original`), and returns the copy's name and what
/// was done.
fn write_back(
    path: &Path,
    update: &DocxUpdate,
    keep_original: bool,
) -> Result<(Option<PathBuf>, UpdateReport), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let (new, report) = update_docx(&bytes, update).map_err(|e| e.to_string())?;
    let kept = if keep_original {
        Some(backup(path).map_err(|e| e.to_string())?)
    } else {
        None
    };
    textweaver_core::fs::write_atomic(path, &new).map_err(|e| e.to_string())?;
    Ok((kept, report))
}

pub(super) fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.display().to_string())
}

impl App {
    /// `save_changes_to_word`: the review written into the open Word file.
    pub(crate) fn save_changes_to_word(&mut self) -> Vec<Effect> {
        if self.session.is_none() || self.changes_blocked() {
            return vec![Effect::Redraw];
        }
        let Some(path) = self
            .session
            .as_ref()
            .and_then(|s| s.doc.meta.path.clone())
            .filter(|p| {
                p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("docx"))
            })
        else {
            let msg = self.msg("changes-save-not-word");
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        if s.review.is_empty() {
            let msg = self.msg("changes-save-nothing");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let update = docx_update(&s.doc, &s.review);
        let file = file_name(&path);
        let (kept, report) = match write_back(&path, &update, s.review.backup.is_none()) {
            Ok(done) => done,
            Err(error) => {
                let msg = self.msg_args(
                    "changes-save-failed",
                    &args!["file" => file, "error" => error],
                );
                self.tell(&msg);
                return vec![Effect::Redraw];
            }
        };
        if let Some(s) = self.session.as_mut() {
            let backup = s.review.backup.take();
            s.review = Review {
                backup: kept.clone().or(backup),
                ..Review::default()
            };
            s.disk = crate::disk::FileStamp::of(&path);
            if !report.new_comment_ids.is_empty() {
                // New comments take the ids written, so the next save
                // finds them in the file.
                let before = textweaver_formats::comments(&s.doc.meta);
                let mut after = before.clone();
                for c in &mut after {
                    if let Some((_, id)) =
                        report.new_comment_ids.iter().find(|(old, _)| *old == c.id)
                    {
                        c.id = id.clone();
                    }
                }
                textweaver_formats::set_comments(&mut s.doc.meta, &after);
                sync_comment_notes(&mut s.notes, &s.doc, &before, &after);
                self.persist_marks();
            }
        }
        let mut msg = match &kept {
            Some(b) => self.msg_args(
                "changes-saved-backup",
                &args!["file" => file, "backup" => file_name(b)],
            ),
            None => self.msg_args("changes-saved", &args!["file" => file]),
        };
        if report.unplaced > 0 {
            msg.push(' ');
            msg.push_str(&self.msg_args("changes-save-unplaced", &args!["n" => report.unplaced]));
        }
        self.tell(&msg);
        vec![Effect::Redraw]
    }
}
