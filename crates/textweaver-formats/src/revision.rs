//! Saying tracked changes in place ([`RevisionMode::Marked`]), shared by
//! the DOCX, ODT, and RTF loaders.
//!
//! [`RevisionMode::Marked`]: crate::RevisionMode::Marked

use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::Marker;

use crate::builder::{Builder, OpenId};

/// What a tracked change did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChangeKind {
    /// Text added.
    Inserted,
    /// Text removed.
    Deleted,
    /// Text moved away from here (Word `w:moveFrom`).
    MovedAway,
    /// Text moved here (Word `w:moveTo`).
    MovedHere,
}

impl ChangeKind {
    fn verb(self) -> &'static str {
        match self {
            ChangeKind::Inserted => "inserted",
            ChangeKind::Deleted => "deleted",
            ChangeKind::MovedAway => "moved away",
            ChangeKind::MovedHere => "moved here",
        }
    }

    /// Insertions are underlined and deletions struck through, as word
    /// processors show them.
    fn marker(self) -> MarkerKind {
        match self {
            ChangeKind::Inserted | ChangeKind::MovedHere => MarkerKind::Underline,
            ChangeKind::Deleted | ChangeKind::MovedAway => MarkerKind::Strikethrough,
        }
    }
}

/// "inserted by Ada Example", or "deleted" when no author is given.
pub(crate) fn phrase(kind: ChangeKind, author: Option<&str>) -> String {
    match author.map(str::trim).filter(|a| !a.is_empty()) {
        Some(a) => format!("{} by {a}", kind.verb()),
        None => kind.verb().to_owned(),
    }
}

/// An open change: its marker and whether any text came.
pub(crate) struct Open {
    id: OpenId,
}

/// Starts saying a change: "(inserted by Ada Example:", then the changed
/// text goes under the change's marker until [`close`].
pub(crate) fn open(
    b: &mut Builder,
    kind: ChangeKind,
    author: Option<&str>,
    date: Option<&str>,
) -> Open {
    let words = phrase(kind, author);
    b.space();
    b.text(&format!("({words}:"));
    b.space();
    let mut m = Marker::new(kind.marker(), CharRange::empty(0)).with_label(words);
    if let Some(d) = date.map(str::trim).filter(|d| !d.is_empty()) {
        m = m.with_reference(d);
    }
    Open { id: b.open(m) }
}

/// Ends a change opened with [`open`]: ")".
pub(crate) fn close(b: &mut Builder, open: Open) {
    b.close(open.id);
    b.close_punct(")");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_read_in_place_with_their_markers() {
        let mut b = Builder::new();
        b.text("The cat");
        let o = open(
            &mut b,
            ChangeKind::Inserted,
            Some("Ada Example"),
            Some("2026-09-01"),
        );
        b.text("s ");
        close(&mut b, o);
        b.text(". Then ");
        let o = open(&mut b, ChangeKind::Deleted, None, None);
        b.text("old ");
        close(&mut b, o);
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
    }
}
