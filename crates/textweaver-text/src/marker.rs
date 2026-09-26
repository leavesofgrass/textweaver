//! Markers and marker navigation.
//!
//! A [`Marker`] annotates a range of canonical text with structure (heading,
//! list item, table row, link, ...). A [`MarkerIndex`] answers "next heading",
//! "previous table", "which list am I in" over a document's sorted markers.
//! With [`MarkerTables`] (per-kind index tables, Paperback's `marker.rs`
//! approach) every lookup is a binary search over the markers of one kind;
//! without them it falls back to scanning the sorted slice.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange, Direction, EditOutcome, MarkerKind};

/// Label carried by the header row of a table (`MarkerKind::TableRow`).
pub const HEADER_ROW_LABEL: &str = "header";

/// A structural or inline annotation on a range of canonical text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marker {
    /// What the range is.
    pub kind: MarkerKind,
    /// The annotated chars.
    pub range: CharRange,
    /// Meaning depends on the kind; 0 when not applicable:
    ///
    /// - `Heading`: the heading level, 1 to 6.
    /// - `List`, `ListItem`: the nesting depth, from 1.
    /// - `Code`: 1 for a code block, 0 for an inline code span.
    /// - `Footnote`: 1 for a footnote body, 0 for a reference to one.
    pub level: u8,
    /// Display label, when it differs from the text: the number of an
    /// ordered list item (`"2."`), a table caption, [`HEADER_ROW_LABEL`] on a
    /// table's header row, a code block's language.
    pub label: Option<String>,
    /// Link target, footnote id, image source.
    pub reference: Option<String>,
}

impl Marker {
    /// A marker with no level, label, or reference.
    pub fn new(kind: MarkerKind, range: CharRange) -> Self {
        Marker {
            kind,
            range,
            level: 0,
            label: None,
            reference: None,
        }
    }

    /// Sets the level (builder style).
    pub fn with_level(mut self, level: u8) -> Self {
        self.level = level;
        self
    }

    /// Sets the label (builder style).
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the reference (builder style).
    pub fn with_reference(mut self, reference: impl Into<String>) -> Self {
        self.reference = Some(reference.into());
        self
    }

    /// Moves the marker across an edit (ranges do not grow at their edges).
    pub fn shift(&mut self, outcome: &EditOutcome) {
        self.range = outcome.map_range(self.range);
    }

    /// True for markers that delimit blocks of text (a heading, a list item,
    /// a table row, a code block...) rather than inline spans.
    pub fn is_block(&self) -> bool {
        match self.kind {
            MarkerKind::Heading
            | MarkerKind::Paragraph
            | MarkerKind::ListItem
            | MarkerKind::List
            | MarkerKind::Table
            | MarkerKind::TableRow
            | MarkerKind::Quote => true,
            MarkerKind::Code | MarkerKind::Footnote => self.level == 1,
            MarkerKind::TableCell
            | MarkerKind::Link
            | MarkerKind::Image
            | MarkerKind::PageBreak
            | MarkerKind::SectionBreak
            | MarkerKind::Bold
            | MarkerKind::Italic
            | MarkerKind::Underline
            | MarkerKind::Strikethrough
            | MarkerKind::Rule
            | MarkerKind::Math => false,
        }
    }

    /// True for the header row of a table.
    pub fn is_header_row(&self) -> bool {
        self.kind == MarkerKind::TableRow && self.label.as_deref() == Some(HEADER_ROW_LABEL)
    }

    /// Sort key: by start, longer (enclosing) ranges first, then containers
    /// before their contents. Documents keep their markers in this order.
    pub fn sort_key(&self) -> (CharPos, std::cmp::Reverse<CharPos>, u8, MarkerKind) {
        (
            self.range.start,
            std::cmp::Reverse(self.range.end),
            nesting_rank(self.kind),
            self.kind,
        )
    }
}

/// Containers sort before their contents when ranges tie.
fn nesting_rank(kind: MarkerKind) -> u8 {
    match kind {
        MarkerKind::SectionBreak | MarkerKind::PageBreak | MarkerKind::Rule => 0,
        MarkerKind::Quote => 1,
        MarkerKind::List | MarkerKind::Table => 2,
        MarkerKind::ListItem | MarkerKind::TableRow => 3,
        MarkerKind::TableCell => 4,
        MarkerKind::Heading | MarkerKind::Paragraph | MarkerKind::Code | MarkerKind::Footnote => 5,
        MarkerKind::Link | MarkerKind::Image | MarkerKind::Math => 6,
        MarkerKind::Bold
        | MarkerKind::Italic
        | MarkerKind::Underline
        | MarkerKind::Strikethrough => 7,
    }
}

fn slot(kind: MarkerKind) -> usize {
    // `MarkerKind` is a field-less enum in declaration order, which is also
    // the order of `MarkerKind::ALL`.
    kind as usize
}

fn to_u32(i: usize) -> u32 {
    u32::try_from(i).unwrap_or(u32::MAX)
}

/// Per-kind index tables over a sorted marker slice: for every kind, the
/// indices of its markers in order of start, and how far they reach (the
/// greatest end among the first n). Built once per document (see
/// `Document::marker_index`) and rebuilt after edits.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MarkerTables {
    by_kind: Vec<Vec<u32>>,
    /// For every kind, `reach[j]` is the greatest `range.end` among its
    /// markers `0..=j`: [`MarkerIndex::enclosing`] stops scanning back once
    /// no earlier marker can still contain the position.
    reach: Vec<Vec<usize>>,
}

impl MarkerTables {
    /// Builds the tables for markers sorted by start.
    pub fn build(markers: &[Marker]) -> Self {
        let mut by_kind = vec![Vec::new(); MarkerKind::ALL.len()];
        let mut reach: Vec<Vec<usize>> = vec![Vec::new(); MarkerKind::ALL.len()];
        for (i, m) in markers.iter().enumerate() {
            let k = slot(m.kind);
            by_kind[k].push(to_u32(i));
            let far = reach[k].last().copied().unwrap_or(0).max(m.range.end.0);
            reach[k].push(far);
        }
        MarkerTables { by_kind, reach }
    }

    /// Indices (into the marker slice) of the markers of `kind`.
    pub fn indices(&self, kind: MarkerKind) -> &[u32] {
        self.by_kind.get(slot(kind)).map_or(&[], Vec::as_slice)
    }

    fn reach(&self, kind: MarkerKind) -> &[usize] {
        self.reach.get(slot(kind)).map_or(&[], Vec::as_slice)
    }
}

/// Next and previous lookups over a document's markers.
///
/// Markers must be sorted by start (documents keep them sorted). Every
/// "next" or "previous" is relative to marker **starts**: forward finds the
/// first marker starting after a position, backward the last one starting
/// before it. A level filter restricts headings to one level or list items
/// to one depth.
#[derive(Clone, Copy, Debug)]
pub struct MarkerIndex<'a> {
    markers: &'a [Marker],
    tables: Option<&'a MarkerTables>,
}

impl<'a> MarkerIndex<'a> {
    /// An index over markers sorted by start, without index tables (every
    /// lookup scans the slice once). Prefer `Document::marker_index`.
    pub fn new(markers: &'a [Marker]) -> Self {
        MarkerIndex {
            markers,
            tables: None,
        }
    }

    /// An index using prebuilt per-kind tables for `markers`.
    pub fn with_tables(markers: &'a [Marker], tables: &'a MarkerTables) -> Self {
        MarkerIndex {
            markers,
            tables: Some(tables),
        }
    }

    /// The markers this index covers.
    pub fn markers(&self) -> &'a [Marker] {
        self.markers
    }

    fn ids(&self, kind: MarkerKind) -> Cow<'a, [u32]> {
        match self.tables {
            Some(t) => Cow::Borrowed(t.indices(kind)),
            None => Cow::Owned(
                self.markers
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| m.kind == kind)
                    .map(|(i, _)| to_u32(i))
                    .collect(),
            ),
        }
    }

    fn get(&self, id: u32) -> &'a Marker {
        &self.markers[id as usize]
    }

    fn matches(m: &Marker, level: Option<u8>) -> bool {
        level.is_none_or(|l| m.level == l)
    }

    /// Every marker of `kind` (and `level`, when set), in document order.
    pub fn iter(&self, kind: MarkerKind, level: Option<u8>) -> impl Iterator<Item = &'a Marker> {
        let ids = self.ids(kind);
        let markers = self.markers;
        (0..ids.len())
            .map(move |i| &markers[ids[i] as usize])
            .filter(move |m| Self::matches(m, level))
    }

    /// Number of markers of `kind` (and `level`).
    pub fn count(&self, kind: MarkerKind, level: Option<u8>) -> usize {
        self.iter(kind, level).count()
    }

    /// The `n`th marker (0-based) of `kind` (and `level`).
    pub fn nth(&self, kind: MarkerKind, level: Option<u8>, n: usize) -> Option<&'a Marker> {
        self.iter(kind, level).nth(n)
    }

    /// The first matching marker strictly after (forward) or before
    /// (backward) `from`, optionally wrapping around the document. The bool
    /// is true when the search wrapped.
    pub fn step(
        &self,
        kind: MarkerKind,
        level: Option<u8>,
        from: CharPos,
        dir: Direction,
        wrap: bool,
    ) -> Option<(&'a Marker, bool)> {
        let ids = self.ids(kind);
        let find = |range: &[u32], rev: bool| -> Option<&'a Marker> {
            let mut it = range.iter().map(|&i| self.get(i));
            if rev {
                it.rev().find(|m| Self::matches(m, level))
            } else {
                it.find(|m| Self::matches(m, level))
            }
        };
        match dir {
            Direction::Forward => {
                let split = ids.partition_point(|&i| self.get(i).range.start <= from);
                if let Some(m) = find(&ids[split..], false) {
                    return Some((m, false));
                }
                if wrap {
                    return find(&ids[..split], false).map(|m| (m, true));
                }
                None
            }
            Direction::Backward => {
                let split = ids.partition_point(|&i| self.get(i).range.start < from);
                if let Some(m) = find(&ids[..split], true) {
                    return Some((m, false));
                }
                if wrap {
                    return find(&ids[split..], true).map(|m| (m, true));
                }
                None
            }
        }
    }

    /// The innermost marker of `kind` containing `pos` (the shortest; the
    /// first in document order among equals).
    ///
    /// With index tables the scan runs back from `pos` only while an
    /// earlier marker can still reach it, so a lookup in a document with
    /// thousands of table rows or code blocks costs a few steps, not one
    /// per marker before `pos` (narration looks up every sentence).
    pub fn enclosing(&self, kind: MarkerKind, pos: CharPos) -> Option<&'a Marker> {
        let ids = self.ids(kind);
        let split = ids.partition_point(|&i| self.get(i).range.start <= pos);
        let reach = self.tables.map(|t| t.reach(kind));
        let mut best: Option<&'a Marker> = None;
        for j in (0..split).rev() {
            if let Some(r) = reach
                && r.get(j).is_some_and(|&far| far <= pos.0)
            {
                break;
            }
            let m = self.get(ids[j]);
            if m.range.contains(pos) && best.is_none_or(|b| m.range.len() <= b.range.len()) {
                best = Some(m);
            }
        }
        best
    }

    /// Every marker (of any kind) whose range contains `pos`, outermost first.
    pub fn containing(&self, pos: CharPos) -> impl Iterator<Item = &'a Marker> {
        let split = self.markers.partition_point(|m| m.range.start <= pos);
        self.markers[..split]
            .iter()
            .filter(move |m| m.range.contains(pos))
    }

    /// Markers (of any kind) starting exactly at `pos`, outermost first.
    pub fn starting_at(&self, pos: CharPos) -> &'a [Marker] {
        let a = self.markers.partition_point(|m| m.range.start < pos);
        let b = self.markers.partition_point(|m| m.range.start <= pos);
        &self.markers[a..b]
    }

    /// Markers (of any kind) starting inside `range`, in order.
    pub fn starting_in(&self, range: CharRange) -> &'a [Marker] {
        let a = self
            .markers
            .partition_point(|m| m.range.start < range.start);
        let b = self.markers.partition_point(|m| m.range.start < range.end);
        &self.markers[a..b.max(a)]
    }

    /// The 1-based ordinal of the last matching marker starting at or before
    /// `pos`, and the total number of matching markers ("heading 3 of 12").
    pub fn ordinal(
        &self,
        kind: MarkerKind,
        level: Option<u8>,
        pos: CharPos,
    ) -> Option<(usize, usize)> {
        let mut total = 0;
        let mut at = 0;
        for m in self.iter(kind, level) {
            total += 1;
            if m.range.start <= pos {
                at = total;
            }
        }
        (at > 0).then_some((at, total))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heading(start: usize, end: usize, level: u8) -> Marker {
        Marker::new(MarkerKind::Heading, CharRange::new(start, end)).with_level(level)
    }

    fn sample() -> Vec<Marker> {
        let mut v = vec![
            heading(0, 5, 1),
            Marker::new(MarkerKind::Paragraph, CharRange::new(7, 20)),
            heading(22, 30, 2),
            Marker::new(MarkerKind::Table, CharRange::new(32, 60)),
            heading(62, 70, 2),
            heading(72, 80, 3),
        ];
        v.sort_by_key(Marker::sort_key);
        v
    }

    #[test]
    fn step_with_and_without_tables_agree() {
        let markers = sample();
        let tables = MarkerTables::build(&markers);
        for index in [
            MarkerIndex::new(&markers),
            MarkerIndex::with_tables(&markers, &tables),
        ] {
            let next = index.step(
                MarkerKind::Heading,
                None,
                CharPos(0),
                Direction::Forward,
                false,
            );
            assert_eq!(
                next.map(|(m, w)| (m.range.start, w)),
                Some((CharPos(22), false))
            );
            let lvl2 = index.step(
                MarkerKind::Heading,
                Some(2),
                CharPos(30),
                Direction::Forward,
                false,
            );
            assert_eq!(lvl2.map(|(m, _)| m.range.start), Some(CharPos(62)));
            let none = index.step(
                MarkerKind::Heading,
                Some(3),
                CharPos(72),
                Direction::Forward,
                false,
            );
            assert!(none.is_none());
            let wrapped = index.step(
                MarkerKind::Heading,
                Some(3),
                CharPos(72),
                Direction::Forward,
                true,
            );
            assert_eq!(
                wrapped.map(|(m, w)| (m.range.start, w)),
                Some((CharPos(72), true))
            );
            let back = index.step(
                MarkerKind::Heading,
                None,
                CharPos(62),
                Direction::Backward,
                false,
            );
            assert_eq!(back.map(|(m, _)| m.range.start), Some(CharPos(22)));
            let back_wrap = index.step(
                MarkerKind::Heading,
                Some(2),
                CharPos(10),
                Direction::Backward,
                true,
            );
            assert_eq!(
                back_wrap.map(|(m, w)| (m.range.start, w)),
                Some((CharPos(62), true))
            );
            assert_eq!(index.count(MarkerKind::Heading, None), 4);
            assert_eq!(
                index
                    .nth(MarkerKind::Heading, Some(2), 1)
                    .map(|m| m.range.start),
                Some(CharPos(62))
            );
            assert_eq!(
                index.ordinal(MarkerKind::Heading, None, CharPos(65)),
                Some((3, 4))
            );
        }
    }

    #[test]
    fn enclosing_picks_innermost() {
        let mut markers = vec![
            Marker::new(MarkerKind::List, CharRange::new(0, 30)).with_level(1),
            Marker::new(MarkerKind::List, CharRange::new(10, 20)).with_level(2),
            Marker::new(MarkerKind::ListItem, CharRange::new(10, 15)).with_level(2),
        ];
        markers.sort_by_key(Marker::sort_key);
        let tables = MarkerTables::build(&markers);
        let index = MarkerIndex::with_tables(&markers, &tables);
        assert_eq!(
            index
                .enclosing(MarkerKind::List, CharPos(12))
                .map(|m| m.level),
            Some(2)
        );
        assert_eq!(
            index
                .enclosing(MarkerKind::List, CharPos(25))
                .map(|m| m.level),
            Some(1)
        );
        assert_eq!(index.containing(CharPos(12)).count(), 3);
        assert_eq!(index.starting_at(CharPos(10)).len(), 2);
        assert_eq!(index.starting_in(CharRange::new(5, 11)).len(), 2);
    }

    #[test]
    fn block_kinds() {
        assert!(
            Marker::new(MarkerKind::Code, CharRange::new(0, 1))
                .with_level(1)
                .is_block()
        );
        assert!(!Marker::new(MarkerKind::Code, CharRange::new(0, 1)).is_block());
        assert!(!Marker::new(MarkerKind::Bold, CharRange::new(0, 1)).is_block());
        let row =
            Marker::new(MarkerKind::TableRow, CharRange::new(0, 1)).with_label(HEADER_ROW_LABEL);
        assert!(row.is_header_row());
    }
}

#[cfg(test)]
mod props {
    use proptest::prelude::*;

    use super::*;

    fn marker() -> impl Strategy<Value = Marker> {
        (0usize..60, 0usize..60, 0usize..3).prop_map(|(a, b, k)| {
            let kind = [MarkerKind::TableRow, MarkerKind::Code, MarkerKind::List][k];
            Marker::new(kind, CharRange::new(a, b))
        })
    }

    proptest! {
        /// The early-stopping scan finds what a scan of every marker finds,
        /// for arbitrary (overlapping, nested, empty) ranges.
        #[test]
        fn enclosing_with_tables_matches_a_full_scan(
            mut markers in proptest::collection::vec(marker(), 0..30),
            pos in 0usize..62,
        ) {
            markers.sort_by_key(Marker::sort_key);
            let tables = MarkerTables::build(&markers);
            let fast = MarkerIndex::with_tables(&markers, &tables);
            let slow = MarkerIndex::new(&markers);
            for kind in [MarkerKind::TableRow, MarkerKind::Code, MarkerKind::List] {
                let a = fast.enclosing(kind, CharPos(pos)).map(|m| m.range);
                let b = slow.enclosing(kind, CharPos(pos)).map(|m| m.range);
                prop_assert_eq!(a, b);
            }
        }
    }
}
