//! Math as plain Unicode text for the reading view: `x²`, `√2`, `1⁄2`, as
//! star's `mathrender.py` drew it (Agent W4g).
//!
//! [`to_unicode`] writes a [`Math`] tree on one line with Unicode
//! characters only, so a terminal can show it without markup:
//!
//! - superscripts and subscripts become Unicode script characters when
//!   every character has one (`x²`, `aᵢ`, `eⁱᵗ`), otherwise `^(…)` and
//!   `_(…)` (`x^(1/y)`);
//! - fractions use the fraction slash U+2044 (`1⁄2`), with brackets around
//!   a numerator or denominator of more than one term (`(a + b)⁄c`);
//!   a binomial's stack uses `¦`, as the UnicodeMath linear format does;
//! - square, cube, and fourth roots use `√`, `∛`, `∜`, other roots a
//!   superscript index (`⁵√x`);
//! - accents over one character use combining marks (`x̂`, `v⃗`);
//! - `\mathbb`, `\mathbf`, `\mathcal`, `\mathfrak`, `\mathsf` and `\mathtt`
//!   letters use the Mathematical Alphanumeric Symbols (`ℝ`, `𝐯`);
//! - binary operators and relations get a space on each side.
//!
//! This is for the eye only. Speech reads the source through
//! [`crate::speak`], which knows the structure; the Unicode form loses
//! some of it (a fraction's extent, a table's rows).

use crate::tree::{AccentKind, Enclosure, Math, Node, NodeKind, OpClass, TableKind, Variant};

/// Writes `math` as one line of Unicode text.
///
/// ```
/// use textweaver_math::{parse_latex, to_unicode};
///
/// assert_eq!(to_unicode(&parse_latex(r"x^2 + \sqrt{2}")), "x² + √2");
/// assert_eq!(to_unicode(&parse_latex(r"\frac{1}{2}")), "1⁄2");
/// ```
pub fn to_unicode(math: &Math) -> String {
    let mut out = String::new();
    write_node(&math.root, &mut out);
    out.trim().to_owned()
}

/// Writes one node.
fn write_node(node: &Node, out: &mut String) {
    match &node.kind {
        NodeKind::Row(items) => write_row(items, out),
        NodeKind::Number(s) | NodeKind::Ident(s) | NodeKind::Text(s) | NodeKind::Error(s) => {
            out.push_str(s);
        }
        NodeKind::Function(name) => out.push_str(name),
        NodeKind::Operator { text, .. } => out.push_str(text),
        NodeKind::Space(_) => {
            if !out.ends_with(' ') {
                out.push(' ');
            }
        }
        NodeKind::Fraction { num, den, bar } => {
            write_operand(num, out);
            out.push(if *bar { '\u{2044}' } else { '\u{a6}' });
            write_operand(den, out);
        }
        NodeKind::Root { index, radicand } => {
            let index_text = index.as_deref().map(unicode_of);
            match index_text.as_deref().map(str::trim) {
                None | Some("2") => out.push('\u{221a}'),
                Some("3") => out.push('\u{221b}'),
                Some("4") => out.push('\u{221c}'),
                Some(i) => {
                    match script(i, Script::Super) {
                        Some(s) => out.push_str(&s),
                        None => {
                            out.push('(');
                            out.push_str(i);
                            out.push(')');
                        }
                    }
                    out.push('\u{221a}');
                }
            }
            write_operand(radicand, out);
        }
        NodeKind::Scripts {
            base,
            sub,
            sup,
            limits: _,
        } => {
            write_node(base, out);
            if let Some(sub) = sub {
                write_script(sub, Script::Sub, out);
            }
            if let Some(sup) = sup {
                write_script(sup, Script::Super, out);
            }
        }
        NodeKind::Fenced { open, close, body } => {
            out.push_str(open);
            write_node(body, out);
            out.push_str(close);
        }
        NodeKind::Accent { base, accent } => write_accent(base, *accent, out),
        NodeKind::Table { kind, rows } => write_table(*kind, rows, out),
        NodeKind::Style { variant, body } => {
            let inner = unicode_of(body);
            out.extend(inner.chars().map(|c| styled(c, *variant)));
        }
        NodeKind::Enclose { notation, body } => {
            let inner = unicode_of(body);
            match notation {
                Enclosure::Cancel => {
                    for c in inner.chars() {
                        out.push(c);
                        if !c.is_whitespace() {
                            out.push('\u{336}');
                        }
                    }
                }
                Enclosure::Box => {
                    out.push('[');
                    out.push_str(&inner);
                    out.push(']');
                }
            }
        }
    }
}

/// A node written on its own.
fn unicode_of(node: &Node) -> String {
    let mut s = String::new();
    write_node(node, &mut s);
    s.trim().to_owned()
}

/// A row: operands side by side, a space on each side of a binary operator
/// or relation (not of a sign: `-x`, `a = -b`), a space after a function
/// name or a large operator before its argument.
fn write_row(items: &[Node], out: &mut String) {
    let mut prev: Option<&Node> = None;
    for item in items {
        let spaced = match item.op_class() {
            Some(OpClass::Binary) => prev.is_some_and(|p| !is_operator_like(p)),
            Some(OpClass::Relation) => true,
            _ => false,
        };
        if spaced {
            push_space(out);
            write_node(item, out);
            push_space(out);
        } else {
            let after_word = prev.is_some_and(|p| {
                matches!(p.kind, NodeKind::Function(_))
                    || matches!(&p.kind, NodeKind::Scripts { base, .. } if matches!(base.kind, NodeKind::Function(_)) || base.op_class() == Some(OpClass::Large))
                    || p.op_class() == Some(OpClass::Large)
                    || p.op_class() == Some(OpClass::Punctuation)
            });
            let opens = item.op_class() == Some(OpClass::Open)
                || matches!(item.kind, NodeKind::Fenced { .. });
            let closes = matches!(
                item.op_class(),
                Some(OpClass::Close | OpClass::Punctuation | OpClass::Postfix)
            );
            if after_word
                && !closes
                && !(opens && prev.is_some_and(|p| matches!(p.kind, NodeKind::Function(_))))
            {
                push_space(out);
            }
            write_node(item, out);
        }
        prev = Some(item);
    }
}

/// True for an operator or an opening bracket: a binary operator after one
/// is a sign.
fn is_operator_like(node: &Node) -> bool {
    matches!(
        node.op_class(),
        Some(
            OpClass::Binary
                | OpClass::Relation
                | OpClass::Open
                | OpClass::Punctuation
                | OpClass::Prefix
                | OpClass::Large
        )
    )
}

/// One space, unless the text already ends with one (or is empty).
fn push_space(out: &mut String) {
    if !out.is_empty() && !out.ends_with(' ') {
        out.push(' ');
    }
}

/// A fraction's part or a root's radicand: in brackets when it is more
/// than one term.
fn write_operand(node: &Node, out: &mut String) {
    let text = unicode_of(node);
    if is_simple(node, &text) {
        out.push_str(&text);
    } else {
        out.push('(');
        out.push_str(&text);
        out.push(')');
    }
}

/// True when `node` reads as one term without brackets: a token, a
/// bracketed group, a script, a root, or text without spaces.
fn is_simple(node: &Node, text: &str) -> bool {
    match &node.kind {
        NodeKind::Row(items) if items.len() == 1 => is_simple(&items[0], text),
        NodeKind::Number(_) | NodeKind::Ident(_) | NodeKind::Function(_) => true,
        NodeKind::Fenced { open, .. } => !open.is_empty(),
        NodeKind::Scripts { .. } | NodeKind::Root { .. } | NodeKind::Accent { .. } => {
            !text.contains(' ')
        }
        NodeKind::Style { body, .. } => is_simple(body, text),
        _ => !text.is_empty() && !text.contains(' ') && text.chars().count() == 1,
    }
}

/// Superscript or subscript.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Script {
    Super,
    Sub,
}

/// A script: Unicode script characters when every character has one,
/// else `^` or `_` and the script, bracketed when longer than one char.
fn write_script(node: &Node, kind: Script, out: &mut String) {
    let text = unicode_of(node);
    // Primes are already raised.
    if kind == Script::Super && text.chars().all(|c| matches!(c, '′' | '″' | '‴' | '⁗')) {
        out.push_str(&text);
        return;
    }
    if let Some(s) = script(&text, kind) {
        out.push_str(&s);
        return;
    }
    out.push(if kind == Script::Super { '^' } else { '_' });
    if text.chars().count() == 1 {
        out.push_str(&text);
    } else {
        out.push('(');
        out.push_str(&text);
        out.push(')');
    }
}

/// `text` in Unicode script characters, or `None` when one of its
/// characters has none. Spaces are dropped (`n + 1` becomes `ⁿ⁺¹`).
fn script(text: &str, kind: Script) -> Option<String> {
    let text: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if text.is_empty() {
        return None;
    }
    text.chars()
        .map(|c| match kind {
            Script::Super => superscript(c),
            Script::Sub => subscript(c),
        })
        .collect()
}

/// The superscript form of `c`.
fn superscript(c: char) -> Option<char> {
    Some(match c {
        '0' => '⁰',
        '1' => '¹',
        '2' => '²',
        '3' => '³',
        '4' => '⁴',
        '5' => '⁵',
        '6' => '⁶',
        '7' => '⁷',
        '8' => '⁸',
        '9' => '⁹',
        '+' => '⁺',
        '-' | '\u{2212}' => '⁻',
        '=' => '⁼',
        '(' => '⁽',
        ')' => '⁾',
        'a' => 'ᵃ',
        'b' => 'ᵇ',
        'c' => 'ᶜ',
        'd' => 'ᵈ',
        'e' => 'ᵉ',
        'f' => 'ᶠ',
        'g' => 'ᵍ',
        'h' => 'ʰ',
        'i' => 'ⁱ',
        'j' => 'ʲ',
        'k' => 'ᵏ',
        'l' => 'ˡ',
        'm' => 'ᵐ',
        'n' => 'ⁿ',
        'o' => 'ᵒ',
        'p' => 'ᵖ',
        'r' => 'ʳ',
        's' => 'ˢ',
        't' => 'ᵗ',
        'u' => 'ᵘ',
        'v' => 'ᵛ',
        'w' => 'ʷ',
        'x' => 'ˣ',
        'y' => 'ʸ',
        'z' => 'ᶻ',
        'T' => 'ᵀ',
        '′' | '″' | '‴' | '*' | '∗' | '∘' | '†' => c,
        _ => return None,
    })
}

/// The subscript form of `c`.
fn subscript(c: char) -> Option<char> {
    Some(match c {
        '0' => '₀',
        '1' => '₁',
        '2' => '₂',
        '3' => '₃',
        '4' => '₄',
        '5' => '₅',
        '6' => '₆',
        '7' => '₇',
        '8' => '₈',
        '9' => '₉',
        '+' => '₊',
        '-' | '\u{2212}' => '₋',
        '=' => '₌',
        '(' => '₍',
        ')' => '₎',
        'a' => 'ₐ',
        'e' => 'ₑ',
        'h' => 'ₕ',
        'i' => 'ᵢ',
        'j' => 'ⱼ',
        'k' => 'ₖ',
        'l' => 'ₗ',
        'm' => 'ₘ',
        'n' => 'ₙ',
        'o' => 'ₒ',
        'p' => 'ₚ',
        'r' => 'ᵣ',
        's' => 'ₛ',
        't' => 'ₜ',
        'u' => 'ᵤ',
        'v' => 'ᵥ',
        'x' => 'ₓ',
        _ => return None,
    })
}

/// An accent: a combining mark after a single character, else the mark
/// after the bracketed base.
fn write_accent(base: &Node, accent: AccentKind, out: &mut String) {
    let text = unicode_of(base);
    let combining = match accent {
        AccentKind::Hat => '\u{302}',
        AccentKind::Bar | AccentKind::Overline => '\u{305}',
        AccentKind::Tilde => '\u{303}',
        AccentKind::Vector => '\u{20d7}',
        AccentKind::LeftArrow => '\u{20d6}',
        AccentKind::LeftRightArrow => '\u{20e1}',
        AccentKind::Dot => '\u{307}',
        AccentKind::DoubleDot => '\u{308}',
        AccentKind::TripleDot => '\u{20db}',
        AccentKind::Acute => '\u{301}',
        AccentKind::Grave => '\u{300}',
        AccentKind::Breve => '\u{306}',
        AccentKind::Check => '\u{30c}',
        AccentKind::Ring => '\u{30a}',
        AccentKind::Arc => '\u{311}',
        AccentKind::Underline => '\u{332}',
        AccentKind::Overbrace | AccentKind::Underbrace => {
            out.push_str(&text);
            return;
        }
    };
    // Lines over or under text mark every character.
    if matches!(
        accent,
        AccentKind::Overline | AccentKind::Underline | AccentKind::Bar
    ) {
        for c in text.chars() {
            out.push(c);
            if !c.is_whitespace() {
                out.push(combining);
            }
        }
        return;
    }
    if text.chars().count() == 1 {
        out.push_str(&text);
        out.push(combining);
    } else {
        out.push('(');
        out.push_str(&text);
        out.push(')');
        out.push(combining);
    }
}

/// A table on one line: cells separated by commas, rows by semicolons, and
/// cases after an opening brace.
fn write_table(kind: TableKind, rows: &[Vec<Node>], out: &mut String) {
    let rows: Vec<String> = rows
        .iter()
        .map(|cells| {
            cells
                .iter()
                .map(unicode_of)
                .filter(|c| !c.is_empty())
                .collect::<Vec<_>>()
                .join(if kind == TableKind::Cases { " " } else { ", " })
        })
        .filter(|r| !r.is_empty())
        .collect();
    out.push_str(&rows.join("; "));
}

/// `c` in a math font, from the Mathematical Alphanumeric Symbols block,
/// or `c` itself when the font has no such letter.
fn styled(c: char, variant: Variant) -> char {
    let (upper, lower, digit): (u32, Option<u32>, Option<u32>) = match variant {
        Variant::Normal | Variant::Italic => return c,
        Variant::Bold => (0x1D400, Some(0x1D41A), Some(0x1D7CE)),
        Variant::BoldItalic => (0x1D468, Some(0x1D482), Some(0x1D7CE)),
        Variant::Script => (0x1D49C, Some(0x1D4B6), None),
        Variant::BoldScript => (0x1D4D0, Some(0x1D4EA), None),
        Variant::Fraktur => (0x1D504, Some(0x1D51E), None),
        Variant::DoubleStruck => (0x1D538, Some(0x1D552), Some(0x1D7D8)),
        Variant::BoldFraktur => (0x1D56C, Some(0x1D586), None),
        Variant::SansSerif => (0x1D5A0, Some(0x1D5BA), Some(0x1D7E2)),
        Variant::BoldSansSerif => (0x1D5D4, Some(0x1D5EE), Some(0x1D7EC)),
        Variant::SansSerifItalic => (0x1D608, Some(0x1D622), Some(0x1D7E2)),
        Variant::SansSerifBoldItalic => (0x1D63C, Some(0x1D656), Some(0x1D7EC)),
        Variant::Monospace => (0x1D670, Some(0x1D68A), Some(0x1D7F6)),
    };
    // Letters that live in the Letterlike Symbols block instead.
    let hole = match (variant, c) {
        (Variant::Script, 'B') => Some('ℬ'),
        (Variant::Script, 'E') => Some('ℰ'),
        (Variant::Script, 'F') => Some('ℱ'),
        (Variant::Script, 'H') => Some('ℋ'),
        (Variant::Script, 'I') => Some('ℐ'),
        (Variant::Script, 'L') => Some('ℒ'),
        (Variant::Script, 'M') => Some('ℳ'),
        (Variant::Script, 'R') => Some('ℛ'),
        (Variant::Script, 'e') => Some('ℯ'),
        (Variant::Script, 'g') => Some('ℊ'),
        (Variant::Script, 'o') => Some('ℴ'),
        (Variant::Fraktur, 'C') => Some('ℭ'),
        (Variant::Fraktur, 'H') => Some('ℌ'),
        (Variant::Fraktur, 'I') => Some('ℑ'),
        (Variant::Fraktur, 'R') => Some('ℜ'),
        (Variant::Fraktur, 'Z') => Some('ℨ'),
        (Variant::DoubleStruck, 'C') => Some('ℂ'),
        (Variant::DoubleStruck, 'H') => Some('ℍ'),
        (Variant::DoubleStruck, 'N') => Some('ℕ'),
        (Variant::DoubleStruck, 'P') => Some('ℙ'),
        (Variant::DoubleStruck, 'Q') => Some('ℚ'),
        (Variant::DoubleStruck, 'R') => Some('ℝ'),
        (Variant::DoubleStruck, 'Z') => Some('ℤ'),
        _ => None,
    };
    if let Some(h) = hole {
        return h;
    }
    let code = match c {
        'A'..='Z' => Some(upper + (u32::from(c) - u32::from('A'))),
        'a'..='z' => lower.map(|l| l + (u32::from(c) - u32::from('a'))),
        '0'..='9' => digit.map(|d| d + (u32::from(c) - u32::from('0'))),
        _ => None,
    };
    code.and_then(char::from_u32).unwrap_or(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_asciimath, parse_latex};

    fn u(src: &str) -> String {
        to_unicode(&parse_latex(src))
    }

    #[test]
    fn scripts_use_unicode_where_they_can() {
        assert_eq!(u("x^2"), "x²");
        assert_eq!(u("x^{n+1}"), "xⁿ⁺¹");
        assert_eq!(u("a_i"), "aᵢ");
        assert_eq!(u("x_{10}^2"), "x₁₀²");
        assert_eq!(u("e^{i t}"), "eⁱᵗ");
        assert_eq!(u("f'(x)"), "f′(x)");
        // No superscript Q: written out.
        assert_eq!(u("x^Q"), "x^Q");
        assert_eq!(u(r"x^{\frac{1}{y}}"), "x^(1⁄y)");
    }

    #[test]
    fn roots_and_fractions() {
        assert_eq!(u(r"\sqrt{2}"), "√2");
        assert_eq!(u(r"\sqrt[3]{x}"), "∛x");
        assert_eq!(u(r"\sqrt[5]{x}"), "⁵√x");
        assert_eq!(u(r"\sqrt{x+1}"), "√(x + 1)");
        assert_eq!(u(r"\frac{1}{2}"), "1⁄2");
        assert_eq!(u(r"\frac{a+b}{c}"), "(a + b)⁄c");
        assert_eq!(u(r"\binom{n}{k}"), "(n¦k)");
    }

    #[test]
    fn operators_are_spaced_but_signs_are_not() {
        assert_eq!(u("a+b=c"), "a + b = c");
        assert_eq!(u("-x"), "\u{2212}x");
        assert_eq!(u("a = -b"), "a = \u{2212}b");
        assert_eq!(u(r"\alpha \le \beta"), "α ≤ β");
        assert_eq!(u(r"\sin x"), "sin x");
        assert_eq!(u(r"\sum_{i=1}^{n} i"), "∑ᵢ₌₁ⁿ i");
    }

    #[test]
    fn fonts_accents_and_tables() {
        assert_eq!(u(r"\mathbb{R}"), "ℝ");
        assert_eq!(u(r"\mathbf{v}"), "𝐯");
        assert_eq!(u(r"\mathcal{L}"), "ℒ");
        assert_eq!(u(r"\hat{x}"), "x\u{302}");
        assert_eq!(u(r"\vec{v}"), "v\u{20d7}");
        assert_eq!(
            u(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}"),
            "(a, b; c, d)"
        );
    }

    #[test]
    fn asciimath_too() {
        assert_eq!(to_unicode(&parse_asciimath("sqrt(2)")), "√2");
        assert_eq!(to_unicode(&parse_asciimath("x^2")), "x²");
    }

    #[test]
    fn never_panics_on_broken_input() {
        for src in [r"\frac{", "x^", r"\sqrt[", "}}}", r"\begin{matrix}", ""] {
            let _ = u(src);
        }
    }
}
