//! Find in reading mode, streamed over the document's rope (Phase 2).
//!
//! `textweaver_text::find_all` searches a copy of the whole text and
//! returns every match: on a 10 MB file that is a 10 MB copy and, for a
//! common word, tens of thousands of stored ranges. This scan keeps the
//! same matching rules (the query compiles to the same regular
//! expression: whitespace-tolerant plain text, case folding, multi-line
//! `^` and `$`, whole words, non-overlapping leftmost matches) but reads
//! the rope a window at a time and hands each match to a callback, so
//! callers keep only what they need: a count, and the matches near the
//! cursor.
//!
//! Windows overlap by [`OVERLAP`] characters, and each is searched with the
//! character before it, so `^` and `\b` see the same context as in the
//! whole text. A match is kept from a window only when it starts before
//! the overlap (the next window sees it whole otherwise), and a match that
//! reaches the end of a window is searched again in a longer one, so
//! greedy patterns are not cut short and `$` sees the next character. A
//! single match longer than [`OVERLAP`] characters that crosses a window
//! edge could be missed; no real search comes near that.

use std::collections::VecDeque;
use std::ops::ControlFlow;

use ropey::Rope;
use textweaver_core::{CharPos, CharRange};
use textweaver_text::{SearchError, SearchQuery};

/// Characters searched per window (about 1 MB of ASCII).
pub(crate) const WINDOW: usize = 1 << 20;

/// Characters two windows share, so a match that crosses a window edge is
/// seen whole by the next window.
pub(crate) const OVERLAP: usize = 64 * 1024;

/// Most matches a search keeps in memory. More are counted, and the ones
/// kept are those around the cursor; stepping past them searches again.
pub(crate) const MAX_STORED_HITS: usize = 10_000;

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Calls `visit` with every match of `query` in `text`, in order, until it
/// returns [`ControlFlow::Break`]. Matches do not overlap; empty matches are
/// skipped. The same matches as `textweaver_text::find_all`.
pub(crate) fn scan(
    text: &Rope,
    query: &SearchQuery,
    mut visit: impl FnMut(CharRange) -> ControlFlow<()>,
) -> Result<(), SearchError> {
    if query.pattern.is_empty() {
        return Ok(());
    }
    let re = query.compile()?;
    let len = text.len_chars();
    // The window's first char, and the first char a match may start at.
    let mut start = 0usize;
    let mut resume = 0usize;
    let mut buf = String::new();
    loop {
        let mut size = WINDOW;
        let keep_before = 'grow: loop {
            let end = start.saturating_add(size).min(len);
            let last = end >= len;
            // Matches must start before this to be kept from this window;
            // later ones are seen whole by the next window.
            let keep_before = if last {
                usize::MAX
            } else {
                end.saturating_sub(OVERLAP).max(start + 1)
            };
            // One char before the window gives `^` and `\b` their context.
            let ctx = start.saturating_sub(1);
            buf.clear();
            for chunk in text.slice(ctx..end).chunks() {
                buf.push_str(chunk);
            }
            let mut chars_before = ctx;
            let mut counted_to = 0usize;
            let mut at = text.char_to_byte(resume.max(start)) - text.char_to_byte(ctx);
            while at <= buf.len() {
                let Some(m) = re.find_at(&buf, at) else {
                    break;
                };
                if m.start() == m.end() {
                    at = next_boundary(&buf, m.end());
                    continue;
                }
                if m.end() == buf.len() && !last {
                    // The match might go on past the window, and `$` or
                    // `\b` at its end needs the next char: a longer window.
                    size = size.saturating_mul(2);
                    continue 'grow;
                }
                chars_before += buf[counted_to..m.start()].chars().count();
                counted_to = m.start();
                if chars_before >= keep_before {
                    break;
                }
                let n = m.as_str().chars().count();
                if query.whole_word {
                    let before = chars_before
                        .checked_sub(1)
                        .map(|i| text.char(i))
                        .is_some_and(is_word_char);
                    let after = chars_before + n < len && is_word_char(text.char(chars_before + n));
                    if before || after {
                        // Retry one char further on, so a rejected match
                        // does not hide a valid one that overlaps it.
                        at = next_boundary(&buf, m.start());
                        continue;
                    }
                }
                if visit(CharRange::new(chars_before, chars_before + n)).is_break() {
                    return Ok(());
                }
                chars_before += n;
                counted_to = m.end();
                at = m.end();
                resume = chars_before;
            }
            if last {
                return Ok(());
            }
            break keep_before;
        };
        start = keep_before;
        resume = resume.max(start);
    }
}

fn next_boundary(text: &str, byte: usize) -> usize {
    text[byte..]
        .chars()
        .next()
        .map_or(text.len() + 1, |c| byte + c.len_utf8())
}

/// The outcome of [`collect_around`]: the total count, and up to a cap of
/// matches around a position.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Around {
    /// Every match in the document.
    pub(crate) total: usize,
    /// The index (among all matches) of `hits[0]`.
    pub(crate) first_index: usize,
    /// Matches kept, in order.
    pub(crate) hits: Vec<CharRange>,
}

/// Counts every match and keeps up to `cap` of them: at most half before
/// `pos`, and the rest from `pos` on.
pub(crate) fn collect_around(
    text: &Rope,
    query: &SearchQuery,
    pos: CharPos,
    cap: usize,
) -> Result<Around, SearchError> {
    let cap = cap.max(2);
    let mut before: VecDeque<CharRange> = VecDeque::new();
    let mut after: Vec<CharRange> = Vec::new();
    let mut total = 0usize;
    let mut before_count = 0usize;
    scan(text, query, |r| {
        total += 1;
        if r.start < pos {
            before_count += 1;
            before.push_back(r);
            if before.len() > cap / 2 {
                before.pop_front();
            }
        } else if after.len() + before.len() < cap {
            after.push(r);
        }
        ControlFlow::Continue(())
    })?;
    let first_index = before_count - before.len();
    let mut hits: Vec<CharRange> = before.into_iter().collect();
    hits.extend(after);
    Ok(Around {
        total,
        first_index,
        hits,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use textweaver_text::{Document, find_all};

    fn all(text: &Rope, q: &SearchQuery) -> Vec<CharRange> {
        let mut v = Vec::new();
        scan(text, q, |r| {
            v.push(r);
            ControlFlow::Continue(())
        })
        .unwrap();
        v
    }

    fn same_as_find_all(text: &str, q: &SearchQuery) {
        let doc = Document::from_plain_text(text);
        let want = find_all(&doc, q).unwrap();
        let got = all(doc.text(), q);
        assert_eq!(
            got.len(),
            want.len(),
            "{:?} in {} bytes",
            q.pattern,
            text.len()
        );
        assert_eq!(got, want, "{:?} in {} bytes", q.pattern, text.len());
    }

    fn queries() -> Vec<SearchQuery> {
        let plain = SearchQuery::plain;
        vec![
            plain("the"),
            plain("reading room"),
            plain("\u{130}"),
            SearchQuery {
                whole_word: true,
                ..plain("cat")
            },
            SearchQuery {
                regex: true,
                ..plain(r"^\w+$")
            },
            SearchQuery {
                regex: true,
                ..plain(r"\bcat\b")
            },
            SearchQuery {
                regex: true,
                ..plain(r"x\s+y")
            },
            SearchQuery {
                regex: true,
                ..plain(r"b+")
            },
            SearchQuery {
                case_sensitive: true,
                ..plain("The")
            },
        ]
    }

    #[test]
    fn small_texts_match_find_all() {
        let texts = [
            "",
            "the cat, the catalog, cat_x, cat.",
            "reading\nroom and reading   room",
            "\u{130}stanbul i\u{307} \u{130}",
            "one\ntwo words\nthree\n",
            "x \n\n  y and x y",
            "abbbbc bb b",
            "The the THE",
        ];
        for t in texts {
            for q in queries() {
                same_as_find_all(t, &q);
            }
        }
    }

    #[test]
    fn matches_across_window_edges_are_found_whole() {
        // Varied line lengths so window edges fall everywhere; matches that
        // cross lines (whitespace runs, long b runs) straddle the edges.
        let mut text = String::new();
        let mut i = 0u64;
        while text.len() < 3 * WINDOW + 12345 {
            i += 1;
            let n = (i * 7919 % 97) as usize;
            text.push_str(&"word ".repeat(n % 13));
            if i % 5 == 0 {
                text.push_str("reading\n  room ");
            }
            if i % 11 == 0 {
                text.push_str(&"b".repeat(n * 3));
            }
            if i % 17 == 0 {
                text.push_str("x\n\n y ");
            }
            text.push_str("the cat\n");
        }
        for q in queries() {
            same_as_find_all(&text, &q);
        }
    }

    #[test]
    fn one_long_line_and_long_matches_read_whole() {
        // No line breaks at all, like a 1 MB one-line file.
        let text = "the catalog cat ".repeat(3 * WINDOW / 16 + 7);
        for q in queries() {
            same_as_find_all(&text, &q);
        }
        let mut text = "a ".repeat(10);
        text.push_str(&"b".repeat(WINDOW + 100));
        text.push_str(" c");
        let q = SearchQuery {
            regex: true,
            ..SearchQuery::plain("b+")
        };
        same_as_find_all(&text, &q);
    }

    #[test]
    fn collect_around_keeps_a_window_and_counts_the_rest() {
        let text = "x ".repeat(1000);
        let rope = Rope::from_str(&text);
        let q = SearchQuery::plain("x");
        let a = collect_around(&rope, &q, CharPos(1000), 100).unwrap();
        assert_eq!(a.total, 1000);
        assert_eq!(a.hits.len(), 100);
        assert_eq!(a.first_index, 450);
        assert_eq!(a.hits[0].start, CharPos(900));
        assert!(a.hits.windows(2).all(|w| w[0].start < w[1].start));
        let a = collect_around(&rope, &q, CharPos(0), 100).unwrap();
        assert_eq!((a.first_index, a.hits.len()), (0, 100));
        let a = collect_around(&rope, &q, CharPos(2000), 100).unwrap();
        assert_eq!((a.first_index, a.hits.len()), (950, 50));
    }
}
