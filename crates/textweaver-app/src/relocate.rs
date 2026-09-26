//! Positions that survive outside edits (Phase 2).

use textweaver_store::TextStamp;
use textweaver_text::Document;

/// The stamp of a document's canonical text: its length and hash.
pub(crate) fn text_stamp(doc: &Document) -> TextStamp {
    TextStamp::of(doc.text().chunks())
}
