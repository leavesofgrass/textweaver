//! Inline math (LaTeX-ish notation), ported from `star/ttstext/mathspeech.py`
//! (`_normalize_math_inline`, Part 2 section 5.B.4) in Star's order.
//!
//! Deliberate fixes (Part 2 section 7.2):
//!
//! - Q6: subscripts apply only to a single-letter base (`x_i`, `x_{ij}`) or
//!   after a command or group (`\alpha_i`, `\hat{x}_i`), so prose such as
//!   `snake_case` and `my_var_name` is left alone.
//! - Q7: `x^2` at the end of the text is "x squared" (Star needed a
//!   following character).
//! - Q8: the cleanup (unknown commands, braces) runs only when the text
//!   contained math notation, never globally; spaces are managed by padding
//!   instead of collapsing and stripping the whole utterance; `$...$` must
//!   look like inline math (no space just inside the dollars, no digit right
//!   after the closing one), so "$5 and $10" is not math; Greek names and
//!   functions need a command boundary (`\alphabet` is not `\alpha`).

use regex::Captures;
use textweaver_core::OffsetMap;

use super::Transform;
use super::rewrite::{Piece, Rule, apply_rules_tracked, char_after, is_word, then};

const GREEK: [&str; 29] = [
    "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "lambda", "mu", "nu",
    "xi", "pi", "rho", "sigma", "tau", "phi", "chi", "psi", "omega", "Gamma", "Delta", "Theta",
    "Lambda", "Pi", "Sigma", "Phi", "Psi", "Omega",
];

const FUNCTIONS: [(&str, &str); 14] = [
    ("arcsin", "arcsine"),
    ("arccos", "arccosine"),
    ("arctan", "arctangent"),
    ("sin", "sine"),
    ("cos", "cosine"),
    ("tan", "tangent"),
    ("cot", "cotangent"),
    ("sec", "secant"),
    ("csc", "cosecant"),
    ("ln", "natural log"),
    ("log", "log"),
    ("exp", "e to the power"),
    ("lim", "limit"),
    ("sum", "sum"),
];

/// Operators: LaTeX commands (checked for a command boundary) and symbols.
const OPERATORS: [(&[&str], &str); 14] = [
    (&[r"\times", "×"], "times"),
    (&[r"\div", "÷"], "divided by"),
    (&[r"\pm"], "plus or minus"),
    (&[r"\neq"], "not equal to"),
    (&[r"\leq", r"\le", "≤"], "less than or equal to"),
    (&[r"\geq", r"\ge", "≥"], "greater than or equal to"),
    (&[r"\approx", "≈"], "approximately equal to"),
    (&[r"\infty", "∞"], "infinity"),
    (&[r"\rightarrow", r"\to", "→"], "approaches"),
    (&[r"\leftarrow", "←"], "from"),
    (&[r"\nabla"], "gradient of"),
    (&[r"\partial"], "partial"),
    (&[r"\prod"], "product"),
    (&[r"\int"], "integral"),
];

fn keep(g: usize) -> Piece {
    Piece::Keep(g)
}

fn text(s: &str) -> Piece {
    Piece::Text(s.to_owned())
}

/// A LaTeX command `\name` that is not the prefix of a longer command.
fn command(name: &str, spoken: &'static str) -> Rule {
    let pattern = regex::escape(name);
    let is_cmd = name.starts_with('\\');
    Rule::with(&pattern, move |c: &Captures<'_>, s: &str| {
        let m = c.get(0)?;
        if is_cmd && char_after(s, m.end()).is_some_and(|a| a.is_ascii_alphabetic()) {
            return None;
        }
        Some(vec![text(&format!(" {spoken} "))])
    })
    .padded()
}

/// True when the `_` at byte `i` follows a base a subscript can attach to.
fn subscript_base(s: &str, i: usize) -> bool {
    let before = &s[..i];
    match before.chars().next_back() {
        Some('}' | ')' | ']') => true,
        Some(c) if c.is_alphabetic() => {
            let run_start = before
                .char_indices()
                .rev()
                .find(|(_, ch)| !ch.is_alphabetic());
            match run_start {
                // A command such as \alpha.
                Some((_, '\\')) => true,
                Some((j, ch)) => {
                    let run = &before[j + ch.len_utf8()..];
                    run.chars().count() == 1 && !is_word(ch)
                }
                None => before.chars().count() == 1,
            }
        }
        _ => false,
    }
}

fn subscript(pattern: &str, pieces: Vec<Piece>) -> Rule {
    Rule::with(pattern, move |c: &Captures<'_>, s: &str| {
        let m = c.get(0)?;
        if !subscript_base(s, m.start()) {
            return None;
        }
        Some(pieces.clone())
    })
    .padded()
}

/// Rules for delimiters and notation (the math context).
fn notation_rules() -> Vec<Rule> {
    let mut r = vec![
        // 1. Delimiters.
        Rule::new(r"\$\$([\s\S]+?)\$\$", " \\1 ").padded(),
        Rule::with(r"\$([^$\n]+?)\$", |c, s| {
            let m = c.get(0)?;
            let inner = c.get(1)?.as_str();
            let spaced =
                inner.starts_with(char::is_whitespace) || inner.ends_with(char::is_whitespace);
            let digit_after = char_after(s, m.end()).is_some_and(|a| a.is_ascii_digit());
            (!spaced && !digit_after).then(|| vec![text(" "), keep(1), text(" ")])
        })
        .padded(),
        Rule::new(r"\\\[([\s\S]+?)\\\]", " \\1 ").padded(),
        Rule::new(r"\\\(([\s\S]+?)\\\)", " \\1 ").padded(),
        // 2. Accents.
        Rule::new(r"\\bar\{(\w+)\}", "\\1-bar"),
        Rule::new(r"\\hat\{(\w+)\}", "\\1-hat"),
        Rule::new(r"\\tilde\{(\w+)\}", "\\1-tilde"),
        Rule::new(r"\\overline\{([^}]+)\}", "\\1 bar"),
        // 3. Fractions and roots.
        Rule::new(r"\\frac\{([^}]+)\}\{([^}]+)\}", "\\1 over \\2").padded(),
        Rule::new(r"\\sqrt\[([^\]]+)\]\{([^}]+)\}", "\\1-th root of \\2").padded(),
        Rule::new(r"\\sqrt\{([^}]+)\}", "square root of \\1").padded(),
        // 4. Powers.
        Rule::new(r"\^\{-1\}", " inverse").padded(),
        Rule::with(r"\^\{2\}|\^2", |c, s| {
            let m = c.get(0)?;
            let bare = m.as_str() == "^2";
            if bare && char_after(s, m.end()).is_some_and(is_word) {
                return None;
            }
            Some(vec![text(" squared")])
        })
        .padded(),
        Rule::with(r"\^\{3\}|\^3", |c, s| {
            let m = c.get(0)?;
            let bare = m.as_str() == "^3";
            if bare && char_after(s, m.end()).is_some_and(is_word) {
                return None;
            }
            Some(vec![text(" cubed")])
        })
        .padded(),
        Rule::new(r"\^\{([^}]+)\}", " to the \\1").padded(),
        Rule::new(r"\^(\w)", " to the \\1").padded(),
        // 5. Subscripts.
        subscript(
            r"_\{(\w)(\w+)\}",
            vec![text(" sub "), keep(1), text(" "), keep(2)],
        ),
        subscript(r"_\{(\w+)\}", vec![text(" sub "), keep(1)]),
        subscript(r"_(\w+)", vec![text(" sub "), keep(1)]),
    ];
    // 6. Greek.
    for g in GREEK {
        let name = format!("\\{g}");
        let spoken: &'static str = g;
        r.push(command(&name, spoken));
    }
    // 7. Functions.
    for (cmd, spoken) in FUNCTIONS {
        r.push(command(&format!("\\{cmd}"), spoken));
    }
    // 8. Operators written as commands.
    for (names, spoken) in OPERATORS {
        for n in names.iter().filter(|n| n.starts_with('\\')) {
            r.push(command(n, spoken));
        }
    }
    r
}

/// Operators written as symbols; they do not by themselves make text math.
fn symbol_rules() -> Vec<Rule> {
    OPERATORS
        .iter()
        .flat_map(|(names, spoken)| {
            names
                .iter()
                .filter(|n| !n.starts_with('\\'))
                .map(|n| command(n, spoken))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Cleanup, only in a math context.
fn cleanup_rules() -> Vec<Rule> {
    vec![
        Rule::new(r"\\[a-zA-Z]+", " ").padded(),
        Rule::new(r"[{}]", ""),
        // Collapse runs of spaces the replacements left behind.
        Rule::new(r"( ) +", "\\1"),
    ]
}

/// The math transform.
pub struct Math {
    notation: Vec<Rule>,
    symbols: Vec<Rule>,
    cleanup: Vec<Rule>,
}

impl std::fmt::Debug for Math {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Math").finish_non_exhaustive()
    }
}

impl Default for Math {
    fn default() -> Self {
        Math {
            notation: notation_rules(),
            symbols: symbol_rules(),
            cleanup: cleanup_rules(),
        }
    }
}

impl Transform for Math {
    fn name(&self) -> &'static str {
        "math"
    }

    fn apply(&self, input: &str) -> (String, OffsetMap) {
        let (text, map, is_math) = apply_rules_tracked(input, &self.notation);
        let mut acc = (text, map);
        for r in &self.symbols {
            let step = r.apply(&acc.0);
            acc = then(acc, step);
        }
        if is_math {
            for r in &self.cleanup {
                let step = r.apply(&acc.0);
                acc = then(acc, step);
            }
        }
        acc
    }
}

/// Star `_normalize_math_inline` (with the fixes listed in the module docs).
pub fn normalize_math(text: &str) -> String {
    Math::default().apply(text).0
}
