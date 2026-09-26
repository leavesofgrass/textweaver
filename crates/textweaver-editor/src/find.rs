//! Find and replace in edit mode (Star's `gui/mixin_find.py`, Part 3 §4.4),
//! with its offset bugs fixed (§7 item 35):
//!
//! - matches never overlap, so Replace All cannot garble self-overlapping
//!   text (`aa` in `aaa` is one match);
//! - case-insensitive matching folds each character separately and maps
//!   matches back to char offsets, so characters whose lowercase form has a
//!   different length (`İ`) do not shift later matches;
//! - positions are char offsets throughout (Star mixed code points with
//!   Qt's UTF-16 positions);
//! - "replace one" replaces the match at the caret instead of skipping it.

use std::ops::ControlFlow;

use ropey::Rope;
use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange, CoreError, Edit};

use crate::{Editor, Selection};

/// How to match.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FindOptions {
    /// Match case exactly (Star: always case-insensitive).
    pub case_sensitive: bool,
    /// Match whole words only.
    pub whole_word: bool,
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Characters folded and searched at a time: memory stays bounded however
/// long the text is.
const WINDOW: usize = 256 * 1024;

/// Appends `c` as matching sees it: lowercased unless `case_sensitive`.
/// No allocation per character (the first port built a string for each).
fn push_folded(out: &mut String, c: char, opts: FindOptions) {
    if opts.case_sensitive {
        out.push(c);
    } else {
        out.extend(c.to_lowercase());
    }
}

/// Calls `visit` with every non-overlapping match of `query` in `text`, in
/// order, until it returns [`ControlFlow::Break`].
///
/// The text is folded and searched a window at a time, straight from the
/// rope; windows overlap by more than a match can span, so no match is
/// cut. A match must start and end on whole characters of the original
/// text (a folded form longer than its character cannot be half matched).
pub fn for_each_match(
    text: &Rope,
    query: &str,
    opts: FindOptions,
    mut visit: impl FnMut(CharRange) -> ControlFlow<()>,
) {
    if query.is_empty() {
        return;
    }
    let mut needle = String::new();
    for c in query.chars() {
        push_folded(&mut needle, c, opts);
    }
    // A match covers at most one character per folded byte of the needle.
    let overlap = needle.len() + 1;
    let len = text.len_chars();
    let mut hay = String::new();
    // Folded byte offset where each character of the window starts, and
    // one past the end.
    let mut starts: Vec<usize> = Vec::new();
    let mut start = 0usize;
    let mut resume = 0usize;
    loop {
        let end = start.saturating_add(WINDOW).min(len);
        let last = end >= len;
        let keep_before = if last {
            usize::MAX
        } else {
            end.saturating_sub(overlap).max(start + 1)
        };
        hay.clear();
        starts.clear();
        for c in text.slice(start..end).chars() {
            starts.push(hay.len());
            push_folded(&mut hay, c, opts);
        }
        starts.push(hay.len());
        let mut from = starts[resume.saturating_sub(start).min(starts.len() - 1)];
        while let Some(i) = hay[from..].find(&needle) {
            let b = from + i;
            let e = b + needle.len();
            let step = b + hay[b..].chars().next().map_or(1, char::len_utf8);
            let (Ok(s), Ok(en)) = (starts.binary_search(&b), starts.binary_search(&e)) else {
                // The match starts or ends inside one char's folded form.
                from = step;
                continue;
            };
            let (s, en) = (start + s, start + en);
            if s >= keep_before {
                break;
            }
            let bounded = !opts.whole_word
                || ((s == 0 || !is_word(text.char(s - 1)))
                    && (en >= len || !is_word(text.char(en))));
            if bounded {
                if visit(CharRange::new(s, en)).is_break() {
                    return;
                }
                from = e;
                resume = en;
            } else {
                from = step;
            }
        }
        if last {
            return;
        }
        start = keep_before;
        resume = resume.max(start);
    }
}

/// Every non-overlapping match of `query` in `text`, in order.
pub fn find_all(text: &Rope, query: &str, opts: FindOptions) -> Vec<CharRange> {
    let mut out = Vec::new();
    for_each_match(text, query, opts, |r| {
        out.push(r);
        ControlFlow::Continue(())
    });
    out
}

/// How many non-overlapping matches `text` has, without keeping them.
pub fn count_matches(text: &Rope, query: &str, opts: FindOptions) -> usize {
    let mut n = 0;
    for_each_match(text, query, opts, |_| {
        n += 1;
        ControlFlow::Continue(())
    });
    n
}

/// The first match starting at or after `from`, wrapping to the first match
/// when `wrap` is set. Stops searching at the answer.
pub fn find_next(
    text: &Rope,
    query: &str,
    from: CharPos,
    opts: FindOptions,
    wrap: bool,
) -> Option<CharRange> {
    let mut first = None;
    let mut found = None;
    for_each_match(text, query, opts, |r| {
        first.get_or_insert(r);
        if r.start >= from {
            found = Some(r);
            return ControlFlow::Break(());
        }
        ControlFlow::Continue(())
    });
    found.or(if wrap { first } else { None })
}

/// The last match starting before `from`, wrapping to the last match when
/// `wrap` is set.
pub fn find_previous(
    text: &Rope,
    query: &str,
    from: CharPos,
    opts: FindOptions,
    wrap: bool,
) -> Option<CharRange> {
    let mut before = None;
    let mut last = None;
    for_each_match(text, query, opts, |r| {
        if r.start < from {
            before = Some(r);
        } else if !wrap {
            return ControlFlow::Break(());
        }
        last = Some(r);
        ControlFlow::Continue(())
    });
    before.or(if wrap { last } else { None })
}

/// Replaces the match at the selection: the selection itself when it is a
/// match, else the first match at or after the caret (wrapping). One undo
/// step. Selects the next match afterwards, if any. Returns the replaced
/// range (pre-edit), or `None` when nothing matched.
pub fn replace_one(
    ed: &mut Editor,
    query: &str,
    replacement: &str,
    opts: FindOptions,
) -> Result<Option<CharRange>, CoreError> {
    let sel = ed.selection().range();
    // The selection when it is a match, else the first match at or after
    // it, else (wrapping) the first match: one pass, stopping early.
    let mut first = None;
    let mut target = None;
    for_each_match(ed.text(), query, opts, |r| {
        first.get_or_insert(r);
        if r == sel || r.start >= sel.start {
            target = Some(r);
            return ControlFlow::Break(());
        }
        ControlFlow::Continue(())
    });
    let target = target.or(first);
    let Some(target) = target else {
        return Ok(None);
    };
    let out = ed.apply(Edit::replace(target, replacement))?;
    let after = out.inserted.end;
    let next = find_next(ed.text(), query, after, opts, true);
    ed.set_selection(match next {
        Some(r) => Selection::new(r.start, r.end),
        None => Selection::caret(after),
    });
    Ok(Some(target))
}

/// Replaces every match as one undo step. Returns the number replaced.
pub fn replace_all(
    ed: &mut Editor,
    query: &str,
    replacement: &str,
    opts: FindOptions,
) -> Result<usize, CoreError> {
    Ok(replace_all_ranges(ed, query, replacement, opts)?.len())
}

/// Replaces every match as one undo step, searching once. Returns the
/// ranges replaced (in the text before the edit, in document order), so a
/// caller can shift its own positions without searching again.
pub fn replace_all_ranges(
    ed: &mut Editor,
    query: &str,
    replacement: &str,
    opts: FindOptions,
) -> Result<Vec<CharRange>, CoreError> {
    let all = find_all(ed.text(), query, opts);
    if all.is_empty() {
        return Ok(all);
    }
    // Back to front, so each range is still valid when its turn comes.
    let edits: Vec<Edit> = all
        .iter()
        .rev()
        .map(|r| Edit::replace(*r, replacement))
        .collect();
    ed.apply_group(edits)?;
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rope(s: &str) -> Rope {
        Rope::from_str(s)
    }

    fn ci() -> FindOptions {
        FindOptions::default()
    }

    #[test]
    fn non_overlapping_matches() {
        assert_eq!(
            find_all(&rope("aaa"), "aa", ci()),
            vec![CharRange::new(0, 2)]
        );
        let mut ed = Editor::new("aaa");
        assert_eq!(replace_all(&mut ed, "aa", "b", ci()).unwrap(), 1);
        assert_eq!(ed.text().to_string(), "ba");
    }

    #[test]
    fn case_folding_keeps_offsets() {
        // `İ` lowercases to two chars; later matches must not shift.
        let t = rope("İx cat Cat");
        assert_eq!(
            find_all(&t, "cat", ci()),
            vec![CharRange::new(3, 6), CharRange::new(7, 10)]
        );
        assert_eq!(
            find_all(
                &t,
                "cat",
                FindOptions {
                    case_sensitive: true,
                    whole_word: false
                }
            ),
            vec![CharRange::new(3, 6)]
        );
        assert_eq!(find_all(&t, "i", ci()), vec![]);
    }

    #[test]
    fn whole_words() {
        let opts = FindOptions {
            whole_word: true,
            ..ci()
        };
        assert_eq!(
            find_all(&rope("cat catalog cat_x cat."), "cat", opts),
            vec![CharRange::new(0, 3), CharRange::new(18, 21)]
        );
    }

    #[test]
    fn next_previous_and_wrap() {
        let t = rope("a b a b a");
        assert_eq!(
            find_next(&t, "a", CharPos(1), ci(), false),
            Some(CharRange::new(4, 5))
        );
        assert_eq!(find_next(&t, "a", CharPos(9), ci(), false), None);
        assert_eq!(
            find_next(&t, "a", CharPos(9), ci(), true),
            Some(CharRange::new(0, 1))
        );
        assert_eq!(
            find_previous(&t, "a", CharPos(4), ci(), false),
            Some(CharRange::new(0, 1))
        );
        assert_eq!(
            find_previous(&t, "a", CharPos(0), ci(), true),
            Some(CharRange::new(8, 9))
        );
        assert_eq!(find_all(&t, "", ci()), vec![]);
    }

    /// The first port's algorithm: the whole text folded at once.
    fn reference(text: &str, query: &str, opts: FindOptions) -> Vec<CharRange> {
        if query.is_empty() {
            return Vec::new();
        }
        let chars: Vec<char> = text.chars().collect();
        let fold = |c: char| -> String {
            if opts.case_sensitive {
                c.to_string()
            } else {
                c.to_lowercase().collect()
            }
        };
        let mut hay = String::new();
        let mut starts = Vec::new();
        for &c in &chars {
            starts.push(hay.len());
            hay.push_str(&fold(c));
        }
        starts.push(hay.len());
        let needle: String = query.chars().map(fold).collect();
        let mut out = Vec::new();
        let mut from = 0;
        while let Some(i) = hay[from..].find(&needle) {
            let b = from + i;
            let e = b + needle.len();
            let step = b + hay[b..].chars().next().map_or(1, char::len_utf8);
            let (Ok(s), Ok(end)) = (starts.binary_search(&b), starts.binary_search(&e)) else {
                from = step;
                continue;
            };
            let ok = !opts.whole_word
                || ((s == 0 || !is_word(chars[s - 1]))
                    && (end >= chars.len() || !is_word(chars[end])));
            if ok {
                out.push(CharRange::new(s, end));
                from = e;
            } else {
                from = step;
            }
        }
        out
    }

    #[test]
    fn windows_find_what_the_whole_text_finds() {
        // Over three windows, with matches (and folding characters that
        // change length) falling on every window edge.
        let mut text = String::new();
        let mut i = 0u64;
        while text.chars().count() < 3 * WINDOW + 999 {
            i += 1;
            text.push_str(["cat ", "Cat", "\u{130}x", "catalog ", "aaa", "\n"][(i % 6) as usize]);
            if i % 7 == 0 {
                text.push_str(&"a".repeat((i % 23) as usize));
            }
        }
        let rope = Rope::from_str(&text);
        let opts = [
            ci(),
            FindOptions {
                case_sensitive: true,
                whole_word: false,
            },
            FindOptions {
                case_sensitive: false,
                whole_word: true,
            },
        ];
        for q in ["cat", "aa", "\u{130}x", "a\ncat", "t c"] {
            for o in opts {
                let want = reference(&text, q, o);
                assert_eq!(find_all(&rope, q, o), want, "{q:?} {o:?}");
                assert_eq!(count_matches(&rope, q, o), want.len());
            }
        }
        let mid = CharPos(text.chars().count() / 2);
        let all = find_all(&rope, "cat", ci());
        let next = all.iter().find(|r| r.start >= mid).copied();
        assert_eq!(find_next(&rope, "cat", mid, ci(), true), next);
        let prev = all.iter().rev().find(|r| r.start < mid).copied();
        assert_eq!(find_previous(&rope, "cat", mid, ci(), true), prev);
    }

    #[test]
    fn replace_all_reports_the_ranges_it_replaced() {
        let mut ed = Editor::new("a cat, a Cat.");
        let r = replace_all_ranges(&mut ed, "cat", "dog", ci()).unwrap();
        assert_eq!(r, vec![CharRange::new(2, 5), CharRange::new(9, 12)]);
        assert_eq!(ed.text().to_string(), "a dog, a dog.");
        ed.undo().unwrap();
        assert_eq!(ed.text().to_string(), "a cat, a Cat.");
    }

    #[test]
    fn replace_one_uses_the_match_at_the_caret() {
        let mut ed = Editor::new("x y x y");
        ed.set_selection(Selection::caret(CharPos(1)));
        assert_eq!(
            replace_one(&mut ed, "x", "Z", ci()).unwrap(),
            Some(CharRange::new(4, 5))
        );
        assert_eq!(ed.text().to_string(), "x y Z y");
        // Wrapped to the first match and selected it.
        assert_eq!(ed.selection(), Selection::new(0, 1));
        replace_one(&mut ed, "x", "Z", ci()).unwrap();
        assert_eq!(ed.text().to_string(), "Z y Z y");
        assert_eq!(replace_one(&mut ed, "x", "Z", ci()).unwrap(), None);
    }
}
