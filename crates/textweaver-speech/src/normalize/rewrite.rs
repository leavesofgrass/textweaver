//! A small rewrite engine: regex rules whose replacements keep offset maps.
//!
//! star's normalization is a chain of `re.sub` calls. Each [`Rule`] here is
//! one such substitution, but its replacement is a list of [`Piece`]s: text
//! that is spoken instead of the source ([`Piece::Text`]) and capture groups
//! kept as they are ([`Piece::Keep`]). From that, the engine knows exactly
//! which spoken bytes came from which source chars:
//!
//! - kept groups are `Literal`;
//! - source between kept groups is `Expanded` into the text pieces that sit
//!   between the same groups, or `Elided` when there is no such text;
//! - text with no source between two adjacent groups is `Inserted`.
//!
//! Python's lookbehind and lookahead are not available in the `regex`
//! crate; rules express them as a predicate on the match and its context
//! ([`Rule::accept`]), which may also compute the replacement.

use std::ops::Range;

use regex::{Captures, Regex};
use textweaver_core::{CharPos, CharRange, OffsetMap, SpokenBuilder};

/// One piece of a replacement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Piece {
    /// Capture group `n`, kept literally.
    Keep(usize),
    /// Text spoken in place of source.
    Text(String),
}

/// Parses a Python-style template: `\1` keeps group 1, everything else is
/// text.
pub(crate) fn template(t: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut text = String::new();
    let mut chars = t.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\'
            && let Some(d) = chars.peek().and_then(|d| d.to_digit(10))
        {
            chars.next();
            if !text.is_empty() {
                out.push(Piece::Text(std::mem::take(&mut text)));
            }
            out.push(Piece::Keep(d as usize));
            continue;
        }
        text.push(c);
    }
    if !text.is_empty() {
        out.push(Piece::Text(text));
    }
    out
}

/// What a rule does with one match: `None` rejects it (the search goes on
/// one char later), `Some(pieces)` replaces it.
pub(crate) type Replacer = dyn Fn(&Captures<'_>, &str) -> Option<Vec<Piece>> + Send + Sync;

/// A cheap test on a match (its text) before its capture groups are built:
/// `false` rejects it without allocating.
pub(crate) type Prefilter = dyn Fn(&str) -> bool + Send + Sync;

/// One substitution.
///
/// Applying a rule allocates nothing when it does not change the text: the
/// search uses `find_at`, which needs no capture slots, and capture groups
/// are built only for a match that passes the [`Prefilter`]. A rule whose
/// pattern matches often but rarely applies (a word start, say) should
/// have a prefilter.
pub(crate) struct Rule {
    re: Regex,
    replace: Box<Replacer>,
    prefilter: Option<Box<Prefilter>>,
    /// Trim a leading space of the replacement when the output already ends
    /// in whitespace (or is empty), and a trailing space when the input
    /// continues with whitespace (or ends). Replaces star's global
    /// "collapse spaces and strip" cleanup, which moved every later offset.
    pad: bool,
}

impl std::fmt::Debug for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rule")
            .field("re", &self.re.as_str())
            .finish()
    }
}

impl Rule {
    /// A rule with a fixed template (see [`template`]).
    pub(crate) fn new(pattern: &str, tpl: &str) -> Self {
        let pieces = template(tpl);
        Self::with(pattern, move |_, _| Some(pieces.clone()))
    }

    /// A rule whose replacement is computed (or rejected) per match.
    pub(crate) fn with(
        pattern: &str,
        f: impl Fn(&Captures<'_>, &str) -> Option<Vec<Piece>> + Send + Sync + 'static,
    ) -> Self {
        // Built-in patterns are constants checked by the tests; user input
        // goes through `Rule::try_with` instead.
        let re = Regex::new(pattern).unwrap_or_else(|e| panic!("bad built-in pattern: {e}"));
        Rule {
            re,
            replace: Box::new(f),
            prefilter: None,
            pad: false,
        }
    }

    /// Like [`with`](Self::with) for patterns built from user input.
    pub(crate) fn try_with(
        pattern: &str,
        f: impl Fn(&Captures<'_>, &str) -> Option<Vec<Piece>> + Send + Sync + 'static,
    ) -> Option<Self> {
        let re = Regex::new(pattern).ok()?;
        Some(Rule {
            re,
            replace: Box::new(f),
            prefilter: None,
            pad: false,
        })
    }

    /// Adds a [`Prefilter`]: a match whose text fails it is rejected before
    /// its capture groups are built.
    pub(crate) fn prefiltered(mut self, f: impl Fn(&str) -> bool + Send + Sync + 'static) -> Self {
        self.prefilter = Some(Box::new(f));
        self
    }

    /// Turns on space padding (see the field docs).
    pub(crate) fn padded(mut self) -> Self {
        self.pad = true;
        self
    }

    /// Applies the rule once over `input`. `None` when nothing matched.
    pub(crate) fn apply(&self, input: &str) -> Option<(String, OffsetMap)> {
        let mut b = SpokenBuilder::new();
        let mut cur = 0usize; // bytes of input consumed
        let mut cur_char = 0usize; // chars of input consumed
        let mut pos = 0usize;
        let mut any = false;
        while pos <= input.len() {
            // `find_at` allocates nothing; capture slots are built only for
            // a match that may be replaced.
            let Some(found) = self.re.find_at(input, pos) else {
                break;
            };
            if found.start() == found.end()
                || self.prefilter.as_ref().is_some_and(|f| !f(found.as_str()))
            {
                pos = next_boundary(input, found.start());
                continue;
            }
            let Some(caps) = self.re.captures_at(input, pos) else {
                break;
            };
            let Some(m) = caps.get(0) else {
                break;
            };
            let accepted = if m.start() == m.end() {
                None
            } else {
                (self.replace)(&caps, input)
            };
            let Some(pieces) = accepted else {
                pos = next_boundary(input, m.start());
                continue;
            };
            // Literal text before the match.
            if m.start() > cur {
                b.push_literal(&input[cur..m.start()], CharPos(cur_char));
                cur_char += input[cur..m.start()].chars().count();
                cur = m.start();
            }
            let char_at = |byte: usize| cur_char + input[cur..byte].chars().count();
            self.emit(&mut b, input, &caps, m.range(), pieces, &char_at);
            cur_char = char_at(m.end());
            cur = m.end();
            any = true;
            pos = m.end();
        }
        if !any {
            return None;
        }
        if cur < input.len() {
            b.push_literal(&input[cur..], CharPos(cur_char));
        }
        Some(b.finish())
    }

    fn emit(
        &self,
        b: &mut SpokenBuilder,
        input: &str,
        caps: &Captures<'_>,
        m: Range<usize>,
        pieces: Vec<Piece>,
        char_at: &dyn Fn(usize) -> usize,
    ) {
        // Kept groups in order, and the texts around them.
        let mut keeps: Vec<Range<usize>> = Vec::new();
        let mut texts: Vec<String> = vec![String::new()];
        let mut valid = true;
        for p in &pieces {
            match p {
                Piece::Text(t) => {
                    if let Some(s) = texts.last_mut() {
                        s.push_str(t);
                    }
                }
                Piece::Keep(g) => match caps.get(*g) {
                    Some(k) if k.start() < k.end() => {
                        let after_prev = keeps.last().is_none_or(|prev| prev.end <= k.start());
                        if !after_prev || k.start() < m.start || k.end() > m.end {
                            valid = false;
                        }
                        keeps.push(k.range());
                        texts.push(String::new());
                    }
                    // An empty or missing group contributes nothing.
                    _ => {}
                },
            }
        }
        if self.pad {
            let out_ws = b.text().chars().next_back().is_none_or(char::is_whitespace);
            if out_ws && let Some(first) = texts.first_mut() {
                *first = first.trim_start_matches(' ').to_owned();
            }
            let in_ws = input[m.end..]
                .chars()
                .next()
                .is_none_or(char::is_whitespace);
            if in_ws && let Some(last) = texts.last_mut() {
                *last = last.trim_end_matches(' ').to_owned();
            }
        }
        if !valid {
            // Groups out of order: speak the whole replacement in place of
            // the whole match.
            let mut s = String::new();
            for p in &pieces {
                match p {
                    Piece::Text(t) => s.push_str(t),
                    Piece::Keep(g) => s.push_str(caps.get(*g).map_or("", |k| k.as_str())),
                }
            }
            b.push_expanded(&s, CharRange::new(char_at(m.start), char_at(m.end)));
            return;
        }
        let mut gap_start = m.start;
        for (i, text) in texts.iter().enumerate() {
            let gap_end = keeps.get(i).map_or(m.end, |k| k.start);
            let gap = CharRange::new(char_at(gap_start), char_at(gap_end));
            match (text.is_empty(), gap.is_empty()) {
                (true, true) => {}
                (true, false) => b.push_elided(gap),
                (false, false) => b.push_expanded(text, gap),
                (false, true) => b.push_inserted(text, gap.start),
            }
            if let Some(k) = keeps.get(i) {
                b.push_literal(&input[k.clone()], CharPos(char_at(k.start)));
                gap_start = k.end;
            }
        }
    }
}

/// The byte just after the char starting at `i` (or past the end).
fn next_boundary(s: &str, i: usize) -> usize {
    s[i..]
        .chars()
        .next()
        .map_or(s.len() + 1, |c| i + c.len_utf8())
}

/// The char before byte `i`, if any.
pub(crate) fn char_before(s: &str, i: usize) -> Option<char> {
    s[..i].chars().next_back()
}

/// The char at byte `i`, if any.
pub(crate) fn char_after(s: &str, i: usize) -> Option<char> {
    s[i..].chars().next()
}

/// True for regex `\w` chars (letters, digits, underscore).
pub(crate) fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Applies `rules` in order, each over the previous output, composing the
/// maps. The result maps the final text to `input` chars.
pub(crate) fn apply_rules(input: &str, rules: &[Rule]) -> (String, OffsetMap) {
    apply_rules_changed(input, rules).unwrap_or_else(|| super::identity(input))
}

/// [`apply_rules`], or `None` (allocating nothing) when no rule changed
/// the text.
pub(crate) fn apply_rules_changed(input: &str, rules: &[Rule]) -> Option<(String, OffsetMap)> {
    let mut acc: Option<(String, OffsetMap)> = None;
    for r in rules {
        let step = r.apply(acc.as_ref().map_or(input, |a| a.0.as_str()));
        if step.is_some() {
            acc = Some(match acc {
                None => step?,
                Some(a) => then(a, step),
            });
        }
    }
    acc
}

/// Composes a later step onto an accumulated `(text, map)`.
pub(crate) fn then(
    acc: (String, OffsetMap),
    step: Option<(String, OffsetMap)>,
) -> (String, OffsetMap) {
    match step {
        None => acc,
        Some((out, m)) => {
            let map = OffsetMap::compose(&acc.1, &acc.0, &m);
            (out, map)
        }
    }
}

#[cfg(test)]
mod tests {
    use textweaver_core::SpanKind;

    use super::*;

    #[test]
    fn template_parses_groups() {
        assert_eq!(
            template(" \\1 over \\2"),
            vec![
                Piece::Text(" ".into()),
                Piece::Keep(1),
                Piece::Text(" over ".into()),
                Piece::Keep(2)
            ]
        );
    }

    #[test]
    fn kept_groups_are_literal_and_gaps_expand_or_elide() {
        let r = Rule::new(r"\\frac\{([^}]+)\}\{([^}]+)\}", "\\1 over \\2");
        let input = "x = \\frac{a}{b}!";
        let (out, map) = r.apply(input).unwrap();
        assert_eq!(out, "x = a over b!");
        map.check_invariants(&out).unwrap();
        let kinds: Vec<SpanKind> = map.spans().iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            [
                SpanKind::Literal,
                SpanKind::Elided,
                SpanKind::Literal,
                SpanKind::Expanded,
                SpanKind::Literal,
                SpanKind::Elided,
                SpanKind::Literal
            ]
        );
        // "over" highlights the "}{" between the operands.
        let over = out.find("over").unwrap() as u32;
        assert_eq!(
            map.to_source(&out, over..over + 4),
            Some(CharRange::new(11, 13))
        );
    }

    #[test]
    fn padding_avoids_double_spaces() {
        let r = Rule::new(r"\\times", " times ").padded();
        assert_eq!(r.apply("a \\times b").unwrap().0, "a times b");
        assert_eq!(r.apply("a\\times b").unwrap().0, "a times b");
        assert_eq!(r.apply("\\times").unwrap().0, "times");
    }

    #[test]
    fn rejected_matches_continue_one_char_later() {
        // Only numbers not preceded by a dot.
        let r = Rule::with(r"[0-9]+", |c, s| {
            let m = c.get(0)?;
            (char_before(s, m.start()) != Some('.')).then(|| vec![Piece::Text("N".into())])
        });
        assert_eq!(r.apply("1.23 45").unwrap().0, "N.2N N");
    }

    #[test]
    fn adjacent_groups_with_text_between_insert() {
        let r = Rule::new(r"_\{(\w)(\w+)\}", " sub \\1 \\2");
        let (out, map) = r.apply("x_{ij}").unwrap();
        assert_eq!(out, "x sub i j");
        map.check_invariants(&out).unwrap();
    }
}
