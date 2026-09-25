//! Find in document: plain or regex, case, whole word, wrap, direction.
//!
//! Fixes Star's `search-backward` bug by construction: backward search looks
//! for the last match that starts before `from`.

use regex::RegexBuilder;
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

/// Search failures.
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    /// The pattern is not a valid regular expression.
    #[error("invalid pattern: {0}")]
    InvalidPattern(#[from] regex::Error),
}

/// Every match, in document order. Matches do not overlap.
pub fn find_all(doc: &Document, query: &SearchQuery) -> Result<Vec<CharRange>, SearchError> {
    if query.pattern.is_empty() {
        return Ok(Vec::new());
    }
    let mut pat = if query.regex {
        query.pattern.clone()
    } else {
        regex::escape(&query.pattern)
    };
    if query.whole_word {
        pat = format!(r"\b(?:{pat})\b");
    }
    let re = RegexBuilder::new(&pat)
        .case_insensitive(!query.case_sensitive)
        .build()?;
    let text = doc.text().to_string();
    // Byte offsets to char offsets in one forward pass.
    let mut out = Vec::new();
    let mut chars_before = 0usize;
    let mut last_byte = 0usize;
    for m in re.find_iter(&text) {
        if m.start() == m.end() {
            continue;
        }
        chars_before += text[last_byte..m.start()].chars().count();
        let len = m.as_str().chars().count();
        out.push(CharRange::new(chars_before, chars_before + len));
        chars_before += len;
        last_byte = m.end();
    }
    Ok(out)
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

    #[test]
    fn backward_search_goes_backward() {
        let d = Document::from_plain_text("cat dog cat dog cat");
        let q = SearchQuery {
            pattern: "cat".into(),
            direction: Direction::Backward,
            ..SearchQuery::default()
        };
        let (r, wrapped) = find(&d, CharPos(12), &q).unwrap().unwrap();
        assert_eq!((r, wrapped), (CharRange::new(8, 11), false));
    }

    #[test]
    fn whole_word_and_case() {
        let d = Document::from_plain_text("Cat category cat");
        let q = SearchQuery {
            pattern: "cat".into(),
            whole_word: true,
            ..SearchQuery::default()
        };
        assert_eq!(find_all(&d, &q).unwrap().len(), 2);
        let q = SearchQuery {
            case_sensitive: true,
            ..q
        };
        assert_eq!(find_all(&d, &q).unwrap(), vec![CharRange::new(13, 16)]);
    }
}
