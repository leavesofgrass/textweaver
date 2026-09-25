//! The document: canonical text plus markers.

use std::cell::OnceCell;
use std::path::PathBuf;

use ropey::Rope;
use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange, CoreError, Edit, EditOutcome};

use crate::marker::Marker;

/// Facts about where a document came from.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentMeta {
    /// Title from the source (HTML `<title>`, front matter, first heading).
    pub title: Option<String>,
    /// Path the document was loaded from, if any.
    pub path: Option<PathBuf>,
    /// Id of the loader that produced it ("text", "markdown", "html", ...).
    pub format: String,
    /// BCP 47 language tag, when the source declares one.
    pub language: Option<String>,
    /// Author, when the source declares one.
    pub author: Option<String>,
}

/// Lazily built char to UTF-16 index, for GUI toolkits that address text in
/// UTF-16 code units (ADR-0002).
#[derive(Clone, Debug, Default)]
pub struct DisplayIndex {
    /// UTF-16 offset of every char, plus the end offset.
    utf16: Vec<u32>,
}

impl DisplayIndex {
    /// Builds the index for `text`.
    pub fn build(text: &Rope) -> Self {
        let mut utf16 = Vec::with_capacity(text.len_chars() + 1);
        let mut at: u32 = 0;
        for c in text.chars() {
            utf16.push(at);
            // A char is at most two UTF-16 units.
            at += c.len_utf16() as u32;
        }
        utf16.push(at);
        DisplayIndex { utf16 }
    }

    /// UTF-16 offset of `pos` (clamped to the end).
    pub fn to_utf16(&self, pos: CharPos) -> u32 {
        self.utf16[pos.0.min(self.utf16.len() - 1)]
    }

    /// Char position of a UTF-16 offset (rounded down to a char start).
    pub fn to_char(&self, utf16: u32) -> CharPos {
        CharPos(
            self.utf16
                .partition_point(|&u| u <= utf16)
                .saturating_sub(1),
        )
    }
}

/// Canonical text plus structure.
///
/// Canonical text keeps Star's shape: paragraphs separated by one blank line,
/// headings and list items as bare lines, tables one row per line under a
/// `Table` marker, images as their alt text, code as text under a `Code`
/// marker. Line endings are always `\n`.
#[derive(Clone, Debug, Default)]
pub struct Document {
    /// Source facts.
    pub meta: DocumentMeta,
    text: Rope,
    markers: Vec<Marker>,
    display: OnceCell<DisplayIndex>,
}

impl Document {
    /// A document from text and markers. Markers are sorted by start.
    pub fn new(meta: DocumentMeta, text: Rope, mut markers: Vec<Marker>) -> Self {
        markers.sort_by_key(|m| (m.range.start, std::cmp::Reverse(m.range.end)));
        Document {
            meta,
            text,
            markers,
            display: OnceCell::new(),
        }
    }

    /// A marker-less document from plain text. `\r\n` and `\r` become `\n`.
    pub fn from_plain_text(text: &str) -> Self {
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        Document::new(
            DocumentMeta {
                format: "text".to_owned(),
                ..DocumentMeta::default()
            },
            Rope::from_str(&normalized),
            Vec::new(),
        )
    }

    /// The canonical text.
    pub fn text(&self) -> &Rope {
        &self.text
    }

    /// Length in chars.
    pub fn len_chars(&self) -> usize {
        self.text.len_chars()
    }

    /// True when the document has no text.
    pub fn is_empty(&self) -> bool {
        self.text.len_chars() == 0
    }

    /// The end position.
    pub fn end(&self) -> CharPos {
        CharPos(self.text.len_chars())
    }

    /// The markers, sorted by start.
    pub fn markers(&self) -> &[Marker] {
        &self.markers
    }

    /// The text of `range` (clamped to the document).
    pub fn slice(&self, range: CharRange) -> String {
        self.text
            .slice(range.clamp_to(self.len_chars()).to_range())
            .to_string()
    }

    /// The char at `pos`, if any.
    pub fn char_at(&self, pos: CharPos) -> Option<char> {
        (pos.0 < self.len_chars()).then(|| self.text.char(pos.0))
    }

    /// The UTF-16 display index, built on first use.
    pub fn display(&self) -> &DisplayIndex {
        self.display.get_or_init(|| DisplayIndex::build(&self.text))
    }

    /// Applies an edit to the text, shifts the markers, and drops the
    /// display index.
    pub fn apply(&mut self, edit: &Edit) -> Result<EditOutcome, CoreError> {
        let (outcome, _inverse) = edit.apply_to_rope(&mut self.text)?;
        for m in &mut self.markers {
            m.shift(&outcome);
        }
        self.display = OnceCell::new();
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_normalizes_newlines() {
        let d = Document::from_plain_text("a\r\nb\rc");
        assert_eq!(d.text().to_string(), "a\nb\nc");
    }

    #[test]
    fn display_index_counts_utf16() {
        let d = Document::from_plain_text("a😀b");
        assert_eq!(d.display().to_utf16(CharPos(2)), 3);
        assert_eq!(d.display().to_char(3), CharPos(2));
    }
}
