//! Find in document: plain or regex, case, whole word, wrap, direction.
//!
//! Decisions (each fixes a star bug, the star parity reference Part 1 §5):
//!
//! - **One haystack.** The whole canonical text is searched, so a match can
//!   span a line break (star's TUI searched each wrapped display line apart).
//! - **Plain search is whitespace-tolerant.** A run of whitespace in a plain
//!   pattern matches any run of whitespace, so `"reading room"` finds
//!   `"reading\nroom"` in a plain-text file whose lines wrap.
//! - **Case-insensitive offsets are exact.** Matching uses Unicode case
//!   folding on the original text, never a lowercased copy whose length can
//!   differ (star's `İ` bug).
//! - **Whole word** means the match is not preceded or followed by a letter,
//!   digit, or underscore. It works for patterns that begin or end with
//!   punctuation, unlike a plain `\b` wrapper.
//! - **Matches do not overlap.** [`find_all`] reports leftmost-first,
//!   non-overlapping matches (star reported overlapping ones, which corrupted
//!   its Replace All). Next and previous step through that list.
//! - **Backward means backward.** A backward search finds the last match
//!   that starts before `from` (star's `search-backward` searched forward).
//! - **Regex mode** is multi-line: `^` and `$` match at line starts and ends.
//!   An invalid pattern is an error for the caller to announce, not a silent
//!   fallback to plain search.

use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange, Direction};

use crate::Document;

/// A search request.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQuery {
    /// The text or regular expression to find.
    pub pattern: String,
    /// Treat `pattern` as a regular expression.
    pub regex: bool,
    /// Match case exactly.
    pub case_sensitive: bool,
    /// Only match whole words.
    pub whole_word: bool,
    /// Wrap around the document ends.
    pub wrap: bool,
    /// Search direction.
    pub direction: Direction,
}

impl SearchQuery {
    /// A plain, case-insensitive, forward, wrapping search for `pattern`
    /// (the find bar's defaults).
    pub fn plain(pattern: impl Into<String>) -> Self {
        SearchQuery {
            pattern: pattern.into(),
            wrap: true,
            ..SearchQuery::default()
        }
    }

    /// Compiles the query into the regular expression it runs.
    pub fn compile(&self) -> Result<Regex, SearchError> {
        let pat = if self.regex {
            self.pattern.clone()
        } else {
            plain_pattern(&self.pattern)
        };
        Ok(RegexBuilder::new(&pat)
            .case_insensitive(!self.case_sensitive)
            .multi_line(true)
            .build()?)
    }
}

/// Search failures.
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    /// The pattern is not a valid regular expression.
    #[error("invalid pattern: {0}")]
    InvalidPattern(#[from] regex::Error),
}

/// Escapes a plain pattern, letting each whitespace run match any whitespace.
fn plain_pattern(pattern: &str) -> String {
    let mut out = String::new();
    let mut in_space = false;
    for c in pattern.chars() {
        if c.is_whitespace() {
            if !in_space {
                out.push_str(r"\s+");
                in_space = true;
            }
        } else {
            in_space = false;
            out.push_str(&regex::escape(c.encode_utf8(&mut [0; 4])));
        }
    }
    out
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Every match, in document order. Matches do not overlap; empty matches are
/// skipped.
pub fn find_all(doc: &Document, query: &SearchQuery) -> Result<Vec<CharRange>, SearchError> {
    if query.pattern.is_empty() {
        return Ok(Vec::new());
    }
    let re = query.compile()?;
    let text = doc.text().to_string();
    let mut out = Vec::new();
    // Byte offsets to char offsets in one forward pass.
    let mut chars_before = 0usize;
    let mut counted_to = 0usize;
    let mut at = 0usize;
    while at <= text.len() {
        let Some(m) = re.find_at(&text, at) else {
            break;
        };
        if m.start() == m.end() {
            at = next_boundary(&text, m.end());
            continue;
        }
        if query.whole_word {
            let before = text[..m.start()].chars().next_back();
            let after = text[m.end()..].chars().next();
            if before.is_some_and(is_word_char) || after.is_some_and(is_word_char) {
                // Retry one char further on, so a rejected match does not
                // hide a valid one that overlaps it.
                at = next_boundary(&text, m.start());
                continue;
            }
        }
        chars_before += text[counted_to..m.start()].chars().count();
        let len = m.as_str().chars().count();
        out.push(CharRange::new(chars_before, chars_before + len));
        chars_before += len;
        counted_to = m.end();
        at = m.end();
    }
    Ok(out)
}

fn next_boundary(text: &str, byte: usize) -> usize {
    text[byte..]
        .chars()
        .next()
        .map_or(text.len() + 1, |c| byte + c.len_utf8())
}

/// The next match after `from` (forward) or the last match before `from`
/// (backward). The bool is true when the search wrapped.
pub fn find(
    doc: &Document,
    from: CharPos,
    query: &SearchQuery,
) -> Result<Option<(CharRange, bool)>, SearchError> {
    let all = find_all(doc, query)?;
    let hit = match query.direction {
        Direction::Forward => all.iter().find(|r| r.start > from),
        Direction::Backward => all.iter().rev().find(|r| r.start < from),
    };
    Ok(match (hit, query.wrap) {
        (Some(r), _) => Some((*r, false)),
        (None, true) => match query.direction {
            Direction::Forward => all.first(),
            Direction::Backward => all.last(),
        }
        .map(|r| (*r, true)),
        (None, false) => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(pattern: &str) -> SearchQuery {
        SearchQuery {
            pattern: pattern.into(),
            ..SearchQuery::default()
        }
    }

    #[test]
    fn backward_search_goes_backward() {
        let d = Document::from_plain_text("cat dog cat dog cat");
        let query = SearchQuery {
            direction: Direction::Backward,
            ..q("cat")
        };
        let (r, wrapped) = find(&d, CharPos(12), &query).unwrap().unwrap();
        assert_eq!((r, wrapped), (CharRange::new(8, 11), false));
        let (r, wrapped) = find(
            &d,
            CharPos(0),
            &SearchQuery {
                wrap: true,
                ..query
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!((r, wrapped), (CharRange::new(16, 19), true));
    }

    #[test]
    fn whole_word_and_case() {
        let d = Document::from_plain_text("Cat category cat");
        let query = SearchQuery {
            whole_word: true,
            ..q("cat")
        };
        assert_eq!(find_all(&d, &query).unwrap().len(), 2);
        let query = SearchQuery {
            case_sensitive: true,
            ..query
        };
        assert_eq!(find_all(&d, &query).unwrap(), vec![CharRange::new(13, 16)]);
    }

    #[test]
    fn whole_word_retries_after_a_rejected_match() {
        let d = Document::from_plain_text("aaa aa (x) x");
        let query = SearchQuery {
            whole_word: true,
            ..q("aa")
        };
        assert_eq!(find_all(&d, &query).unwrap(), vec![CharRange::new(4, 6)]);
        let paren = SearchQuery {
            whole_word: true,
            ..q("(x)")
        };
        assert_eq!(find_all(&d, &paren).unwrap(), vec![CharRange::new(7, 10)]);
    }

    #[test]
    fn matches_do_not_overlap_and_span_lines() {
        let d = Document::from_plain_text("aaaa\nreading\n  room");
        assert_eq!(find_all(&d, &q("aa")).unwrap().len(), 2);
        let hits = find_all(&d, &q("reading room")).unwrap();
        assert_eq!(hits, vec![CharRange::new(5, 19)]);
    }

    #[test]
    fn unicode_case_folding_keeps_offsets() {
        let d = Document::from_plain_text("İstanbul ÉCOLE école");
        let hits = find_all(&d, &q("école")).unwrap();
        assert_eq!(hits, vec![CharRange::new(9, 14), CharRange::new(15, 20)]);
    }

    #[test]
    fn regex_is_multiline_and_errors_are_reported() {
        let d = Document::from_plain_text("one\ntwo\nthree");
        let query = SearchQuery {
            regex: true,
            ..q("^t\\w+")
        };
        assert_eq!(find_all(&d, &query).unwrap().len(), 2);
        let bad = SearchQuery {
            regex: true,
            ..q("(")
        };
        assert!(find_all(&d, &bad).is_err());
        let empty = SearchQuery {
            regex: true,
            ..q("x*")
        };
        assert!(find_all(&d, &empty).unwrap().is_empty());
    }
}
