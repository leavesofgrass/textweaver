//! The document: canonical text plus markers.

use std::cell::OnceCell;
use std::collections::BTreeMap;
use std::path::PathBuf;

use ropey::Rope;
use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange, CoreError, Edit, EditOutcome};

use crate::marker::{Marker, MarkerIndex, MarkerTables};

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
    /// Other declared metadata (Markdown front matter keys, HTML `<meta>`
    /// names), never part of the spoken text.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, String>,
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

/// True for the chars the rope treats as line breaks.
pub(crate) fn is_line_break(c: char) -> bool {
    matches!(
        c,
        '\n' | '\r' | '\u{0B}' | '\u{0C}' | '\u{85}' | '\u{2028}' | '\u{2029}'
    )
}

/// Canonical text plus structure.
///
/// Canonical text (ADR-0002): paragraphs separated by one blank line,
/// headings and list items as bare lines, tables one row per line under a
/// `Table` marker (cells separated by `" | "`), images as their alt text,
/// code as text under a `Code` marker. Line endings are always `\n`.
///
/// Markers are kept sorted by [`Marker::sort_key`]; per-kind index tables for
/// [`Document::marker_index`] and the UTF-16 [`DisplayIndex`] are built on
/// first use and dropped by [`Document::apply`].
///
/// A document serializes as `{ meta, text, markers }`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(into = "DocumentData", from = "DocumentData")]
pub struct Document {
    /// Source facts.
    pub meta: DocumentMeta,
    text: Rope,
    markers: Vec<Marker>,
    display: OnceCell<DisplayIndex>,
    tables: OnceCell<MarkerTables>,
    /// Sorted numbers of the blank (whitespace-only) lines.
    blank_lines: OnceCell<Vec<usize>>,
}

/// The serialized shape of a [`Document`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentData {
    /// Source facts.
    pub meta: DocumentMeta,
    /// Canonical text.
    pub text: String,
    /// Markers, sorted.
    pub markers: Vec<Marker>,
}

impl From<Document> for DocumentData {
    fn from(d: Document) -> Self {
        DocumentData {
            text: d.text.to_string(),
            meta: d.meta,
            markers: d.markers,
        }
    }
}

impl From<DocumentData> for Document {
    fn from(d: DocumentData) -> Self {
        Document::new(d.meta, Rope::from_str(&d.text), d.markers)
    }
}

impl PartialEq for Document {
    fn eq(&self, other: &Self) -> bool {
        self.meta == other.meta && self.text == other.text && self.markers == other.markers
    }
}

impl Document {
    /// A document from text and markers. Markers are sorted by
    /// [`Marker::sort_key`] and clamped to the text.
    pub fn new(meta: DocumentMeta, text: Rope, mut markers: Vec<Marker>) -> Self {
        let len = text.len_chars();
        for m in &mut markers {
            m.range = m.range.clamp_to(len);
        }
        markers.sort_by_key(Marker::sort_key);
        Document {
            meta,
            text,
            markers,
            display: OnceCell::new(),
            tables: OnceCell::new(),
            blank_lines: OnceCell::new(),
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

    /// The whole document as a range.
    pub fn full_range(&self) -> CharRange {
        CharRange::new(0, self.len_chars())
    }

    /// The markers, sorted by start.
    pub fn markers(&self) -> &[Marker] {
        &self.markers
    }

    /// Next/previous lookups over the markers, backed by per-kind index
    /// tables built on first use.
    pub fn marker_index(&self) -> MarkerIndex<'_> {
        let tables = self
            .tables
            .get_or_init(|| MarkerTables::build(&self.markers));
        MarkerIndex::with_tables(&self.markers, tables)
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

    /// Number of lines. A final line break does not start another line, and
    /// an empty document has one (empty) line.
    pub fn line_count(&self) -> usize {
        let n = self.text.len_lines();
        let len = self.len_chars();
        if len > 0 && is_line_break(self.text.char(len - 1)) {
            n - 1
        } else {
            n
        }
    }

    /// The 0-based line containing `pos` (clamped to the last line).
    pub fn line_of(&self, pos: CharPos) -> usize {
        let pos = pos.0.min(self.len_chars());
        self.text
            .char_to_line(pos)
            .min(self.line_count().saturating_sub(1))
    }

    /// The chars of line `line` (0-based), without its line break. Lines past
    /// the end give an empty range at the end of the document.
    pub fn line_range(&self, line: usize) -> CharRange {
        if line >= self.line_count() {
            return CharRange::empty(self.len_chars());
        }
        let start = self.text.line_to_char(line);
        let mut end = if line + 1 < self.text.len_lines() {
            self.text.line_to_char(line + 1)
        } else {
            self.len_chars()
        };
        if end > start && is_line_break(self.text.char(end - 1)) {
            end -= 1;
            if end > start && self.text.char(end) == '\n' && self.text.char(end - 1) == '\r' {
                end -= 1;
            }
        }
        CharRange::new(start, end)
    }

    /// Line and column (both 0-based, column in chars) of `pos`.
    pub fn line_col(&self, pos: CharPos) -> (usize, usize) {
        let line = self.line_of(pos);
        let start = self.text.line_to_char(line);
        (line, pos.0.min(self.len_chars()).saturating_sub(start))
    }

    /// True when line `line` holds only whitespace (or nothing).
    pub fn line_is_blank(&self, line: usize) -> bool {
        if line >= self.line_count() {
            return true;
        }
        self.blank_lines().binary_search(&line).is_ok()
    }

    /// The sorted numbers of the blank (whitespace-only) lines, built on
    /// first use, so paragraph boundaries are a binary search away even in a
    /// multi-megabyte paragraph.
    pub fn blank_lines(&self) -> &[usize] {
        self.blank_lines.get_or_init(|| {
            let n = self.line_count();
            self.text
                .lines()
                .take(n)
                .enumerate()
                .filter(|(_, l)| l.chars().all(char::is_whitespace))
                .map(|(i, _)| i)
                .collect()
        })
    }

    /// The paragraph (run of non-blank lines) containing line `line`, as its
    /// first and last line; `None` for a blank line.
    pub fn paragraph_lines(&self, line: usize) -> Option<(usize, usize)> {
        let n = self.line_count();
        let blanks = self.blank_lines();
        match blanks.binary_search(&line) {
            Ok(_) => None,
            Err(_) if line >= n => None,
            Err(i) => {
                let first = if i == 0 { 0 } else { blanks[i - 1] + 1 };
                let last = blanks.get(i).map_or(n - 1, |&b| b - 1);
                Some((first, last))
            }
        }
    }

    /// Applies an edit to the text, shifts the markers (keeping them sorted),
    /// and drops the display index and marker tables.
    pub fn apply(&mut self, edit: &Edit) -> Result<EditOutcome, CoreError> {
        let (outcome, _inverse) = edit.apply_to_rope(&mut self.text)?;
        for m in &mut self.markers {
            m.shift(&outcome);
        }
        // Shifting is monotone, but collapsed ranges can reorder ties.
        self.markers.sort_by_key(Marker::sort_key);
        self.display = OnceCell::new();
        self.tables = OnceCell::new();
        self.blank_lines = OnceCell::new();
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use textweaver_core::MarkerKind;

    use super::*;

    #[test]
    fn plain_text_normalizes_newlines() {
        let d = Document::from_plain_text("a\r\nb\rc");
        assert_eq!(d.text().to_string(), "a\nb\nc");
    }

    #[test]
    fn blank_line_index_and_paragraphs() {
        let mut d = Document::from_plain_text("a\nb\n  \nc\n\nd\n");
        assert_eq!(d.blank_lines(), &[2, 4]);
        assert_eq!(d.paragraph_lines(1), Some((0, 1)));
        assert_eq!(d.paragraph_lines(2), None);
        assert_eq!(d.paragraph_lines(3), Some((3, 3)));
        assert_eq!(d.paragraph_lines(5), Some((5, 5)));
        assert_eq!(d.paragraph_lines(6), None);
        assert!(d.line_is_blank(4) && !d.line_is_blank(5));
        // An edit drops the index; the rebuilt one sees the new text.
        d.apply(&Edit::insert(CharPos(4), "x")).unwrap();
        assert_eq!(d.blank_lines(), &[4]);
        assert_eq!(d.paragraph_lines(0), Some((0, 3)));
    }

    #[test]
    fn display_index_counts_utf16() {
        let d = Document::from_plain_text("a😀b");
        assert_eq!(d.display().to_utf16(CharPos(2)), 3);
        assert_eq!(d.display().to_char(3), CharPos(2));
    }

    #[test]
    fn lines() {
        let d = Document::from_plain_text("ab\n\ncd\n");
        assert_eq!(d.line_count(), 3);
        assert_eq!(d.line_range(0), CharRange::new(0, 2));
        assert_eq!(d.line_range(1), CharRange::new(3, 3));
        assert_eq!(d.line_range(2), CharRange::new(4, 6));
        assert_eq!(d.line_range(3), CharRange::empty(7));
        assert!(d.line_is_blank(1));
        assert_eq!(d.line_of(CharPos(7)), 2);
        assert_eq!(d.line_col(CharPos(5)), (2, 1));
        assert_eq!(Document::from_plain_text("").line_count(), 1);
    }

    #[test]
    fn apply_shifts_and_resorts_markers() {
        let text = "Title\n\nBody text.";
        let mut d = Document::new(
            DocumentMeta::default(),
            Rope::from_str(text),
            vec![
                Marker::new(MarkerKind::Paragraph, CharRange::new(7, 17)),
                Marker::new(MarkerKind::Heading, CharRange::new(0, 5)).with_level(1),
            ],
        );
        assert_eq!(d.markers()[0].kind, MarkerKind::Heading);
        assert_eq!(d.marker_index().count(MarkerKind::Heading, None), 1);
        // Text inserted at a range edge stays outside it.
        d.apply(&Edit::insert(0, "My ")).unwrap();
        assert_eq!(d.markers()[0].range, CharRange::new(3, 8));
        assert_eq!(d.markers()[1].range, CharRange::new(10, 20));
        d.apply(&Edit::delete(0..10)).unwrap();
        // The heading collapses; the paragraph (now longer) sorts first.
        assert_eq!(d.markers()[0].kind, MarkerKind::Paragraph);
        assert_eq!(d.markers()[1].range, CharRange::empty(0));
        assert_eq!(d.marker_index().count(MarkerKind::Paragraph, None), 1);
    }

    #[test]
    fn serde_round_trip() {
        let d = Document::new(
            DocumentMeta {
                title: Some("T".into()),
                ..DocumentMeta::default()
            },
            Rope::from_str("T\n\nx"),
            vec![Marker::new(MarkerKind::Heading, CharRange::new(0, 1)).with_level(1)],
        );
        let json = serde_json::to_string(&d).unwrap();
        let back: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(back, d);
    }
}

#[cfg(test)]
mod props {
    use proptest::prelude::*;
    use textweaver_core::MarkerKind;

    use super::*;

    fn marker(len: usize) -> impl Strategy<Value = Marker> {
        (0usize..=len, 0usize..=len, 0usize..MarkerKind::ALL.len())
            .prop_map(|(a, b, k)| Marker::new(MarkerKind::ALL[k], CharRange::new(a, b)))
    }

    proptest! {
        #[test]
        fn marker_shifting_keeps_ranges_ordered_and_in_bounds(
            text in "[a-z \n]{0,40}",
            markers in proptest::collection::vec(marker(40), 0..12),
            edits in proptest::collection::vec((0usize..50, 0usize..50, "[a-z\n]{0,6}"), 1..6),
        ) {
            let mut d = Document::new(DocumentMeta::default(), Rope::from_str(&text), markers);
            for (a, b, ins) in edits {
                let len = d.len_chars();
                let edit = Edit::replace(CharRange::new(a.min(len), b.min(len)), ins);
                let before: Vec<Marker> = d.markers().to_vec();
                let outcome = d.apply(&edit).unwrap();
                let len = d.len_chars();
                for m in d.markers() {
                    prop_assert!(m.range.start <= m.range.end);
                    prop_assert!(m.range.end.0 <= len);
                }
                for w in d.markers().windows(2) {
                    prop_assert!(w[0].sort_key() <= w[1].sort_key());
                }
                // Every marker moved exactly as the edit outcome says.
                let mut expect: Vec<CharRange> =
                    before.iter().map(|m| outcome.map_range(m.range)).collect();
                let mut got: Vec<CharRange> = d.markers().iter().map(|m| m.range).collect();
                expect.sort_by_key(|r| (r.start, r.end));
                got.sort_by_key(|r| (r.start, r.end));
                prop_assert_eq!(got, expect);
                // The index tables agree with a scan after the edit.
                for kind in MarkerKind::ALL {
                    let scan = d.markers().iter().filter(|m| m.kind == kind).count();
                    prop_assert_eq!(d.marker_index().count(kind, None), scan);
                }
            }
        }
    }
}
