//! Shared helpers: word segments of a string, byte-to-char conversion, and
//! the ranges an aid skips (code, URLs).

use textweaver_core::{CharPos, CharRange, MarkerKind};
use textweaver_text::Document;
use unicode_segmentation::UnicodeSegmentation;

/// Converts increasing byte offsets of `text` into canonical positions,
/// counting chars incrementally so a whole pass is `O(len)`.
pub(crate) struct ByteToPos<'a> {
    text: &'a str,
    base: CharPos,
    byte: usize,
    chars: usize,
}

impl<'a> ByteToPos<'a> {
    pub(crate) fn new(text: &'a str, base: CharPos) -> Self {
        ByteToPos {
            text,
            base,
            byte: 0,
            chars: 0,
        }
    }

    /// The position of byte `b`. Calls must not go backwards; a smaller `b`
    /// restarts the count from the beginning (correct, only slower).
    pub(crate) fn pos(&mut self, b: usize) -> CharPos {
        let b = b.min(self.text.len());
        if b < self.byte {
            self.byte = 0;
            self.chars = 0;
        }
        self.chars += self.text[self.byte..b].chars().count();
        self.byte = b;
        self.base.saturating_add(self.chars)
    }
}

/// UAX #29 word segments of `text` that contain an alphanumeric char, as
/// `(byte offset, segment)`. This is the same rule as `textweaver-text`'s
/// word unit, without its joining of hyphenated compounds (each part of
/// `well-known` is its own word here, which is what bionic reading and
/// syllable splitting want).
pub(crate) fn word_segments(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.split_word_bound_indices()
        .filter(|(_, w)| w.chars().any(char::is_alphanumeric))
}

/// True for a whitespace-delimited token that looks like a URL, a domain
/// with a path, or an email address.
pub(crate) fn is_url_like(token: &str) -> bool {
    let t = token.trim_matches(|c: char| {
        matches!(
            c,
            '(' | ')' | '[' | ']' | '<' | '>' | '"' | '\'' | ',' | ';' | '.'
        )
    });
    if t.is_empty() {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    if lower.contains("://") || lower.starts_with("www.") || lower.starts_with("mailto:") {
        return true;
    }
    // user@example.org
    if let Some(at) = t.find('@') {
        let (user, host) = (&t[..at], &t[at + 1..]);
        if !user.is_empty() && host.contains('.') && !host.starts_with('.') {
            return true;
        }
    }
    // example.org/path
    if let Some(slash) = t.find('/') {
        let host = &t[..slash];
        if host.contains('.')
            && host
                .chars()
                .all(|c| c.is_alphanumeric() || c == '.' || c == '-')
        {
            return true;
        }
    }
    false
}

/// True for a token that reads as code rather than prose: `snake_case`,
/// `a::b`, `f()`, `x->y`, `a=b`.
pub(crate) fn is_code_like(token: &str) -> bool {
    token.contains('_')
        || token.contains("::")
        || token.contains("()")
        || token.contains("->")
        || token.contains("=>")
        || (token.contains('=') && !token.starts_with('=') && !token.ends_with('='))
}

/// A sorted set of non-overlapping char ranges to skip.
#[derive(Clone, Debug, Default)]
pub(crate) struct SkipSet {
    ranges: Vec<CharRange>,
}

impl SkipSet {
    pub(crate) fn new(mut ranges: Vec<CharRange>) -> Self {
        ranges.retain(|r| !r.is_empty());
        ranges.sort_by_key(|r| (r.start, r.end));
        let mut out: Vec<CharRange> = Vec::with_capacity(ranges.len());
        for r in ranges {
            match out.last_mut() {
                Some(last) if r.start <= last.end => last.end = last.end.max(r.end),
                _ => out.push(r),
            }
        }
        SkipSet { ranges: out }
    }

    /// True when `r` shares a char with any skipped range.
    pub(crate) fn overlaps(&self, r: CharRange) -> bool {
        let i = self.ranges.partition_point(|s| s.end <= r.start);
        self.ranges
            .get(i)
            .is_some_and(|s| s.start < r.end.max(r.start.saturating_add(1)))
    }
}

/// Ranges of whitespace-delimited tokens in `text` that are URLs or code
/// identifiers, and of inline `` `code` `` spans.
pub(crate) fn text_skip_ranges(
    text: &str,
    base: CharPos,
    urls: bool,
    code: bool,
) -> Vec<CharRange> {
    let mut out = Vec::new();
    let mut conv = ByteToPos::new(text, base);
    if urls || code {
        let mut start: Option<usize> = None;
        let push = |a: usize, b: usize, conv: &mut ByteToPos<'_>, out: &mut Vec<CharRange>| {
            let tok = &text[a..b];
            if (urls && is_url_like(tok)) || (code && is_code_like(tok)) {
                let s = conv.pos(a);
                let e = conv.pos(b);
                out.push(CharRange::new(s, e));
            }
        };
        for (i, c) in text.char_indices() {
            if c.is_whitespace() {
                if let Some(a) = start.take() {
                    push(a, i, &mut conv, &mut out);
                }
            } else if start.is_none() {
                start = Some(i);
            }
        }
        if let Some(a) = start {
            push(a, text.len(), &mut conv, &mut out);
        }
    }
    if code {
        let mut conv = ByteToPos::new(text, base);
        let mut open: Option<usize> = None;
        for (i, c) in text.char_indices() {
            match c {
                '`' => match open.take() {
                    Some(a) => {
                        let s = conv.pos(a);
                        let e = conv.pos(i + 1);
                        out.push(CharRange::new(s, e));
                    }
                    None => open = Some(i),
                },
                '\n' => open = None,
                _ => {}
            }
        }
    }
    out
}

/// `Code` marker ranges of `doc` that intersect `range`.
pub(crate) fn code_marker_ranges(doc: &Document, range: CharRange) -> Vec<CharRange> {
    doc.markers()
        .iter()
        .filter(|m| m.kind == MarkerKind::Code && m.range.intersects(range))
        .map(|m| m.range)
        .collect()
}

/// Number of extended grapheme clusters in `s`, with an ASCII fast path.
pub(crate) fn grapheme_len(s: &str) -> usize {
    if s.is_ascii() && !s.contains('\r') {
        s.len()
    } else {
        s.graphemes(true).count()
    }
}

/// Byte offset and length of grapheme `n` of `s`, with an ASCII fast path.
pub(crate) fn nth_grapheme(s: &str, n: usize) -> Option<(usize, usize)> {
    if s.is_ascii() && !s.contains('\r') {
        (n < s.len()).then_some((n, 1))
    } else {
        s.grapheme_indices(true).nth(n).map(|(i, g)| (i, g.len()))
    }
}

/// Bytes taken by the first `n` graphemes of `s`.
pub(crate) fn grapheme_prefix_bytes(s: &str, n: usize) -> usize {
    if s.is_ascii() && !s.contains('\r') {
        n.min(s.len())
    } else {
        s.grapheme_indices(true).nth(n).map_or(s.len(), |(i, _)| i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_and_emails() {
        assert!(is_url_like("https://example.org/a"));
        assert!(is_url_like("(www.example.org)."));
        assert!(is_url_like("jon@example.org"));
        assert!(is_url_like("example.org/path"));
        assert!(!is_url_like("and/or"));
        assert!(!is_url_like("e.g."));
        assert!(!is_url_like("@"));
    }

    #[test]
    fn code_tokens() {
        assert!(is_code_like("snake_case"));
        assert!(is_code_like("std::fmt"));
        assert!(is_code_like("print()"));
        assert!(!is_code_like("ordinary"));
        assert!(!is_code_like("=")); // a lone sign is prose ("x = 2")
    }

    #[test]
    fn skip_set_merges_and_queries() {
        let s = SkipSet::new(vec![
            CharRange::new(5, 8),
            CharRange::new(0, 2),
            CharRange::new(7, 10),
        ]);
        assert!(s.overlaps(CharRange::new(1, 3)));
        assert!(!s.overlaps(CharRange::new(2, 5)));
        assert!(s.overlaps(CharRange::new(9, 12)));
        assert!(!s.overlaps(CharRange::new(10, 12)));
        assert!(s.overlaps(CharRange::empty(6)));
    }

    #[test]
    fn backtick_spans_are_code() {
        let r = text_skip_ranges("run `cargo test` now", CharPos(10), false, true);
        assert_eq!(r, vec![CharRange::new(14, 26)]);
    }

    #[test]
    fn grapheme_helpers() {
        assert_eq!(grapheme_len("word"), 4);
        assert_eq!(grapheme_len("cafe\u{301}"), 4);
        assert_eq!(nth_grapheme("word", 2), Some((2, 1)));
        assert_eq!(nth_grapheme("word", 4), None);
        assert_eq!(nth_grapheme("e\u{301}x", 1), Some((3, 1)));
        assert_eq!(grapheme_prefix_bytes("word", 9), 4);
        assert_eq!(grapheme_prefix_bytes("e\u{301}x", 1), 3);
    }

    #[test]
    fn byte_to_pos_counts_chars() {
        let t = "é a";
        let mut c = ByteToPos::new(t, CharPos(3));
        assert_eq!(c.pos(2), CharPos(4));
        assert_eq!(c.pos(4), CharPos(6));
        assert_eq!(c.pos(0), CharPos(3));
    }
}
