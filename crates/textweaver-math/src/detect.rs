//! Finding math in plain text: `$…$`, `$$…$$`, `\(…\)`, `\[…\]`, and
//! backtick ASCIIMath, without mistaking prices for math.
//!
//! Inline `$` follows Pandoc's `tex_math_dollars` rules, which keep
//! currency out: the opening `$` must be followed by a non-space, the
//! closing `$` must follow a non-space and must not be followed by a digit,
//! and the first `$` after the opening one is the only candidate. So
//! "$5 and $10" and "$5-$10" are not math, while "$x$" and "$5$" are.
//! Escaped `\$` never delimits. No delimiter pair spans a blank line.

use serde::{Deserialize, Serialize};
use textweaver_core::CharRange;

use crate::Notation;

/// Which delimiters to look for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DetectOptions {
    /// `$…$` and `$$…$$`.
    pub dollars: bool,
    /// `\(…\)` and `\[…\]`.
    pub brackets: bool,
    /// ASCIIMath between this delimiter on both sides (usually a
    /// backtick). Off by default, because in Markdown a backtick is code.
    pub asciimath: Option<char>,
}

impl Default for DetectOptions {
    fn default() -> Self {
        DetectOptions {
            dollars: true,
            brackets: true,
            asciimath: None,
        }
    }
}

/// The delimiter a math region was written with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Delimiter {
    /// `$…$`.
    Dollar,
    /// `$$…$$`.
    DoubleDollar,
    /// `\(…\)`.
    Paren,
    /// `\[…\]`.
    Bracket,
    /// ASCIIMath delimiter (a backtick by default).
    AsciiMath,
}

/// One math region found in text. Positions are chars.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MathRegion {
    /// The region including its delimiters.
    pub range: CharRange,
    /// The math inside the delimiters.
    pub content: CharRange,
    /// The notation of the content.
    pub notation: Notation,
    /// Display (block) math: `$$…$$` and `\[…\]`.
    pub display: bool,
    /// The delimiter used.
    pub delimiter: Delimiter,
}

/// Finds math regions in `text`, in order and without overlaps.
pub fn find_math(text: &str, opts: &DetectOptions) -> Vec<MathRegion> {
    let s: Vec<char> = text.chars().collect();
    find_math_chars(&s, opts)
}

pub(crate) fn find_math_chars(s: &[char], opts: &DetectOptions) -> Vec<MathRegion> {
    let n = s.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        let c = s[i];
        if c == '\\' {
            if opts.brackets && i + 1 < n && (s[i + 1] == '(' || s[i + 1] == '[') {
                let display = s[i + 1] == '[';
                let close = if display { ']' } else { ')' };
                if let Some(j) = find_pair(s, i + 2, &['\\', close]) {
                    let content = CharRange::new(i + 2, j);
                    if !blank(s, content) {
                        out.push(MathRegion {
                            range: CharRange::new(i, j + 2),
                            content,
                            notation: Notation::Latex,
                            display,
                            delimiter: if display {
                                Delimiter::Bracket
                            } else {
                                Delimiter::Paren
                            },
                        });
                        i = j + 2;
                        continue;
                    }
                }
            }
            // Skip the escaped character (`\$` is a literal dollar).
            i += 2;
            continue;
        }
        if c == '$' && opts.dollars {
            if i + 1 < n && s[i + 1] == '$' {
                if let Some(j) = find_pair(s, i + 2, &['$', '$']) {
                    let content = CharRange::new(i + 2, j);
                    if !blank(s, content) {
                        out.push(MathRegion {
                            range: CharRange::new(i, j + 2),
                            content,
                            notation: Notation::Latex,
                            display: true,
                            delimiter: Delimiter::DoubleDollar,
                        });
                        i = j + 2;
                        continue;
                    }
                }
                i += 2;
                continue;
            }
            if let Some(j) = inline_dollar_close(s, i) {
                out.push(MathRegion {
                    range: CharRange::new(i, j + 1),
                    content: CharRange::new(i + 1, j),
                    notation: Notation::Latex,
                    display: false,
                    delimiter: Delimiter::Dollar,
                });
                i = j + 1;
                continue;
            }
            i += 1;
            continue;
        }
        if Some(c) == opts.asciimath {
            if let Some(j) = (i + 1..n).find(|&j| s[j] == c || s[j] == '\n')
                && s[j] == c
            {
                let content = CharRange::new(i + 1, j);
                if !blank(s, content) {
                    out.push(MathRegion {
                        range: CharRange::new(i, j + 1),
                        content,
                        notation: Notation::AsciiMath,
                        display: false,
                        delimiter: Delimiter::AsciiMath,
                    });
                }
                i = j + 1;
                continue;
            }
            i += 1;
            continue;
        }
        i += 1;
    }
    out
}

/// True when `r` holds only white space.
fn blank(s: &[char], r: CharRange) -> bool {
    s[r.to_range()].iter().all(|c| c.is_whitespace())
}

/// True at a blank line: a newline followed by optional spaces and another
/// newline.
fn blank_line_at(s: &[char], j: usize) -> bool {
    if s[j] != '\n' {
        return false;
    }
    let mut k = j + 1;
    while k < s.len() && s[k] != '\n' && s[k].is_whitespace() {
        k += 1;
    }
    k < s.len() && s[k] == '\n'
}

/// The index of the next unescaped two-char `pat` at or after `from`,
/// stopping at a blank line.
fn find_pair(s: &[char], from: usize, pat: &[char; 2]) -> Option<usize> {
    let mut j = from;
    while j + 1 < s.len() {
        if blank_line_at(s, j) {
            return None;
        }
        if s[j] == pat[0] && s[j + 1] == pat[1] {
            return Some(j);
        }
        if s[j] == '\\' {
            // An escaped character: `\$`, or `\\` (a line break, never the
            // start of `\)`).
            j += 2;
            continue;
        }
        j += 1;
    }
    None
}

/// The closing `$` of an inline `$` at `open`, under Pandoc's rules.
fn inline_dollar_close(s: &[char], open: usize) -> Option<usize> {
    let first = *s.get(open + 1)?;
    if first.is_whitespace() || first == '$' {
        return None;
    }
    let mut j = open + 1;
    while j < s.len() {
        if blank_line_at(s, j) {
            return None;
        }
        match s[j] {
            '\\' => {
                j += 2;
                continue;
            }
            '$' => {
                let before = s[j - 1];
                let after_digit = s.get(j + 1).is_some_and(|c| c.is_ascii_digit());
                return (!before.is_whitespace() && !after_digit).then_some(j);
            }
            _ => j += 1,
        }
    }
    None
}
