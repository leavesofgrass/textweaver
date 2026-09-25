//! Text units: the grapheme, word, sentence, line, or paragraph at a position.
//!
//! Phase 0 placeholder rules: graphemes via UAX #29; words are maximal runs of
//! alphanumeric chars plus `'` and `-` (Star's `\b\w[\w'-]*`); sentences end
//! after `.`, `!`, or `?` followed by whitespace; paragraphs are separated by
//! blank lines. Agent A replaces these with UAX #29 plus the abbreviation list
//! and parity tests against `fixtures/star-parity`.

use textweaver_core::{CharPos, CharRange, Unit};
use unicode_segmentation::UnicodeSegmentation;

use crate::Document;
use crate::marker::MarkerIndex;

/// All ranges of `unit` in the document, in order. Placeholder for Agent A's
/// segment iterators; fine for small documents.
pub fn segments(doc: &Document, unit: Unit) -> Vec<CharRange> {
    let text = doc.text().to_string();
    let chars: Vec<char> = text.chars().collect();
    match unit {
        Unit::Grapheme => {
            let mut at = 0;
            text.graphemes(true)
                .map(|g| {
                    let n = g.chars().count();
                    let r = CharRange::new(at, at + n);
                    at += n;
                    r
                })
                .collect()
        }
        Unit::Word => runs(&chars, |c| c.is_alphanumeric() || c == '\'' || c == '-')
            .into_iter()
            .filter(|r| chars[r.to_range()].iter().any(|c| c.is_alphanumeric()))
            .collect(),
        Unit::Sentence => sentences(&chars),
        Unit::Line => lines(&chars),
        Unit::Paragraph => paragraphs(&chars),
        Unit::Document => vec![CharRange::new(0, chars.len())],
        Unit::Marker { kind, level } => doc
            .markers()
            .iter()
            .filter(|m| m.kind == kind && level.is_none_or(|l| m.level == l))
            .map(|m| m.range)
            .collect(),
    }
}

/// The range of `unit` containing `pos`, or the next one after it.
pub fn unit_at(doc: &Document, pos: CharPos, unit: Unit) -> Option<CharRange> {
    if let Unit::Marker { kind, .. } = unit {
        if let Some(m) = MarkerIndex::new(doc.markers()).enclosing(kind, pos) {
            return Some(m.range);
        }
    }
    let segs = segments(doc, unit);
    segs.iter()
        .find(|r| r.contains(pos))
        .or_else(|| segs.iter().find(|r| r.start >= pos))
        .copied()
}

fn runs(chars: &[char], pred: impl Fn(char) -> bool) -> Vec<CharRange> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, &c) in chars.iter().enumerate() {
        match (pred(c), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push(CharRange::new(s, i));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push(CharRange::new(s, chars.len()));
    }
    out
}

fn trim(chars: &[char], r: CharRange) -> Option<CharRange> {
    let mut s = r.start.0;
    let mut e = r.end.0;
    while s < e && chars[s].is_whitespace() {
        s += 1;
    }
    while e > s && chars[e - 1].is_whitespace() {
        e -= 1;
    }
    (s < e).then(|| CharRange::new(s, e))
}

fn sentences(chars: &[char]) -> Vec<CharRange> {
    let mut out = Vec::new();
    let mut start = 0;
    let n = chars.len();
    for i in 0..n {
        let end_punct = matches!(chars[i], '.' | '!' | '?');
        let next_space = i + 1 >= n || chars[i + 1].is_whitespace();
        let blank = chars[i] == '\n' && i + 1 < n && chars[i + 1] == '\n';
        if (end_punct && next_space) || blank {
            if let Some(r) = trim(chars, CharRange::new(start, i + 1)) {
                out.push(r);
            }
            start = i + 1;
        }
    }
    if let Some(r) = trim(chars, CharRange::new(start, n)) {
        out.push(r);
    }
    out
}

fn lines(chars: &[char]) -> Vec<CharRange> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, &c) in chars.iter().enumerate() {
        if c == '\n' {
            out.push(CharRange::new(start, i));
            start = i + 1;
        }
    }
    if start < chars.len() || chars.is_empty() {
        out.push(CharRange::new(start, chars.len()));
    }
    out
}

fn paragraphs(chars: &[char]) -> Vec<CharRange> {
    lines(chars)
        .split(|l| chars[l.to_range()].iter().all(|c| c.is_whitespace()))
        .filter(|g| !g.is_empty())
        .map(|g| CharRange::new(g[0].start, g[g.len() - 1].end))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn naive_units_on_plain_text() {
        let d = Document::from_plain_text("One two. Three!\n\nFour five?");
        assert_eq!(segments(&d, Unit::Word).len(), 5);
        assert_eq!(segments(&d, Unit::Sentence).len(), 3);
        assert_eq!(segments(&d, Unit::Paragraph).len(), 2);
        assert_eq!(
            unit_at(&d, CharPos(5), Unit::Word),
            Some(CharRange::new(4, 7))
        );
    }
}
