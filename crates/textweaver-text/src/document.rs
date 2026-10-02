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

/// Char positions to and from the offsets GUI toolkits use: UTF-16 code
/// units (Windows and macOS text controls, ADR-0002) and UTF-8 bytes
/// (Parley and AccessKit).
///
/// Since Wave 3 the index is the document's rope itself: ropey keeps UTF-16
/// and byte counts in its tree, so every lookup is `O(log n)` and building
/// the index is a rope clone (shared, no copy). The earlier index was a
/// `Vec<u32>` of every char's UTF-16 offset: 40 MB and a full scan for a
/// 10-million-character document.
#[derive(Clone, Debug, Default)]
pub struct DisplayIndex {
    text: Rope,
}

impl DisplayIndex {
    /// The index for `text` (a rope clone: no copy, no scan).
    pub fn build(text: &Rope) -> Self {
        DisplayIndex { text: text.clone() }
    }

    /// UTF-16 offset of `pos` (clamped to the end).
    pub fn to_utf16(&self, pos: CharPos) -> u32 {
        let pos = pos.0.min(self.text.len_chars());
        u32::try_from(self.text.char_to_utf16_cu(pos)).unwrap_or(u32::MAX)
    }

    /// Char position of a UTF-16 offset: rounded down to a char start (an
    /// offset inside a surrogate pair maps to its char), clamped to the end.
    pub fn to_char(&self, utf16: u32) -> CharPos {
        let end = self.text.len_utf16_cu();
        let u = usize::try_from(utf16).map_or(end, |u| u.min(end));
        CharPos(self.text.utf16_cu_to_char(u))
    }

    /// UTF-8 byte offset of `pos` (clamped to the end).
    pub fn to_byte(&self, pos: CharPos) -> usize {
        self.text.char_to_byte(pos.0.min(self.text.len_chars()))
    }

    /// Char position of a UTF-8 byte offset: rounded down to a char start,
    /// clamped to the end.
    pub fn byte_to_char(&self, byte: usize) -> CharPos {
        CharPos(self.text.byte_to_char(byte.min(self.text.len_bytes())))
    }

    /// The text's length in UTF-16 code units.
    pub fn len_utf16(&self) -> usize {
        self.text.len_utf16_cu()
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
    /// The text ends with a line break (which starts no line of its own).
    /// Kept up to date by `new` and `apply`, so counting lines, which every
    /// line lookup does, costs no rope lookup.
    ends_with_break: bool,
}

/// True when `text` ends with a line break.
fn ends_with_break(text: &Rope) -> bool {
    let len = text.len_chars();
    len > 0 && is_line_break(text.char(len - 1))
}

/// Shifts sorted markers by an edit and keeps them sorted, touching only
/// what the edit can change.
///
/// - Markers that start after the removed range move by the edit's delta,
///   start and end alike: a plain addition, and their order is kept.
/// - Markers that end before it do not move.
/// - The rest (markers that reach into the edit, a handful) are mapped by
///   [`EditOutcome::map_range`].
///
/// Two neighbors can only end up out of order when one of them was
/// mapped (ranges collapsed into the edit; an empty range there moves to
/// the edit's start, a range starting there to its end). So only a mapped
/// marker's neighbors are compared. When some are out of order, the slice
/// from the first disorder to the last is widened to every marker starting
/// within the slice's starts and sorted, which gives exactly what a full
/// stable sort gives: equal keys share a start, so they are all in the
/// slice and keep their order.
fn shift_markers(markers: &mut [Marker], outcome: &EditOutcome) {
    let removed = outcome.removed;
    // Sorted by start: everything from `after` on starts past the edit.
    let after = markers.partition_point(|m| m.range.start <= removed.end);
    let (head, tail) = markers.split_at_mut(after);
    let (from, to) = (removed.end.0, outcome.inserted.end.0);
    for m in tail {
        m.range.start = CharPos(m.range.start.0 - from + to);
        m.range.end = CharPos(m.range.end.0 - from + to);
    }
    let mut first: Option<usize> = None;
    let mut last = 0;
    let mut mapped = Vec::new();
    for (i, m) in head.iter_mut().enumerate() {
        if m.range.end >= removed.start {
            m.shift(outcome);
            mapped.push(i);
        }
    }
    let out_of_order = |a: &Marker, b: &Marker| a.sort_key() > b.sort_key();
    for &i in &mapped {
        if i > 0 && out_of_order(&markers[i - 1], &markers[i]) {
            first = Some(first.map_or(i - 1, |f| f.min(i - 1)));
            last = last.max(i - 1);
        }
        if i + 1 < markers.len() && out_of_order(&markers[i], &markers[i + 1]) {
            first = Some(first.map_or(i, |f| f.min(i)));
            last = last.max(i);
        }
    }
    let Some(first) = first else {
        return;
    };
    // Widen the slice until everything before it starts before all of it,
    // and everything after it starts after all of it.
    let (mut lo, mut hi) = (first, last + 2);
    let low = markers[lo..hi].iter().map(|m| m.range.start).min();
    let high = markers[lo..hi].iter().map(|m| m.range.start).max();
    let (Some(low), Some(high)) = (low, high) else {
        return;
    };
    while lo > 0 && markers[lo - 1].range.start >= low {
        lo -= 1;
    }
    while hi < markers.len() && markers[hi].range.start <= high {
        hi += 1;
    }
    markers[lo..hi].sort_by_key(Marker::sort_key);
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
        // Loaders that already emit sorted markers skip the sort.
        if !markers.is_sorted_by_key(Marker::sort_key) {
            markers.sort_by_key(Marker::sort_key);
        }
        Document {
            meta,
            ends_with_break: ends_with_break(&text),
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
        self.text.len_lines() - usize::from(self.ends_with_break)
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
        let next = if line + 1 < self.text.len_lines() {
            self.text.line_to_char(line + 1)
        } else {
            self.len_chars()
        };
        CharRange::new(start, self.content_end(start, next))
    }

    /// `line_range(line).start`, at one rope lookup instead of two.
    pub(crate) fn line_start(&self, line: usize) -> CharPos {
        if line >= self.line_count() {
            return CharPos(self.len_chars());
        }
        CharPos(self.text.line_to_char(line))
    }

    /// The end of a line's chars without its break, for the line starting
    /// at `start` whose next line starts at `next` (or the text ends there).
    fn content_end(&self, start: usize, next: usize) -> usize {
        let mut end = next;
        if end > start && is_line_break(self.text.char(end - 1)) {
            end -= 1;
            if end > start && self.text.char(end) == '\n' && self.text.char(end - 1) == '\r' {
                end -= 1;
            }
        }
        end
    }

    /// The line break positions (the end of each line's chars) of lines
    /// `first..last`, with the start of the line after each: what
    /// [`line_range`](Self::line_range) gives for line `l` and `l + 1`, at
    /// one rope lookup per line instead of four. `last` must be a line of
    /// the document.
    pub(crate) fn line_breaks(
        &self,
        first: usize,
        last: usize,
    ) -> impl Iterator<Item = (CharPos, CharPos)> + '_ {
        let mut start = if first < last {
            self.text.line_to_char(first)
        } else {
            0
        };
        (first..last).map(move |line| {
            let next = self.text.line_to_char(line + 1);
            let brk = self.content_end(start, next);
            start = next;
            (CharPos(brk), CharPos(next))
        })
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
        self.blank_lines.get_or_init(|| self.all_blank_lines())
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
    /// updates the blank-line table if it was built, and drops the display
    /// index and marker tables.
    pub fn apply(&mut self, edit: &Edit) -> Result<EditOutcome, CoreError> {
        let old_lines = self.text.len_lines();
        let (outcome, _inverse) = edit.apply_to_rope(&mut self.text)?;
        shift_markers(&mut self.markers, &outcome);
        self.display = OnceCell::new();
        self.tables = OnceCell::new();
        self.ends_with_break = ends_with_break(&self.text);
        if let Some(blanks) = self.blank_lines.take() {
            let blanks = self.update_blank_lines(blanks, &outcome, old_lines);
            self.blank_lines = OnceCell::from(blanks);
        }
        Ok(outcome)
    }

    /// The blank-line table after an edit, from the one before it: only the
    /// lines the edit reached are looked at again (typing on 10 MB cost a
    /// full rebuild, 18 ms, at every keystroke that needed the table).
    ///
    /// The text before the edit's start is unchanged, and so is the text
    /// from just past its end, so every line before the one holding the
    /// start (less one, for a `\r` the edit joins to a `\n`) keeps its
    /// number, and every line past the one holding the char after the edit
    /// moves by the change in the line count.
    fn update_blank_lines(
        &self,
        mut blanks: Vec<usize>,
        outcome: &EditOutcome,
        old_lines: usize,
    ) -> Vec<usize> {
        let len = self.text.len_chars();
        let new_lines = self.text.len_lines();
        let first = self
            .text
            .char_to_line(outcome.inserted.start.0.min(len))
            .saturating_sub(1);
        let new_last = self
            .text
            .char_to_line(outcome.inserted.end.0.saturating_add(1).min(len));
        // Lines past `new_last` are the old lines past `old_last`.
        let Some(old_last) = (new_last + old_lines).checked_sub(new_lines) else {
            return self.all_blank_lines();
        };
        if old_last < first {
            return self.all_blank_lines();
        }
        let lo = blanks.partition_point(|&l| l < first);
        let hi = blanks.partition_point(|&l| l <= old_last);
        let count = self.line_count();
        let fresh: Vec<usize> = (first..=new_last.min(count.saturating_sub(1)))
            .filter(|&l| l < count && self.line_text_is_blank(l))
            .collect();
        let tail = blanks.split_off(hi);
        blanks.truncate(lo);
        blanks.extend(fresh);
        // Every `l` here is past `old_last`.
        blanks.extend(
            tail.into_iter()
                .map(|l| l - old_last + new_last)
                .filter(|&l| l < count),
        );
        blanks
    }

    /// Every blank line, by scanning the whole text.
    fn all_blank_lines(&self) -> Vec<usize> {
        let n = self.line_count();
        self.text
            .lines()
            .take(n)
            .enumerate()
            .filter(|(_, l)| l.chars().all(char::is_whitespace))
            .map(|(i, _)| i)
            .collect()
    }

    /// True when line `line` of the rope holds only whitespace.
    fn line_text_is_blank(&self, line: usize) -> bool {
        self.text.line(line).chars().all(char::is_whitespace)
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
        // Inside the surrogate pair: the emoji itself.
        assert_eq!(d.display().to_char(2), CharPos(1));
        // Past the end: the end.
        assert_eq!(d.display().to_char(99), CharPos(3));
        assert_eq!(d.display().to_utf16(CharPos(99)), 4);
        assert_eq!(d.display().len_utf16(), 4);
        // UTF-8: the emoji is four bytes.
        assert_eq!(d.display().to_byte(CharPos(2)), 5);
        assert_eq!(d.display().byte_to_char(3), CharPos(1));
        assert_eq!(d.display().byte_to_char(99), CharPos(3));
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

        /// The blank-line table an edit updates is the table a full scan
        /// of the edited text builds, for every kind of line break and
        /// whitespace, edits that join or split `\r\n`, and edits at the
        /// ends.
        #[test]
        fn updated_blank_lines_match_a_rebuild(
            text in "[a \\t\\n\\r\\u{2028}\\u{85}\\u{0B}]{0,40}",
            edits in proptest::collection::vec(
                (0usize..45, 0usize..45, "[a \\n\\r\\u{2029}]{0,5}"),
                1..8,
            ),
        ) {
            let mut d = Document::new(DocumentMeta::default(), Rope::from_str(&text), Vec::new());
            for (a, b, ins) in edits {
                let _ = d.blank_lines();
                let len = d.len_chars();
                let edit = Edit::replace(CharRange::new(a.min(len), b.min(len)), ins);
                d.apply(&edit).unwrap();
                let fresh = Document::new(DocumentMeta::default(), d.text().clone(), Vec::new());
                prop_assert_eq!(d.blank_lines(), fresh.blank_lines());
            }
        }

        /// Shifting only the markers an edit reaches, and sorting only
        /// where it reordered them, gives exactly what mapping every marker
        /// and a full stable sort give, ties (same range and kind, other
        /// levels) included.
        #[test]
        fn partial_shift_and_sort_match_the_full_ones(
            text in "[a-z \n]{0,40}",
            markers in proptest::collection::vec(
                (0usize..=40, 0usize..=40, 0usize..4, 0u8..3)
                    .prop_map(|(a, b, k, l)| {
                        Marker::new(MarkerKind::ALL[k], CharRange::new(a, b)).with_level(l)
                    }),
                0..16,
            ),
            edits in proptest::collection::vec((0usize..50, 0usize..50, "[a-z\n]{0,6}"), 1..6),
        ) {
            let mut d = Document::new(DocumentMeta::default(), Rope::from_str(&text), markers);
            for (a, b, ins) in edits {
                let len = d.len_chars();
                let edit = Edit::replace(CharRange::new(a.min(len), b.min(len)), ins);
                let mut expect: Vec<Marker> = d.markers().to_vec();
                let outcome = d.apply(&edit).unwrap();
                for m in &mut expect {
                    m.shift(&outcome);
                }
                expect.sort_by_key(Marker::sort_key);
                prop_assert_eq!(d.markers(), expect.as_slice());
            }
        }

        /// The line count kept by `new` and `apply`, and the quicker line
        /// lookups the segmenter uses, agree with the rope and with
        /// `line_range`, for every kind of line break, before and after
        /// edits.
        #[test]
        fn line_lookups_agree_with_line_range(
            text in "[ab\\n\\r\\u{2028}\\u{85} ]{0,40}",
            edits in proptest::collection::vec((0usize..45, 0usize..45, "[a\\n\\r\\u{2029}]{0,4}"), 0..4),
        ) {
            let mut d = Document::new(DocumentMeta::default(), Rope::from_str(&text), Vec::new());
            for step in 0..=edits.len() {
                let rope = d.text().clone();
                let len = rope.len_chars();
                let ends = len > 0 && is_line_break(rope.char(len - 1));
                prop_assert_eq!(d.line_count(), rope.len_lines() - usize::from(ends));
                let n = d.line_count();
                for line in 0..n + 2 {
                    prop_assert_eq!(d.line_start(line), d.line_range(line).start);
                }
                for first in 0..n {
                    for last in first..n {
                        let got: Vec<(CharPos, CharPos)> = d.line_breaks(first, last).collect();
                        let want: Vec<(CharPos, CharPos)> = (first..last)
                            .map(|l| (d.line_range(l).end, d.line_range(l + 1).start))
                            .collect();
                        prop_assert_eq!(got, want);
                    }
                }
                if let Some((a, b, ins)) = edits.get(step) {
                    let len = d.len_chars();
                    let edit = Edit::replace(CharRange::new((*a).min(len), (*b).min(len)), ins.clone());
                    d.apply(&edit).unwrap();
                }
            }
        }
    }
}
