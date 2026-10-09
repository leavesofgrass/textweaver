//! How Find and Replace match: case, whole words, regular expressions, and
//! matches across lines (task B1-fr).
//!
//! One set of options serves reading Find (Ctrl+F), the replace loop
//! (Alt+F in edit mode), and the window's find and replace panel. The
//! options last for the session; they start off, and the search options
//! list (the Find menu and the palette) or the replace loop's keys change
//! them.
//!
//! - **Plain text** (regular expression off) matches as before: in edit
//!   mode exactly as typed (`textweaver_editor::find`), when reading with
//!   whitespace runs matching any whitespace.
//! - **Regular expressions** use the `regex` crate through the same
//!   streamed scan as reading Find (`crate::find_scan`): `^` and `$` match
//!   at line starts and ends, and `\n` or `\s` may match a line break. The
//!   replacement may use capture groups: `$1`, `${name}`, and `$$` for a
//!   dollar sign.
//! - **Across lines** also lets `.` match a line break, so a match can run
//!   on from one line to the next (the `s` flag).
//! - **An invalid pattern** is said in words with the character where it
//!   fails ("Invalid pattern at character 4: unclosed group."), never a
//!   crash and never a silent plain search.

use std::ops::ControlFlow;

use ropey::Rope;
use serde::{Deserialize, Serialize};
use textweaver_core::CharRange;
use textweaver_editor::FindOptions;
use textweaver_text::{SearchError, SearchQuery};

/// How Find and Replace match ([`crate::App::search_options`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchOptions {
    /// Match case exactly; off, `cat` also finds `Cat`.
    pub match_case: bool,
    /// Whole words only; on, `cat` does not find `catalog`.
    pub whole_words: bool,
    /// The pattern is a regular expression, and the replacement may use
    /// its capture groups (`$1`, `${name}`, `$$` for a dollar sign).
    pub regex: bool,
    /// With a regular expression, `.` matches a line break too, so a
    /// match may run across lines.
    pub across_lines: bool,
}

/// Why a pattern cannot be searched, in parts a message puts in words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PatternProblem {
    /// The character where the pattern fails, counting from 1, when the
    /// engine says where.
    pub(crate) at: Option<usize>,
    /// The engine's reason, such as "unclosed group".
    pub(crate) reason: String,
}

impl SearchOptions {
    /// The options of the plain matcher used in edit mode.
    pub(crate) fn find_options(self) -> FindOptions {
        FindOptions {
            case_sensitive: self.match_case,
            whole_word: self.whole_words,
        }
    }

    /// The query reading Find and regular expressions run for `pattern`.
    pub(crate) fn query(self, pattern: &str) -> SearchQuery {
        let pattern = if self.regex && self.across_lines {
            format!("(?s){pattern}")
        } else {
            pattern.to_owned()
        };
        SearchQuery {
            pattern,
            regex: self.regex,
            case_sensitive: self.match_case,
            whole_word: self.whole_words,
            ..SearchQuery::plain("")
        }
    }

    /// The message ids of the options that are on, in their order.
    pub(crate) fn on(self) -> Vec<&'static str> {
        [
            (self.match_case, "search-option-match-case"),
            (self.whole_words, "search-option-whole-words"),
            (self.regex, "search-option-regex"),
            (
                self.regex && self.across_lines,
                "search-option-across-lines",
            ),
        ]
        .into_iter()
        .filter_map(|(on, id)| on.then_some(id))
        .collect()
    }
}

/// Why `pattern` cannot be searched with `opts`, if it cannot. Plain text
/// always can.
pub(crate) fn pattern_problem(pattern: &str, opts: SearchOptions) -> Option<PatternProblem> {
    if !opts.regex || pattern.is_empty() {
        return None;
    }
    // The user's pattern alone, so the place is counted in what was typed.
    let query = SearchOptions {
        across_lines: false,
        ..opts
    }
    .query(pattern);
    match query.compile() {
        Ok(_) => None,
        Err(SearchError::InvalidPattern(e)) => Some(problem_of(&e)),
    }
}

/// The place and reason of a `regex` error. A syntax error's text shows
/// the pattern, a line of carets under the failing part, and
/// `error: reason`; the caret's column is the place, in characters.
fn problem_of(e: &regex::Error) -> PatternProblem {
    let text = e.to_string();
    let lines: Vec<&str> = text.lines().collect();
    let at = lines.windows(2).find_map(|w| {
        // The pattern line and the caret line are indented four spaces.
        let carets = w[1].strip_prefix("    ")?;
        let ok = w[0].starts_with("    ")
            && carets.trim_start().starts_with('^')
            && carets.trim().chars().all(|c| c == '^');
        ok.then(|| carets.chars().take_while(|c| *c == ' ').count() + 1)
    });
    let reason = lines
        .iter()
        .find_map(|l| l.strip_prefix("error: "))
        .map_or_else(
            || text.split_whitespace().collect::<Vec<_>>().join(" "),
            |r| r.trim().trim_end_matches('.').to_owned(),
        );
    PatternProblem { at, reason }
}

/// Calls `visit` with every match of `pattern` in `text` with `opts`, in
/// order, until it breaks. An invalid pattern has no matches (check it
/// with [`pattern_problem`] first, to say why).
pub(crate) fn each_match(
    text: &Rope,
    pattern: &str,
    opts: SearchOptions,
    visit: impl FnMut(CharRange) -> ControlFlow<()>,
) {
    if opts.regex {
        let _ = crate::find_scan::scan(text, &opts.query(pattern), visit);
    } else {
        textweaver_editor::find::for_each_match(text, pattern, opts.find_options(), visit);
    }
}

/// Every match of `pattern` in `text`.
pub(crate) fn all_matches(text: &Rope, pattern: &str, opts: SearchOptions) -> Vec<CharRange> {
    let mut out = Vec::new();
    each_match(text, pattern, opts, |r| {
        out.push(r);
        ControlFlow::Continue(())
    });
    out
}

/// How many matches of `pattern` `text` has.
pub(crate) fn count(text: &Rope, pattern: &str, opts: SearchOptions) -> usize {
    let mut n = 0;
    each_match(text, pattern, opts, |_| {
        n += 1;
        ControlFlow::Continue(())
    });
    n
}

/// What the match `m` of `pattern` becomes when replaced with `with`: the
/// replacement as typed for plain text; for a regular expression, with
/// its capture groups filled in (`$1`, `${name}`, `$$`).
pub(crate) fn replacement(
    text: &Rope,
    m: CharRange,
    pattern: &str,
    with: &str,
    opts: SearchOptions,
) -> String {
    if !opts.regex || !with.contains('$') {
        return with.to_owned();
    }
    let Ok(re) = opts.query(pattern).compile() else {
        return with.to_owned();
    };
    let len = text.len_chars();
    let (s, e) = (m.start.0.min(len), m.end.0.min(len));
    // The match's lines, with one char on each side, so `^`, `$`, `\b`,
    // `\A` and `\z` see what they saw in the whole text.
    let first = text.char_to_line(s);
    let last = text.char_to_line(e);
    let from = text.line_to_char(first).saturating_sub(1);
    let to = if last + 1 < text.len_lines() {
        (text.line_to_char(last + 1) + 1).min(len)
    } else {
        len
    };
    let expand = |hay: &str, at: usize| -> Option<String> {
        let caps = re.captures_at(hay, at)?;
        let whole = caps.get(0)?;
        let same = whole.start() == at && hay[at..whole.end()].chars().count() == e - s;
        same.then(|| {
            let mut out = String::new();
            caps.expand(with, &mut out);
            out
        })
    };
    let hay = text.slice(from..to).to_string();
    let at = text.char_to_byte(s) - text.char_to_byte(from);
    if let Some(out) = expand(&hay, at) {
        return out;
    }
    // shortcut: a match that depends on text beyond its own lines searches
    // the whole text again; fine one step at a time, upgrade by keeping
    // the captures from the scan if this ever shows in a profile.
    let hay = text.to_string();
    expand(&hay, text.char_to_byte(s)).unwrap_or_else(|| with.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATE: &str = r"(?<y>\d{4})-(\d\d)-(\d\d)";

    fn re() -> SearchOptions {
        SearchOptions {
            regex: true,
            ..SearchOptions::default()
        }
    }

    #[test]
    fn regex_finds_and_captures() {
        let t = Rope::from_str("color, colour\nColor 2026-10-09\n");
        assert_eq!(all_matches(&t, "colou?r", re()).len(), 3);
        let case = SearchOptions {
            match_case: true,
            ..re()
        };
        assert_eq!(all_matches(&t, "colou?r", case).len(), 2);
        let d = all_matches(&t, DATE, re());
        assert_eq!(d.len(), 1);
        assert_eq!(
            replacement(&t, d[0], DATE, "$3/$2/${y} $$5", re()),
            "09/10/2026 $5"
        );
        // Plain replacements keep dollar signs as typed.
        let plain = SearchOptions::default();
        let p = all_matches(&t, "color", plain);
        assert_eq!(replacement(&t, p[0], "color", "$1", plain), "$1");
    }

    #[test]
    fn regex_with_whole_words_and_line_anchors() {
        let t = Rope::from_str("cat catalog\ncat. scat\n");
        let whole = SearchOptions {
            whole_words: true,
            ..re()
        };
        assert_eq!(
            all_matches(&t, "c.t", whole),
            vec![CharRange::new(0, 3), CharRange::new(12, 15)]
        );
        assert_eq!(
            all_matches(&t, "^cat", re()),
            vec![CharRange::new(0, 3), CharRange::new(12, 15)]
        );
        // `^` in the replacement's context is still a line start.
        assert_eq!(
            replacement(&t, CharRange::new(12, 15), "^(c)at", "${1}ow", re()),
            "cow"
        );
    }

    #[test]
    fn across_lines_lets_a_dot_match_a_line_break() {
        let t = Rope::from_str("one\ntwo\n");
        assert!(all_matches(&t, "one.two", re()).is_empty());
        let across = SearchOptions {
            across_lines: true,
            ..re()
        };
        assert_eq!(
            all_matches(&t, "one.two", across),
            vec![CharRange::new(0, 7)]
        );
        assert_eq!(
            replacement(&t, CharRange::new(0, 7), "(o..).(t..)", "$2 $1", across),
            "two one"
        );
        // `\n` matches a line break without the option.
        assert_eq!(count(&t, r"one\ntwo", re()), 1);
    }

    #[test]
    fn invalid_patterns_say_where_they_fail() {
        let p = pattern_problem("ab(cd", re()).unwrap();
        assert_eq!(p.at, Some(3));
        assert_eq!(p.reason, "unclosed group");
        let p = pattern_problem("x{2,1}", re()).unwrap();
        assert!(p.at.is_some(), "{p:?}");
        assert!(!p.reason.is_empty() && !p.reason.contains('\n'), "{p:?}");
        // Across lines does not shift the place.
        let across = SearchOptions {
            across_lines: true,
            ..re()
        };
        assert_eq!(pattern_problem("ab(cd", across).unwrap().at, Some(3));
        // Plain text is never invalid, and an invalid pattern finds nothing.
        assert_eq!(pattern_problem("ab(cd", SearchOptions::default()), None);
        assert_eq!(count(&Rope::from_str("ab(cd"), "ab(cd", re()), 0);
    }
}
