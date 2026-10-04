use std::fmt;
use std::ops::Range;

use serde::{Deserialize, Serialize};

/// A position in a document, counted in Unicode scalar values (Rust `char`s)
/// from the start of the canonical text.
///
/// This is the canonical, persisted position type. It is compatible with the
/// character offsets star stores (Python `str` indices are code points too).
/// Byte offsets and UTF-16 display units are derived from it, never persisted.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct CharPos(pub usize);

impl CharPos {
    /// The start of the document.
    pub const ZERO: CharPos = CharPos(0);

    /// Creates a position from a char index.
    pub const fn new(index: usize) -> Self {
        CharPos(index)
    }

    /// The char index.
    pub const fn get(self) -> usize {
        self.0
    }

    /// Adds `n` chars, saturating at `usize::MAX`.
    pub const fn saturating_add(self, n: usize) -> Self {
        CharPos(self.0.saturating_add(n))
    }

    /// Subtracts `n` chars, saturating at zero.
    pub const fn saturating_sub(self, n: usize) -> Self {
        CharPos(self.0.saturating_sub(n))
    }

    /// Clamps the position to `0..=len`.
    pub fn clamp_to(self, len: usize) -> Self {
        CharPos(self.0.min(len))
    }
}

impl From<usize> for CharPos {
    fn from(value: usize) -> Self {
        CharPos(value)
    }
}

impl From<CharPos> for usize {
    fn from(value: CharPos) -> Self {
        value.0
    }
}

impl fmt::Display for CharPos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A half-open range of chars, `start..end`, with `start <= end`.
///
/// Unlike `std::ops::Range` this type is `Copy`, which keeps markers,
/// selections, and highlight ranges cheap to pass around.
///
/// Deserializing orders the endpoints as [`CharRange::new`] does, so a
/// reversed range in a hand-edited or synced state file cannot reach code
/// that slices text with it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "RangeData")]
pub struct CharRange {
    /// First char in the range.
    pub start: CharPos,
    /// One past the last char in the range.
    pub end: CharPos,
}

/// The serialized shape of a [`CharRange`], before its ends are ordered.
#[derive(Deserialize)]
struct RangeData {
    start: CharPos,
    end: CharPos,
}

impl From<RangeData> for CharRange {
    fn from(d: RangeData) -> Self {
        CharRange::new(d.start, d.end)
    }
}

impl CharRange {
    /// Creates a range. The endpoints are ordered, so `new(5, 2)` is `2..5`.
    pub fn new(a: impl Into<CharPos>, b: impl Into<CharPos>) -> Self {
        let (a, b) = (a.into(), b.into());
        if a <= b {
            CharRange { start: a, end: b }
        } else {
            CharRange { start: b, end: a }
        }
    }

    /// An empty range at `at`.
    pub fn empty(at: impl Into<CharPos>) -> Self {
        let at = at.into();
        CharRange { start: at, end: at }
    }

    /// Number of chars in the range.
    pub fn len(&self) -> usize {
        self.end.0.saturating_sub(self.start.0)
    }

    /// True when the range covers no chars.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// True when `pos` lies in `start..end`.
    pub fn contains(&self, pos: CharPos) -> bool {
        self.start <= pos && pos < self.end
    }

    /// True when `other` lies entirely within this range.
    pub fn contains_range(&self, other: CharRange) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    /// True when the two ranges share at least one char.
    pub fn intersects(&self, other: CharRange) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// The chars shared by both ranges, if any.
    pub fn intersection(&self, other: CharRange) -> Option<CharRange> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        (start < end).then_some(CharRange { start, end })
    }

    /// The smallest range covering both ranges.
    pub fn cover(&self, other: CharRange) -> CharRange {
        CharRange {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Clamps both endpoints to `0..=len`.
    pub fn clamp_to(&self, len: usize) -> CharRange {
        CharRange {
            start: self.start.clamp_to(len),
            end: self.end.clamp_to(len),
        }
    }

    /// The range as `usize` indices, for slicing APIs such as `Rope::slice`.
    pub fn to_range(&self) -> Range<usize> {
        self.start.0..self.end.0
    }
}

impl From<Range<usize>> for CharRange {
    fn from(r: Range<usize>) -> Self {
        CharRange::new(r.start, r.end)
    }
}

impl From<Range<CharPos>> for CharRange {
    fn from(r: Range<CharPos>) -> Self {
        CharRange::new(r.start, r.end)
    }
}

impl fmt::Display for CharRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// Direction of a navigation, search, or reading step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Towards the end of the document.
    #[default]
    Forward,
    /// Towards the start of the document.
    Backward,
}

impl Direction {
    /// The opposite direction.
    pub fn reverse(self) -> Self {
        match self {
            Direction::Forward => Direction::Backward,
            Direction::Backward => Direction::Forward,
        }
    }
}

/// Which side of an edit a position sticks to when the edit touches it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Bias {
    /// Stay before inserted text.
    #[default]
    Before,
    /// Move after inserted text.
    After,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializing_orders_endpoints() {
        // A reversed range in a state file used to survive loading, and
        // `len` underflowed (and ropey's slice panicked) on it.
        let r: CharRange = serde_json::from_str(r#"{"start":9,"end":4}"#).unwrap();
        assert_eq!(r, CharRange::new(4, 9));
        assert_eq!(r.len(), 5);
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, r#"{"start":4,"end":9}"#);
        let back: CharRange = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn range_orders_endpoints() {
        let r = CharRange::new(5, 2);
        assert_eq!(r, CharRange::new(2, 5));
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn range_queries() {
        let r = CharRange::new(2, 5);
        assert!(r.contains(CharPos(2)));
        assert!(!r.contains(CharPos(5)));
        assert!(r.intersects(CharRange::new(4, 9)));
        assert!(!r.intersects(CharRange::new(5, 9)));
        assert_eq!(
            r.intersection(CharRange::new(4, 9)),
            Some(CharRange::new(4, 5))
        );
        assert_eq!(r.cover(CharRange::new(7, 9)), CharRange::new(2, 9));
        assert!(CharRange::empty(3).is_empty());
    }

    #[test]
    fn charpos_serializes_as_number() {
        let json = serde_json::to_string(&CharPos(42)).unwrap();
        assert_eq!(json, "42");
    }
}
