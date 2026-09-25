//! Mapping from spoken text back to source text (ADR-0005).
//!
//! Star generated the spoken text and the displayed text separately and
//! re-aligned them with `difflib`, which drifted whenever normalization
//! changed the text. textweaver builds the mapping while it builds the spoken
//! text, so every spoken byte knows which source chars it came from.
//!
//! An [`OffsetMap`] is a sorted list of [`Span`]s. Spoken positions are byte
//! offsets into the spoken `String` (engines report bytes); source positions
//! are [`CharPos`]. Spans tile the spoken text exactly; source ranges are
//! monotonically non-decreasing and never overlap.
//!
//! Span kinds:
//!
//! - [`SpanKind::Literal`]: spoken text identical to the source text, char for
//!   char. A position inside maps to the exact source char.
//! - [`SpanKind::Expanded`]: spoken text replaces a source token ("Dr." spoken
//!   as "Doctor"). Any position inside highlights the whole source token.
//!   Expansions never split a source token.
//! - [`SpanKind::Inserted`]: spoken text with no source ("heading level 2",
//!   "row 3"). Maps to no highlight; `source` is an empty anchor range where
//!   reading resumes if paused inside it.
//! - [`SpanKind::Elided`]: source text that is not spoken (Markdown `**`,
//!   skipped code). `spoken` is an empty range at the position the source was
//!   dropped.

use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::{CharPos, CharRange, CoreError};

/// How a span of spoken text relates to the source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanKind {
    /// Identical text, char for char.
    Literal,
    /// Spoken text replaces a whole source token.
    Expanded,
    /// Spoken text with no source counterpart.
    Inserted,
    /// Source text that is not spoken.
    Elided,
}

/// One piece of an [`OffsetMap`].
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    /// Byte range in the spoken text. Empty for [`SpanKind::Elided`].
    pub spoken: Range<u32>,
    /// Char range in the source. Empty (an anchor) for [`SpanKind::Inserted`].
    pub source: CharRange,
    /// How the two relate.
    pub kind: SpanKind,
}

impl Span {
    fn spoken_len(&self) -> u32 {
        self.spoken.end - self.spoken.start
    }

    /// True for spans that can be highlighted (literal or expanded).
    pub fn maps_to_source(&self) -> bool {
        matches!(self.kind, SpanKind::Literal | SpanKind::Expanded)
    }
}

/// Mapping from byte ranges of spoken text to char ranges of source text.
///
/// Build one with [`SpokenBuilder`], or [`OffsetMap::identity`] when the spoken
/// text equals the source. Combine the maps of successive transforms with
/// [`OffsetMap::compose`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OffsetMap {
    spans: Vec<Span>,
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).expect("spoken text longer than 4 GiB")
}

/// Largest char boundary of `s` that is `<= byte`.
fn floor_boundary(s: &str, byte: usize) -> usize {
    let mut b = byte.min(s.len());
    while !s.is_char_boundary(b) {
        b -= 1;
    }
    b
}

/// Number of chars in `s[a..b]`, rounding both ends down to char boundaries.
fn chars_between(s: &str, a: usize, b: usize) -> usize {
    let a = floor_boundary(s, a);
    let b = floor_boundary(s, b.max(a));
    s[a..b].chars().count()
}

/// Byte offset reached after advancing `n` chars from byte `from` in `s`.
fn advance_chars(s: &str, from: usize, n: usize) -> usize {
    let from = floor_boundary(s, from);
    s[from..]
        .char_indices()
        .nth(n)
        .map_or(s.len(), |(i, _)| from + i)
}

impl OffsetMap {
    /// A map for spoken text identical to the source text starting at
    /// `source_start`.
    pub fn identity(text: &str, source_start: CharPos) -> Self {
        let mut b = SpokenBuilder::new();
        b.push_literal(text, source_start);
        b.finish().1
    }

    /// Creates a map from spans, checking every invariant against `spoken`.
    pub fn from_spans(spans: Vec<Span>, spoken: &str) -> Result<Self, CoreError> {
        let map = OffsetMap { spans };
        map.check_invariants(spoken)?;
        Ok(map)
    }

    /// The spans, sorted by spoken position.
    pub fn spans(&self) -> &[Span] {
        &self.spans
    }

    /// True when the map has no spans.
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    /// The smallest source range covering every span, anchors included.
    pub fn source_extent(&self) -> Option<CharRange> {
        let first = self.spans.first()?;
        let last = self.spans.last()?;
        Some(CharRange::new(first.source.start, last.source.end))
    }

    /// Shifts every source position by `delta` chars. Used when a map built
    /// for a slice is placed at its position in the document.
    pub fn offset_source(&mut self, delta: usize) {
        for s in &mut self.spans {
            s.source.start = s.source.start.saturating_add(delta);
            s.source.end = s.source.end.saturating_add(delta);
        }
    }

    /// The source chars that produced spoken bytes `bytes`, for highlighting.
    ///
    /// Literal spans map exactly; expanded spans contribute their whole source
    /// token; inserted and elided spans contribute nothing. An empty `bytes`
    /// range is treated as the single position `bytes.start`. Returns `None`
    /// when nothing in the range maps to the source.
    pub fn to_source(&self, spoken: &str, bytes: Range<u32>) -> Option<CharRange> {
        let (qs, qe) = if bytes.start >= bytes.end {
            (bytes.start, bytes.start.saturating_add(1))
        } else {
            (bytes.start, bytes.end)
        };
        let mut out: Option<CharRange> = None;
        for span in self.overlapping(qs, qe) {
            let part = match span.kind {
                SpanKind::Literal => {
                    let a = qs.max(span.spoken.start) as usize;
                    let b = qe.min(span.spoken.end) as usize;
                    let skip = chars_between(spoken, span.spoken.start as usize, a);
                    let take = chars_between(spoken, a, b).max(1);
                    let start = span.source.start.saturating_add(skip);
                    let end = start.saturating_add(take).min(span.source.end);
                    CharRange::new(start, end)
                }
                SpanKind::Expanded => span.source,
                SpanKind::Inserted | SpanKind::Elided => continue,
            };
            out = Some(out.map_or(part, |o| o.cover(part)));
        }
        out
    }

    /// Byte offset in the spoken text where speech should start to begin at
    /// source position `pos` (resume from the cursor).
    ///
    /// Picks the first highlightable span that ends after `pos`: inside a
    /// literal span the exact byte, otherwise the span start. `None` when `pos`
    /// is past everything this map speaks.
    pub fn to_spoken(&self, spoken: &str, pos: CharPos) -> Option<u32> {
        let span = self
            .spans
            .iter()
            .find(|s| s.maps_to_source() && s.source.end > pos)?;
        if span.kind == SpanKind::Literal && span.source.start < pos {
            let n = pos.0 - span.source.start.0;
            Some(to_u32(advance_chars(spoken, span.spoken.start as usize, n)))
        } else {
            Some(span.spoken.start)
        }
    }

    /// The source position to resume from after pausing at spoken byte `byte`.
    ///
    /// Inside a literal span: the exact char. Inside an expanded span: the
    /// token start (resume may repeat, never skips). Inside an inserted span:
    /// its anchor. Past the end: the end of the source extent.
    pub fn resume_source(&self, spoken: &str, byte: u32) -> Option<CharPos> {
        let span = self
            .spans
            .iter()
            .find(|s| s.kind != SpanKind::Elided && s.spoken.end > byte)
            .or_else(|| self.spans.last())?;
        Some(match span.kind {
            SpanKind::Literal if byte > span.spoken.start => {
                let n = chars_between(spoken, span.spoken.start as usize, byte as usize);
                span.source.start.saturating_add(n).min(span.source.end)
            }
            SpanKind::Literal | SpanKind::Expanded | SpanKind::Inserted => span.source.start,
            SpanKind::Elided => span.source.end,
        })
    }

    fn overlapping(&self, qs: u32, qe: u32) -> impl Iterator<Item = &Span> {
        // Spans tile the spoken text in order, so binary-search the first span
        // that ends after `qs` and walk forward while spans start before `qe`.
        let first = self.spans.partition_point(|s| s.spoken.end <= qs);
        self.spans[first..]
            .iter()
            .take_while(move |s| s.spoken.start < qe)
            .filter(move |s| s.spoken_len() > 0 && s.spoken.end > qs)
    }

    /// Checks every invariant against the spoken text:
    ///
    /// 1. spans tile `0..spoken.len()` in order (elided spans are zero-width
    ///    and sit between neighbours);
    /// 2. every spoken boundary is a char boundary;
    /// 3. source ranges are non-decreasing and never overlap;
    /// 4. kind shapes: literal and expanded are non-empty on both sides,
    ///    inserted has an empty source, elided has an empty spoken range and a
    ///    non-empty source;
    /// 5. literal spans have as many chars as their source range.
    pub fn check_invariants(&self, spoken: &str) -> Result<(), CoreError> {
        let err = |m: String| Err(CoreError::InvalidOffsetMap(m));
        let mut at: u32 = 0;
        let mut src_at = CharPos::ZERO;
        for (i, s) in self.spans.iter().enumerate() {
            if s.spoken.start != at {
                return err(format!(
                    "span {i} starts at byte {} not {at}",
                    s.spoken.start
                ));
            }
            if s.spoken.end < s.spoken.start {
                return err(format!("span {i} has a reversed spoken range"));
            }
            for b in [s.spoken.start, s.spoken.end] {
                if b as usize > spoken.len() || !spoken.is_char_boundary(b as usize) {
                    return err(format!("span {i} byte {b} is not a char boundary"));
                }
            }
            if i > 0 && s.source.start < src_at {
                return err(format!("span {i} source {} goes backwards", s.source));
            }
            let spoken_empty = s.spoken.start == s.spoken.end;
            let ok = match s.kind {
                SpanKind::Literal => {
                    !spoken_empty
                        && !s.source.is_empty()
                        && spoken[s.spoken.start as usize..s.spoken.end as usize]
                            .chars()
                            .count()
                            == s.source.len()
                }
                SpanKind::Expanded => !spoken_empty && !s.source.is_empty(),
                SpanKind::Inserted => !spoken_empty && s.source.is_empty(),
                SpanKind::Elided => spoken_empty && !s.source.is_empty(),
            };
            if !ok {
                return err(format!("span {i} ({:?}) has an invalid shape", s.kind));
            }
            at = s.spoken.end;
            src_at = s.source.end;
        }
        if !self.spans.is_empty() && at as usize != spoken.len() {
            return err(format!("spans end at byte {at}, text has {}", spoken.len()));
        }
        Ok(())
    }

    /// Composes two successive transforms.
    ///
    /// `first` maps `mid` (bytes) to the source (chars); `second` maps the
    /// output (bytes) to `mid` (chars, as positions in `mid`). The result maps
    /// the output directly to the source.
    pub fn compose(first: &OffsetMap, mid: &str, second: &OffsetMap) -> OffsetMap {
        // mid char index -> mid byte offset, including the end position.
        let mut char_to_byte: Vec<u32> = mid.char_indices().map(|(i, _)| to_u32(i)).collect();
        char_to_byte.push(to_u32(mid.len()));
        let mid_byte = |c: CharPos| char_to_byte[c.0.min(char_to_byte.len() - 1)];

        let mut out: Vec<Span> = Vec::new();
        for s2 in &second.spans {
            let a = mid_byte(s2.source.start);
            let b = mid_byte(s2.source.end);
            match s2.kind {
                SpanKind::Literal => {
                    // Output bytes equal mid bytes here, so split along the
                    // first map's spans and carry their kinds through.
                    let base = s2.spoken.start;
                    for f in &first.spans {
                        let zero = f.spoken.start == f.spoken.end;
                        if zero {
                            let x = f.spoken.start;
                            if a <= x && x < b {
                                let o = base + (x - a);
                                out.push(Span {
                                    spoken: o..o,
                                    source: f.source,
                                    kind: f.kind,
                                });
                            }
                            continue;
                        }
                        let x = f.spoken.start.max(a);
                        let y = f.spoken.end.min(b);
                        if x >= y {
                            continue;
                        }
                        let o = (base + (x - a))..(base + (y - a));
                        let source = match f.kind {
                            SpanKind::Literal => {
                                let skip = chars_between(mid, f.spoken.start as usize, x as usize);
                                let take = chars_between(mid, x as usize, y as usize);
                                let st = f.source.start.saturating_add(skip);
                                CharRange::new(st, st.saturating_add(take))
                            }
                            SpanKind::Expanded => f.source,
                            SpanKind::Inserted => f.source,
                            SpanKind::Elided => unreachable!("elided spans are zero-width"),
                        };
                        out.push(Span {
                            spoken: o,
                            source,
                            kind: f.kind,
                        });
                    }
                }
                SpanKind::Expanded => {
                    let src = first.source_union(mid, a, b, false);
                    match src {
                        Some(source) => out.push(Span {
                            spoken: s2.spoken.clone(),
                            source,
                            kind: SpanKind::Expanded,
                        }),
                        None => {
                            let anchor = first.resume_source(mid, a).unwrap_or_default();
                            out.push(Span {
                                spoken: s2.spoken.clone(),
                                source: CharRange::empty(anchor),
                                kind: SpanKind::Inserted,
                            });
                        }
                    }
                }
                SpanKind::Inserted => {
                    let anchor = first.resume_source(mid, a).unwrap_or_default();
                    out.push(Span {
                        spoken: s2.spoken.clone(),
                        source: CharRange::empty(anchor),
                        kind: SpanKind::Inserted,
                    });
                }
                SpanKind::Elided => {
                    if let Some(source) = first.source_union(mid, a, b, true) {
                        out.push(Span {
                            spoken: s2.spoken.clone(),
                            source,
                            kind: SpanKind::Elided,
                        });
                    }
                }
            }
        }
        OffsetMap {
            spans: normalize(out),
        }
    }

    /// Union of the source ranges behind mid bytes `a..b`, optionally
    /// including elided spans that sit inside the range.
    fn source_union(&self, mid: &str, a: u32, b: u32, include_elided: bool) -> Option<CharRange> {
        let mut acc = if a < b {
            self.to_source(mid, a..b)
        } else {
            None
        };
        if include_elided {
            for f in &self.spans {
                if f.kind == SpanKind::Elided && a <= f.spoken.start && f.spoken.start <= b {
                    acc = Some(acc.map_or(f.source, |r| r.cover(f.source)));
                }
            }
        }
        acc
    }
}

/// Restores the invariants after composition: merges adjacent literal spans,
/// fuses spans whose sources overlap into one expanded span, and clamps
/// anchors so sources never go backwards.
fn normalize(spans: Vec<Span>) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::with_capacity(spans.len());
    for s in spans {
        if let Some(prev) = out.last_mut() {
            let contiguous = prev.spoken.end == s.spoken.start;
            // Literal continuation.
            if contiguous
                && prev.kind == SpanKind::Literal
                && s.kind == SpanKind::Literal
                && prev.source.end == s.source.start
            {
                prev.spoken.end = s.spoken.end;
                prev.source.end = s.source.end;
                continue;
            }
            // Two pieces of the same source token (or overlapping tokens):
            // one expanded span covering both.
            let overlap =
                s.source.start < prev.source.end && prev.maps_to_source() && s.maps_to_source();
            if contiguous && overlap {
                prev.spoken.end = s.spoken.end;
                prev.source = prev.source.cover(s.source);
                prev.kind = SpanKind::Expanded;
                continue;
            }
        }
        let mut s = s;
        if let Some(prev) = out.last() {
            if s.source.start < prev.source.end {
                match s.kind {
                    SpanKind::Inserted => s.source = CharRange::empty(prev.source.end),
                    SpanKind::Elided => {
                        let start = prev.source.end;
                        if s.source.end <= start {
                            continue;
                        }
                        s.source = CharRange::new(start, s.source.end);
                    }
                    SpanKind::Literal | SpanKind::Expanded => {}
                }
            }
        }
        out.push(s);
    }
    out
}

/// Builds spoken text and its [`OffsetMap`] together.
///
/// Every normalization transform uses one of these: it walks the source,
/// pushes pieces in order, and returns `(String, OffsetMap)`.
#[derive(Clone, Debug, Default)]
pub struct SpokenBuilder {
    text: String,
    spans: Vec<Span>,
}

impl SpokenBuilder {
    /// An empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// The spoken text built so far.
    pub fn text(&self) -> &str {
        &self.text
    }

    fn end(&self) -> u32 {
        to_u32(self.text.len())
    }

    /// Appends `s`, identical to the source starting at `source_start`.
    pub fn push_literal(&mut self, s: &str, source_start: CharPos) {
        if s.is_empty() {
            return;
        }
        let start = self.end();
        self.text.push_str(s);
        let end = self.end();
        let source = CharRange::new(source_start, source_start.saturating_add(s.chars().count()));
        if let Some(prev) = self.spans.last_mut() {
            if prev.kind == SpanKind::Literal
                && prev.spoken.end == start
                && prev.source.end == source.start
            {
                prev.spoken.end = end;
                prev.source.end = source.end;
                return;
            }
        }
        self.spans.push(Span {
            spoken: start..end,
            source,
            kind: SpanKind::Literal,
        });
    }

    /// Appends `s`, spoken in place of the source token `source`. An empty
    /// `s` records the token as elided.
    pub fn push_expanded(&mut self, s: &str, source: CharRange) {
        if s.is_empty() {
            self.push_elided(source);
            return;
        }
        if source.is_empty() {
            self.push_inserted(s, source.start);
            return;
        }
        let start = self.end();
        self.text.push_str(s);
        self.spans.push(Span {
            spoken: start..self.end(),
            source,
            kind: SpanKind::Expanded,
        });
    }

    /// Appends `s`, which has no source; `at` is where reading resumes if
    /// paused inside it.
    pub fn push_inserted(&mut self, s: &str, at: CharPos) {
        if s.is_empty() {
            return;
        }
        let start = self.end();
        self.text.push_str(s);
        self.spans.push(Span {
            spoken: start..self.end(),
            source: CharRange::empty(at),
            kind: SpanKind::Inserted,
        });
    }

    /// Records that `source` is not spoken.
    pub fn push_elided(&mut self, source: CharRange) {
        if source.is_empty() {
            return;
        }
        let at = self.end();
        self.spans.push(Span {
            spoken: at..at,
            source,
            kind: SpanKind::Elided,
        });
    }

    /// The spoken text and its map.
    pub fn finish(self) -> (String, OffsetMap) {
        (self.text, OffsetMap { spans: self.spans })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(a: usize, b: usize) -> CharRange {
        CharRange::new(a, b)
    }

    /// "Dr. Smith paid $5." with "Dr." expanded and "$5" expanded.
    fn sample() -> (String, OffsetMap) {
        let mut b = SpokenBuilder::new();
        b.push_expanded("Doctor", r(0, 3)); // "Dr."
        b.push_literal(" Smith paid ", CharPos(3));
        b.push_expanded("five dollars", r(15, 17)); // "$5"
        b.push_literal(".", CharPos(17));
        b.finish()
    }

    #[test]
    fn builder_produces_valid_map() {
        let (text, map) = sample();
        assert_eq!(text, "Doctor Smith paid five dollars.");
        map.check_invariants(&text).unwrap();
        assert_eq!(map.source_extent(), Some(r(0, 18)));
    }

    #[test]
    fn expanded_highlights_whole_token() {
        let (text, map) = sample();
        // "Doc" inside "Doctor" -> the whole "Dr."
        assert_eq!(map.to_source(&text, 0..3), Some(r(0, 3)));
        // "dollars" -> "$5"
        let d = to_u32(text.find("dollars").unwrap());
        assert_eq!(map.to_source(&text, d..d + 7), Some(r(15, 17)));
    }

    #[test]
    fn literal_maps_exactly() {
        let (text, map) = sample();
        let s = to_u32(text.find("Smith").unwrap());
        assert_eq!(map.to_source(&text, s..s + 5), Some(r(4, 9)));
    }

    #[test]
    fn inserted_maps_to_none_and_resumes_at_anchor() {
        let mut b = SpokenBuilder::new();
        b.push_inserted("heading level 2, ", CharPos(10));
        b.push_literal("Intro", CharPos(10));
        let (text, map) = b.finish();
        map.check_invariants(&text).unwrap();
        assert_eq!(map.to_source(&text, 0..7), None);
        assert_eq!(map.resume_source(&text, 3), Some(CharPos(10)));
        let i = to_u32(text.find("Intro").unwrap());
        assert_eq!(map.to_source(&text, i..i + 5), Some(r(10, 15)));
    }

    #[test]
    fn elided_is_zero_width() {
        let mut b = SpokenBuilder::new();
        b.push_elided(r(0, 2)); // "**"
        b.push_literal("bold", CharPos(2));
        b.push_elided(r(6, 8));
        let (text, map) = b.finish();
        assert_eq!(text, "bold");
        map.check_invariants(&text).unwrap();
        assert_eq!(map.to_source(&text, 0..4), Some(r(2, 6)));
    }

    #[test]
    fn to_spoken_and_resume_roundtrip_on_literal() {
        let text = "héllo wörld";
        let map = OffsetMap::identity(text, CharPos(100));
        let b = map.to_spoken(text, CharPos(107)).unwrap();
        assert_eq!(&text[b as usize..], "örld");
        assert_eq!(map.resume_source(text, b), Some(CharPos(107)));
    }

    #[test]
    fn to_spoken_inside_expansion_starts_at_token() {
        let (text, map) = sample();
        assert_eq!(map.to_spoken(&text, CharPos(1)), Some(0));
        assert_eq!(
            map.to_spoken(&text, CharPos(16)),
            Some(to_u32(text.find("five").unwrap()))
        );
    }

    #[test]
    fn multibyte_word_event_maps_to_chars() {
        let text = "naïve café";
        let map = OffsetMap::identity(text, CharPos(0));
        let s = to_u32(text.find("café").unwrap());
        assert_eq!(map.to_source(text, s..to_u32(text.len())), Some(r(6, 10)));
    }

    #[test]
    fn invariants_reject_gaps_and_bad_shapes() {
        let text = "abc";
        let bad = vec![Span {
            spoken: 1..3,
            source: r(0, 2),
            kind: SpanKind::Literal,
        }];
        assert!(OffsetMap::from_spans(bad, text).is_err());
        let bad = vec![Span {
            spoken: 0..3,
            source: r(0, 2),
            kind: SpanKind::Literal,
        }];
        assert!(OffsetMap::from_spans(bad, text).is_err());
        let bad = vec![Span {
            spoken: 0..3,
            source: r(0, 1),
            kind: SpanKind::Inserted,
        }];
        assert!(OffsetMap::from_spans(bad, text).is_err());
    }

    #[test]
    fn compose_expansion_after_literal() {
        // T1: "Dr. Who" -> "Doctor Who"; T2: uppercase "Who" as an expansion.
        let src = "Dr. Who";
        let mut b1 = SpokenBuilder::new();
        b1.push_expanded("Doctor", r(0, 3));
        b1.push_literal(" Who", CharPos(3));
        let (mid, m1) = b1.finish();
        let mut b2 = SpokenBuilder::new();
        b2.push_literal("Doctor ", CharPos(0));
        b2.push_expanded("WHO", r(7, 10));
        let (out, m2) = b2.finish();
        let m = OffsetMap::compose(&m1, &mid, &m2);
        m.check_invariants(&out).unwrap();
        assert_eq!(m.to_source(&out, 0..3), Some(r(0, 3)));
        assert_eq!(m.to_source(&out, 7..10), Some(r(4, 7)));
        assert_eq!(&src[4..7], "Who");
    }

    #[test]
    fn compose_with_identity_is_neutral() {
        let (text, map) = sample();
        let id = OffsetMap::identity(&text, CharPos(0));
        let left = OffsetMap::compose(&map, &text, &id);
        assert_eq!(left, map);
    }
}

#[cfg(test)]
mod props {
    use proptest::prelude::*;

    use super::*;

    #[derive(Clone, Debug)]
    enum Op {
        Keep,
        Expand(String),
        Elide,
        InsertBefore(String),
    }

    fn word() -> impl Strategy<Value = String> {
        proptest::string::string_regex("[a-zé]{1,6}").unwrap()
    }

    fn op() -> impl Strategy<Value = Op> {
        prop_oneof![
            4 => Just(Op::Keep),
            2 => word().prop_map(Op::Expand),
            1 => Just(Op::Elide),
            1 => word().prop_map(Op::InsertBefore),
        ]
    }

    /// Applies one op per whitespace-separated token of `input`, keeping the
    /// separating spaces literal. Returns the output text and its map.
    fn transform(input: &str, ops: &[Op]) -> (String, OffsetMap) {
        let mut b = SpokenBuilder::new();
        let mut pos = 0usize; // char index into input
        for (i, tok) in input.split(' ').enumerate() {
            if i > 0 {
                b.push_literal(" ", CharPos(pos));
                pos += 1;
            }
            let n = tok.chars().count();
            let range = CharRange::new(pos, pos + n);
            match ops.get(i % ops.len().max(1)).unwrap_or(&Op::Keep) {
                Op::Keep => b.push_literal(tok, CharPos(pos)),
                Op::Expand(w) => b.push_expanded(w, range),
                Op::Elide => b.push_elided(range),
                Op::InsertBefore(w) => {
                    b.push_inserted(w, CharPos(pos));
                    b.push_literal(tok, CharPos(pos));
                }
            }
            pos += n;
        }
        b.finish()
    }

    fn source() -> impl Strategy<Value = String> {
        proptest::collection::vec(word(), 1..12).prop_map(|w| w.join(" "))
    }

    proptest! {
        #[test]
        fn builder_maps_are_valid(src in source(), ops in proptest::collection::vec(op(), 1..6)) {
            let (out, map) = transform(&src, &ops);
            prop_assert!(map.check_invariants(&out).is_ok(), "{:?}", map.check_invariants(&out));
        }

        #[test]
        fn literal_spans_reproduce_source(src in source(), ops in proptest::collection::vec(op(), 1..6)) {
            let (out, map) = transform(&src, &ops);
            let chars: Vec<char> = src.chars().collect();
            for s in map.spans().iter().filter(|s| s.kind == SpanKind::Literal) {
                let spoken = &out[s.spoken.start as usize..s.spoken.end as usize];
                let original: String = chars[s.source.to_range()].iter().collect();
                prop_assert_eq!(spoken, original);
            }
        }

        #[test]
        fn composition_is_valid_and_literal_exact(
            src in source(),
            ops1 in proptest::collection::vec(op(), 1..6),
            ops2 in proptest::collection::vec(op(), 1..6),
        ) {
            let (mid, m1) = transform(&src, &ops1);
            let (out, m2) = transform(&mid, &ops2);
            let m = OffsetMap::compose(&m1, &mid, &m2);
            prop_assert!(m.check_invariants(&out).is_ok(), "{:?}\n{:?}", m.check_invariants(&out), m);
            let chars: Vec<char> = src.chars().collect();
            for s in m.spans().iter().filter(|s| s.kind == SpanKind::Literal) {
                let spoken = &out[s.spoken.start as usize..s.spoken.end as usize];
                let original: String = chars[s.source.to_range()].iter().collect();
                prop_assert_eq!(spoken, original);
            }
            if let Some(ext) = m.source_extent() {
                prop_assert!(ext.end.0 <= chars.len());
            }
        }

        #[test]
        fn identity_composition_preserves_mapping(
            src in source(),
            ops in proptest::collection::vec(op(), 1..6),
        ) {
            let (out, m) = transform(&src, &ops);
            let id_src = OffsetMap::identity(&src, CharPos(0));
            let left = OffsetMap::compose(&id_src, &src, &m);
            let id_out = OffsetMap::identity(&out, CharPos(0));
            let right = OffsetMap::compose(&m, &out, &id_out);
            for (i, _) in out.char_indices() {
                let i = to_u32(i);
                let want = m.to_source(&out, i..i + 1);
                prop_assert_eq!(left.to_source(&out, i..i + 1), want);
                prop_assert_eq!(right.to_source(&out, i..i + 1), want);
            }
        }

        #[test]
        fn to_source_stays_inside_extent(
            src in source(),
            ops in proptest::collection::vec(op(), 1..6),
            a in 0u32..64, len in 0u32..16,
        ) {
            let (out, m) = transform(&src, &ops);
            let a = a.min(to_u32(out.len()));
            let b = (a + len).min(to_u32(out.len()));
            if let (Some(r), Some(ext)) = (m.to_source(&out, a..b), m.source_extent()) {
                prop_assert!(ext.contains_range(r));
            }
        }
    }
}
