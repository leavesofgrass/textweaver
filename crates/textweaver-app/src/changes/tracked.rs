//! Tracked edits from edit mode (task B1-t3): with `[authoring]
//! track_changes` on, saving edit mode's work on a Word file writes each
//! changed word into the file as a tracked change (`w:ins`, `w:del`) with
//! `[authoring] author` and the time from the clock, instead of asking for
//! a Markdown name.
//!
//! Edit mode edits the Word file's text as Markdown, so the last saved
//! Markdown and the new one are compared word by word, with the Markdown
//! marks left out (they are not words in the Word file), and each changed
//! stretch is placed in the file by the words around it
//! (`textweaver_writers::docx_update::track_edits`). shortcut: a change
//! across paragraphs, or in a link, a field, or a run holding more than
//! text, cannot be placed; then nothing is written and Save As keeps the
//! edits as Markdown. Formatting changes are not tracked.

use std::path::Path;

use textweaver_lexicon::args;
use textweaver_writers::docx_update::{TrackedEdit, backup, track_edits};

use crate::app::App;
use crate::command::Effect;
use crate::edit::SaveThen;

/// Words of context on each side of a change.
const CONTEXT: usize = 4;
/// The largest comparison table, in cells; beyond it the changed middle
/// is one change.
const MAX_CELLS: usize = 4_000_000;
/// A paragraph break among the words.
const BREAK: &str = "\n";

/// The words of a Markdown text, without its marks (heading and list
/// markers, emphasis, code and table marks), with a [`BREAK`] after each
/// line that has words.
fn words(markdown: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in markdown.lines() {
        let before = out.len();
        for (k, w) in line.split_whitespace().enumerate() {
            let marker = k == 0
                && (w.chars().all(|c| c == '#')
                    || matches!(w, "-" | "*" | "+" | ">")
                    || (w.len() > 1
                        && w.ends_with(['.', ')'])
                        && w[..w.len() - 1].chars().all(|c| c.is_ascii_digit())));
            let t = w.trim_matches(['*', '_', '`', '|']);
            if !marker && !t.is_empty() {
                out.push(t.to_owned());
            }
        }
        if out.len() > before {
            out.push(BREAK.to_owned());
        }
    }
    out
}

/// The changed stretches between `old` and `new`, as (old range, new
/// range), in order.
fn hunks(old: &[String], new: &[String]) -> Vec<(std::ops::Range<usize>, std::ops::Range<usize>)> {
    let pre = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let suf = old[pre..]
        .iter()
        .rev()
        .zip(new[pre..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let (a, b) = (&old[pre..old.len() - suf], &new[pre..new.len() - suf]);
    if a.is_empty() && b.is_empty() {
        return Vec::new();
    }
    if a.len().saturating_mul(b.len()) > MAX_CELLS || a.is_empty() || b.is_empty() {
        return vec![(pre..pre + a.len(), pre..pre + b.len())];
    }
    // Longest common subsequence, from the end.
    let (n, m) = (a.len(), b.len());
    let mut lcs = vec![0u32; (n + 1) * (m + 1)];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i * (m + 1) + j] = if a[i] == b[j] {
                lcs[(i + 1) * (m + 1) + j + 1] + 1
            } else {
                lcs[(i + 1) * (m + 1) + j].max(lcs[i * (m + 1) + j + 1])
            };
        }
    }
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    let mut open: Option<(usize, usize)> = None;
    while i < n || j < m {
        if i < n && j < m && a[i] == b[j] {
            if let Some((oi, oj)) = open.take() {
                out.push((pre + oi..pre + i, pre + oj..pre + j));
            }
            i += 1;
            j += 1;
            continue;
        }
        open.get_or_insert((i, j));
        if j < m && (i == n || lcs[i * (m + 1) + j + 1] >= lcs[(i + 1) * (m + 1) + j]) {
            j += 1;
        } else {
            i += 1;
        }
    }
    if let Some((oi, oj)) = open {
        out.push((pre + oi..pre + n, pre + oj..pre + m));
    }
    out
}

/// Up to [`CONTEXT`] words, stopping at a paragraph break.
fn context<'a>(words: impl Iterator<Item = &'a String>) -> Vec<&'a str> {
    words
        .take_while(|w| *w != BREAK)
        .take(CONTEXT)
        .map(String::as_str)
        .collect()
}

/// The tracked edits that turn `old` Markdown into `new`, last first (so
/// each one's later words are already as `new` has them), and how many
/// changes cross a paragraph break and cannot be tracked.
pub(crate) fn tracked_edits(old: &str, new: &str) -> (Vec<TrackedEdit>, usize) {
    let (a, b) = (words(old), words(new));
    let mut edits = Vec::new();
    let mut crossing = 0;
    for (o, n) in hunks(&a, &b).into_iter().rev() {
        if a[o.clone()].iter().chain(&b[n.clone()]).any(|w| w == BREAK) {
            crossing += 1;
            continue;
        }
        let mut before = context(a[..o.start].iter().rev());
        before.reverse();
        edits.push(TrackedEdit {
            before: before.join(" "),
            deleted: a[o].join(" "),
            inserted: b[n.clone()].join(" "),
            after: context(b[n.end..].iter()).join(" "),
        });
    }
    (edits, crossing)
}

impl App {
    /// Ctrl+S (or Save on leaving) in edit mode on a Word file with
    /// `[authoring] track_changes` on: the edits since the last save go
    /// into the file as tracked changes, after a copy of the original is
    /// kept (the first time). `None` when this save is not one of those.
    pub(crate) fn save_tracked(&mut self, then: SaveThen) -> Option<Vec<Effect>> {
        if !self.settings.authoring.track_changes {
            return None;
        }
        let edit = self.edit.as_mut()?;
        let path = edit.session.doc().path.clone().filter(|p| {
            p.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("docx"))
        })?;
        let old = edit.session.document_text().to_owned();
        let req = match edit.session.begin_save_into(&path) {
            Ok(r) => r,
            Err(e) => {
                let msg = self.msg_args("edit-save-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
                return Some(vec![Effect::Redraw]);
            }
        };
        let new = req.text.to_string();
        let file = super::save::file_name(&path);
        let (edits, crossing) = tracked_edits(&old, &new);
        let author = self.comment_author();
        let date = textweaver_store::time::rfc3339(textweaver_store::now_ts());
        let keep_original = self
            .session
            .as_ref()
            .is_none_or(|s| s.review.backup.is_none());
        let result = write_tracked(&path, &edits, crossing, &author, &date, keep_original);
        let (kept, written) = match result {
            Ok(done) => done,
            Err(Refused::Unplaced(n)) => {
                let msg =
                    self.msg_args("changes-tracked-refused", &args!["n" => n, "file" => file]);
                self.error(&msg);
                return Some(vec![Effect::Redraw]);
            }
            Err(Refused::Failed(error)) => {
                let msg = self.msg_args(
                    "changes-save-failed",
                    &args!["file" => file, "error" => error],
                );
                self.error(&msg);
                return Some(vec![Effect::Redraw]);
            }
        };
        if let Some(edit) = self.edit.as_mut() {
            edit.session.finish_save(req, new);
        }
        self.after_save(&path, false);
        if let Some(s) = self.session.as_mut() {
            s.disk = crate::disk::FileStamp::of(&path);
            if kept.is_some() {
                s.review.backup.clone_from(&kept);
            }
        }
        let mut msg = self.msg_args(
            "changes-tracked-saved",
            &args!["n" => written, "file" => file],
        );
        if let Some(b) = &kept {
            msg.push(' ');
            msg.push_str(&self.msg_args(
                "changes-original-kept",
                &args!["backup" => super::save::file_name(b)],
            ));
        }
        self.tell(&msg);
        Some(match then {
            SaveThen::Stay => vec![Effect::Redraw],
            SaveThen::Leave(after) => self.leave_edit(None, None, after),
        })
    }
}

/// Why tracked edits were not written.
enum Refused {
    /// This many changes cannot be placed; nothing was written.
    Unplaced(usize),
    /// Reading or writing the file failed.
    Failed(String),
}

/// Writes `edits` into the Word file at `path` as tracked changes, after
/// keeping a copy of the original (when `keep_original`). Nothing is
/// written when any change cannot be placed. Returns the copy's name and
/// how many changes were written.
fn write_tracked(
    path: &Path,
    edits: &[TrackedEdit],
    crossing: usize,
    author: &str,
    date: &str,
    keep_original: bool,
) -> Result<(Option<std::path::PathBuf>, usize), Refused> {
    let failed = |e: &dyn std::fmt::Display| Refused::Failed(e.to_string());
    let bytes = std::fs::read(path).map_err(|e| failed(&e))?;
    let (new, report) = track_edits(&bytes, edits, author, date).map_err(|e| failed(&e))?;
    if report.unplaced + crossing > 0 {
        return Err(Refused::Unplaced(report.unplaced + crossing));
    }
    let kept = if keep_original {
        Some(backup(path).map_err(|e| failed(&e))?)
    } else {
        None
    };
    textweaver_core::fs::write_atomic(path, &new).map_err(|e| failed(&e))?;
    Ok((kept, report.written))
}
