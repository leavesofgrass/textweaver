//! Math for the writers: a formula from a `Math` marker's text (the LaTeX
//! source with its delimiters, as the Markdown loader keeps it), parsed by
//! `textweaver-math` and written for each format:
//!
//! - **PDF**: a linear Unicode form on the page (`πr²`, `(a + b)/2`,
//!   `√x`) inside a `Formula` tag whose alt text is the spoken form ("pi r
//!   squared"), as PDF/UA asks;
//! - **DOCX**: Office Math (OMML: fractions, scripts, roots, delimiters,
//!   accents, matrices), which Word draws and reads aloud;
//! - **EPUB**: presentation MathML with the source as `alttext`;
//! - **braille**: the spoken form, which grade 1 UEB spells out readably.
//!
//! Nothing is ever printed with its raw `$…$` delimiters.

use textweaver_core::Verbosity;
use textweaver_math::{
    AccentKind, Enclosure, Math, MathMlOptions, Node, NodeKind, OpClass, SpeechOptions, TableKind,
    Variant, parse_latex, speak, to_mathml,
};

use crate::model::{Block, Inline, Style};
use crate::xml;

/// Deeper trees are written as their source text, so a hostile formula
/// cannot exhaust the stack.
const MAX_DEPTH: usize = 64;

/// One formula.
#[derive(Clone, Debug)]
pub(crate) struct Formula {
    /// Display (block) math rather than inline.
    pub display: bool,
    math: Math,
}

impl Formula {
    /// The formula in a `Math` marker's text: `$…$`, `$$…$$`, `\(…\)`, or
    /// `\[…\]` (the delimiters are optional). `display` is the marker's
    /// level; `$$` and `\[` also make it display math.
    pub fn from_marked(text: &str, display: bool) -> Formula {
        let t = text.trim();
        let (body, display) =
            if let Some(b) = t.strip_prefix("$$").and_then(|b| b.strip_suffix("$$")) {
                (b, true)
            } else if let Some(b) = t.strip_prefix("\\[").and_then(|b| b.strip_suffix("\\]")) {
                (b, true)
            } else if let Some(b) = t.strip_prefix("\\(").and_then(|b| b.strip_suffix("\\)")) {
                (b, display)
            } else if let Some(b) = t
                .strip_prefix('$')
                .and_then(|b| b.strip_suffix('$'))
                .filter(|b| !b.is_empty())
            {
                (b, display)
            } else {
                (t, display)
            };
        Formula {
            display,
            math: parse_latex(body.trim()),
        }
    }

    /// The LaTeX source, without delimiters.
    #[cfg(test)]
    pub fn source(&self) -> &str {
        &self.math.source
    }

    /// The formula as it is read aloud ("x squared plus 1").
    pub fn spoken(&self) -> String {
        let s = speak(&self.math, &SpeechOptions::new(Verbosity::Normal)).text;
        let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
        if s.is_empty() {
            self.math.source.clone()
        } else {
            s
        }
    }

    /// A linear Unicode form for print (`πr²`, `(x + 1)/2`, `√x`).
    pub fn linear(&self) -> String {
        let s = linear(&self.math, &self.math.root, 0);
        let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
        if s.is_empty() {
            self.math.source.clone()
        } else {
            s
        }
    }

    /// Presentation MathML with the source as `alttext`.
    pub fn mathml(&self) -> String {
        to_mathml(&self.math, &MathMlOptions::new(self.display))
    }

    /// Office Math (OMML): an `m:oMath`, inside an `m:oMathPara` for
    /// display math.
    pub fn omml(&self) -> String {
        let body = omml(&self.math, &self.math.root, 0);
        if self.display {
            format!("<m:oMathPara><m:oMath>{body}</m:oMath></m:oMathPara>")
        } else {
            format!("<m:oMath>{body}</m:oMath>")
        }
    }
}

// ---------------------------------------------------------------------------
// Braille (ADR-0036).

/// In braille ASCII from [`MathBraille`]: a space that must not end a line
/// (it binds a Nemeth switch indicator to the math beside it).
pub(crate) const BRAILLE_NBSP: char = '\u{1}';
/// In braille ASCII from [`MathBraille`]: a place a long expression may be
/// divided without an indicator (before an operation sign). No cell.
pub(crate) const BRAILLE_BREAK: char = '\u{2}';
/// In braille ASCII from [`MathBraille`]: the word is math, so a forced
/// division gets no line continuation indicator and never follows an
/// indicator. No cell.
pub(crate) const BRAILLE_MATH: char = '\u{3}';

/// Where a formula's braille goes in text before translation.
const PLACEHOLDER_OPEN: char = '\u{E000}';
const PLACEHOLDER_CLOSE: char = '\u{E001}';

/// The Nemeth switch indicators in UEB text (BANA, Guidance for
/// Transcription Using the Nemeth Code within UEB Contexts): the opening
/// indicator (dots 456, 146) is followed by a space, the terminator (dots
/// 456, 156) follows a space.
const NEMETH_OPEN: &str = "_%";
const NEMETH_CLOSE: &str = "_:";

/// Braille for the formulas of one BRF file: each formula becomes a
/// placeholder in the text, and its braille is spliced in after the text
/// around it is translated ([`MathBraille::splice`]).
///
/// With the `mathcat` feature, formulas are written in the chosen math code
/// by MathCAT. A formula MathCAT cannot write, one the parser had to
/// repair, and every formula without the feature is written as its spoken
/// words, translated with the text around it; [`MathBraille::summary`]
/// says so once.
pub(crate) struct MathBraille {
    #[cfg_attr(not(feature = "mathcat"), allow(dead_code))]
    code: crate::MathCode,
    #[cfg_attr(not(feature = "mathcat"), allow(dead_code))]
    grade2: bool,
    /// Braille ASCII for each placeholder, in order.
    cells: std::cell::RefCell<Vec<String>>,
    /// Formulas written as their words.
    words: std::cell::Cell<usize>,
}

impl MathBraille {
    pub fn new(options: &crate::BrailleOptions) -> Self {
        MathBraille {
            code: options.math_code,
            grade2: options.grade == crate::BrailleGrade::Two,
            cells: std::cell::RefCell::new(Vec::new()),
            words: std::cell::Cell::new(0),
        }
    }

    /// The text `formula` stands as until translation: a placeholder for
    /// its braille, or its spoken words.
    pub fn text_for(&self, formula: &Formula) -> String {
        match self.braille(formula) {
            Some(ascii) => {
                let mut cells = self.cells.borrow_mut();
                let n = cells.len();
                cells.push(ascii);
                format!("{PLACEHOLDER_OPEN}{n}{PLACEHOLDER_CLOSE}")
            }
            None => {
                self.words.set(self.words.get() + 1);
                formula.spoken()
            }
        }
    }

    /// One sentence for the file's report when any formula was written as
    /// its words, naming the code.
    pub fn summary(&self) -> Option<String> {
        let n = self.words.get();
        if n == 0 {
            return None;
        }
        let what = if n == 1 {
            "1 formula is".to_owned()
        } else {
            format!("{n} formulas are")
        };
        Some(if cfg!(feature = "mathcat") {
            format!(
                "{what} in spoken words: MathCAT could not write {} in {} braille.",
                if n == 1 { "it" } else { "them" },
                self.code.name()
            )
        } else {
            format!("{what} in spoken words: this build has no math braille.")
        })
    }

    #[cfg(feature = "mathcat")]
    fn braille(&self, formula: &Formula) -> Option<String> {
        use textweaver_mathcat::{BrailleCode, BrailleOptions, braille_mathml, mathml_for};
        if !formula.math.diagnostics.is_empty() {
            return None;
        }
        let source = formula.math.source.trim();
        // A number alone stays in UEB: BANA's guidance switches to Nemeth
        // for math that is more than a single number.
        if self.code == crate::MathCode::Nemeth
            && !source.is_empty()
            && source
                .chars()
                .all(|c| c.is_ascii_digit() || c == '.' || c == ',')
        {
            let t = crate::ueb::translate(source);
            return t.unsupported.is_empty().then_some(t.braille);
        }
        let code = match self.code {
            crate::MathCode::Nemeth => BrailleCode::Nemeth,
            crate::MathCode::Ueb => BrailleCode::Ueb,
        };
        let options = BrailleOptions {
            code,
            grade2: self.grade2,
        };
        braille_mathml(&mathml_for(&formula.math, formula.display), &options)
            .ok()
            .map(|cells| self.layout(&crate::ueb::from_unicode(&cells)))
    }

    #[cfg(not(feature = "mathcat"))]
    fn braille(&self, _formula: &Formula) -> Option<String> {
        None
    }

    /// The braille ASCII of an expression, ready for the line wrapper:
    /// every word marked as math, division points before operation signs,
    /// and for Nemeth the switch indicators bound to the math beside them.
    ///
    /// Spaces inside math stand around comparison signs (and after a
    /// function name). A short spaced word after the first (a comparison
    /// sign such as `.K`, or a function name such as `SIN`) is bound to the
    /// word after it, so a line divides before a comparison sign, never
    /// right after one.
    #[cfg_attr(not(feature = "mathcat"), allow(dead_code))]
    fn layout(&self, ascii: &str) -> String {
        /// The most cells a word bound to the next may have.
        const SHORT: usize = 3;
        let nemeth = self.code == crate::MathCode::Nemeth;
        let mut out = String::with_capacity(ascii.len() + 16);
        out.push(BRAILLE_MATH);
        if nemeth {
            out.push_str(NEMETH_OPEN);
            out.push(BRAILLE_NBSP);
        }
        let words: Vec<Vec<char>> = ascii.split(' ').map(|w| w.chars().collect()).collect();
        for (k, chars) in words.iter().enumerate() {
            if k > 0 {
                if k >= 2 && words[k - 1].len() <= SHORT {
                    out.push(BRAILLE_NBSP);
                } else {
                    out.push(' ');
                    out.push(BRAILLE_MATH);
                }
            }
            for (i, &c) in chars.iter().enumerate() {
                if i > 0 && division_before(chars, i, nemeth) {
                    out.push(BRAILLE_BREAK);
                }
                out.push(c);
            }
        }
        if nemeth {
            out.push(BRAILLE_NBSP);
            out.push_str(NEMETH_CLOSE);
        }
        out
    }

    /// `translated` (braille of text with placeholders) with each
    /// placeholder replaced by its formula's braille.
    pub fn splice(&self, translated: &str) -> String {
        if !translated.contains(PLACEHOLDER_OPEN) {
            return translated.to_owned();
        }
        let cells = self.cells.borrow();
        let mut out = String::with_capacity(translated.len() + 32);
        let mut rest = translated;
        while let Some(at) = rest.find(PLACEHOLDER_OPEN) {
            out.push_str(&rest[..at]);
            let after = &rest[at + PLACEHOLDER_OPEN.len_utf8()..];
            let Some(end) = after.find(PLACEHOLDER_CLOSE) else {
                rest = after;
                continue;
            };
            if let Some(b) = after[..end]
                .parse::<usize>()
                .ok()
                .and_then(|n| cells.get(n))
            {
                out.push_str(b);
            }
            rest = &after[end + PLACEHOLDER_CLOSE.len_utf8()..];
        }
        out.push_str(rest);
        out
    }
}

/// Splits text with formula placeholders into the pieces to translate and
/// the placeholders between them, so the placeholders never reach a
/// translator. Each piece keeps a single space where it had space at
/// either end (liblouis may trim it).
pub(crate) fn split_placeholders(text: &str) -> Vec<Piece<'_>> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find(PLACEHOLDER_OPEN) {
        let Some(len) = rest[at..].find(PLACEHOLDER_CLOSE) else {
            break;
        };
        let end = at + len + PLACEHOLDER_CLOSE.len_utf8();
        if at > 0 {
            out.push(Piece::Text(&rest[..at]));
        }
        out.push(Piece::Math(&rest[at..end]));
        rest = &rest[end..];
    }
    if !rest.is_empty() {
        out.push(Piece::Text(rest));
    }
    out
}

/// A piece of text around formula placeholders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Piece<'a> {
    /// Print text, to translate.
    Text(&'a str),
    /// A placeholder, kept as it is.
    Math(&'a str),
}

/// Operation signs a long expression may be divided before (the sign
/// starts the next line): plus and minus in Nemeth, with the baseline
/// indicator (dot 5) that comes back from a superscript or subscript kept
/// before the sign; plus, minus, times, and divided by (dot 5 and a second
/// cell) in UEB. Never right after an indicator, so an indicator stays
/// with the symbol it modifies.
fn division_before(chars: &[char], i: usize, nemeth: bool) -> bool {
    const INDICATORS: &str = ",;^\".#_@";
    let prev = chars[i - 1];
    if INDICATORS.contains(prev) {
        return false;
    }
    let c = chars[i];
    let sign = |c: Option<&char>| matches!(c, Some('+' | '-'));
    if nemeth {
        sign(Some(&c)) || (c == '"' && sign(chars.get(i + 1)))
    } else {
        c == '"' && matches!(chars.get(i + 1), Some('6' | '-' | '8' | '/'))
    }
}

/// Replaces every math span in `blocks` with `text(formula)` as plain text
/// (braille writes the spoken form).
pub(crate) fn replace_spans(blocks: &mut [Block], text: &dyn Fn(&Formula) -> String) {
    fn inlines(v: &mut [Inline], text: &dyn Fn(&Formula) -> String) {
        for i in v.iter_mut() {
            if let Inline::Span(Style::Math { display }, children) = i {
                let f = Formula::from_marked(&Inline::plain(children), *display);
                *i = Inline::Text(text(&f));
            } else if let Inline::Span(_, children) = i {
                inlines(children, text);
            }
        }
    }
    for b in blocks.iter_mut() {
        match b {
            Block::Heading { content, .. }
            | Block::Paragraph(content)
            | Block::Footnote { content, .. } => inlines(content, text),
            Block::List(list) => {
                for item in &mut list.items {
                    replace_spans(&mut item.blocks, text);
                }
            }
            Block::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        inlines(cell, text);
                    }
                }
            }
            Block::Quote(inner) => replace_spans(inner, text),
            Block::Code { .. }
            | Block::Figure(_)
            | Block::SectionBreak { .. }
            | Block::PageBreak { .. }
            | Block::Rule => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Linear Unicode.

fn superscript(c: char) -> Option<char> {
    Some(match c {
        '0' => '\u{2070}',
        '1' => '\u{B9}',
        '2' => '\u{B2}',
        '3' => '\u{B3}',
        '4'..='9' => char::from_u32(0x2074 + (c as u32 - '4' as u32))?,
        '+' => '\u{207A}',
        '-' | '\u{2212}' => '\u{207B}',
        '=' => '\u{207C}',
        '(' => '\u{207D}',
        ')' => '\u{207E}',
        'n' => '\u{207F}',
        'i' => '\u{2071}',
        _ => return None,
    })
}

fn subscript(c: char) -> Option<char> {
    Some(match c {
        '0'..='9' => char::from_u32(0x2080 + (c as u32 - '0' as u32))?,
        '+' => '\u{208A}',
        '-' | '\u{2212}' => '\u{208B}',
        '=' => '\u{208C}',
        '(' => '\u{208D}',
        ')' => '\u{208E}',
        'a' => '\u{2090}',
        'e' => '\u{2091}',
        'o' => '\u{2092}',
        'x' => '\u{2093}',
        'h' => '\u{2095}',
        'k' => '\u{2096}',
        'l' => '\u{2097}',
        'm' => '\u{2098}',
        'n' => '\u{2099}',
        'p' => '\u{209A}',
        's' => '\u{209B}',
        't' => '\u{209C}',
        'i' => '\u{1D62}',
        'j' => '\u{2C7C}',
        'r' => '\u{1D63}',
        'u' => '\u{1D64}',
        'v' => '\u{1D65}',
        _ => return None,
    })
}

/// `s` in raised or lowered characters when every char has one.
fn shifted(s: &str, map: fn(char) -> Option<char>) -> Option<String> {
    let t: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if t.is_empty() {
        return None;
    }
    t.chars().map(map).collect()
}

/// True for a node written as one token (no parentheses needed around it).
fn is_atom(node: &Node) -> bool {
    match &node.kind {
        NodeKind::Number(_) | NodeKind::Ident(_) | NodeKind::Fenced { .. } => true,
        NodeKind::Row(items) => items.len() == 1 && is_atom(&items[0]),
        NodeKind::Style { body, .. } => is_atom(body),
        NodeKind::Scripts { .. } | NodeKind::Accent { .. } => true,
        _ => false,
    }
}

fn wrap(s: String, atom: bool) -> String {
    if atom { s } else { format!("({s})") }
}

fn combining(accent: AccentKind) -> Option<char> {
    Some(match accent {
        AccentKind::Hat => '\u{302}',
        AccentKind::Bar => '\u{304}',
        AccentKind::Overline => '\u{305}',
        AccentKind::Tilde => '\u{303}',
        AccentKind::Vector => '\u{20D7}',
        AccentKind::LeftArrow => '\u{20D6}',
        AccentKind::LeftRightArrow => '\u{20E1}',
        AccentKind::Dot => '\u{307}',
        AccentKind::DoubleDot => '\u{308}',
        AccentKind::TripleDot => '\u{20DB}',
        AccentKind::Acute => '\u{301}',
        AccentKind::Grave => '\u{300}',
        AccentKind::Breve => '\u{306}',
        AccentKind::Check => '\u{30C}',
        AccentKind::Ring => '\u{30A}',
        AccentKind::Arc => '\u{311}',
        AccentKind::Underline => '\u{332}',
        AccentKind::Overbrace | AccentKind::Underbrace => return None,
    })
}

/// Double-struck capitals (`\mathbb{R}` as ℝ).
fn double_struck(c: char) -> char {
    match c {
        'C' => '\u{2102}',
        'H' => '\u{210D}',
        'N' => '\u{2115}',
        'P' => '\u{2119}',
        'Q' => '\u{211A}',
        'R' => '\u{211D}',
        'Z' => '\u{2124}',
        'A'..='Z' => char::from_u32(0x1D538 + (c as u32 - 'A' as u32)).unwrap_or(c),
        'a'..='z' => char::from_u32(0x1D552 + (c as u32 - 'a' as u32)).unwrap_or(c),
        '0'..='9' => char::from_u32(0x1D7D8 + (c as u32 - '0' as u32)).unwrap_or(c),
        _ => c,
    }
}

fn linear(math: &Math, node: &Node, depth: usize) -> String {
    if depth > MAX_DEPTH {
        return math.source_text(node.span);
    }
    let sub = |n: &Node| linear(math, n, depth + 1);
    match &node.kind {
        NodeKind::Row(items) => {
            let mut out = String::new();
            for (i, item) in items.iter().enumerate() {
                let prev = i.checked_sub(1).map(|p| &items[p]);
                let s = sub(item);
                match item.op_class() {
                    Some(OpClass::Binary) => {
                        // A leading or unary operator takes no spaces.
                        let unary = prev.is_none_or(|p| {
                            matches!(
                                p.op_class(),
                                Some(
                                    OpClass::Binary
                                        | OpClass::Relation
                                        | OpClass::Open
                                        | OpClass::Punctuation
                                        | OpClass::Large
                                )
                            )
                        });
                        if unary {
                            out.push_str(&s);
                        } else {
                            out.push_str(&format!(" {s} "));
                        }
                    }
                    Some(OpClass::Relation) => out.push_str(&format!(" {s} ")),
                    Some(OpClass::Punctuation) => out.push_str(&format!("{s} ")),
                    _ => {
                        // A function name is followed by a space before its
                        // argument ("sin x").
                        if prev.is_some_and(|p| matches!(p.kind, NodeKind::Function(_)))
                            && !s.starts_with('(')
                        {
                            out.push(' ');
                        }
                        out.push_str(&s);
                    }
                }
            }
            out
        }
        NodeKind::Number(s)
        | NodeKind::Ident(s)
        | NodeKind::Function(s)
        | NodeKind::Text(s)
        | NodeKind::Error(s) => s.clone(),
        NodeKind::Operator { text, .. } => text.clone(),
        NodeKind::Space(_) => " ".to_owned(),
        NodeKind::Fraction { num, den, bar } => {
            let (n, d) = (wrap(sub(num), is_atom(num)), wrap(sub(den), is_atom(den)));
            if *bar {
                format!("{n}/{d}")
            } else {
                format!("({}\u{A6}{})", sub(num), sub(den))
            }
        }
        NodeKind::Root { index, radicand } => {
            let r = wrap(sub(radicand), is_atom(radicand));
            match index.as_deref().map(sub).as_deref() {
                None => format!("\u{221A}{r}"),
                Some("3") => format!("\u{221B}{r}"),
                Some("4") => format!("\u{221C}{r}"),
                Some(i) => match shifted(i, superscript) {
                    Some(sup) => format!("{sup}\u{221A}{r}"),
                    None => format!("root({i}, {r})"),
                },
            }
        }
        NodeKind::Scripts {
            base, sub: lo, sup, ..
        } => {
            let mut out = sub(base);
            if let Some(lo) = lo {
                let s = sub(lo);
                match shifted(&s, subscript) {
                    Some(t) => out.push_str(&t),
                    None => out.push_str(&format!("_{}", wrap(s, is_atom(lo)))),
                }
            }
            if let Some(hi) = sup {
                let s = sub(hi);
                match shifted(&s, superscript) {
                    Some(t) => out.push_str(&t),
                    None => out.push_str(&format!("^{}", wrap(s, is_atom(hi)))),
                }
            }
            out
        }
        NodeKind::Fenced { open, close, body } => format!("{open}{}{close}", sub(body)),
        NodeKind::Accent { base, accent } => {
            let b = sub(base);
            match (combining(*accent), b.chars().count()) {
                (Some(mark), 1) => format!("{b}{mark}"),
                (Some(mark), _) if matches!(accent, AccentKind::Overline) => {
                    b.chars().flat_map(|c| [c, mark]).collect()
                }
                _ => format!("{}({b})", accent.mark()),
            }
        }
        NodeKind::Table { kind, rows } => {
            let rows: Vec<String> = rows
                .iter()
                .map(|r| r.iter().map(sub).collect::<Vec<_>>().join(", "))
                .collect();
            match kind {
                TableKind::Cases => format!("{{ {}", rows.join("; ")),
                _ => rows.join("; "),
            }
        }
        NodeKind::Style { variant, body } => {
            let b = sub(body);
            if matches!(variant, Variant::DoubleStruck) {
                b.chars().map(double_struck).collect()
            } else {
                b
            }
        }
        NodeKind::Enclose { notation, body } => match notation {
            Enclosure::Cancel => sub(body).chars().flat_map(|c| [c, '\u{336}']).collect(),
            Enclosure::Box => format!("[{}]", sub(body)),
        },
    }
}

// ---------------------------------------------------------------------------
// Office Math (OMML).

fn run(text: &str, props: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let rpr = if props.is_empty() {
        String::new()
    } else {
        format!("<m:rPr>{props}</m:rPr>")
    };
    format!(
        "<m:r>{rpr}<m:t xml:space=\"preserve\">{}</m:t></m:r>",
        xml::text(text)
    )
}

fn omml(math: &Math, node: &Node, depth: usize) -> String {
    if depth > MAX_DEPTH {
        return run(&math.source_text(node.span), "<m:nor/>");
    }
    let sub = |n: &Node| omml(math, n, depth + 1);
    let e = |n: &Node| format!("<m:e>{}</m:e>", omml(math, n, depth + 1));
    match &node.kind {
        NodeKind::Row(items) => items.iter().map(sub).collect(),
        NodeKind::Number(s) | NodeKind::Ident(s) | NodeKind::Error(s) => run(s, ""),
        NodeKind::Function(s) => run(s, "<m:sty m:val=\"p\"/>"),
        NodeKind::Text(s) => run(s, "<m:nor/>"),
        NodeKind::Operator { text, .. } => run(text, ""),
        NodeKind::Space(_) => run(" ", ""),
        NodeKind::Fraction { num, den, bar } => {
            let pr = if *bar {
                String::new()
            } else {
                "<m:fPr><m:type m:val=\"noBar\"/></m:fPr>".to_owned()
            };
            format!(
                "<m:f>{pr}<m:num>{}</m:num><m:den>{}</m:den></m:f>",
                sub(num),
                sub(den)
            )
        }
        NodeKind::Root { index, radicand } => match index {
            None => format!(
                "<m:rad><m:radPr><m:degHide m:val=\"1\"/></m:radPr><m:deg/>{}</m:rad>",
                e(radicand)
            ),
            Some(i) => format!("<m:rad><m:deg>{}</m:deg>{}</m:rad>", sub(i), e(radicand)),
        },
        NodeKind::Scripts {
            base,
            sub: lo,
            sup,
            limits,
        } => {
            if *limits {
                let mut out = sub(base);
                if let Some(lo) = lo {
                    out = format!(
                        "<m:limLow><m:e>{out}</m:e><m:lim>{}</m:lim></m:limLow>",
                        sub(lo)
                    );
                }
                if let Some(hi) = sup {
                    out = format!(
                        "<m:limUpp><m:e>{out}</m:e><m:lim>{}</m:lim></m:limUpp>",
                        sub(hi)
                    );
                }
                return out;
            }
            match (lo, sup) {
                (Some(lo), Some(hi)) => format!(
                    "<m:sSubSup>{}<m:sub>{}</m:sub><m:sup>{}</m:sup></m:sSubSup>",
                    e(base),
                    sub(lo),
                    sub(hi)
                ),
                (Some(lo), None) => {
                    format!("<m:sSub>{}<m:sub>{}</m:sub></m:sSub>", e(base), sub(lo))
                }
                (None, Some(hi)) => {
                    format!("<m:sSup>{}<m:sup>{}</m:sup></m:sSup>", e(base), sub(hi))
                }
                (None, None) => sub(base),
            }
        }
        NodeKind::Fenced { open, close, body } => format!(
            "<m:d><m:dPr><m:begChr m:val=\"{}\"/><m:endChr m:val=\"{}\"/></m:dPr>{}</m:d>",
            xml::attr(open),
            xml::attr(close),
            e(body)
        ),
        NodeKind::Accent { base, accent } => match accent {
            AccentKind::Overline | AccentKind::Underline => format!(
                "<m:bar><m:barPr><m:pos m:val=\"{}\"/></m:barPr>{}</m:bar>",
                if accent.is_under() { "bot" } else { "top" },
                e(base)
            ),
            AccentKind::Overbrace | AccentKind::Underbrace => format!(
                "<m:groupChr><m:groupChrPr><m:chr m:val=\"{}\"/><m:pos m:val=\"{}\"/></m:groupChrPr>{}</m:groupChr>",
                accent.mark(),
                if accent.is_under() { "bot" } else { "top" },
                e(base)
            ),
            _ => {
                let chr = combining(*accent).map_or_else(|| accent.mark().to_owned(), String::from);
                format!(
                    "<m:acc><m:accPr><m:chr m:val=\"{}\"/></m:accPr>{}</m:acc>",
                    xml::attr(&chr),
                    e(base)
                )
            }
        },
        NodeKind::Table { kind, rows } => match kind {
            TableKind::Aligned => {
                let rows: String = rows
                    .iter()
                    .map(|r| format!("<m:e>{}</m:e>", r.iter().map(sub).collect::<String>()))
                    .collect();
                format!("<m:eqArr>{rows}</m:eqArr>")
            }
            _ => {
                let rows: String = rows
                    .iter()
                    .map(|r| format!("<m:mr>{}</m:mr>", r.iter().map(e).collect::<String>()))
                    .collect();
                format!("<m:m>{rows}</m:m>")
            }
        },
        NodeKind::Style { variant, body } => match variant {
            Variant::DoubleStruck => {
                let text: String = linear(math, body, depth + 1)
                    .chars()
                    .map(double_struck)
                    .collect();
                run(&text, "")
            }
            Variant::Normal => upright(&sub(body)),
            _ => sub(body),
        },
        NodeKind::Enclose { body, .. } => {
            format!("<m:borderBox>{}</m:borderBox>", e(body))
        }
    }
}

/// Runs made upright (`\mathrm`).
fn upright(runs: &str) -> String {
    runs.replace("<m:r><m:t", "<m:r><m:rPr><m:sty m:val=\"p\"/></m:rPr><m:t")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(src: &str) -> Formula {
        Formula::from_marked(src, false)
    }

    #[test]
    fn delimiters_are_stripped() {
        assert_eq!(f("$\\pi r^2$").source(), "\\pi r^2");
        assert!(f("$$x$$").display);
        assert!(f("\\[x\\]").display);
        assert!(!f("\\(x\\)").display);
        assert_eq!(f("x + 1").source(), "x + 1");
        assert!(Formula::from_marked("$x$", true).display);
    }

    #[test]
    fn linear_forms_read_like_print() {
        assert_eq!(f("$\\pi r^2$").linear(), "\u{3C0}r\u{B2}");
        assert_eq!(f("$\\frac{a+b}{2}$").linear(), "(a + b)/2");
        assert_eq!(f("$\\frac{a}{b}$").linear(), "a/b");
        assert_eq!(f("$\\sqrt{x}$").linear(), "\u{221A}x");
        assert_eq!(f("$\\sqrt[3]{x+1}$").linear(), "\u{221B}(x + 1)");
        assert_eq!(
            f("$x_1 + x_{n+1}$").linear(),
            "x\u{2081} + x\u{2099}\u{208A}\u{2081}"
        );
        assert_eq!(f("$e^{i\\pi} = -1$").linear(), "e^(i\u{3C0}) = \u{2212}1");
        assert_eq!(f("$\\sin x$").linear(), "sin x");
        assert_eq!(f("$\\mathbb{R}$").linear(), "\u{211D}");
        assert_eq!(f("$\\bar{x}$").linear(), "x\u{304}");
        assert!(!f("$\\pi r^2$").linear().contains('$'));
    }

    #[test]
    fn spoken_and_mathml() {
        assert_eq!(f("$\\pi r^2$").spoken(), "pi r squared");
        let m = f("$x^2$").mathml();
        assert!(m.starts_with("<math"), "{m}");
        assert!(m.contains("<msup>"), "{m}");
    }

    #[test]
    fn omml_is_well_formed_office_math() {
        for src in [
            "$\\frac{a}{b}$",
            "$\\sqrt[3]{x}$",
            "$x_i^2$",
            "$\\sum_{i=1}^{n} i$",
            "$\\left( x \\right)$",
            "$\\hat{x} + \\overline{y}$",
            "$\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}$",
            "$\\text{if } x < y$",
            "$\\mathbb{R}$",
        ] {
            let o = format!(
                "<w xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\">{}</w>",
                f(src).omml()
            );
            let doc = roxmltree::Document::parse(&o).unwrap_or_else(|e| panic!("{src}: {e}\n{o}"));
            assert!(doc.descendants().any(|n| n.tag_name().name() == "oMath"));
        }
        let o = f("$\\frac{a}{b}$").omml();
        assert!(o.contains("<m:f><m:num>"), "{o}");
        let d = Formula::from_marked("$$x$$", true).omml();
        assert!(d.starts_with("<m:oMathPara><m:oMath>"), "{d}");
    }

    #[test]
    fn deep_formulas_do_not_overflow() {
        let src = format!("${}x{}$", "{".repeat(5_000), "}".repeat(5_000));
        let f = f(&src);
        let _ = f.linear();
        let _ = f.omml();
    }
}
