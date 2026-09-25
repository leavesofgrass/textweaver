//! Markers and marker navigation.

use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange, Direction, EditOutcome, MarkerKind};

/// A structural or inline annotation on a range of canonical text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marker {
    /// What the range is.
    pub kind: MarkerKind,
    /// The annotated chars.
    pub range: CharRange,
    /// Heading level (1 to 6) or list depth (from 1); 0 when not applicable.
    pub level: u8,
    /// Display label (heading text, table caption), when it differs from the text.
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

    /// Moves the marker across an edit (ranges do not grow at their edges).
    pub fn shift(&mut self, outcome: &EditOutcome) {
        self.range = outcome.map_range(self.range);
    }
}

/// Next and previous lookups over a document's markers.
///
/// Phase 0: linear scans over the sorted slice. Agent A may add per-kind
/// index tables (Paperback's `marker.rs` approach).
#[derive(Clone, Copy, Debug)]
pub struct MarkerIndex<'a> {
    markers: &'a [Marker],
}

impl<'a> MarkerIndex<'a> {
    /// An index over markers sorted by start.
    pub fn new(markers: &'a [Marker]) -> Self {
        MarkerIndex { markers }
    }

    fn matches(m: &Marker, kind: MarkerKind, level: Option<u8>) -> bool {
        m.kind == kind && level.is_none_or(|l| m.level == l)
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
        let mut it = self
            .markers
            .iter()
            .filter(|m| Self::matches(m, kind, level));
        let found = match dir {
            Direction::Forward => it.clone().find(|m| m.range.start > from),
            Direction::Backward => it.clone().rev().find(|m| m.range.start < from),
        };
        match (found, wrap) {
            (Some(m), _) => Some((m, false)),
            (None, true) => match dir {
                Direction::Forward => it.next().map(|m| (m, true)),
                Direction::Backward => it.next_back().map(|m| (m, true)),
            },
            (None, false) => None,
        }
    }

    /// The innermost marker of `kind` containing `pos`.
    pub fn enclosing(&self, kind: MarkerKind, pos: CharPos) -> Option<&'a Marker> {
        self.markers
            .iter()
            .filter(|m| m.kind == kind && m.range.contains(pos))
            .min_by_key(|m| m.range.len())
    }
}
