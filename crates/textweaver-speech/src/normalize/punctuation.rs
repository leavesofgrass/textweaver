//! Punctuation verbosity, character names, and split caps (new in
//! textweaver; Star had none of them, the model is Omnivox's).
//!
//! Punctuation levels ([`PunctuationLevel`]):
//!
//! - `None`: symbols are not spoken; they are dropped (replaced by a space
//!   where they joined two words), so engines do not read them either.
//!   Sentence punctuation stays for prosody.
//! - `Some` (default): symbols that carry meaning in prose are spoken by
//!   name (`@` "at", `#` "number", `/` "slash", `&` "and", `+` "plus", ...).
//!   Sentence punctuation stays for prosody.
//! - `All`: every punctuation mark is spoken by name, except an apostrophe
//!   or hyphen inside a word ("don't", "twenty-four").
//!
//! At every level, symbols that are words in science and medicine are
//! named: Greek letters ("TNF-α" is "TNF alpha", "ΔG" is "delta G"), the
//! micro sign ("µ" is "micro"; a Greek mu before a Latin letter, "μm", is
//! also "micro"), and powers of ten ("× 10^9" is "times ten to the ninth",
//! "10⁻³" is "ten to the negative third"). The minus sign `−`, `⇌` ("in
//! equilibrium with") and the arrows are named like the other symbols. Math
//! regions are spoken by the math transform before this step and are not
//! changed by it.

use textweaver_core::{CharPos, OffsetMap, PunctuationLevel, SpokenBuilder};

use super::Transform;
use super::rewrite::{Piece, Rule, char_after, char_before, is_word, then};

/// `(char, name, spoken at Some)`.
const NAMES: &[(char, &str, bool)] = &[
    ('@', "at", true),
    ('#', "number", true),
    ('&', "and", true),
    ('*', "star", true),
    ('+', "plus", true),
    ('=', "equals", true),
    ('<', "less than", true),
    ('>', "greater than", true),
    ('|', "bar", true),
    ('\\', "backslash", true),
    ('/', "slash", true),
    ('^', "caret", true),
    ('~', "tilde", true),
    // Currency and percent are read by the numbers transform or by the
    // engine; they are named only at `All`.
    ('%', "percent", false),
    ('$', "dollar", false),
    ('€', "euro", false),
    ('£', "pound", false),
    ('¢', "cent", false),
    ('¥', "yen", false),
    ('§', "section", true),
    ('¶', "paragraph", true),
    ('°', "degrees", true),
    ('©', "copyright", true),
    ('®', "registered", true),
    ('™', "trademark", true),
    ('•', "bullet", true),
    ('±', "plus or minus", true),
    ('×', "times", true),
    ('÷', "divided by", true),
    ('\u{2212}', "minus", true),
    ('⇌', "in equilibrium with", true),
    ('→', "right arrow", true),
    ('←', "left arrow", true),
    ('↔', "left right arrow", true),
    ('.', "dot", false),
    (',', "comma", false),
    (';', "semicolon", false),
    (':', "colon", false),
    ('!', "exclamation", false),
    ('?', "question", false),
    ('\'', "apostrophe", false),
    ('"', "quote", false),
    ('\u{2018}', "left quote", false),
    ('\u{2019}', "right quote", false),
    ('\u{201C}', "left double quote", false),
    ('\u{201D}', "right double quote", false),
    ('(', "left paren", false),
    (')', "right paren", false),
    ('[', "left bracket", false),
    (']', "right bracket", false),
    ('{', "left brace", false),
    ('}', "right brace", false),
    ('-', "dash", false),
    ('\u{2013}', "en dash", false),
    ('\u{2014}', "em dash", false),
    ('\u{2026}', "ellipsis", false),
    ('_', "underscore", false),
    ('`', "backtick", false),
];

/// Letters that engines skip or misread, named at every punctuation level
/// because they are words, not punctuation: the Greek alphabet (upper case
/// as plain names, "ΔG" is "delta G") and the micro sign.
const LETTERS: &[(char, &str)] = &[
    ('α', "alpha"),
    ('β', "beta"),
    ('γ', "gamma"),
    ('δ', "delta"),
    ('ε', "epsilon"),
    ('ϵ', "epsilon"),
    ('ζ', "zeta"),
    ('η', "eta"),
    ('θ', "theta"),
    ('ϑ', "theta"),
    ('ι', "iota"),
    ('κ', "kappa"),
    ('λ', "lambda"),
    ('μ', "mu"),
    ('ν', "nu"),
    ('ξ', "xi"),
    ('ο', "omicron"),
    ('π', "pi"),
    ('ρ', "rho"),
    ('σ', "sigma"),
    ('ς', "sigma"),
    ('τ', "tau"),
    ('υ', "upsilon"),
    ('φ', "phi"),
    ('ϕ', "phi"),
    ('χ', "chi"),
    ('ψ', "psi"),
    ('ω', "omega"),
    ('Α', "alpha"),
    ('Β', "beta"),
    ('Γ', "gamma"),
    ('Δ', "delta"),
    ('Ε', "epsilon"),
    ('Ζ', "zeta"),
    ('Η', "eta"),
    ('Θ', "theta"),
    ('Ι', "iota"),
    ('Κ', "kappa"),
    ('Λ', "lambda"),
    ('Μ', "mu"),
    ('Ν', "nu"),
    ('Ξ', "xi"),
    ('Ο', "omicron"),
    ('Π', "pi"),
    ('Ρ', "rho"),
    ('Σ', "sigma"),
    ('Τ', "tau"),
    ('Υ', "upsilon"),
    ('Φ', "phi"),
    ('Χ', "chi"),
    ('Ψ', "psi"),
    ('Ω', "omega"),
    ('µ', "micro"),
];

/// The spoken name of a character, for [`speak_char`](crate::SpeechService::speak_char)
/// and punctuation verbosity: punctuation and symbols by name, Greek letters
/// and the micro sign by name, whitespace as "space", "tab", "new line".
/// `None` for Latin letters and digits (spoken as themselves) and for
/// characters without a name.
pub fn char_name(c: char) -> Option<&'static str> {
    match c {
        ' ' | '\u{a0}' => Some("space"),
        '\t' => Some("tab"),
        '\n' | '\r' => Some("new line"),
        _ => NAMES
            .iter()
            .find(|(ch, _, _)| *ch == c)
            .map(|(_, n, _)| *n)
            .or_else(|| letter_name(c)),
    }
}

fn letter_name(c: char) -> Option<&'static str> {
    LETTERS.iter().find(|(ch, _)| *ch == c).map(|(_, n)| *n)
}

/// Superscript digits and minus, for "10⁹".
fn superscript(c: char) -> Option<char> {
    Some(match c {
        '⁰' => '0',
        '¹' => '1',
        '²' => '2',
        '³' => '3',
        '⁴' => '4',
        '⁵' => '5',
        '⁶' => '6',
        '⁷' => '7',
        '⁸' => '8',
        '⁹' => '9',
        '⁻' => '-',
        _ => return None,
    })
}

/// Powers of ten outside math: "10^9" and "10⁹" are "ten to the ninth",
/// "10^-3" is "ten to the negative third", and a times sign written as `x`
/// or `*` just before one is "times" ("1.5 x 10^9"). A `×` is named by the
/// names table (or by the math transform) as "times".
fn powers_of_ten() -> Rule {
    Rule::with(
        r"(?:([xX*])[ ]?)?10(?:\^\(?([-\u{2212}]?[0-9]{1,3})\)?|([⁻]?[⁰¹²³⁴⁵⁶⁷⁸⁹]{1,3}))",
        |c, s| {
            let m = c.get(0)?;
            if char_after(s, m.end()).is_some_and(char::is_alphanumeric) {
                return None;
            }
            let times = match c.get(1) {
                // A times sign stands alone: "1.5 x 10^9", not "box 10^9".
                Some(x) => match char_before(s, x.start()) {
                    Some(b) if b.is_alphabetic() => return None,
                    // "2*10^3" is "2 times ten to the third".
                    Some(b) if b.is_alphanumeric() => " times ",
                    _ => "times ",
                },
                // "10" starts a number: not "210^3" or "v1.10^2".
                None => {
                    if char_before(s, m.start()).is_some_and(|b| b.is_alphanumeric() || b == '.') {
                        return None;
                    }
                    ""
                }
            };
            let exp: String = match (c.get(2), c.get(3)) {
                (Some(e), _) => e.as_str().replace('\u{2212}', "-"),
                (_, Some(e)) => e.as_str().chars().filter_map(superscript).collect(),
                _ => return None,
            };
            let (sign, digits) = match exp.strip_prefix('-') {
                Some(d) => ("negative ", d),
                None => ("", exp.as_str()),
            };
            let n: i64 = digits.parse().ok()?;
            Some(vec![Piece::Text(format!(
                "{times}ten to the {sign}{}",
                super::words::ordinal_to_words(n)
            ))])
        },
    )
}

fn spoken_at(c: char, level: PunctuationLevel) -> bool {
    NAMES
        .iter()
        .find(|(ch, _, _)| *ch == c)
        .is_some_and(|(_, _, some)| match level {
            PunctuationLevel::None | PunctuationLevel::Some => *some,
            PunctuationLevel::All => true,
        })
}

/// The punctuation transform.
pub struct Punctuation {
    level: PunctuationLevel,
    /// Powers of ten, before the names.
    powers: Rule,
    rule: Rule,
}

impl std::fmt::Debug for Punctuation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Punctuation")
            .field("level", &self.level)
            .finish_non_exhaustive()
    }
}

impl Punctuation {
    /// A transform for `level`.
    pub fn new(level: PunctuationLevel) -> Self {
        let class: String = NAMES
            .iter()
            .filter(|(c, _, _)| spoken_at(*c, level))
            .map(|(c, _, _)| regex::escape(&c.to_string()))
            .collect();
        let letters: String = LETTERS
            .iter()
            .map(|(c, _)| regex::escape(&c.to_string()))
            .collect();
        let pattern = format!("-?[{letters}]|[{class}]");
        let rule = Rule::with(&pattern, move |c, s| {
            let m = c.get(0)?;
            let before = char_before(s, m.start());
            let after = char_after(s, m.end());
            let ch = m.as_str().chars().next_back()?;
            if let Some(name) = letter_name(ch) {
                // A letter is named at every level. "TNF-α" is "TNF alpha";
                // "β-blocker" is "beta-blocker"; "5α" is "5 alpha".
                let hyphen = m.as_str().starts_with('-');
                if hyphen && !before.is_some_and(char::is_alphanumeric) {
                    return None;
                }
                // "µ" or "μ" before a Latin letter is the micro prefix.
                let name =
                    if matches!(ch, 'µ' | 'μ') && after.is_some_and(|a| a.is_ascii_alphabetic()) {
                        "micro"
                    } else {
                        name
                    };
                let lead = if before.is_some_and(char::is_alphanumeric) {
                    " "
                } else {
                    ""
                };
                let trail = if after.is_some_and(char::is_alphanumeric) {
                    " "
                } else {
                    ""
                };
                return Some(vec![Piece::Text(format!("{lead}{name}{trail}"))]);
            }
            let ch = m.as_str().chars().next()?;
            let inside_word = before.is_some_and(char::is_alphanumeric)
                && after.is_some_and(char::is_alphanumeric);
            if matches!(ch, '\'' | '\u{2019}' | '-') && inside_word {
                return None;
            }
            if ch == '.'
                && before.is_some_and(|b| b.is_ascii_digit())
                && after.is_some_and(|a| a.is_ascii_digit())
            {
                return None;
            }
            if level == PunctuationLevel::None {
                let joins = before.is_some_and(is_word) && after.is_some_and(is_word);
                return Some(vec![Piece::Text(if joins { " " } else { "" }.into())]);
            }
            let name = char_name(ch)?;
            Some(vec![Piece::Text(format!(" {name} "))])
        })
        .padded();
        Punctuation {
            level,
            powers: powers_of_ten(),
            rule,
        }
    }
}

impl Transform for Punctuation {
    fn name(&self) -> &'static str {
        "punctuation"
    }

    fn apply(&self, input: &str) -> (String, OffsetMap) {
        match self.powers.apply(input) {
            None => self
                .rule
                .apply(input)
                .unwrap_or_else(|| super::identity(input)),
            Some(acc) => {
                let step = self.rule.apply(&acc.0);
                then(acc, step)
            }
        }
    }
}

/// Speaks capitals inside words separately: "camelCase" as "camel Case",
/// "XMLHttpRequest" as "XML Http Request" (Omnivox's split caps).
#[derive(Clone, Copy, Debug, Default)]
pub struct SplitCaps;

impl Transform for SplitCaps {
    fn name(&self) -> &'static str {
        "split_caps"
    }

    fn apply(&self, input: &str) -> (String, OffsetMap) {
        let chars: Vec<(usize, char)> = input.char_indices().collect();
        let mut b = SpokenBuilder::new();
        let mut seg_start = 0usize; // byte
        let mut seg_char = 0usize;
        for (k, &(i, c)) in chars.iter().enumerate() {
            if k == 0 {
                continue;
            }
            let prev = chars[k - 1].1;
            let next = chars.get(k + 1).map(|&(_, n)| n);
            let lower_upper = prev.is_lowercase() && c.is_uppercase();
            let acronym_end =
                prev.is_uppercase() && c.is_uppercase() && next.is_some_and(char::is_lowercase);
            if lower_upper || acronym_end {
                b.push_literal(&input[seg_start..i], CharPos(seg_char));
                b.push_inserted(" ", CharPos(k));
                seg_start = i;
                seg_char = k;
            }
        }
        b.push_literal(&input[seg_start..], CharPos(seg_char));
        let (text, map) = b.finish();
        if map.is_empty() && !input.is_empty() {
            return super::identity(input);
        }
        (text, map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(t: &dyn Transform, s: &str) -> String {
        let (out, map) = t.apply(s);
        map.check_invariants(&out).unwrap();
        out
    }

    #[test]
    fn punctuation_levels() {
        let text = "Mail me@example.com, or C++ and/or #5! Don't stop - twenty-four.";
        assert_eq!(
            run(&Punctuation::new(PunctuationLevel::Some), text),
            "Mail me at example.com, or C plus plus and slash or number 5! Don't stop - twenty-four."
        );
        assert_eq!(
            run(&Punctuation::new(PunctuationLevel::None), text),
            "Mail me example.com, or C and or 5! Don't stop - twenty-four."
        );
        assert_eq!(
            run(
                &Punctuation::new(PunctuationLevel::All),
                "Hi, you. Don't (x) 3.5 a-b"
            ),
            "Hi comma you dot Don't left paren x right paren 3.5 a-b"
        );
    }

    #[test]
    fn split_caps_inserts_spaces() {
        assert_eq!(
            run(&SplitCaps, "camelCase XMLHttpRequest iPhone ok"),
            "camel Case XML Http Request i Phone ok"
        );
        let (out, map) = SplitCaps.apply("fooBar");
        assert_eq!(
            map.to_source(&out, 4..7),
            Some(textweaver_core::CharRange::new(3, 6))
        );
    }

    #[test]
    fn char_names() {
        assert_eq!(char_name(','), Some("comma"));
        assert_eq!(char_name(' '), Some("space"));
        assert_eq!(char_name('a'), None);
        assert_eq!(char_name('@'), Some("at"));
    }
}
