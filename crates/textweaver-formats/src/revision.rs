//! Tracked changes, shared by the DOCX, ODT, and RTF loaders: said in
//! place ([`RevisionMode::Marked`]) or read as the final text, and either
//! way recorded as a [`DocumentChange`] with the range of canonical text
//! that accepting or rejecting it replaces.
//!
//! [`RevisionMode::Marked`]: crate::RevisionMode::Marked

use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::Marker;

pub(crate) use crate::annotations::ChangeKind;
use crate::annotations::DocumentChange;
use crate::builder::{Builder, OpenId};

fn verb(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Inserted => "inserted",
        ChangeKind::Deleted => "deleted",
        ChangeKind::MovedAway => "moved away",
        ChangeKind::MovedHere => "moved here",
    }
}

/// Insertions are underlined and deletions struck through, as word
/// processors show them.
fn marker(kind: ChangeKind) -> MarkerKind {
    if kind.adds_text() {
        MarkerKind::Underline
    } else {
        MarkerKind::Strikethrough
    }
}

/// "inserted by Ada Example", or "deleted" when no author is given.
pub(crate) fn phrase(kind: ChangeKind, author: Option<&str>) -> String {
    match author.map(str::trim).filter(|a| !a.is_empty()) {
        Some(a) => format!("{} by {a}", verb(kind)),
        None => verb(kind).to_owned(),
    }
}

/// Whitespace collapsed to single spaces, trimmed.
fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// An open change: what it is, where it started, and its marker when it is
/// said in place.
pub(crate) struct Open {
    said: Option<OpenId>,
    change: DocumentChange,
    /// Char where the recorded range starts.
    start: usize,
    /// Byte where the changed text starts in the builder's text.
    text_byte: usize,
    /// No space came between the text before and this change.
    glued: bool,
    /// Deleted text not written to the canonical text (final reading).
    deleted: String,
}

impl Open {
    /// Deleted text read as the final text: kept for the record (rejecting
    /// the change puts it back), not written.
    pub(crate) fn push_deleted(&mut self, text: &str) {
        self.deleted.push_str(text);
    }
}

/// Starts a change. Said in place (`say`), it opens with "(inserted by Ada
/// Example:", and the changed text goes under the change's marker until
/// [`close`]; otherwise nothing is written, and the caller writes the text
/// of an insertion and passes the text of a deletion to
/// [`Open::push_deleted`].
pub(crate) fn open(
    b: &mut Builder,
    kind: ChangeKind,
    author: Option<&str>,
    date: Option<&str>,
    id: Option<&str>,
    say: bool,
) -> Open {
    let author = author.map(str::trim).filter(|a| !a.is_empty());
    let date = date.map(str::trim).filter(|d| !d.is_empty());
    let change = DocumentChange {
        id: id.map(str::trim).unwrap_or_default().to_owned(),
        kind,
        author: author.unwrap_or_default().to_owned(),
        date: date.unwrap_or_default().to_owned(),
        ..DocumentChange::default()
    };
    let glued = b.joined();
    let len0 = b.len_chars();
    let byte0 = b.as_str().len();
    if !say {
        return Open {
            said: None,
            change,
            start: len0,
            text_byte: byte0,
            glued,
            deleted: String::new(),
        };
    }
    let words = phrase(kind, author);
    b.space();
    b.text(&format!("({words}:"));
    let written = &b.as_str()[byte0..];
    let lead = written.find('(').unwrap_or(0);
    let start = len0 + written[..lead].chars().count();
    b.space();
    let mut m = Marker::new(marker(kind), CharRange::empty(0)).with_label(words);
    if let Some(d) = date {
        m = m.with_reference(d);
    }
    let said = Some(b.open(m));
    Open {
        said,
        change,
        start,
        text_byte: b.as_str().len(),
        glued,
        deleted: String::new(),
    }
}

/// Ends a change opened with [`open`] (writing ")" when it is said) and
/// records it in `out` when it changed any text.
pub(crate) fn close(b: &mut Builder, open: Open, out: &mut Vec<DocumentChange>) {
    let Open {
        said,
        mut change,
        start,
        text_byte,
        glued,
        deleted,
    } = open;
    let written = b.as_str().get(text_byte..).unwrap_or_default();
    if let Some(id) = said {
        change.text = collapse(written);
        b.close(id);
        b.close_punct(")");
        change.range = CharRange::new(start, b.len_chars());
        change.glued = glued;
    } else if change.kind.adds_text() {
        let lead = written.len() - written.trim_start().len();
        change.text = collapse(written);
        let from = start + written[..lead].chars().count();
        change.range = CharRange::new(from, b.len_chars().max(from));
        change.glued = glued && lead == 0;
    } else {
        change.text = collapse(&deleted);
        // At the next paragraph or line when one is pending, so the text
        // put back on rejecting goes there.
        change.range = CharRange::empty(b.next_start());
        change.glued = glued && !deleted.starts_with(char::is_whitespace);
    }
    if !change.text.is_empty() {
        out.push(change);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_read_in_place_with_their_markers() {
        let mut out = Vec::new();
        let mut b = Builder::new();
        b.text("The cat");
        let o = open(
            &mut b,
            ChangeKind::Inserted,
            Some("Ada Example"),
            Some("2026-09-01"),
            Some("7"),
            true,
        );
        b.text("s ");
        close(&mut b, o, &mut out);
        b.text(". Then ");
        let o = open(&mut b, ChangeKind::Deleted, None, None, None, true);
        b.text("old ");
        close(&mut b, o, &mut out);
        b.text("new.");
        let (text, markers) = b.finish();
        assert_eq!(
            text,
            "The cat (inserted by Ada Example: s). Then (deleted: old) new."
        );
        let under = markers
            .iter()
            .find(|m| m.kind == MarkerKind::Underline)
            .expect("an insertion marker");
        assert_eq!(&text[under.range.to_range()], "s");
        assert_eq!(under.label.as_deref(), Some("inserted by Ada Example"));
        assert_eq!(under.reference.as_deref(), Some("2026-09-01"));
        let struck = markers
            .iter()
            .find(|m| m.kind == MarkerKind::Strikethrough)
            .expect("a deletion marker");
        assert_eq!(&text[struck.range.to_range()], "old");
        assert_eq!(phrase(ChangeKind::MovedHere, Some(" ")), "moved here");

        assert_eq!(out.len(), 2);
        assert_eq!(
            &text[out[0].range.to_range()],
            "(inserted by Ada Example: s)"
        );
        assert_eq!(out[0].text, "s");
        assert_eq!(out[0].id, "7");
        assert_eq!(out[0].author, "Ada Example");
        assert_eq!(out[0].date, "2026-09-01");
        assert!(out[0].glued);
        assert_eq!(&text[out[1].range.to_range()], "(deleted: old)");
        assert!(!out[1].glued);
    }

    #[test]
    fn changes_read_as_the_final_text_are_recorded_too() {
        let mut out = Vec::new();
        let mut b = Builder::new();
        b.text("The cat");
        let o = open(&mut b, ChangeKind::Inserted, None, None, None, false);
        b.text("s");
        close(&mut b, o, &mut out);
        b.text(" sat ");
        let mut o = open(
            &mut b,
            ChangeKind::Deleted,
            Some("Bo Example"),
            None,
            None,
            false,
        );
        o.push_deleted("down ");
        close(&mut b, o, &mut out);
        b.text("there.");
        let (text, _) = b.finish();
        assert_eq!(text, "The cats sat there.");
        assert_eq!(&text[out[0].range.to_range()], "s");
        assert!(out[0].glued);
        assert_eq!(out[1].range, CharRange::empty(12));
        assert_eq!(out[1].text, "down");
        assert!(!out[1].glued);
    }
}
