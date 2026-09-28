//! Comments and tracked changes that travel with a document.
//!
//! Word comments (`w:comment`, with replies and resolved state from
//! `w15:commentEx`) and OpenDocument annotations (`office:annotation`) are
//! not part of the canonical text: a loader records them as
//! [`DocumentComment`]s, each anchored at the range of text it is about,
//! in the document's properties under [`COMMENTS_PROPERTY`] (JSON), so they
//! survive the document cache. [`comments`] reads them back; the reader
//! turns them into notes.
//!
//! Tracked changes are counted under [`REVISIONS_PROPERTY`] whichever way
//! they are read ([`RevisionMode`](crate::RevisionMode)); [`revision_count`]
//! reads the count.

use serde::{Deserialize, Serialize};
use textweaver_core::CharRange;
use textweaver_text::DocumentMeta;

/// The `DocumentMeta::properties` key holding the document's comments, as
/// a JSON array of [`DocumentComment`].
pub const COMMENTS_PROPERTY: &str = "textweaver.comments";

/// The `DocumentMeta::properties` key holding the number of tracked
/// changes (insertions, deletions, and moves) in the document.
pub const REVISIONS_PROPERTY: &str = "textweaver.revisions";

/// Most comments kept from one document; the rest are dropped with a
/// warning, so a hostile file cannot fill the notes list.
pub const MAX_COMMENTS: usize = 10_000;

/// Longest comment or reply text kept, in chars; longer ones are cut.
pub const MAX_COMMENT_CHARS: usize = 4_000;

/// A comment in a document, anchored at the text it is about.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DocumentComment {
    /// The comment's id in its document (Word's `w:id`, ODF's
    /// `office:name`, or a number in reading order).
    pub id: String,
    /// The commented text in the canonical text; empty for a comment at a
    /// point.
    pub range: CharRange,
    /// Who wrote it, when the document says.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub author: String,
    /// When, as written in the document (ISO 8601 in Word and ODF).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub date: String,
    /// The comment's text, paragraphs joined with spaces.
    pub text: String,
    /// Replies, in order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub replies: Vec<CommentReply>,
    /// Marked resolved (done) in the document.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub resolved: bool,
}

/// A reply to a [`DocumentComment`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CommentReply {
    /// Who wrote it.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub author: String,
    /// When, as written in the document.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub date: String,
    /// The reply's text.
    pub text: String,
}

impl DocumentComment {
    /// The comment in one sentence or a few, to read aloud or show as a
    /// note: "Comment by Ada Example: check this date. Reply by Bo
    /// Example: fixed. Resolved."
    pub fn spoken(&self) -> String {
        let mut out = match self.author.as_str() {
            "" => format!("Comment: {}", self.text),
            a => format!("Comment by {a}: {}", self.text),
        };
        for r in &self.replies {
            end_sentence(&mut out);
            match r.author.as_str() {
                "" => out.push_str(&format!(" Reply: {}", r.text)),
                a => out.push_str(&format!(" Reply by {a}: {}", r.text)),
            }
        }
        if self.resolved {
            end_sentence(&mut out);
            out.push_str(" Resolved.");
        }
        out
    }
}

/// Ends `s` with a full stop unless it already ends a sentence.
fn end_sentence(s: &mut String) {
    let t = s.trim_end().len();
    s.truncate(t);
    if !s.ends_with(['.', '!', '?', ':']) {
        s.push('.');
    }
}

/// The comments a loader recorded on a document, in document order.
/// Unreadable or missing data gives none.
pub fn comments(meta: &DocumentMeta) -> Vec<DocumentComment> {
    meta.properties
        .get(COMMENTS_PROPERTY)
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default()
}

/// The number of tracked changes a loader counted in the document.
pub fn revision_count(meta: &DocumentMeta) -> usize {
    meta.properties
        .get(REVISIONS_PROPERTY)
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// Whitespace collapsed, cut at [`MAX_COMMENT_CHARS`].
pub(crate) fn clean_text(s: &str) -> String {
    let joined = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if joined.chars().count() <= MAX_COMMENT_CHARS {
        return joined;
    }
    let mut cut: String = joined.chars().take(MAX_COMMENT_CHARS).collect();
    cut.push_str("...");
    cut
}

/// Records `comments` (sorted by position, at most [`MAX_COMMENTS`]) and
/// the revision count on `meta`. Nothing is written for none.
pub(crate) fn record(
    meta: &mut DocumentMeta,
    mut comments: Vec<DocumentComment>,
    revisions: usize,
) {
    if revisions > 0 {
        meta.properties
            .insert(REVISIONS_PROPERTY.to_owned(), revisions.to_string());
    }
    if comments.is_empty() {
        return;
    }
    if comments.len() > MAX_COMMENTS {
        comments.truncate(MAX_COMMENTS);
        crate::add_warning(
            meta,
            "This document has more comments than textweaver keeps, so only the first ten thousand are read.",
        );
    }
    comments.sort_by_key(|c| (c.range.start, c.range.end));
    if let Ok(json) = serde_json::to_string(&comments) {
        meta.properties.insert(COMMENTS_PROPERTY.to_owned(), json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_round_trip_through_the_properties() {
        let mut meta = DocumentMeta::default();
        assert!(comments(&meta).is_empty());
        let c = DocumentComment {
            id: "0".into(),
            range: CharRange::new(4, 9),
            author: "Ada Example".into(),
            date: "2026-09-01T10:00:00Z".into(),
            text: "Check this date".into(),
            replies: vec![CommentReply {
                author: "Bo Example".into(),
                date: String::new(),
                text: "Fixed!".into(),
            }],
            resolved: true,
        };
        let point = DocumentComment {
            id: "1".into(),
            range: CharRange::empty(2),
            text: "Here.".into(),
            ..DocumentComment::default()
        };
        record(&mut meta, vec![c.clone(), point.clone()], 3);
        assert_eq!(comments(&meta), vec![point.clone(), c.clone()]);
        assert_eq!(revision_count(&meta), 3);
        assert_eq!(
            c.spoken(),
            "Comment by Ada Example: Check this date. Reply by Bo Example: Fixed! Resolved."
        );
        assert_eq!(point.spoken(), "Comment: Here.");
    }

    #[test]
    fn long_text_is_cut_and_nothing_recorded_for_none() {
        let long = "word ".repeat(MAX_COMMENT_CHARS);
        let t = clean_text(&long);
        assert!(t.ends_with("..."));
        assert_eq!(t.chars().count(), MAX_COMMENT_CHARS + 3);
        let mut meta = DocumentMeta::default();
        record(&mut meta, Vec::new(), 0);
        assert!(meta.properties.is_empty());
    }
}
