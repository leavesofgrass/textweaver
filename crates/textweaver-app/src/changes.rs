//! Tracked changes and comments: a list to review them, accept or reject
//! changes, and reply to, resolve, delete, or add comments (task B1-t1).
//!
//! The loaders record each tracked change (DOCX `w:ins`, `w:del`,
//! `w:moveFrom`, `w:moveTo`; ODF change regions; RTF revision marks) as a
//! [`DocumentChange`] and each comment thread as a [`DocumentComment`] in
//! the document's properties (`textweaver_formats::annotations`). This
//! module lists them, one row each, meaning first:
//!
//! - "Inserted: 'renal', by Ada Example, Tuesday, March 3, 2026"
//! - "Deleted: 'rarely', by Bo Example, date not recorded"
//! - "Comment by Bo Example: check this date, 1 reply, resolved"
//!
//! Dates are said in full from the document's ISO date, or "date not
//! recorded"; a date is never guessed. Enter goes to the place. `a`
//! accepts and `r` rejects the change, Shift with either does every change
//! by the same author, and the Accept all changes and Reject all changes
//! commands do all. On a comment, F2 replies, Space resolves or reopens,
//! Delete deletes (after a y or n question), and `n` adds a comment on the
//! selection or the sentence at the cursor, as the Add comment command
//! does.
//!
//! Accepting and rejecting edit the canonical document in memory
//! ([`resolve_change`]): reading, search, and the study tools see the
//! result at once, and each decision is kept in the session's [`Review`],
//! for writing back to the Word file. The comments' notes (crate::notes)
//! follow every comment change. The two halves of a move are decided
//! together ([`resolve_with_pair`]), and Accept all and Reject all ask
//! once first ("Accept all 12 changes? y or n"), as Replace all does.
//!
//! Save changes to the Word file (task B1-t2) writes the review into the
//! original `.docx` in place (`textweaver_writers::docx_update`), after a
//! copy of the original is kept beside it on the first save. Comments and
//! replies textweaver adds carry `[authoring] author`, or "textweaver"
//! when it is empty; the name is never taken from the computer.

use std::path::PathBuf;

use textweaver_core::{Bias, CharPos, CharRange, Edit, EditOutcome, MarkerKind};
use textweaver_formats::{ChangeKind, CommentReply, DocumentChange, DocumentComment};
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;
use textweaver_store::Note;
use textweaver_text::Document;

use crate::app::{App, ListKind};
use crate::command::{Effect, PromptPurpose};
use crate::list_model::ListKey;
use crate::nav::ReadAfter;

/// Longest changed or comment text shown in a row, in chars.
const ROW_TEXT_CHARS: usize = 80;

/// One row of the changes list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Row {
    /// A tracked change, by its index among the document's changes.
    Change(usize),
    /// A comment thread, by its index among the document's comments.
    Comment(usize),
}

/// An accepted or rejected change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeDecision {
    /// The change, as it was recorded when the document was read.
    pub change: DocumentChange,
    /// Accepted (true) or rejected (false).
    pub accepted: bool,
}

/// What was decided while reviewing the open document, in order: the
/// record a later save to the Word file follows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Review {
    /// Changes accepted or rejected.
    pub decisions: Vec<ChangeDecision>,
    /// Comments deleted, as they were.
    pub deleted_comments: Vec<DocumentComment>,
    /// A comment was replied to, resolved, reopened, or added; the
    /// document's comments hold the result.
    pub comments_changed: bool,
    /// The copy of the original Word file kept before the first save of
    /// the review, once made.
    pub backup: Option<PathBuf>,
}

impl Review {
    /// True when nothing was decided or changed since the last save.
    pub fn is_empty(&self) -> bool {
        self.decisions.is_empty() && self.deleted_comments.is_empty() && !self.comments_changed
    }
}

/// Whitespace collapsed to single spaces, trimmed.
fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `s` cut to [`ROW_TEXT_CHARS`], with "..." when cut.
fn short(s: &str) -> String {
    let s = collapse(s);
    if s.chars().count() <= ROW_TEXT_CHARS {
        return s;
    }
    let mut cut: String = s.chars().take(ROW_TEXT_CHARS).collect();
    cut.push_str("...");
    cut
}

/// A document's ISO 8601 date ("2026-03-03T09:15:00Z") said in full:
/// "Tuesday, March 3, 2026". The date is taken as written, without a
/// time zone conversion. `None` when it is not a real date.
pub fn spoken_date(c: &Catalog, iso: &str) -> Option<String> {
    let day = iso.trim().get(..10)?;
    let ts = textweaver_store::time::parse_timestamp(day)?;
    // Rejects dates that do not exist (February 30).
    if !textweaver_store::time::rfc3339(ts).starts_with(day) {
        return None;
    }
    // 1970-01-01 was a Thursday; 0 is Sunday.
    let weekday = (ts.div_euclid(86_400) + 4).rem_euclid(7);
    let month: u32 = day.get(5..7)?.parse().ok()?;
    let date: u32 = day.get(8..10)?.parse().ok()?;
    Some(c.fmt(
        "changes-date",
        &args![
            "weekday" => c.tr(&format!("changes-weekday-{weekday}")),
            "month" => c.tr(&format!("changes-month-{month}")),
            "day" => date.to_string(),
            "year" => day[..4].to_owned(),
        ],
    ))
}

/// A change's kind in words: "Inserted", "Deleted", "Moved away", "Moved
/// here".
fn kind_words(c: &Catalog, kind: ChangeKind) -> String {
    c.tr(match kind {
        ChangeKind::Inserted => "changes-kind-inserted",
        ChangeKind::Deleted => "changes-kind-deleted",
        ChangeKind::MovedAway => "changes-kind-moved-away",
        ChangeKind::MovedHere => "changes-kind-moved-here",
    })
}

/// A change's row: "Inserted: 'renal', by Ada Example, Tuesday, March 3,
/// 2026", with "author not recorded" and "date not recorded" for what the
/// document leaves out.
pub fn change_row(c: &Catalog, change: &DocumentChange) -> String {
    let who = match change.author.trim() {
        "" => c.tr("changes-no-author"),
        a => c.fmt("changes-by", &args!["author" => a]),
    };
    let when = spoken_date(c, &change.date).unwrap_or_else(|| c.tr("changes-no-date"));
    c.fmt(
        "changes-row",
        &args![
            "kind" => kind_words(c, change.kind),
            "text" => short(&change.text),
            "who" => who,
            "when" => when,
        ],
    )
}

/// A comment thread's row: "Comment by Bo Example: check this date, 1
/// reply, resolved".
pub fn comment_row(c: &Catalog, comment: &DocumentComment) -> String {
    let text = short(&comment.text);
    let mut row = match comment.author.trim() {
        "" => c.fmt("changes-comment", &args!["text" => text]),
        a => c.fmt("changes-comment-by", &args!["author" => a, "text" => text]),
    };
    let n = comment.replies.len();
    if n > 0 {
        row.push_str(", ");
        row.push_str(&c.fmt("changes-replies", &args!["n" => n]));
    }
    if comment.resolved {
        row.push_str(", ");
        row.push_str(&c.tr("changes-resolved"));
    }
    row
}

/// True for the characters a removed word leaves no space before ("cat."
/// not "cat .").
fn closing(c: char) -> bool {
    matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']')
}

/// The edit that accepting (`accept`) or rejecting a change makes to
/// `doc`, or `None` when the text stays as it is (accepting an insertion
/// read as the final text). Inserted text is kept or removed, deleted text
/// removed or put back, and a change said in place ("(inserted by Ada
/// Example: renal)") becomes its text or nothing. The spaces around it
/// stay single.
pub fn change_edit(doc: &Document, change: &DocumentChange, accept: bool) -> Option<Edit> {
    let r = change.range.clamp_to(doc.len_chars());
    let keep = change.kind.adds_text() == accept;
    let before = |p: usize| p.checked_sub(1).and_then(|q| doc.char_at(CharPos(q)));
    let at = |p: usize| doc.char_at(CharPos(p));
    let (start, end) = (r.start.0, r.end.0);
    if r.is_empty() {
        // Deleted text read as the final text: put back on keeping.
        if !keep || change.text.is_empty() {
            return None;
        }
        let mut text = change.text.clone();
        if !change.glued && before(start).is_some_and(|c| !c.is_whitespace()) {
            text.insert(0, ' ');
        }
        if at(start).is_some_and(char::is_alphanumeric) {
            text.push(' ');
        }
        return Some(Edit::insert(CharPos(start), text));
    }
    let said = collapse(&doc.slice(r)) != change.text;
    if keep {
        if !said {
            return None;
        }
        let from = if change.glued && before(start) == Some(' ') {
            start - 1
        } else {
            start
        };
        return Some(Edit::replace(
            CharRange::new(from, end),
            change.text.clone(),
        ));
    }
    // At a line start, the space after goes; else the space before, when
    // the next character is a space or closing punctuation.
    if before(start).is_none_or(|c| c == '\n') && at(end) == Some(' ') {
        return Some(Edit::delete(CharRange::new(start, end + 1)));
    }
    let from =
        if before(start) == Some(' ') && at(end).is_none_or(|c| c.is_whitespace() || closing(c)) {
            start - 1
        } else {
            start
        };
    Some(Edit::delete(CharRange::new(from, end)))
}

/// Accepts (`accept`) or rejects change `i` of those recorded on `doc`:
/// edits the text, moves the other changes and the comments with it, drops
/// the change from the record, and returns it with the edit's outcome
/// (`None` when the text did not change). `None` when there is no change
/// `i`.
pub fn resolve_change(
    doc: &mut Document,
    i: usize,
    accept: bool,
) -> Option<(DocumentChange, Option<EditOutcome>)> {
    let mut changes = textweaver_formats::changes(&doc.meta);
    if i >= changes.len() {
        return None;
    }
    let change = changes.remove(i);
    let outcome = change_edit(doc, &change, accept).and_then(|e| doc.apply(&e).ok());
    if let Some(o) = &outcome {
        for c in &mut changes {
            c.range = o.map_range(c.range);
        }
        let mut comments = textweaver_formats::comments(&doc.meta);
        if !comments.is_empty() {
            for c in &mut comments {
                c.range = o.map_range(c.range);
            }
            textweaver_formats::set_comments(&mut doc.meta, &comments);
        }
        drop_said_markers(doc);
    }
    textweaver_formats::set_changes(&mut doc.meta, &changes);
    Some((change, outcome))
}

/// Accepts or rejects change `i` as [`resolve_change`] does and, for one
/// half of a move, the other half with it: the two are decided together.
/// Returns each change decided with its edit's outcome.
pub fn resolve_with_pair(
    doc: &mut Document,
    i: usize,
    accept: bool,
) -> Vec<(DocumentChange, Option<EditOutcome>)> {
    let Some(first) = resolve_change(doc, i, accept) else {
        return Vec::new();
    };
    let pair = first.0.pair.clone();
    let mut done = vec![first];
    while !pair.is_empty() {
        let Some(j) = textweaver_formats::changes(&doc.meta)
            .iter()
            .position(|c| c.pair == pair)
        else {
            break;
        };
        match resolve_change(doc, j, accept) {
            Some(d) => done.push(d),
            None => break,
        }
    }
    done
}

/// Accepts or rejects every change recorded on `doc` (by `author` only,
/// when given). Returns the changes decided, in document order.
pub fn resolve_all(doc: &mut Document, accept: bool, author: Option<&str>) -> Vec<DocumentChange> {
    resolve_all_with_outcomes(doc, accept, author).0
}

/// [`resolve_all`], with the outcome of every text edit made.
fn resolve_all_with_outcomes(
    doc: &mut Document,
    accept: bool,
    author: Option<&str>,
) -> (Vec<DocumentChange>, Vec<EditOutcome>) {
    let mut done = Vec::new();
    let mut outcomes = Vec::new();
    // From the end, so the earlier positions stay put; a move's other
    // half goes with it.
    while let Some(i) = textweaver_formats::changes(&doc.meta)
        .iter()
        .rposition(|c| author.is_none_or(|a| c.author == a))
    {
        for (c, o) in resolve_with_pair(doc, i, accept) {
            done.push(c);
            outcomes.extend(o);
        }
    }
    done.reverse();
    (done, outcomes)
}

/// Drops the empty insertion and deletion markers a change said in place
/// leaves when it is replaced.
fn drop_said_markers(doc: &mut Document) {
    let said = |m: &textweaver_text::Marker| {
        m.range.is_empty()
            && m.label.is_some()
            && matches!(m.kind, MarkerKind::Underline | MarkerKind::Strikethrough)
    };
    if doc.markers().iter().any(said) {
        let markers = doc.markers().iter().filter(|m| !said(m)).cloned().collect();
        *doc = Document::new(doc.meta.clone(), doc.text().clone(), markers);
    }
}

/// The note id of a comment the document carries (as crate::notes makes
/// it).
fn note_id(c: &DocumentComment) -> String {
    format!("comment-{}", c.id)
}

impl App {
    /// The changes list's rows and their text, in document order.
    fn changes_rows(&self) -> (Vec<Row>, Vec<String>) {
        let Some(s) = self.session.as_ref() else {
            return (Vec::new(), Vec::new());
        };
        let changes = textweaver_formats::changes(&s.doc.meta);
        let comments = textweaver_formats::comments(&s.doc.meta);
        let mut rows: Vec<(CharPos, Row)> = changes
            .iter()
            .enumerate()
            .map(|(i, c)| (c.range.start, Row::Change(i)))
            .chain(
                comments
                    .iter()
                    .enumerate()
                    .map(|(i, c)| (c.range.start, Row::Comment(i))),
            )
            .collect();
        rows.sort_by_key(|(at, row)| (*at, matches!(row, Row::Comment(_))));
        let cat = self.cat();
        let items = rows
            .iter()
            .map(|(_, row)| match *row {
                Row::Change(i) => change_row(cat, &changes[i]),
                Row::Comment(i) => comment_row(cat, &comments[i]),
            })
            .collect();
        (rows.into_iter().map(|(_, r)| r).collect(), items)
    }

    /// `list_changes`: the changes and comments list, after its
    /// introduction.
    pub(crate) fn list_changes(&mut self) -> Vec<Effect> {
        let (rows, items) = self.changes_rows();
        if rows.is_empty() {
            let msg = self.msg("changes-none");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let title = self.msg("changes-title");
        let intro = self.msg_args(
            "changes-intro",
            &args!["title" => title.as_str(), "n" => rows.len()],
        );
        self.list = Some(ListKind::Changes(rows));
        self.tell(&intro);
        vec![Effect::ShowList { title, items }]
    }

    /// The list shown again after a change, quietly, keeping its focus;
    /// closed when nothing is left.
    fn reshow_changes(&mut self) -> Vec<Effect> {
        let (rows, items) = self.changes_rows();
        if rows.is_empty() {
            self.list = None;
            self.list_model = None;
            return vec![Effect::Redraw];
        }
        self.list = Some(ListKind::Changes(rows));
        vec![Effect::ShowList {
            title: self.msg("changes-title"),
            items,
        }]
    }

    /// Enter on row `n`: the cursor goes to the change or comment, and it
    /// is said.
    pub(crate) fn choose_change_row(&mut self, rows: &[Row], n: usize) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let target = match rows.get(n) {
            Some(Row::Change(i)) => textweaver_formats::changes(&s.doc.meta)
                .get(*i)
                .map(|c| c.range.start),
            Some(Row::Comment(i)) => textweaver_formats::comments(&s.doc.meta)
                .get(*i)
                .map(|c| c.range.start),
            None => None,
        };
        let Some(target) = target else {
            return;
        };
        let target = target.clamp_to(s.doc.len_chars());
        let line = crate::text_util::line_of(&s.doc, target);
        let text = s.doc.slice(s.doc.line_range(line));
        let msg = self.nav_message(None, target, &collapse(&text));
        self.jump(target, true, ReadAfter::Follow, &msg);
    }

    /// The changes list's own keys: `a` accepts and `r` rejects the
    /// change, `A` and `R` every change by its author, `n` adds a comment.
    /// `None` for other keys and other lists.
    pub(crate) fn changes_list_key(&mut self, key: ListKey) -> Option<Vec<Effect>> {
        let Some(ListKind::Changes(rows)) = self.list.clone() else {
            return None;
        };
        let n = self.list_model.as_ref().map_or(0, |l| l.selected);
        let ListKey::Char(c) = key else {
            return None;
        };
        match c {
            'a' | 'r' | 'A' | 'R' => {
                let accept = c.eq_ignore_ascii_case(&'a');
                Some(self.decide_row(&rows, n, accept, c.is_uppercase()))
            }
            'n' => {
                self.list = None;
                self.list_model = None;
                Some(self.prompt(PromptPurpose::CommentText))
            }
            _ => None,
        }
    }

    /// Says why the document cannot be changed now (edit mode), or nothing.
    fn changes_blocked(&mut self) -> bool {
        if self.edit.is_some() {
            let msg = self.msg("changes-edit-mode");
            self.tell(&msg);
            return true;
        }
        false
    }

    /// Accepts or rejects the change on row `n`, or (`by_author`) every
    /// change by its author.
    fn decide_row(&mut self, rows: &[Row], n: usize, accept: bool, by_author: bool) -> Vec<Effect> {
        let Some(&Row::Change(i)) = rows.get(n) else {
            let msg = self.msg("changes-not-a-change");
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        if self.changes_blocked() {
            return vec![Effect::Redraw];
        }
        if by_author {
            let Some(author) = self.session.as_ref().and_then(|s| {
                textweaver_formats::changes(&s.doc.meta)
                    .get(i)
                    .map(|c| c.author.clone())
            }) else {
                return vec![Effect::Redraw];
            };
            self.decide_all(accept, Some(&author));
            return self.reshow_changes();
        }
        let decided = self.with_document_edit(accept, |doc| {
            let (mut cs, mut os) = (Vec::new(), Vec::new());
            for (c, o) in resolve_with_pair(doc, i, accept) {
                cs.push(c);
                os.extend(o);
            }
            (cs, os)
        });
        if let Some(c) = decided.first() {
            let msg = self.msg_args(
                if accept {
                    "changes-accepted"
                } else {
                    "changes-rejected"
                },
                &args!["kind" => kind_words(self.cat(), c.kind), "text" => short(&c.text)],
            );
            self.tell(&msg);
        }
        self.reshow_changes()
    }

    /// Accepts or rejects every change, or every change by `author`, and
    /// says how many.
    pub(crate) fn decide_all(&mut self, accept: bool, author: Option<&str>) -> Vec<Effect> {
        if self.session.is_none() || self.changes_blocked() {
            return vec![Effect::Redraw];
        }
        let decided =
            self.with_document_edit(accept, |doc| resolve_all_with_outcomes(doc, accept, author));
        let n = decided.len();
        let msg = match (n, author) {
            (0, _) => self.msg("changes-none-left"),
            (_, Some(a)) if !a.is_empty() => self.msg_args(
                if accept {
                    "changes-accepted-author"
                } else {
                    "changes-rejected-author"
                },
                &args!["n" => n, "author" => a],
            ),
            _ => self.msg_args(
                if accept {
                    "changes-accepted-all"
                } else {
                    "changes-rejected-all"
                },
                &args!["n" => n],
            ),
        };
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    /// Runs `edit` on the open document, then moves every position with
    /// its text edits (notes, highlights, bookmarks, history, the cursor)
    /// and records the changes decided (accepted or not) in the session's
    /// review.
    fn with_document_edit(
        &mut self,
        accepted: bool,
        edit: impl FnOnce(&mut Document) -> (Vec<DocumentChange>, Vec<EditOutcome>),
    ) -> Vec<DocumentChange> {
        let Some(s) = self.session.as_mut() else {
            return Vec::new();
        };
        let (decided, outcomes) = edit(&mut s.doc);
        if !outcomes.is_empty() {
            crate::notes::shift_marks(&mut s.notes, &mut s.highlights, &mut s.bookmarks, &outcomes);
            for o in &outcomes {
                s.history.shift(o);
                s.cursor = o.map_pos(s.cursor, Bias::Before);
            }
            s.cursor = s.cursor.clamp_to(s.doc.len_chars());
            s.selection = None;
            s.selection_anchor = None;
            s.revision = crate::app::next_revision();
            s.find = None;
            s.spoken = None;
            s.spoken_sentence = None;
        }
        s.review
            .decisions
            .extend(decided.iter().map(|c| ChangeDecision {
                change: c.clone(),
                accepted,
            }));
        decided
    }

    /// The comment on row `n` of the changes list, by index, or says the
    /// row is a change.
    fn comment_index(&mut self, rows: &[Row], n: usize) -> Option<usize> {
        match rows.get(n) {
            Some(Row::Comment(i)) => Some(*i),
            _ => {
                let msg = self.msg("changes-not-a-comment");
                self.tell(&msg);
                None
            }
        }
    }

    /// F2 on a comment: asks for a reply.
    pub(crate) fn reply_comment_prompt(&mut self, rows: &[Row], n: usize) -> Vec<Effect> {
        let Some(i) = self.comment_index(rows, n) else {
            return vec![Effect::Redraw];
        };
        self.list = None;
        self.pending_item = Some(i);
        self.prompt(PromptPurpose::CommentReply)
    }

    /// Changes the open document's comments with `change`, keeps their
    /// notes in step, and says `done`.
    fn edit_comments(
        &mut self,
        change: impl FnOnce(&mut Vec<DocumentComment>, &mut Review),
        done: &str,
    ) {
        let msg = self.msg(done);
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let mut comments = textweaver_formats::comments(&s.doc.meta);
        let before = comments.clone();
        change(&mut comments, &mut s.review);
        comments.sort_by_key(|c| (c.range.start, c.range.end));
        textweaver_formats::set_comments(&mut s.doc.meta, &comments);
        s.review.comments_changed = true;
        sync_comment_notes(&mut s.notes, &s.doc, &before, &comments);
        self.persist_marks();
        self.tell(&msg);
    }

    /// The answer to the reply prompt: the reply is added to the comment,
    /// with the author from `[authoring] author` and the date from the
    /// clock.
    pub(crate) fn answer_comment_reply(&mut self, text: &str) -> Vec<Effect> {
        let text = text.trim().to_owned();
        let Some(i) = self.pending_item.take() else {
            return vec![Effect::Redraw];
        };
        if text.is_empty() {
            let msg = self.msg("common-cancelled");
            self.note(&msg);
            return self.reshow_changes();
        }
        let author = self.comment_author();
        let date = textweaver_store::time::rfc3339(textweaver_store::now_ts());
        self.edit_comments(
            |comments, _| {
                if let Some(c) = comments.get_mut(i) {
                    c.replies.push(CommentReply { author, date, text });
                }
            },
            "changes-replied",
        );
        self.reshow_changes()
    }

    /// Space on a comment: resolved, or open again.
    pub(crate) fn toggle_comment_resolved(&mut self, rows: &[Row], n: usize) -> Vec<Effect> {
        let Some(i) = self.comment_index(rows, n) else {
            return vec![Effect::Redraw];
        };
        let resolved = self.session.as_ref().is_some_and(|s| {
            textweaver_formats::comments(&s.doc.meta)
                .get(i)
                .is_some_and(|c| c.resolved)
        });
        self.edit_comments(
            |comments, _| {
                if let Some(c) = comments.get_mut(i) {
                    c.resolved = !resolved;
                }
            },
            if resolved {
                "changes-reopened"
            } else {
                "changes-resolved-done"
            },
        );
        self.reshow_changes()
    }

    /// Delete on a comment, after its question: the thread is deleted.
    pub(crate) fn delete_comment_row(&mut self, rows: &[Row], n: usize) -> Vec<Effect> {
        let Some(&Row::Comment(i)) = rows.get(n) else {
            return self.reshow_changes();
        };
        self.edit_comments(
            |comments, review| {
                if i < comments.len() {
                    review.deleted_comments.push(comments.remove(i));
                }
            },
            "changes-comment-deleted",
        );
        self.reshow_changes()
    }

    /// The answer to the new comment prompt: a comment on the selection,
    /// or the sentence at the cursor.
    pub(crate) fn answer_new_comment(&mut self, text: &str) -> Vec<Effect> {
        let text = text.trim().to_owned();
        if text.is_empty() {
            let msg = self.msg("common-cancelled");
            self.note(&msg);
            return vec![Effect::Redraw];
        }
        let Some(range) = self.note_target() else {
            let msg = self.msg("notes-nothing-to-attach");
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        let author = self.comment_author();
        let date = textweaver_store::time::rfc3339(textweaver_store::now_ts());
        self.edit_comments(
            |comments, _| {
                let id = comments
                    .iter()
                    .filter_map(|c| c.id.parse::<u64>().ok())
                    .max()
                    .map_or(0, |m| m + 1);
                comments.push(DocumentComment {
                    id: id.to_string(),
                    range,
                    author,
                    date,
                    text,
                    ..DocumentComment::default()
                });
            },
            "changes-comment-added",
        );
        vec![Effect::Redraw]
    }

    /// The name on comments and replies textweaver adds: `[authoring]
    /// author`, or "textweaver" when it is empty. Never the computer's or
    /// the account's name.
    fn comment_author(&self) -> String {
        match self.settings.authoring.author.trim() {
            "" => "textweaver".to_owned(),
            a => a.to_owned(),
        }
    }

    /// "Accept all 12 changes? y or n": the question Accept all and
    /// Reject all ask first, or `None` when there is nothing to decide (or
    /// edit mode stops it), so the command says why at once.
    pub(crate) fn decide_all_question(&self, accept: bool) -> Option<String> {
        if self.edit.is_some() {
            return None;
        }
        let n = textweaver_formats::changes(&self.session.as_ref()?.doc.meta).len();
        (n > 0).then(|| {
            self.msg_args(
                if accept {
                    "changes-accept-all-question"
                } else {
                    "changes-reject-all-question"
                },
                &args!["n" => n],
            )
        })
    }

    /// What was decided while reviewing the open document's changes and
    /// comments, for writing them back to its file.
    pub fn review(&self) -> Option<&Review> {
        self.session.as_ref().map(|s| &s.review)
    }
}

/// Keeps the comments' notes in step after the comments went from
/// `before` to `after`: a note per comment, saying it as it is now.
fn sync_comment_notes(
    notes: &mut Vec<Note>,
    doc: &Document,
    before: &[DocumentComment],
    after: &[DocumentComment],
) {
    for gone in before
        .iter()
        .filter(|b| !after.iter().any(|a| a.id == b.id))
    {
        let id = note_id(gone);
        notes.retain(|n| n.id != id);
    }
    for c in after {
        let id = note_id(c);
        if let Some(n) = notes.iter_mut().find(|n| n.id == id) {
            n.note = c.spoken();
            n.tags.retain(|t| t != "resolved");
            if c.resolved {
                n.tags.push("resolved".to_owned());
            }
        }
    }
    // New comments become notes as the loaded ones did.
    crate::notes::add_document_comments_from(notes, doc, after);
}

#[cfg(feature = "publish")]
mod save;
#[cfg(feature = "publish")]
pub use save::docx_update;

#[cfg(not(feature = "publish"))]
impl App {
    /// `save_changes_to_word` in a build without the `publish` feature,
    /// which links no writers: says it is not in this build.
    pub(crate) fn save_changes_to_word(&mut self) -> Vec<Effect> {
        let msg = self.msg("changes-save-not-in-build");
        self.tell(&msg);
        vec![Effect::Redraw]
    }
}

#[cfg(test)]
mod tests;
