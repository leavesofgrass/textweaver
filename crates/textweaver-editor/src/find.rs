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

/// Every non-overlapping match of `query` in `text`, in order.
pub fn find_all(text: &Rope, query: &str, opts: FindOptions) -> Vec<CharRange> {
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
    // Folded haystack plus the folded byte offset where each char starts.
    let mut hay = String::new();
    let mut starts: Vec<usize> = Vec::with_capacity(chars.len() + 1);
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
        let (Ok(s), Ok(end)) = (starts.binary_search(&b), starts.binary_search(&e)) else {
            // The match starts or ends inside one char's folded form.
            from = b + hay[b..].chars().next().map_or(1, char::len_utf8);
            continue;
        };
        let bounded = !opts.whole_word
            || ((s == 0 || !is_word(chars[s - 1])) && (end >= chars.len() || !is_word(chars[end])));
        if bounded {
            out.push(CharRange::new(s, end));
            from = e;
        } else {
            from = b + hay[b..].chars().next().map_or(1, char::len_utf8);
        }
    }
    out
}

/// The first match starting at or after `from`, wrapping to the first match
/// when `wrap` is set.
pub fn find_next(
    text: &Rope,
    query: &str,
    from: CharPos,
    opts: FindOptions,
    wrap: bool,
) -> Option<CharRange> {
    let all = find_all(text, query, opts);
    all.iter()
        .find(|r| r.start >= from)
        .or(if wrap { all.first() } else { None })
        .copied()
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
    let all = find_all(text, query, opts);
    all.iter()
        .rev()
        .find(|r| r.start < from)
        .or(if wrap { all.last() } else { None })
        .copied()
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
    let all = find_all(ed.text(), query, opts);
    let target = all
        .iter()
        .find(|r| **r == sel)
        .or_else(|| all.iter().find(|r| r.start >= sel.start))
        .or(all.first())
        .copied();
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
    let all = find_all(ed.text(), query, opts);
    if all.is_empty() {
        return Ok(0);
    }
    // Back to front, so each range is still valid when its turn comes.
    let edits: Vec<Edit> = all
        .iter()
        .rev()
        .map(|r| Edit::replace(*r, replacement))
        .collect();
    ed.apply_group(edits)?;
    Ok(all.len())
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
