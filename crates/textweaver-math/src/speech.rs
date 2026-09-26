//! Spoken math: natural English with ClearSpeak-style wording at three
//! verbosity levels, built with [`SpokenBuilder`] so every spoken word maps
//! back to the source chars it came from (ADR-0005).
//!
//! # Wording
//!
//! | Construct | Low | Normal | High |
//! |---|---|---|---|
//! | `\frac{a}{b}` | a over b | a over b | the fraction a over b end fraction |
//! | `\frac{x+1}{2}` | fraction x plus 1 over 2 | the fraction with numerator x plus 1 and denominator 2 | (normal) + end fraction |
//! | `\frac{3}{4}` | 3 fourths | 3 fourths | the fraction 3 over 4 end fraction |
//! | `\sqrt{x}` | square root of x | square root of x | the square root of x end root |
//! | `x^2`, `x^3` | x squared, x cubed | (same) | (same) |
//! | `x^n` | x to the n | x to the n-th power | x to the n-th power |
//! | `x^{n+1}` | x to the n plus 1 | x raised to the n plus 1 power | x raised to the exponent n plus 1 end exponent |
//! | `x_i` | x sub i | x sub i | x sub i |
//! | `\sum_{i=1}^n` | sum from i equals 1 to n of | the sum from … | the sum from … |
//! | `\|x\|` | absolute value of x | the absolute value of x | … end absolute value |
//!
//! End markers ("end fraction", "end root", "end sub", "end exponent") are
//! spoken at high verbosity, and at the other levels only when the part is
//! not simple and something follows it, so "x plus 1 over 2" never
//! becomes ambiguous.
//!
//! # Offset maps
//!
//! Words that equal their source (`x`, `3.5`, `\text{if}`) are literal
//! spans; words that stand for source ("squared" for `^2`, "alpha" for
//! `\alpha`, "over" for `}{`) are expanded spans; words with no source of
//! their own ("end fraction", "row 1:") are inserted. Where English word
//! order differs from source order ("x hat" for `\hat{x}`), the word that
//! would point backwards becomes an inserted span, so the map stays valid.

use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange, OffsetMap, SpokenBuilder, Verbosity};

use crate::symbols::{function_words, is_trig, takes_limits, words};
use crate::{AccentKind, Enclosure, Math, Node, NodeKind, OpClass, TableKind, Variant};

/// Options for spoken math.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpeechOptions {
    /// How explicit the wording is.
    pub verbosity: Verbosity,
}

impl SpeechOptions {
    /// Options at `verbosity`.
    pub fn new(verbosity: Verbosity) -> Self {
        SpeechOptions { verbosity }
    }
}

/// Spoken text and its map back to the math source (chars from 0).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spoken {
    /// The text to hand to the speech engine.
    pub text: String,
    /// Map from spoken bytes to source chars of [`Math::source`].
    pub map: OffsetMap,
}

/// Speaks a whole expression.
pub fn speak(math: &Math, opts: &SpeechOptions) -> Spoken {
    speak_node(math, &math.root, opts)
}

/// Speaks one node of `math` (as navigation does). Source positions in the
/// map are chars of [`Math::source`].
pub fn speak_node(math: &Math, node: &Node, opts: &SpeechOptions) -> Spoken {
    let mut b = SpokenBuilder::new();
    speak_node_into(math, node, opts, &mut b, 0);
    let (text, map) = b.finish();
    Spoken { text, map }
}

/// Appends the spoken form of `node` to `b`, with source positions shifted
/// by `base` chars. Returns false when nothing was spoken.
pub(crate) fn speak_node_into(
    math: &Math,
    node: &Node,
    opts: &SpeechOptions,
    b: &mut SpokenBuilder,
    base: usize,
) -> bool {
    let chars = math.source_chars();
    let mut s = Speaker::new(&chars, opts.verbosity);
    s.node(node);
    s.finish_into(b, base)
}

#[derive(Clone, Copy, Debug)]
enum Src {
    /// Source chars the word stands for.
    Map(CharRange),
    /// No source; resume anchor.
    At(usize),
}

#[derive(Clone, Debug)]
struct Piece {
    text: String,
    src: Src,
    /// No space before this piece (punctuation).
    glue: bool,
}

/// How a function takes its argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Apply {
    /// Trigonometric: "sine x", "sine of x" only with parentheses.
    Trig,
    /// Named function: "log of x".
    Of,
    /// A letter used as a function (`f`): "f of x" only with parentheses.
    Letter,
}

struct Speaker<'a> {
    src: &'a [char],
    v: Verbosity,
    out: Vec<Piece>,
    /// Something follows the current node at an enclosing level, so a
    /// complex part needs an end marker.
    more: bool,
    /// Inside set braces: `|` and `:` read "such that".
    in_set: bool,
    /// The current row has a colon, so `→` reads "to" (`f: A → B`).
    arrow_to: bool,
}

fn range(a: impl Into<CharPos>, b: impl Into<CharPos>) -> CharRange {
    let (a, b) = (a.into(), b.into());
    if a <= b {
        CharRange::new(a, b)
    } else {
        CharRange::empty(a)
    }
}

impl<'a> Speaker<'a> {
    fn new(src: &'a [char], v: Verbosity) -> Self {
        Speaker {
            src,
            v,
            out: Vec::new(),
            more: false,
            in_set: false,
            arrow_to: false,
        }
    }

    fn low(&self) -> bool {
        self.v == Verbosity::Low
    }

    fn high(&self) -> bool {
        self.v == Verbosity::High
    }

    /// "the " except at low verbosity.
    fn the(&self) -> &'static str {
        if self.low() { "" } else { "the " }
    }

    /// Speaks `text` for the source chars `r`. When the text appears
    /// verbatim inside `r`, the piece narrows to that occurrence and
    /// becomes a literal span.
    fn say(&mut self, text: &str, r: CharRange) {
        if text.is_empty() {
            return;
        }
        let src = if r.is_empty() {
            Src::At(r.start.0)
        } else {
            Src::Map(self.narrow(text, r))
        };
        self.out.push(Piece {
            text: text.to_owned(),
            src,
            glue: false,
        });
    }

    fn narrow(&self, text: &str, r: CharRange) -> CharRange {
        let t: Vec<char> = text.chars().collect();
        let end = r.end.0.min(self.src.len());
        let start = r.start.0.min(end);
        let hay = &self.src[start..end];
        if t.len() <= hay.len()
            && let Some(k) = hay.windows(t.len()).position(|w| w == t.as_slice())
        {
            return CharRange::new(start + k, start + k + t.len());
        }
        r
    }

    fn ins(&mut self, text: &str, at: impl Into<CharPos>) {
        if text.is_empty() {
            return;
        }
        self.out.push(Piece {
            text: text.to_owned(),
            src: Src::At(at.into().0),
            glue: false,
        });
    }

    /// Punctuation attached to the previous word (a pause for the engine).
    fn glue(&mut self, text: &str, src: Src) {
        self.out.push(Piece {
            text: text.to_owned(),
            src,
            glue: true,
        });
    }

    /// Speaks `n` with `more` set as given.
    fn part(&mut self, n: &Node, more: bool) {
        let saved = self.more;
        self.more = more;
        self.node(n);
        self.more = saved;
    }

    /// The range from a script marker (`^`, `_`) before `child` to the
    /// child's start, or an empty range at the child when there is none.
    fn lead(&self, child: &Node, mark: char) -> CharRange {
        let mut j = child.span.start.0;
        while j > 0 {
            let c = self.src.get(j - 1).copied().unwrap_or(' ');
            if c == mark {
                return range(j - 1, child.span.start);
            }
            if c.is_whitespace() || c == '{' || c == '(' || c == '[' {
                j -= 1;
            } else {
                break;
            }
        }
        CharRange::empty(child.span.start)
    }

    fn node(&mut self, n: &Node) {
        match &n.kind {
            NodeKind::Row(items) => self.row(items),
            NodeKind::Number(s) => self.say(s, n.span),
            NodeKind::Ident(s) => self.ident(s, n.span),
            NodeKind::Function(name) => self.say(&function_words(name), n.span),
            NodeKind::Operator { text, class } => self.operator(text, *class, n.span),
            NodeKind::Text(t) => {
                let t = t.trim();
                if !t.is_empty() {
                    self.say(t, n.span);
                }
            }
            NodeKind::Space(_) => {}
            NodeKind::Fraction { num, den, bar } => self.fraction(n, num, den, *bar),
            NodeKind::Root { index, radicand } => self.root(n, index.as_deref(), radicand),
            NodeKind::Scripts { .. } => {
                if is_big_op(n) {
                    self.big_op(n, false);
                } else {
                    self.scripts(n);
                }
            }
            NodeKind::Fenced { open, close, body } => self.fenced(n, open, close, body, false),
            NodeKind::Accent { base, accent } => self.accent(n, base, *accent),
            NodeKind::Table { kind, rows } => self.table(n, *kind, rows, "", ""),
            NodeKind::Style { variant, body } => self.style(n, *variant, body),
            NodeKind::Enclose { notation, body } => self.enclose(n, *notation, body),
            NodeKind::Error(s) => {
                let t = s.trim_start_matches('\\').trim();
                if !t.is_empty() {
                    self.say(t, n.span);
                }
            }
        }
    }

    fn row(&mut self, items: &[Node]) {
        let items: Vec<&Node> = items
            .iter()
            .filter(|n| !matches!(n.kind, NodeKind::Space(_)))
            .collect();
        let outer_more = self.more;
        let saved_arrow = self.arrow_to;
        self.arrow_to = items.iter().any(|n| n.is_op(":"));
        let mut i = 0;
        while i < items.len() {
            let it = items[i];
            let prev = i.checked_sub(1).map(|j| items[j]);
            let next = items.get(i + 1).copied();
            self.more = outer_more || next.is_some();
            if let NodeKind::Operator { text, .. } = &it.kind
                && unary_context(prev)
            {
                let w = match text.as_str() {
                    "−" | "-" => Some("negative"),
                    "+" => Some("positive"),
                    _ => None,
                };
                if let Some(w) = w {
                    self.say(w, it.span);
                    i += 1;
                    continue;
                }
            }
            if is_big_op(it) {
                let has_next = next.is_some_and(starts_operand);
                self.big_op(it, has_next);
                i += 1;
                continue;
            }
            if let Some(apply) = function_like(it) {
                self.node(it);
                if let Some(n) = next {
                    if let NodeKind::Fenced { open, close, body } = &n.kind
                        && open == "("
                        && close == ")"
                    {
                        self.ins("of", n.span.start);
                        self.more = outer_more || items.get(i + 2).is_some();
                        if is_simple(body) && !self.high() {
                            self.node(body);
                        } else {
                            self.fenced(n, open, close, body, true);
                        }
                        i += 2;
                        continue;
                    }
                    if apply == Apply::Of && starts_operand(n) {
                        self.ins("of", n.span.start);
                    }
                }
                i += 1;
                continue;
            }
            self.node(it);
            i += 1;
        }
        self.more = outer_more;
        self.arrow_to = saved_arrow;
    }

    fn ident(&mut self, s: &str, span: CharRange) {
        if let Some(w) = words(s) {
            let w = if self.low() {
                w.trim_start_matches("capital ").trim_start_matches("the ")
            } else {
                w
            };
            self.say(w, span);
            return;
        }
        if self.high() && s.chars().count() == 1 && s.chars().all(char::is_uppercase) {
            self.ins("cap", span.start);
        }
        self.say(s, span);
    }

    fn operator(&mut self, text: &str, _class: OpClass, span: CharRange) {
        let w: String = match text {
            "→" | "⟶" if self.arrow_to => "to".into(),
            "|" | "∣" | ":" if self.in_set => "such that".into(),
            "," | ";" if self.low() => {
                self.glue(text, Src::Map(span));
                return;
            }
            "." => {
                self.glue(".", Src::Map(span));
                return;
            }
            _ => match words(text) {
                Some(w) => w.into(),
                None => text.into(),
            },
        };
        self.say(&w, span);
    }

    fn fraction(&mut self, n: &Node, num: &Node, den: &Node, bar: bool) {
        let prefix = range(n.span.start, num.span.start);
        let mid = range(num.span.end, den.span.start);
        let tail = range(den.span.end, n.span.end);
        let outer = self.more;
        if !bar {
            self.part(num, false);
            self.say("above", mid);
            self.part(den, outer);
            return;
        }
        if !self.high()
            && let Some((a, b)) = common_fraction(num, den)
        {
            self.say(&a, num.span);
            self.say(&b, den.span);
            return;
        }
        let simple = fraction_part_simple(num) && fraction_part_simple(den);
        match (self.v, simple) {
            (Verbosity::High, true) => {
                self.say("the fraction", prefix);
                self.part(num, false);
                self.say("over", mid);
                self.part(den, false);
                self.say("end fraction", tail);
            }
            (_, true) => {
                self.part(num, false);
                self.say("over", mid);
                self.part(den, outer);
                // "d over d x, e to the x": a pause ends a two-word
                // denominator.
                if outer && matches!(den.kind, NodeKind::Row(_)) {
                    self.glue(",", Src::At(den.span.end.0));
                }
            }
            (Verbosity::Low, false) => {
                self.say("fraction", prefix);
                self.part(num, false);
                self.say("over", mid);
                self.part(den, false);
                if outer {
                    self.say("end fraction", tail);
                }
            }
            (v, false) => {
                self.say("the fraction with numerator", prefix);
                self.part(num, false);
                self.say("and denominator", mid);
                self.part(den, false);
                if outer || v == Verbosity::High {
                    self.say("end fraction", tail);
                }
            }
        }
    }

    fn root(&mut self, n: &Node, index: Option<&Node>, rad: &Node) {
        let outer = self.more;
        let end = self.high() || (!is_simple(rad) && outer);
        let head = range(n.span.start, rad.span.start);
        let tail = range(rad.span.end, n.span.end);
        let high_the = if self.high() { "the " } else { "" };
        match index.map(|ix| (ix, index_kind(ix))) {
            None | Some((_, IndexKind::Square)) => {
                self.say(&format!("{high_the}square root of"), head);
            }
            Some((_, IndexKind::Cube)) => self.say(&format!("{high_the}cube root of"), head),
            Some((ix, IndexKind::Ordinal(word))) => {
                if self.high() {
                    self.ins("the", n.span.start);
                }
                self.say(&word, ix.span);
                self.say("root of", range(ix.span.end, rad.span.start));
            }
            Some((ix, IndexKind::Other)) => {
                self.say(
                    &format!("{}root with index", self.the()),
                    range(n.span.start, ix.span.start),
                );
                self.part(ix, false);
                self.say("of", range(ix.span.end, rad.span.start));
            }
        }
        self.part(rad, !end && outer);
        if end {
            self.say("end root", tail);
        }
    }

    fn scripts(&mut self, n: &Node) {
        let NodeKind::Scripts {
            base,
            sub,
            sup,
            limits,
        } = &n.kind
        else {
            return;
        };
        let (sub, sup) = (sub.as_deref(), sup.as_deref());
        if function_like(base).is_some() {
            self.function_scripts(base, sub, sup);
            return;
        }
        if *limits {
            self.stacked(base, sub, sup);
            return;
        }
        let outer = self.more;
        match &base.kind {
            // `(-1)^n` keeps its parentheses so the minus stays inside.
            NodeKind::Fenced { open, close, body } => self.fenced(base, open, close, body, true),
            _ => self.part(base, true),
        }
        self.more = outer;
        if let Some(s) = sub {
            self.subscript(s, sup.is_some());
        }
        if let Some(p) = sup {
            self.superscript(p);
        }
    }

    /// Scripts set directly below and above a base that is not a large
    /// operator (`\overset{def}{=}`, `\underbrace{…}_{n}`).
    fn stacked(&mut self, base: &Node, sub: Option<&Node>, sup: Option<&Node>) {
        let outer = self.more;
        self.part(base, true);
        if let Some(s) = sub {
            self.say("with", self.lead(s, '_'));
            self.part(s, false);
            self.ins("below", s.span.end);
        }
        if let Some(p) = sup {
            self.say("with", self.lead(p, '^'));
            self.part(p, false);
            self.ins("above", p.span.end);
        }
        self.more = outer;
    }

    fn subscript(&mut self, s: &Node, has_sup: bool) {
        let outer = self.more;
        self.say("sub", self.lead(s, '_'));
        let end = !simple_script(s) && (self.high() || outer || has_sup);
        self.part(s, !end && (outer || has_sup));
        if end {
            self.ins("end sub", s.span.end);
        }
    }

    fn superscript(&mut self, p: &Node) {
        let outer = self.more;
        let op = self.lead(p, '^');
        let whole = if op.is_empty() {
            p.span
        } else {
            range(op.start, p.span.end)
        };
        match sup_kind(p) {
            SupKind::Prime(w) => self.say(w, whole),
            SupKind::Degrees => self.say("degrees", whole),
            SupKind::Squared => self.say("squared", whole),
            SupKind::Cubed => self.say("cubed", whole),
            SupKind::Ordinal(num, ord) => {
                self.say("to the", op);
                if self.low() {
                    self.say(&num, p.span);
                } else {
                    self.say(&ord, p.span);
                    self.ins("power", p.span.end);
                }
            }
            SupKind::Letter => {
                self.say("to the", op);
                self.part(p, false);
            }
            SupKind::Simple => {
                self.say("to the", op);
                self.part(p, false);
                if !self.low() {
                    self.ins("power", p.span.end);
                }
            }
            SupKind::Operator(w) => {
                self.say("superscript", op);
                self.say(&w, p.span);
            }
            SupKind::Complex => match self.v {
                Verbosity::Low => {
                    self.say("to the", op);
                    self.part(p, false);
                    if outer {
                        self.ins("end exponent", p.span.end);
                    }
                }
                Verbosity::Normal => {
                    self.say("raised to the", op);
                    self.part(p, false);
                    self.ins("power", p.span.end);
                }
                Verbosity::High => {
                    self.say("raised to the exponent", op);
                    self.part(p, false);
                    self.ins("end exponent", p.span.end);
                }
            },
        }
    }

    /// `\sin^2`, `\sin^{-1}`, `\log_2`, `f'`.
    fn function_scripts(&mut self, base: &Node, sub: Option<&Node>, sup: Option<&Node>) {
        let named = matches!(base.kind, NodeKind::Function(_));
        if named
            && let Some(p) = sup
            && is_minus_one(p)
        {
            let op = self.lead(p, '^');
            let whole = if op.is_empty() {
                p.span
            } else {
                range(op.start, p.span.end)
            };
            self.say("inverse", whole);
            self.part(base, true);
            if let Some(s) = sub {
                self.subscript(s, false);
            }
            return;
        }
        self.part(base, true);
        if let Some(s) = sub {
            if matches!(&base.kind, NodeKind::Function(f) if f == "log" || f == "lg") {
                self.say("base", self.lead(s, '_'));
                self.part(s, true);
            } else {
                self.subscript(s, sup.is_some());
            }
        }
        if let Some(p) = sup {
            self.superscript(p);
        }
    }

    /// `\sum_{i=1}^{n}`, `\int_0^1`, `\lim_{x\to0}`: "the sum from i equals
    /// 1 to n of".
    fn big_op(&mut self, n: &Node, has_next: bool) {
        let (base, sub, sup) = match &n.kind {
            NodeKind::Scripts { base, sub, sup, .. } => (&**base, sub.as_deref(), sup.as_deref()),
            _ => (n, None, None),
        };
        let (word, is_lim) = match &base.kind {
            NodeKind::Function(f) => (
                function_words(f),
                matches!(f.as_str(), "lim" | "liminf" | "limsup" | "Lim"),
            ),
            NodeKind::Operator { text, .. } => (words(text).unwrap_or(text).to_owned(), false),
            NodeKind::Scripts { .. } => {
                // Nested scripts on a big operator: speak the inner one.
                self.node(base);
                (String::new(), false)
            }
            _ => (String::new(), false),
        };
        if !word.is_empty() {
            self.say(&format!("{}{word}", self.the()), base.span);
        }
        match (sub, sup) {
            (Some(s), Some(p)) => {
                self.say(if is_lim { "as" } else { "from" }, self.lead(s, '_'));
                self.part(s, false);
                self.say("to", self.lead(p, '^'));
                self.part(p, false);
            }
            (Some(s), None) => {
                self.say(if is_lim { "as" } else { "over" }, self.lead(s, '_'));
                self.part(s, false);
            }
            (None, Some(p)) => {
                self.say("to", self.lead(p, '^'));
                self.part(p, false);
            }
            (None, None) => {}
        }
        if has_next {
            self.ins("of", n.span.end);
        }
    }

    fn fenced(&mut self, n: &Node, open: &str, close: &str, body: &Node, keep: bool) {
        let open_r = range(n.span.start, body.span.start);
        let close_r = range(body.span.end, n.span.end);
        if let NodeKind::Table { kind, rows } = &body.kind {
            self.table(n, *kind, rows, open, close);
            return;
        }
        if let NodeKind::Fraction {
            num,
            den,
            bar: false,
        } = &body.kind
            && open == "("
        {
            let outer = self.more;
            self.part(num, false);
            self.say("choose", range(num.span.end, den.span.start));
            self.part(den, outer);
            return;
        }
        match (open, close) {
            ("|", "|") => self.named_fence(body, "absolute value", open_r, close_r),
            ("‖", "‖") => self.named_fence(body, "norm", open_r, close_r),
            ("⌊", "⌋") => self.named_fence(body, "floor", open_r, close_r),
            ("⌈", "⌉") => self.named_fence(body, "ceiling", open_r, close_r),
            ("{", "}") => self.set(body, open_r, close_r),
            ("", "") => self.node(body),
            _ => {
                if !keep && !self.high() && open == "(" && close == ")" && is_simple(body) {
                    self.node(body);
                    return;
                }
                let outer = self.more;
                if !open.is_empty() {
                    self.say(words(open).unwrap_or(open), open_r);
                }
                self.part(body, false);
                if !close.is_empty() {
                    self.say(words(close).unwrap_or(close), close_r);
                }
                self.more = outer;
            }
        }
    }

    fn named_fence(&mut self, body: &Node, word: &str, open_r: CharRange, close_r: CharRange) {
        let outer = self.more;
        let end = self.high() || (!is_simple(body) && outer);
        self.say(&format!("{}{word} of", self.the()), open_r);
        self.part(body, !end && outer);
        if end {
            self.say(&format!("end {word}"), close_r);
        }
    }

    fn set(&mut self, body: &Node, open_r: CharRange, close_r: CharRange) {
        let outer = self.more;
        let items: &[Node] = match &body.kind {
            NodeKind::Row(items) => items,
            _ => std::slice::from_ref(body),
        };
        if items.is_empty() {
            self.say(
                &format!("{}empty set", self.the()),
                range(open_r.start, close_r.end),
            );
            return;
        }
        let split = items
            .iter()
            .position(|n| n.is_op("|") || n.is_op("∣") || n.is_op(":"));
        let saved = self.in_set;
        match split {
            Some(k) => {
                self.say(&format!("{}set of all", self.the()), open_r);
                self.more = true;
                self.row(&items[..k]);
                self.in_set = true;
                self.node(&items[k]);
                self.in_set = saved;
                self.more = false;
                self.row(&items[k + 1..]);
            }
            None => {
                self.say(&format!("{}set", self.the()), open_r);
                self.part(body, false);
            }
        }
        self.more = outer;
        if self.high() || outer {
            self.say("end set", close_r);
        }
    }

    fn accent(&mut self, n: &Node, base: &Node, accent: AccentKind) {
        let head = range(n.span.start, base.span.start);
        let tail = range(base.span.end, n.span.end);
        let outer = self.more;
        let (word, postfix) = accent_words(accent);
        let short = is_simple(base) || letters_only(base);
        if short {
            if postfix {
                self.part(base, true);
                self.say(word, head);
            } else {
                self.say(word, head);
                self.part(base, outer);
            }
            return;
        }
        let end = self.high() || outer;
        self.say(&format!("{word} of"), head);
        self.part(base, false);
        if end {
            self.say(&format!("end {word}"), tail);
        }
    }

    fn table(&mut self, n: &Node, kind: TableKind, rows: &[Vec<Node>], open: &str, close: &str) {
        let outer = self.more;
        let nrows = rows.len();
        let ncols = rows.iter().map(Vec::len).max().unwrap_or(0);
        let first = rows.iter().flatten().next().map(|c| c.span.start);
        let last = rows.iter().flatten().last().map(|c| c.span.end);
        let head = range(n.span.start, first.unwrap_or(n.span.end));
        let tail = range(last.unwrap_or(n.span.end), n.span.end);
        let the = self.the();
        match kind {
            TableKind::Matrix | TableKind::Array => {
                let noun = if open == "|" && close == "|" {
                    "determinant"
                } else if kind == TableKind::Array && open.is_empty() {
                    "table"
                } else {
                    "matrix"
                };
                self.say(&format!("{the}{nrows} by {ncols} {noun}"), head);
                for (r, cells) in rows.iter().enumerate() {
                    let at = cells.first().map_or(n.span.end.0, |c| c.span.start.0);
                    self.glue(";", Src::At(at));
                    self.ins(&format!("row {}:", r + 1), at);
                    for (c, cell) in cells.iter().enumerate() {
                        if c > 0 {
                            self.glue(",", Src::At(cell.span.start.0));
                        }
                        if self.high() {
                            self.ins(&format!("column {}:", c + 1), cell.span.start);
                        }
                        self.cell(cell);
                    }
                }
                if self.high() || outer {
                    self.glue(";", Src::At(tail.start.0));
                    self.say(&format!("end {noun}"), tail);
                }
            }
            TableKind::Cases => {
                let count = if self.low() {
                    "cases".to_owned()
                } else {
                    format!("{nrows} cases")
                };
                self.say(&count, head);
                for (r, cells) in rows.iter().enumerate() {
                    let at = cells.first().map_or(n.span.end.0, |c| c.span.start.0);
                    self.glue(";", Src::At(at));
                    self.ins(&format!("case {}:", r + 1), at);
                    for (c, cell) in cells.iter().enumerate() {
                        if c > 0 {
                            self.glue(",", Src::At(cell.span.start.0));
                        }
                        self.part(cell, false);
                    }
                }
                if self.high() || outer {
                    self.glue(";", Src::At(tail.start.0));
                    self.say("end cases", tail);
                }
            }
            TableKind::Aligned => {
                if nrows <= 1 {
                    for cell in rows.iter().flatten() {
                        self.part(cell, true);
                    }
                    self.more = outer;
                    return;
                }
                self.say(&format!("{nrows} lines"), head);
                for (r, cells) in rows.iter().enumerate() {
                    let at = cells.first().map_or(n.span.end.0, |c| c.span.start.0);
                    self.glue(";", Src::At(at));
                    self.ins(&format!("line {}:", r + 1), at);
                    for cell in cells {
                        self.part(cell, true);
                    }
                }
                if self.high() || outer {
                    self.glue(";", Src::At(tail.start.0));
                    self.say("end lines", tail);
                }
            }
        }
        self.more = outer;
    }

    fn cell(&mut self, cell: &Node) {
        if cell.is_empty_row() {
            self.ins("blank", cell.span.start);
        } else {
            self.part(cell, false);
        }
    }

    fn style(&mut self, n: &Node, variant: Variant, body: &Node) {
        if self.high()
            && let Some(name) = variant.spoken_name()
        {
            self.say(name, range(n.span.start, body.span.start));
        }
        let outer = self.more;
        self.part(body, outer);
    }

    fn enclose(&mut self, n: &Node, notation: Enclosure, body: &Node) {
        let head = range(n.span.start, body.span.start);
        let tail = range(body.span.end, n.span.end);
        let (open, close) = match notation {
            Enclosure::Cancel => ("crossed out", "end crossed out"),
            Enclosure::Box => ("boxed", "end box"),
        };
        self.say(open, head);
        self.part(body, false);
        self.say(close, tail);
    }

    /// Writes the pieces into `b`, keeping the map valid: words whose
    /// source would run backwards become inserted, and anchors are clamped
    /// between their neighbours.
    fn finish_into(self, b: &mut SpokenBuilder, base: usize) -> bool {
        let mut pieces: Vec<Piece> = self
            .out
            .into_iter()
            .filter(|p| !p.text.is_empty())
            .collect();
        if pieces.is_empty() {
            return false;
        }
        // Pass 1: mapped sources must not go backwards.
        let mut last_end = 0usize;
        for p in &mut pieces {
            if let Src::Map(r) = p.src {
                if r.is_empty() {
                    p.src = Src::At(r.start.0);
                } else if r.start.0 < last_end {
                    if r.end.0 > last_end {
                        p.src = Src::Map(CharRange::new(last_end, r.end.0));
                        last_end = r.end.0;
                    } else {
                        p.src = Src::At(last_end);
                    }
                } else {
                    last_end = r.end.0;
                }
            }
        }
        // Pass 2: the first mapped start at or after each piece.
        let n = pieces.len();
        let mut next_start = vec![usize::MAX; n + 1];
        for i in (0..n).rev() {
            next_start[i] = match pieces[i].src {
                Src::Map(r) => r.start.0,
                Src::At(_) => next_start[i + 1],
            };
        }
        let mut cur = 0usize;
        for (i, p) in pieces.iter().enumerate() {
            let upper = next_start[i].max(cur);
            if i > 0 && !p.glue {
                b.push_inserted(" ", CharPos(base + cur));
            }
            match p.src {
                Src::Map(r) => {
                    let literal = r.end.0 <= self.src.len()
                        && self.src[r.start.0..r.end.0]
                            .iter()
                            .copied()
                            .eq(p.text.chars());
                    if literal {
                        b.push_literal(&p.text, CharPos(base + r.start.0));
                    } else {
                        b.push_expanded(&p.text, CharRange::new(base + r.start.0, base + r.end.0));
                    }
                    cur = cur.max(r.end.0);
                }
                Src::At(a) => {
                    let a = a.max(cur).min(upper);
                    b.push_inserted(&p.text, CharPos(base + a));
                    cur = a;
                }
            }
        }
        true
    }
}

/// True when a leading `-` or `+` after `prev` is a sign, not an operation.
fn unary_context(prev: Option<&Node>) -> bool {
    match prev {
        None => true,
        Some(n) => match &n.kind {
            NodeKind::Operator { class, .. } => matches!(
                class,
                OpClass::Binary
                    | OpClass::Relation
                    | OpClass::Open
                    | OpClass::Punctuation
                    | OpClass::Large
                    | OpClass::Prefix
                    | OpClass::Fence
            ),
            NodeKind::Function(_) => true,
            _ => false,
        },
    }
}

/// True when `n` begins an operand (so "of" can precede it).
fn starts_operand(n: &Node) -> bool {
    match &n.kind {
        NodeKind::Operator { class, text } => {
            matches!(class, OpClass::Open | OpClass::Prefix | OpClass::Large)
                || text == "−"
                || text == "∂"
        }
        NodeKind::Space(_) => false,
        _ => true,
    }
}

/// Large operators and limit-taking functions, with or without scripts.
fn is_big_op(n: &Node) -> bool {
    match &n.kind {
        NodeKind::Operator {
            class: OpClass::Large,
            ..
        } => true,
        NodeKind::Function(f) => takes_limits(f),
        NodeKind::Scripts { base, .. } => is_big_op(base),
        _ => false,
    }
}

/// How `n` applies to a following argument, if it is a function.
fn function_like(n: &Node) -> Option<Apply> {
    match &n.kind {
        NodeKind::Function(f) => {
            if takes_limits(f) || f == "mod" || f == "bmod" {
                None
            } else if is_trig(f) {
                Some(Apply::Trig)
            } else {
                Some(Apply::Of)
            }
        }
        NodeKind::Ident(s) if matches!(s.as_str(), "f" | "g" | "h" | "F" | "G" | "H") => {
            Some(Apply::Letter)
        }
        NodeKind::Scripts {
            base,
            limits: false,
            ..
        } => function_like(base),
        _ => None,
    }
}

/// A single number or letter, possibly negative or in a font.
pub(crate) fn is_simple(n: &Node) -> bool {
    match &n.kind {
        NodeKind::Number(_) | NodeKind::Ident(_) => true,
        NodeKind::Style { body, .. } => is_simple(body),
        NodeKind::Row(items) => match items.as_slice() {
            [one] => is_simple(one),
            [sign, x] => {
                (sign.is_op("−") || sign.is_op("+"))
                    && matches!(x.kind, NodeKind::Number(_) | NodeKind::Ident(_))
            }
            _ => false,
        },
        _ => false,
    }
}

/// A fraction part read without "numerator" and "denominator": simple, or
/// an implied product of up to three letters and numbers (`2a`, `dx`,
/// `\partial f`), as ClearSpeak's simple fractions.
fn fraction_part_simple(n: &Node) -> bool {
    if is_simple(n) {
        return true;
    }
    match &n.kind {
        NodeKind::Row(items) => {
            (2..=3).contains(&items.len())
                && items.iter().all(|i| {
                    matches!(i.kind, NodeKind::Number(_) | NodeKind::Ident(_))
                        || matches!(&i.kind, NodeKind::Style { body, .. } if is_simple(body))
                })
        }
        _ => false,
    }
}

/// Simple, or a short run of letters and digits (`x_{ij}`, `a_{12}`).
fn simple_script(n: &Node) -> bool {
    if is_simple(n) {
        return true;
    }
    match &n.kind {
        NodeKind::Row(items) => {
            items.len() <= 3
                && items
                    .iter()
                    .all(|i| matches!(i.kind, NodeKind::Number(_) | NodeKind::Ident(_)))
        }
        _ => false,
    }
}

fn letters_only(n: &Node) -> bool {
    match &n.kind {
        NodeKind::Ident(_) => true,
        NodeKind::Row(items) => !items.is_empty() && items.iter().all(letters_only),
        _ => false,
    }
}

fn is_minus_one(n: &Node) -> bool {
    match &n.kind {
        NodeKind::Row(items) => {
            items.len() == 2
                && items[0].is_op("−")
                && matches!(&items[1].kind, NodeKind::Number(s) if s == "1")
        }
        _ => false,
    }
}

fn integer(n: &Node) -> Option<u64> {
    match &n.kind {
        NodeKind::Number(s) if s.chars().all(|c| c.is_ascii_digit()) => s.parse().ok(),
        _ => None,
    }
}

/// "1 half", "3 fourths": small numeric fractions read as ClearSpeak does.
fn common_fraction(num: &Node, den: &Node) -> Option<(String, String)> {
    let a = integer(num)?;
    let b = integer(den)?;
    if !(1..=19).contains(&a) {
        return None;
    }
    let (one, many) = match b {
        2 => ("half", "halves"),
        3 => ("third", "thirds"),
        4 => ("fourth", "fourths"),
        5 => ("fifth", "fifths"),
        6 => ("sixth", "sixths"),
        7 => ("seventh", "sevenths"),
        8 => ("eighth", "eighths"),
        9 => ("ninth", "ninths"),
        10 => ("tenth", "tenths"),
        _ => return None,
    };
    Some((a.to_string(), if a == 1 { one } else { many }.to_owned()))
}

/// "4th", "21st", "n-th".
fn ordinal(n: u64) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

enum IndexKind {
    Square,
    Cube,
    Ordinal(String),
    Other,
}

fn index_kind(ix: &Node) -> IndexKind {
    if let Some(k) = integer(ix) {
        return match k {
            2 => IndexKind::Square,
            3 => IndexKind::Cube,
            k => IndexKind::Ordinal(ordinal(k)),
        };
    }
    match &ix.kind {
        NodeKind::Ident(s) if s.len() == 1 && s.chars().all(|c| c.is_ascii_alphabetic()) => {
            IndexKind::Ordinal(format!("{s}-th"))
        }
        _ => IndexKind::Other,
    }
}

enum SupKind {
    Prime(&'static str),
    Degrees,
    Squared,
    Cubed,
    /// Digits and their ordinal ("4", "4th").
    Ordinal(String, String),
    /// A letter or Greek letter read without "power" ("e to the x").
    Letter,
    /// A signed number or letter ("to the negative 1 power").
    Simple,
    /// A lone operator (`x^*`): spoken as its name.
    Operator(String),
    Complex,
}

fn sup_kind(p: &Node) -> SupKind {
    match &p.kind {
        NodeKind::Operator { text, .. } => match text.as_str() {
            "′" => SupKind::Prime("prime"),
            "″" => SupKind::Prime("double prime"),
            "‴" => SupKind::Prime("triple prime"),
            "⁗" => SupKind::Prime("quadruple prime"),
            "∘" | "°" => SupKind::Degrees,
            "∗" | "*" | "⋆" => SupKind::Operator("star".into()),
            t => SupKind::Operator(words(t).unwrap_or(t).to_owned()),
        },
        NodeKind::Number(s) if s == "2" => SupKind::Squared,
        NodeKind::Number(s) if s == "3" => SupKind::Cubed,
        NodeKind::Number(s) => match integer(p) {
            Some(k) => SupKind::Ordinal(s.clone(), ordinal(k)),
            None => SupKind::Simple,
        },
        // Index letters read as ordinals ("x to the n-th power"); other
        // letters plainly ("e to the x").
        NodeKind::Ident(s) if s.len() == 1 && "ijklmnpqr".contains(s.as_str()) => {
            SupKind::Ordinal(s.clone(), format!("{s}-th"))
        }
        NodeKind::Ident(_) => SupKind::Letter,
        _ if is_simple(p) => SupKind::Simple,
        _ => SupKind::Complex,
    }
}

/// The word for an accent and whether it follows the base ("x hat") or
/// precedes it ("vector v").
fn accent_words(a: AccentKind) -> (&'static str, bool) {
    match a {
        AccentKind::Hat => ("hat", true),
        AccentKind::Bar | AccentKind::Overline => ("bar", true),
        AccentKind::Tilde => ("tilde", true),
        AccentKind::Dot => ("dot", true),
        AccentKind::DoubleDot => ("double dot", true),
        AccentKind::TripleDot => ("triple dot", true),
        AccentKind::Acute => ("acute", true),
        AccentKind::Grave => ("grave", true),
        AccentKind::Breve => ("breve", true),
        AccentKind::Check => ("check", true),
        AccentKind::Ring => ("ring", true),
        AccentKind::Vector => ("vector", false),
        AccentKind::LeftArrow => ("left arrow over", false),
        AccentKind::LeftRightArrow => ("line", false),
        AccentKind::Arc => ("arc", false),
        AccentKind::Underline => ("underline", false),
        AccentKind::Overbrace => ("overbrace", false),
        AccentKind::Underbrace => ("underbrace", false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals() {
        assert_eq!(ordinal(1), "1st");
        assert_eq!(ordinal(2), "2nd");
        assert_eq!(ordinal(3), "3rd");
        assert_eq!(ordinal(4), "4th");
        assert_eq!(ordinal(11), "11th");
        assert_eq!(ordinal(12), "12th");
        assert_eq!(ordinal(21), "21st");
        assert_eq!(ordinal(112), "112th");
    }
}
