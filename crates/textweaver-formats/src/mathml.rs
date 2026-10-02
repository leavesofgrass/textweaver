//! MathML in EPUB 3 (W4c1, ADR-0029): each `<math>` element becomes LaTeX
//! with its delimiters (`$…$`, or `$$…$$` for display math) under a `Math`
//! marker, as the Markdown and DOCX loaders write math, so speech reads it
//! as math and writers typeset it.
//!
//! The LaTeX is, in order of preference:
//!
//! 1. the book's own TeX, from a `<semantics>` annotation whose encoding
//!    is TeX or LaTeX (MathJax, Pandoc, and LaTeXML write one);
//! 2. the presentation MathML converted: token elements (`mi`, `mn`, `mo`,
//!    `mtext`, `ms`, `mspace`), rows and styles, fractions, roots, scripts
//!    (`msub`, `msup`, `msubsup`, `munder`, `mover`, `munderover`,
//!    `mmultiscripts`), tables, `mfenced`, and `semantics`; an unknown
//!    element reads as its children, and `mphantom` and annotations are
//!    left out.
//!
//! Only when neither gives any math does the loader fall back to the
//! element's `alttext`, as plain text. An image fallback is read by its
//! alt text, as every image is: the `epub:default` of an `epub:switch`
//! whose `epub:case` has no MathML, or an `<img>` inside the math, which
//! the HTML parser moves out after it. At reading time, math under the
//! marker is spoken by MathCAT when the math engine setting asks for it,
//! else by textweaver's own math speech.

use scraper::ElementRef;

use crate::omml::{
    Tex, accent_command, delimiter, function_name, is_atom, math_text, nary_command, takes_limits,
    text_mode,
};

/// Deeper MathML is read as its text, without structure.
const MAX_DEPTH: usize = 64;

/// TeX annotation encodings.
const TEX_ENCODINGS: &[&str] = &[
    "application/x-tex",
    "application/x-latex",
    "text/x-tex",
    "text/x-latex",
    "tex",
    "latex",
    "text/tex",
    "text/latex",
];

/// What a `<math>` element gives the loader.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MathMl {
    /// LaTeX without delimiters, when there is any math.
    pub(crate) latex: Option<String>,
    /// Display (block) math.
    pub(crate) display: bool,
    /// The fallback text: `alttext`, else an image's alt text.
    pub(crate) fallback: Option<String>,
}

/// The local part of a tag name (`m:mfrac` is `mfrac`).
pub(crate) fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Reads a `<math>` element.
pub(crate) fn read(math: ElementRef<'_>) -> MathMl {
    let display = math
        .attr("display")
        .is_some_and(|d| d.trim().eq_ignore_ascii_case("block"))
        || math
            .attr("mode")
            .is_some_and(|m| m.trim().eq_ignore_ascii_case("display"));
    let latex = tex_annotation(math)
        .or_else(|| {
            let mut c = Conv::new();
            Some(collapse(&c.children(math)))
        })
        .filter(|l| !l.trim().is_empty());
    let fallback = math.attr("alttext").map(collapse).filter(|a| !a.is_empty());
    MathMl {
        latex,
        display,
        fallback,
    }
}

/// The TeX source a `<semantics>` annotation carries, without `$` or
/// `\(`/`\[` delimiters.
fn tex_annotation(math: ElementRef<'_>) -> Option<String> {
    let ann = math.descendent_elements().find(|e| {
        local(e.value().name()) == "annotation"
            && e.attr("encoding").is_some_and(|enc| {
                TEX_ENCODINGS
                    .iter()
                    .any(|t| enc.trim().eq_ignore_ascii_case(t))
            })
    })?;
    let text = collapse(&ann.text().collect::<String>());
    let mut t = text.as_str();
    for (open, close) in [("$$", "$$"), ("$", "$"), ("\\(", "\\)"), ("\\[", "\\]")] {
        if let Some(inner) = t.strip_prefix(open).and_then(|r| r.strip_suffix(close)) {
            t = inner;
            break;
        }
    }
    let t = t.trim();
    // A `$` left inside would end the math early in the text.
    (!t.is_empty() && !t.contains('$')).then(|| t.to_owned())
}

/// Token text as math LaTeX, with the characters that mean something to
/// LaTeX or to the math finder escaped.
fn token(text: &str) -> String {
    let t = math_text(text.trim(), false);
    let mut out = String::with_capacity(t.len());
    for c in t.chars() {
        match c {
            '$' | '%' | '#' | '_' => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out
}

/// `s` as a script or command argument: bare when it is one token,
/// otherwise in braces.
fn arg(s: &str) -> String {
    let s = s.trim();
    if is_atom(s) {
        s.to_owned()
    } else {
        format!("{{{s}}}")
    }
}

struct Conv {
    depth: usize,
    /// How tightly the LaTeX of the element just converted binds (see
    /// [`prec`]): content MathML's operators set it so their parents know
    /// when to add parentheses.
    prec: u8,
}

impl Conv {
    fn new() -> Self {
        Conv {
            depth: 0,
            prec: prec::ATOM,
        }
    }
    /// The element children of `el`, in order.
    fn kids<'a>(el: ElementRef<'a>) -> Vec<ElementRef<'a>> {
        el.child_elements().collect()
    }

    /// The LaTeX of `el`'s children, one after another.
    fn children(&mut self, el: ElementRef<'_>) -> String {
        let mut out = Tex::new();
        for k in Self::kids(el) {
            let piece = self.node(k);
            if !piece.is_empty() {
                out.push(&piece);
            }
        }
        out.into_string()
    }

    fn node(&mut self, el: ElementRef<'_>) -> String {
        if self.depth >= MAX_DEPTH {
            return token(&el.text().collect::<String>());
        }
        self.depth += 1;
        self.prec = prec::ATOM;
        let out = self.element(el);
        self.depth -= 1;
        out
    }

    /// The LaTeX of child `i`, or nothing.
    fn nth(&mut self, kids: &[ElementRef<'_>], i: usize) -> String {
        kids.get(i).map(|k| self.node(*k)).unwrap_or_default()
    }

    fn element(&mut self, el: ElementRef<'_>) -> String {
        let name = local(el.value().name());
        let kids = Self::kids(el);
        match name {
            "mi" => {
                let t = collapse(&el.text().collect::<String>());
                if t.chars().count() > 1 && t.chars().all(char::is_alphabetic) {
                    function_name(&t)
                } else {
                    token(&t)
                }
            }
            "mn" => token(&el.text().collect::<String>()),
            "mo" => {
                let t = collapse(&el.text().collect::<String>());
                let n = nary_command(&t);
                if n.starts_with('\\') { n } else { token(&t) }
            }
            "mtext" => {
                let t = collapse(&el.text().collect::<String>());
                if t.is_empty() {
                    " ".into()
                } else {
                    format!("\\text{{{}}}", text_mode(&t))
                }
            }
            "ms" => format!(
                "\\text{{\"{}\"}}",
                text_mode(&collapse(&el.text().collect::<String>()))
            ),
            "mspace" => " ".into(),
            "mphantom" | "annotation" | "annotation-xml" | "none" | "mprescripts" => String::new(),
            "semantics" | "maction" => kids.first().map(|k| self.node(*k)).unwrap_or_default(),
            "mfrac" => {
                let (a, b) = (self.nth(&kids, 0), self.nth(&kids, 1));
                let bar = el
                    .attr("linethickness")
                    .map(str::trim)
                    .is_none_or(|t| !matches!(t, "0" | "0px" | "0pt" | "0em" | "0.0"));
                if bar {
                    format!("\\frac{{{a}}}{{{b}}}")
                } else {
                    format!("{{{a} \\atop {b}}}")
                }
            }
            "msqrt" => format!("\\sqrt{{{}}}", self.children(el)),
            "mroot" => {
                let (base, index) = (self.nth(&kids, 0), self.nth(&kids, 1));
                format!("\\sqrt[{index}]{{{base}}}")
            }
            "msub" => {
                let (base, sub) = (self.nth(&kids, 0), self.nth(&kids, 1));
                format!("{}_{{{}}}", arg(&base), sub.trim())
            }
            "msup" => {
                let (base, sup) = (self.nth(&kids, 0), self.nth(&kids, 1));
                format!("{}^{{{}}}", arg(&base), sup.trim())
            }
            "msubsup" => {
                let (base, sub, sup) = (self.nth(&kids, 0), self.nth(&kids, 1), self.nth(&kids, 2));
                format!("{}_{{{}}}^{{{}}}", arg(&base), sub.trim(), sup.trim())
            }
            "munder" => {
                let base = self.nth(&kids, 0);
                let under_text = kids
                    .get(1)
                    .map(|k| collapse(&k.text().collect::<String>()))
                    .unwrap_or_default();
                let under = self.nth(&kids, 1);
                if takes_limits(base.trim()) {
                    format!("{}_{{{}}}", base.trim(), under.trim())
                } else if matches!(under_text.as_str(), "_" | "‾" | "¯" | "\u{0332}") {
                    format!("\\underline{{{base}}}")
                } else if under_text == "⏟" {
                    format!("\\underbrace{{{base}}}")
                } else {
                    format!("\\underset{{{under}}}{{{base}}}")
                }
            }
            "mover" => {
                let base = self.nth(&kids, 0);
                let over_text = kids
                    .get(1)
                    .map(|k| collapse(&k.text().collect::<String>()))
                    .unwrap_or_default();
                let over = self.nth(&kids, 1);
                if takes_limits(base.trim()) {
                    format!("{}^{{{}}}", base.trim(), over.trim())
                } else if matches!(over_text.as_str(), "‾" | "¯" | "\u{0305}") && !is_atom(&base)
                {
                    format!("\\overline{{{base}}}")
                } else if over_text == "⏞" {
                    format!("\\overbrace{{{base}}}")
                } else if let Some(acc) = accent_command(&over_text) {
                    format!("{acc}{{{base}}}")
                } else {
                    format!("\\overset{{{over}}}{{{base}}}")
                }
            }
            "munderover" => {
                let (base, under, over) =
                    (self.nth(&kids, 0), self.nth(&kids, 1), self.nth(&kids, 2));
                if takes_limits(base.trim()) {
                    format!("{}_{{{}}}^{{{}}}", base.trim(), under.trim(), over.trim())
                } else {
                    format!("\\overset{{{over}}}{{\\underset{{{under}}}{{{base}}}}}")
                }
            }
            "mmultiscripts" => self.multiscripts(&kids),
            "mtable" => self.table(el),
            "mtr" | "mlabeledtr" | "mtd" => self.children(el),
            "mfenced" => self.fenced(el, &kids),
            // Content MathML.
            "apply" | "bind" | "reln" => self.apply(&kids),
            "ci" => self.ci(el),
            "cn" => self.cn(el),
            "csymbol" => csymbol(&collapse(&el.text().collect::<String>())),
            "set" | "list" | "vector" | "interval" | "matrix" | "matrixrow" | "piecewise"
            | "lambda" => self.container(name, el, &kids),
            c if constant(c).is_some() => constant(c).unwrap_or_default().to_owned(),
            c if operator(c).is_some() => {
                operator(c).map(|o| o.symbol).unwrap_or_default().to_owned()
            }
            _ => self.children(el),
        }
    }

    /// `base post-sub post-sup … <mprescripts/> pre-sub pre-sup …`.
    fn multiscripts(&mut self, kids: &[ElementRef<'_>]) -> String {
        let base = self.nth(kids, 0);
        let split = kids
            .iter()
            .position(|k| local(k.value().name()) == "mprescripts")
            .unwrap_or(kids.len());
        let pairs = |this: &mut Self, from: usize, to: usize| -> (String, String) {
            let (mut subs, mut sups) = (Tex::new(), Tex::new());
            let mut i = from;
            while i < to {
                subs.push(&this.nth(kids, i));
                if i + 1 < to {
                    sups.push(&this.nth(kids, i + 1));
                }
                i += 2;
            }
            (subs.into_string(), sups.into_string())
        };
        let (post_sub, post_sup) = pairs(self, 1, split);
        let (pre_sub, pre_sup) = pairs(self, split + 1, kids.len());
        let mut out = String::new();
        if !pre_sub.trim().is_empty() || !pre_sup.trim().is_empty() {
            out.push_str("{}");
            if !pre_sub.trim().is_empty() {
                out.push_str(&format!("_{{{pre_sub}}}"));
            }
            if !pre_sup.trim().is_empty() {
                out.push_str(&format!("^{{{pre_sup}}}"));
            }
        }
        out.push_str(&arg(&base));
        if !post_sub.trim().is_empty() {
            out.push_str(&format!("_{{{post_sub}}}"));
        }
        if !post_sup.trim().is_empty() {
            out.push_str(&format!("^{{{post_sup}}}"));
        }
        out
    }

    fn table(&mut self, el: ElementRef<'_>) -> String {
        let mut rows = Vec::new();
        for row in Self::kids(el) {
            let kind = local(row.value().name());
            if !matches!(kind, "mtr" | "mlabeledtr") {
                continue;
            }
            // A labeled row's first cell is its equation label.
            let skip = usize::from(kind == "mlabeledtr");
            let cells: Vec<String> = Self::kids(row)
                .into_iter()
                .filter(|c| local(c.value().name()) == "mtd")
                .skip(skip)
                .map(|c| self.children(c).trim().to_owned())
                .collect();
            rows.push(cells.join(" & "));
        }
        format!("\\begin{{matrix}} {} \\end{{matrix}}", rows.join(" \\\\ "))
    }

    fn fenced(&mut self, el: ElementRef<'_>, kids: &[ElementRef<'_>]) -> String {
        let open = el.attr("open").unwrap_or("(").trim();
        let close = el.attr("close").unwrap_or(")").trim();
        let seps: Vec<char> = el
            .attr("separators")
            .unwrap_or(",")
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let mut inner = Tex::new();
        for (i, k) in kids.iter().enumerate() {
            if i > 0
                && let Some(sep) = seps.get(i - 1).or(seps.last())
            {
                inner.push_raw(&token(&sep.to_string()));
                inner.push_raw(" ");
            }
            inner.push(&self.node(*k));
        }
        let inner = inner.into_string();
        match (delimiter(open), delimiter(close)) {
            (Some(l), Some(r)) => format!("\\left{l} {inner} \\right{r}"),
            _ => format!("{} {inner} {}", token(open), token(close)),
        }
    }
}

// ---------------------------------------------------------------------
// Content MathML (ADR-0044): `<apply>` and its operators to the same
// LaTeX presentation MathML becomes, so speech and braille read both
// alike. Unknown operators are read by name, as functions.

/// How tightly a piece of LaTeX binds: a parent wraps a child in
/// parentheses when the child binds more loosely than it needs.
mod prec {
    pub(super) const RELATION: u8 = 1;
    pub(super) const LOGIC: u8 = 2;
    pub(super) const BIG: u8 = 3;
    pub(super) const SUM: u8 = 4;
    pub(super) const PRODUCT: u8 = 5;
    pub(super) const UNARY: u8 = 6;
    pub(super) const POWER: u8 = 7;
    pub(super) const ATOM: u8 = 9;
}

/// How a content operator is written.
#[derive(Clone, Copy)]
enum Form {
    /// Between its arguments: `a + b + c`.
    Infix,
    /// A relation, chained: `a < b < c`.
    Relation,
    /// A function: `\sin x`, `\max(a, b)`.
    Function,
    /// Its own shape, written by [`Conv::special`].
    Special,
}

#[derive(Clone, Copy)]
struct Operator {
    symbol: &'static str,
    form: Form,
    prec: u8,
}

const fn op(symbol: &'static str, form: Form, prec: u8) -> Operator {
    Operator { symbol, form, prec }
}

/// Content MathML's operators.
fn operator(name: &str) -> Option<Operator> {
    use Form::{Function as F, Infix as I, Relation as R, Special as S};
    use prec::{ATOM, BIG, LOGIC, POWER, PRODUCT, RELATION, SUM, UNARY};
    Some(match name {
        "plus" => op("+", I, SUM),
        "minus" => op("-", S, SUM),
        "times" => op("\\times", I, PRODUCT),
        "divide" => op("\\div", S, ATOM),
        "power" => op("^", S, POWER),
        "root" => op("\\sqrt", S, ATOM),
        "abs" => op("|", S, ATOM),
        "factorial" => op("!", S, POWER),
        "exp" => op("e", S, POWER),
        "ln" => op("\\ln", F, UNARY),
        "log" => op("\\log", S, UNARY),
        "sin" | "cos" | "tan" | "sec" | "csc" | "cot" | "sinh" | "cosh" | "tanh" | "coth"
        | "arcsin" | "arccos" | "arctan" | "det" => op("", F, UNARY),
        "sech" | "csch" | "arcsec" | "arccsc" | "arccot" | "arcsinh" | "arccosh" | "arctanh" => {
            op("", F, UNARY)
        }
        "max" | "min" | "gcd" | "lcm" => op("", F, ATOM),
        "floor" => op("\\lfloor", S, ATOM),
        "ceiling" => op("\\lceil", S, ATOM),
        "conjugate" => op("\\overline", S, ATOM),
        "transpose" => op("^{T}", S, POWER),
        "determinant" => op("\\det", F, UNARY),
        "card" => op("|", S, ATOM),
        "arg" => op("\\arg", F, UNARY),
        "real" => op("\\Re", F, UNARY),
        "imaginary" => op("\\Im", F, UNARY),
        "eq" => op("=", R, RELATION),
        "neq" => op("\\neq", R, RELATION),
        "lt" => op("<", R, RELATION),
        "gt" => op(">", R, RELATION),
        "leq" => op("\\leq", R, RELATION),
        "geq" => op("\\geq", R, RELATION),
        "approx" => op("\\approx", R, RELATION),
        "equivalent" => op("\\equiv", R, RELATION),
        "factorof" => op("\\mid", R, RELATION),
        "in" => op("\\in", R, RELATION),
        "notin" => op("\\notin", R, RELATION),
        "subset" => op("\\subseteq", R, RELATION),
        "prsubset" => op("\\subset", R, RELATION),
        "notsubset" => op("\\not\\subseteq", R, RELATION),
        "notprsubset" => op("\\not\\subset", R, RELATION),
        "implies" => op("\\Rightarrow", R, RELATION),
        "tendsto" => op("\\to", R, RELATION),
        "and" => op("\\land", I, LOGIC),
        "or" => op("\\lor", I, LOGIC),
        "xor" => op("\\oplus", I, LOGIC),
        "not" => op("\\lnot", S, UNARY),
        "union" => op("\\cup", I, SUM),
        "intersect" => op("\\cap", I, PRODUCT),
        "setdiff" => op("\\setminus", I, SUM),
        "cartesianproduct" => op("\\times", I, PRODUCT),
        "compose" => op("\\circ", I, PRODUCT),
        "sum" => op("\\sum", S, BIG),
        "product" => op("\\prod", S, BIG),
        "int" => op("\\int", S, BIG),
        "limit" => op("\\lim", S, BIG),
        "diff" => op("d", S, PRODUCT),
        "partialdiff" => op("\\partial", S, PRODUCT),
        "forall" => op("\\forall", S, LOGIC),
        "exists" => op("\\exists", S, LOGIC),
        "mean" => op("\\operatorname{mean}", F, ATOM),
        "sdev" => op("\\sigma", F, ATOM),
        "variance" => op("\\operatorname{var}", F, ATOM),
        "median" => op("\\operatorname{median}", F, ATOM),
        "mode" => op("\\operatorname{mode}", F, ATOM),
        "inverse" => op("^{-1}", S, POWER),
        "quotient" => op("\\operatorname{quotient}", F, ATOM),
        "rem" => op("\\bmod", I, PRODUCT),
        "selector" => op("_", S, ATOM),
        _ => return None,
    })
}

/// Content MathML's constants and sets.
fn constant(name: &str) -> Option<&'static str> {
    Some(match name {
        "pi" => "\\pi",
        "exponentiale" => "e",
        "imaginaryi" => "i",
        "infinity" => "\\infty",
        "emptyset" => "\\emptyset",
        "eulergamma" => "\\gamma",
        "true" => "\\text{true}",
        "false" => "\\text{false}",
        "notanumber" => "\\text{NaN}",
        "naturalnumbers" => "\\mathbb{N}",
        "integers" => "\\mathbb{Z}",
        "rationals" => "\\mathbb{Q}",
        "reals" => "\\mathbb{R}",
        "complexes" => "\\mathbb{C}",
        "primes" => "\\mathbb{P}",
        _ => return None,
    })
}

/// A `<csymbol>`: a constant or operator by its content name, else the
/// symbol as an identifier (a word as an operator name).
fn csymbol(text: &str) -> String {
    if let Some(c) = constant(text) {
        return c.to_owned();
    }
    function_name(text)
}

/// Qualifiers an `<apply>` may carry after its operator.
const QUALIFIERS: &[&str] = &[
    "bvar",
    "lowlimit",
    "uplimit",
    "condition",
    "degree",
    "logbase",
    "domainofapplication",
    "momentabout",
];

/// `s` in parentheses when it binds more loosely than `need`.
fn wrap(s: &str, prec: u8, need: u8) -> String {
    if prec < need {
        format!("\\left( {s} \\right)")
    } else {
        s.to_owned()
    }
}

/// An `<apply>`'s parts: the arguments and the qualifiers.
struct Applied<'a> {
    args: Vec<ElementRef<'a>>,
    bvars: Vec<ElementRef<'a>>,
    lower: Option<ElementRef<'a>>,
    upper: Option<ElementRef<'a>>,
    condition: Option<ElementRef<'a>>,
    degree: Option<ElementRef<'a>>,
    logbase: Option<ElementRef<'a>>,
    domain: Option<ElementRef<'a>>,
}

impl<'a> Applied<'a> {
    fn split(kids: &[ElementRef<'a>]) -> Self {
        let mut a = Applied {
            args: Vec::new(),
            bvars: Vec::new(),
            lower: None,
            upper: None,
            condition: None,
            degree: None,
            logbase: None,
            domain: None,
        };
        for k in kids {
            match local(k.value().name()) {
                "bvar" => a.bvars.push(*k),
                "lowlimit" => a.lower = Some(*k),
                "uplimit" => a.upper = Some(*k),
                "condition" => a.condition = Some(*k),
                "degree" => a.degree = Some(*k),
                "logbase" => a.logbase = Some(*k),
                "domainofapplication" => a.domain = Some(*k),
                q if QUALIFIERS.contains(&q) => {}
                _ => a.args.push(*k),
            }
        }
        // An interval argument of `int` or `sum` is its range.
        if a.lower.is_none()
            && a.domain.is_none()
            && let Some(i) = a
                .args
                .iter()
                .position(|k| local(k.value().name()) == "interval")
            && a.args.len() > 1
        {
            a.domain = Some(a.args.remove(i));
        }
        a
    }
}

impl Conv {
    /// A child's LaTeX and how tightly it binds.
    fn part(&mut self, el: ElementRef<'_>) -> (String, u8) {
        let s = self.node(el);
        let p = self.prec;
        (s.trim().to_owned(), p)
    }

    /// The LaTeX inside a qualifier (`<lowlimit><cn>0</cn></lowlimit>`).
    fn inner(&mut self, el: Option<ElementRef<'_>>) -> Option<String> {
        el.map(|e| self.children(e).trim().to_owned())
            .filter(|s| !s.is_empty())
    }

    fn ci(&mut self, el: ElementRef<'_>) -> String {
        if el.child_elements().next().is_some() {
            // Presentation markup inside (`<ci><msub>...`).
            return self.children(el);
        }
        let t = collapse(&el.text().collect::<String>());
        if t.chars().count() > 1 && t.chars().all(char::is_alphabetic) {
            function_name(&t)
        } else {
            token(&t)
        }
    }

    fn cn(&mut self, el: ElementRef<'_>) -> String {
        // The parts around `<sep/>`.
        let mut parts = vec![String::new()];
        for c in el.children() {
            match c.value() {
                scraper::Node::Text(t) => {
                    if let Some(last) = parts.last_mut() {
                        last.push_str(t);
                    }
                }
                scraper::Node::Element(e) if local(e.name()) == "sep" => parts.push(String::new()),
                _ => {}
            }
        }
        let parts: Vec<String> = parts.iter().map(|p| token(&collapse(p))).collect();
        let kind = el.attr("type").unwrap_or("").trim();
        let out = match (kind, parts.as_slice()) {
            ("rational", [a, b]) => format!("\\frac{{{a}}}{{{b}}}"),
            ("e-notation", [a, b]) => format!("{a} \\times 10^{{{b}}}"),
            ("complex-cartesian", [a, b]) => format!("{a} + {b} i"),
            ("complex-polar", [a, b]) => format!("{a} e^{{{b} i}}"),
            _ => parts.join(" "),
        };
        if out.starts_with('-') || kind.starts_with("complex") || kind == "e-notation" {
            self.prec = prec::SUM;
        }
        out
    }

    /// `<apply>`: the operator, then its arguments and qualifiers.
    fn apply(&mut self, kids: &[ElementRef<'_>]) -> String {
        let Some((head, rest)) = kids.split_first() else {
            return String::new();
        };
        let name = local(head.value().name());
        let parts = Applied::split(rest);
        let (out, p) = match operator(name) {
            Some(o) => self.operate(name, o, &parts),
            None => {
                // A function applied (`<ci>f</ci>`, `<csymbol>`), or an
                // operator read by its name.
                let f = match name {
                    "ci" | "csymbol" | "apply" | "fn" => self.part(*head).0,
                    other => function_name(other),
                };
                let args: Vec<String> = parts.args.iter().map(|a| self.part(*a).0).collect();
                (
                    format!("{f}\\left( {} \\right)", args.join(", ")),
                    prec::ATOM,
                )
            }
        };
        self.prec = p;
        out
    }

    fn operate(&mut self, name: &str, o: Operator, a: &Applied<'_>) -> (String, u8) {
        let args: Vec<(String, u8)> = a.args.iter().map(|k| self.part(*k)).collect();
        match o.form {
            Form::Infix => {
                let joined: Vec<String> = args.iter().map(|(s, p)| wrap(s, *p, o.prec)).collect();
                (joined.join(&format!(" {} ", o.symbol)), o.prec)
            }
            Form::Relation => {
                let joined: Vec<String> =
                    args.iter().map(|(s, p)| wrap(s, *p, prec::LOGIC)).collect();
                (joined.join(&format!(" {} ", o.symbol)), o.prec)
            }
            Form::Function => {
                let f = if o.symbol.is_empty() {
                    function_name(name)
                } else {
                    o.symbol.to_owned()
                };
                match args.as_slice() {
                    [(s, p)] if *p == prec::ATOM && o.prec == prec::UNARY => {
                        (format!("{f} {s}"), o.prec)
                    }
                    _ => {
                        let list: Vec<&str> = args.iter().map(|(s, _)| s.as_str()).collect();
                        (
                            format!("{f}\\left( {} \\right)", list.join(", ")),
                            prec::ATOM,
                        )
                    }
                }
            }
            Form::Special => self.special(name, o, a, &args),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn special(
        &mut self,
        name: &str,
        o: Operator,
        a: &Applied<'_>,
        args: &[(String, u8)],
    ) -> (String, u8) {
        let arg = |i: usize| args.get(i).cloned().unwrap_or_default();
        match name {
            "minus" if args.len() == 1 => {
                let (s, p) = arg(0);
                (format!("-{}", wrap(&s, p, prec::UNARY)), prec::UNARY)
            }
            "minus" => {
                let (x, px) = arg(0);
                let (y, py) = arg(1);
                (
                    format!(
                        "{} - {}",
                        wrap(&x, px, prec::SUM),
                        wrap(&y, py, prec::PRODUCT)
                    ),
                    prec::SUM,
                )
            }
            "not" => {
                let (s, p) = arg(0);
                (format!("\\lnot {}", wrap(&s, p, prec::UNARY)), prec::UNARY)
            }
            "divide" => (
                format!("\\frac{{{}}}{{{}}}", arg(0).0, arg(1).0),
                prec::ATOM,
            ),
            "power" => {
                let (b, pb) = arg(0);
                (
                    format!("{}^{{{}}}", wrap(&b, pb, prec::ATOM), arg(1).0),
                    prec::POWER,
                )
            }
            "root" => match self.inner(a.degree) {
                Some(n) if n != "2" => (format!("\\sqrt[{n}]{{{}}}", arg(0).0), prec::ATOM),
                _ => (format!("\\sqrt{{{}}}", arg(0).0), prec::ATOM),
            },
            "abs" | "card" => (format!("\\left| {} \\right|", arg(0).0), prec::ATOM),
            "floor" => (format!("\\lfloor {} \\rfloor", arg(0).0), prec::ATOM),
            "ceiling" => (format!("\\lceil {} \\rceil", arg(0).0), prec::ATOM),
            "conjugate" => (format!("\\overline{{{}}}", arg(0).0), prec::ATOM),
            "factorial" | "transpose" | "inverse" => {
                let (s, p) = arg(0);
                (
                    format!("{}{}", wrap(&s, p, prec::ATOM), o.symbol),
                    prec::POWER,
                )
            }
            "exp" => (format!("e^{{{}}}", arg(0).0), prec::POWER),
            // An entry of a vector or matrix: `a_{i, j}`.
            "selector" => {
                let (base, p) = arg(0);
                let index: Vec<&str> = args.iter().skip(1).map(|(s, _)| s.as_str()).collect();
                (
                    format!("{}_{{{}}}", wrap(&base, p, prec::ATOM), index.join(", ")),
                    prec::ATOM,
                )
            }
            "log" => {
                let base = self.inner(a.logbase);
                let f = match base {
                    Some(b) => format!("\\log_{{{b}}}"),
                    None => "\\log".into(),
                };
                let (s, p) = arg(0);
                if p == prec::ATOM {
                    (format!("{f} {s}"), prec::UNARY)
                } else {
                    (format!("{f}\\left( {s} \\right)"), prec::ATOM)
                }
            }
            "sum" | "product" | "int" | "limit" | "forall" | "exists" => self.big(name, o, a, args),
            "diff" | "partialdiff" => {
                let (f, p) = arg(0);
                let d = o.symbol;
                let vars: Vec<String> = a.bvars.iter().map(|b| self.bvar(*b)).collect();
                let order = a
                    .bvars
                    .first()
                    .and_then(|b| {
                        b.child_elements()
                            .find(|k| local(k.value().name()) == "degree")
                    })
                    .map(|k| self.children(k).trim().to_owned())
                    .or_else(|| self.inner(a.degree))
                    .filter(|n| n != "1");
                let (top, bottom) = match (&order, vars.as_slice()) {
                    (Some(n), [v]) => (format!("{d}^{{{n}}}"), format!("{d} {v}^{{{n}}}")),
                    (_, []) => (d.to_owned(), d.to_owned()),
                    (_, vs) => {
                        let bottom: Vec<String> = vs.iter().map(|v| format!("{d} {v}")).collect();
                        (
                            if vs.len() > 1 {
                                format!("{d}^{{{}}}", vs.len())
                            } else {
                                d.to_owned()
                            },
                            bottom.join(" \\, "),
                        )
                    }
                };
                if vars.is_empty() {
                    // `diff` of a function alone: its derivative.
                    return (format!("{}'", wrap(&f, p, prec::ATOM)), prec::POWER);
                }
                (
                    format!("\\frac{{{top}}}{{{bottom}}} {}", wrap(&f, p, prec::PRODUCT)),
                    prec::PRODUCT,
                )
            }
            _ => {
                let list: Vec<&str> = args.iter().map(|(s, _)| s.as_str()).collect();
                (
                    format!(
                        "{}\\left( {} \\right)",
                        function_name(name),
                        list.join(", ")
                    ),
                    prec::ATOM,
                )
            }
        }
    }

    /// A bound variable's name.
    fn bvar(&mut self, b: ElementRef<'_>) -> String {
        let vars: Vec<String> = b
            .child_elements()
            .filter(|k| local(k.value().name()) != "degree")
            .map(|k| self.part(k).0)
            .collect();
        vars.join(", ")
    }

    /// Sums, products, integrals, limits, and quantifiers.
    fn big(
        &mut self,
        name: &str,
        o: Operator,
        a: &Applied<'_>,
        args: &[(String, u8)],
    ) -> (String, u8) {
        let vars: Vec<String> = a.bvars.iter().map(|b| self.bvar(*b)).collect();
        let var = vars.join(", ");
        let lower = self.inner(a.lower);
        let upper = self.inner(a.upper);
        let condition = self.inner(a.condition);
        let domain = a.domain.map(|d| {
            let kids = Self::kids(d);
            if local(d.value().name()) == "interval" && kids.len() == 2 {
                let lo = self.part(kids[0]).0;
                let hi = self.part(kids[1]).0;
                (Some(lo), Some(hi), None)
            } else {
                (None, None, Some(self.children(d).trim().to_owned()))
            }
        });
        let (lower, upper, condition) = match domain {
            Some((l, u, c)) => (lower.or(l), upper.or(u), condition.or(c)),
            None => (lower, upper, condition),
        };
        let body = args
            .first()
            .map(|(s, p)| wrap(s, *p, prec::PRODUCT))
            .unwrap_or_default();
        let out = match name {
            "int" => {
                let mut s = "\\int".to_owned();
                if let Some(l) = &lower {
                    s.push_str(&format!("_{{{l}}}"));
                } else if let Some(c) = &condition {
                    s.push_str(&format!("_{{{c}}}"));
                }
                if let Some(u) = &upper {
                    s.push_str(&format!("^{{{u}}}"));
                }
                if var.is_empty() {
                    format!("{s} {body}")
                } else {
                    format!("{s} {body} \\, d{var}")
                }
            }
            "limit" => {
                let under = match (&lower, &condition) {
                    (Some(l), _) if !var.is_empty() => format!("{var} \\to {l}"),
                    (_, Some(c)) => c.clone(),
                    _ => var.clone(),
                };
                format!("\\lim_{{{under}}} {body}")
            }
            "forall" | "exists" => {
                let mut s = format!("{} {var}", o.symbol);
                if let Some(c) = &condition {
                    s.push_str(&format!(" \\mid {c}"));
                }
                format!("{s}, {body}")
            }
            _ => {
                let under = match (&lower, &condition) {
                    (Some(l), _) if !var.is_empty() => format!("{var}={l}"),
                    (Some(l), _) => l.clone(),
                    (None, Some(c)) => c.clone(),
                    (None, None) => var.clone(),
                };
                let mut s = o.symbol.to_owned();
                if !under.is_empty() {
                    s.push_str(&format!("_{{{under}}}"));
                }
                if let Some(u) = &upper {
                    s.push_str(&format!("^{{{u}}}"));
                }
                format!("{s} {body}")
            }
        };
        (out, o.prec)
    }

    /// Sets, lists, vectors, intervals, matrices, piecewise functions, and
    /// lambdas.
    fn container(&mut self, name: &str, el: ElementRef<'_>, kids: &[ElementRef<'_>]) -> String {
        let parts = Applied::split(kids);
        let items: Vec<String> = parts.args.iter().map(|k| self.part(*k).0).collect();
        let list = items.join(", ");
        match name {
            "set" => match (parts.bvars.first(), self.inner(parts.condition)) {
                (Some(b), Some(c)) => {
                    let v = self.bvar(*b);
                    format!("\\left\\{{ {v} \\mid {c} \\right\\}}")
                }
                _ => format!("\\left\\{{ {list} \\right\\}}"),
            },
            "list" | "vector" => format!("\\left( {list} \\right)"),
            "interval" => {
                let (l, r) = match el.attr("closure").map(str::trim) {
                    Some("open") => ("(", ")"),
                    Some("open-closed") => ("(", "]"),
                    Some("closed-open") => ("[", ")"),
                    _ => ("[", "]"),
                };
                format!("\\left{l} {list} \\right{r}")
            }
            "matrixrow" => items.join(" & "),
            "matrix" => format!(
                "\\begin{{pmatrix}} {} \\end{{pmatrix}}",
                items.join(" \\\\ ")
            ),
            "piecewise" => {
                let mut rows = Vec::new();
                for k in kids {
                    let piece = Self::kids(*k);
                    match local(k.value().name()) {
                        "piece" => {
                            let v = piece.first().map(|p| self.part(*p).0).unwrap_or_default();
                            let c = piece.get(1).map(|p| self.part(*p).0).unwrap_or_default();
                            rows.push(format!("{v} & {c}"));
                        }
                        "otherwise" => {
                            let v = piece.first().map(|p| self.part(*p).0).unwrap_or_default();
                            rows.push(format!("{v} & \\text{{otherwise}}"));
                        }
                        _ => {}
                    }
                }
                format!("\\begin{{cases}} {} \\end{{cases}}", rows.join(" \\\\ "))
            }
            // lambda: the bound variables map to the body.
            _ => {
                let vars: Vec<String> = parts.bvars.iter().map(|b| self.bvar(*b)).collect();
                format!("{} \\mapsto {list}", vars.join(", "))
            }
        }
    }
}

/// The formula of a MathML file (`.mml`): the first `<math>` element read
/// as LaTeX, presentation or content. `None` when the text has no math.
pub(crate) fn read_file(text: &str) -> Option<MathMl> {
    // The HTML parser knows MathML only unprefixed: `<m:mi>` becomes `<mi>`.
    let lower = text.to_ascii_lowercase();
    let text = match lower.find(":math") {
        Some(i) => {
            let start = lower[..i].rfind('<').map_or(i, |s| s + 1);
            let prefix = &text[start..i];
            if !prefix.is_empty()
                && prefix
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                text.replace(&format!("<{prefix}:"), "<")
                    .replace(&format!("</{prefix}:"), "</")
            } else {
                text.to_owned()
            }
        }
        None => text.to_owned(),
    };
    let html = scraper::Html::parse_document(&text);
    let math = html
        .root_element()
        .descendent_elements()
        .find(|e| local(e.value().name()) == "math")?;
    Some(read(math))
}

/// Largest MathML file read.
pub const MAX_MATHML_BYTES: usize = 8 << 20;

/// Loads MathML files (`.mml`, `.mathml`) as one formula: display math
/// under a `Math` marker, from presentation or content MathML, as math in
/// any other document reads.
#[derive(Clone, Copy, Debug, Default)]
pub struct MathMlLoader;

impl crate::Loader for MathMlLoader {
    fn id(&self) -> &'static str {
        "mathml"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["mml", "mathml"]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(
        &self,
        source: &crate::Source,
        _options: &crate::LoadOptions,
    ) -> Result<textweaver_text::Document, crate::LoadError> {
        use textweaver_core::{CharRange, MarkerKind};
        let head = source.read_head(MAX_MATHML_BYTES + 1)?;
        if head.len() > MAX_MATHML_BYTES {
            return Err(crate::LoadError::Parse(format!(
                "the formula is larger than {} megabytes",
                MAX_MATHML_BYTES >> 20
            )));
        }
        let decoded = crate::decode_source(
            source,
            crate::encoding::sniff_xml_encoding(&head).as_deref(),
        )?;
        let mut meta = crate::meta_for(source, self.id());
        crate::note_encoding(&mut meta, &decoded);
        meta.title = crate::title_from_path(source);
        let math = read_file(&decoded.text)
            .ok_or_else(|| crate::LoadError::Parse("it has no math element".into()))?;
        let text = match (math.latex, math.fallback) {
            (Some(l), _) => format!("$${l}$$"),
            (None, Some(f)) => {
                return Ok(textweaver_text::Document::new(
                    meta,
                    ropey::Rope::from_str(&f),
                    Vec::new(),
                ));
            }
            (None, None) => return Err(crate::LoadError::Parse("the formula is empty".into())),
        };
        let len = text.chars().count();
        let marker =
            textweaver_text::Marker::new(MarkerKind::Math, CharRange::new(0, len)).with_level(1);
        Ok(textweaver_text::Document::new(
            meta,
            ropey::Rope::from_str(&text),
            vec![marker],
        ))
    }
}

#[cfg(test)]
mod tests {
    use scraper::Html;

    use super::*;

    fn read_first(html: &str) -> MathMl {
        let doc = Html::parse_document(html);
        let math = doc
            .root_element()
            .descendent_elements()
            .find(|e| local(e.value().name()) == "math")
            .expect("a math element");
        read(math)
    }

    fn latex(inner: &str) -> String {
        read_first(&format!("<p><math>{inner}</math></p>"))
            .latex
            .unwrap_or_default()
    }

    #[test]
    fn tokens_rows_and_fractions() {
        assert_eq!(latex("<mfrac><mi>a</mi><mi>b</mi></mfrac>"), r"\frac{a}{b}");
        assert_eq!(
            latex("<mrow><msup><mi>x</mi><mn>2</mn></msup><mo>+</mo><mn>1</mn></mrow>"),
            "x^{2}+1"
        );
        assert_eq!(latex("<mi>sin</mi><mo>&#x2061;</mo><mi>x</mi>"), r"\sin x");
        assert_eq!(latex("<mi>α</mi><mo>≤</mo><mi>β</mi>"), r"\alpha\leq\beta");
        assert_eq!(
            latex(r#"<mfrac linethickness="0"><mi>n</mi><mi>k</mi></mfrac>"#),
            r"{n \atop k}"
        );
        assert_eq!(latex("<mtext>if $5</mtext>"), r"\text{if \$5}");
        assert_eq!(latex("<mo>$</mo><mn>5</mn>"), r"\$5");
    }

    #[test]
    fn roots_scripts_and_limits() {
        assert_eq!(
            latex(
                "<msqrt><msup><mi>b</mi><mn>2</mn></msup><mo>-</mo><mn>4</mn><mi>a</mi><mi>c</mi></msqrt>"
            ),
            r"\sqrt{b^{2}-4ac}"
        );
        assert_eq!(latex("<mroot><mi>x</mi><mn>3</mn></mroot>"), r"\sqrt[3]{x}");
        assert_eq!(
            latex("<msubsup><mi>x</mi><mi>i</mi><mn>2</mn></msubsup>"),
            "x_{i}^{2}"
        );
        assert_eq!(
            latex(
                "<munderover><mo>∑</mo><mrow><mi>i</mi><mo>=</mo><mn>1</mn></mrow><mi>n</mi></munderover><mi>i</mi>"
            ),
            r"\sum_{i=1}^{n}i"
        );
        assert_eq!(
            latex("<munder><mi>lim</mi><mrow><mi>x</mi><mo>→</mo><mn>0</mn></mrow></munder>"),
            r"\lim_{x\to0}"
        );
        assert_eq!(latex("<mover><mi>x</mi><mo>^</mo></mover>"), r"\hat{x}");
        assert_eq!(latex("<mover><mi>x</mi><mo>¯</mo></mover>"), r"\bar{x}");
        assert_eq!(
            latex(
                "<mmultiscripts><mi>C</mi><none/><none/><mprescripts/><mn>1</mn><mn>2</mn></mmultiscripts>"
            ),
            r"{}_{1}^{2}C"
        );
    }

    #[test]
    fn tables_fences_and_semantics() {
        assert_eq!(
            latex(
                "<mfenced><mtable><mtr><mtd><mi>a</mi></mtd><mtd><mi>b</mi></mtd></mtr><mtr><mtd><mi>c</mi></mtd><mtd><mi>d</mi></mtd></mtr></mtable></mfenced>"
            ),
            r"\left( \begin{matrix} a & b \\ c & d \end{matrix} \right)"
        );
        assert_eq!(
            latex(r#"<mfenced open="[" close="]" separators=";"><mi>a</mi><mi>b</mi></mfenced>"#),
            r"\left[ a; b \right]"
        );
        // A TeX annotation wins over the presentation markup.
        assert_eq!(
            latex(
                r#"<semantics><mrow><mi>y</mi></mrow><annotation encoding="application/x-tex">\frac{1}{2}</annotation></semantics>"#
            ),
            r"\frac{1}{2}"
        );
        // Other annotations are left out.
        assert_eq!(
            latex(
                r#"<semantics><mi>y</mi><annotation encoding="text/plain">why</annotation></semantics>"#
            ),
            "y"
        );
        assert_eq!(latex("<mphantom><mi>z</mi></mphantom><mi>w</mi>"), "w");
    }

    #[test]
    fn display_alttext_and_image_fallbacks() {
        let m = read_first(
            r#"<math display="block" alttext="x squared"><msup><mi>x</mi><mn>2</mn></msup></math>"#,
        );
        assert!(m.display);
        assert_eq!(m.latex.as_deref(), Some("x^{2}"));
        assert_eq!(m.fallback.as_deref(), Some("x squared"));
        let m = read_first(r#"<math alttext="  the area  "></math>"#);
        assert_eq!(m.latex, None);
        assert_eq!(m.fallback.as_deref(), Some("the area"));
    }

    #[test]
    fn content_arithmetic_powers_roots_and_fractions() {
        let c = |inner: &str| latex(inner);
        assert_eq!(
            c("<apply><plus/><ci>x</ci><apply><times/><cn>2</cn><ci>y</ci></apply></apply>"),
            r"x + 2 \times y"
        );
        assert_eq!(
            c("<apply><times/><apply><plus/><ci>a</ci><ci>b</ci></apply><ci>c</ci></apply>"),
            r"\left( a + b \right) \times c"
        );
        assert_eq!(
            c("<apply><minus/><ci>a</ci><apply><minus/><ci>b</ci><ci>c</ci></apply></apply>"),
            r"a - \left( b - c \right)"
        );
        assert_eq!(c("<apply><minus/><ci>x</ci></apply>"), "-x");
        assert_eq!(
            c("<apply><power/><apply><plus/><ci>x</ci><cn>1</cn></apply><cn>2</cn></apply>"),
            r"\left( x + 1 \right)^{2}"
        );
        assert_eq!(
            c("<apply><divide/><ci>a</ci><ci>b</ci></apply>"),
            r"\frac{a}{b}"
        );
        assert_eq!(
            c("<apply><root/><degree><cn>3</cn></degree><ci>x</ci></apply>"),
            r"\sqrt[3]{x}"
        );
        assert_eq!(c("<apply><root/><ci>x</ci></apply>"), r"\sqrt{x}");
        assert_eq!(c(r#"<cn type="rational">1<sep/>3</cn>"#), r"\frac{1}{3}");
        assert_eq!(c("<apply><abs/><ci>x</ci></apply>"), r"\left| x \right|");
        assert_eq!(c("<apply><factorial/><ci>n</ci></apply>"), "n!");
        assert_eq!(c("<apply><selector/><ci>N</ci><cn>0</cn></apply>"), "N_{0}");
    }

    #[test]
    fn content_relations_functions_sets_and_big_operators() {
        let c = |inner: &str| latex(inner);
        assert_eq!(
            c("<apply><leq/><ci>a</ci><ci>b</ci><ci>c</ci></apply>"),
            r"a \leq b \leq c"
        );
        assert_eq!(c("<apply><sin/><ci>x</ci></apply>"), r"\sin x");
        assert_eq!(
            c("<apply><sin/><apply><plus/><ci>x</ci><pi/></apply></apply>"),
            r"\sin\left( x + \pi \right)"
        );
        assert_eq!(
            c("<apply><log/><logbase><cn>2</cn></logbase><ci>n</ci></apply>"),
            r"\log_{2} n"
        );
        assert_eq!(
            c("<apply><ci>f</ci><ci>x</ci><ci>y</ci></apply>"),
            r"f\left( x, y \right)"
        );
        assert_eq!(
            c(
                "<apply><sum/><bvar><ci>i</ci></bvar><lowlimit><cn>1</cn></lowlimit><uplimit><ci>n</ci></uplimit><apply><power/><ci>i</ci><cn>2</cn></apply></apply>"
            ),
            r"\sum_{i=1}^{n} i^{2}"
        );
        assert_eq!(
            c(
                "<apply><int/><bvar><ci>x</ci></bvar><lowlimit><cn>0</cn></lowlimit><uplimit><infinity/></uplimit><apply><exp/><apply><minus/><ci>x</ci></apply></apply></apply>"
            ),
            r"\int_{0}^{\infty} e^{-x} \, dx"
        );
        assert_eq!(
            c(
                "<apply><int/><bvar><ci>x</ci></bvar><interval><cn>0</cn><cn>1</cn></interval><ci>x</ci></apply>"
            ),
            r"\int_{0}^{1} x \, dx"
        );
        assert_eq!(
            c(
                "<apply><limit/><bvar><ci>x</ci></bvar><lowlimit><cn>0</cn></lowlimit><apply><sin/><ci>x</ci></apply></apply>"
            ),
            r"\lim_{x \to 0} \sin x"
        );
        assert_eq!(
            c(
                "<apply><diff/><bvar><ci>x</ci></bvar><apply><power/><ci>x</ci><cn>2</cn></apply></apply>"
            ),
            r"\frac{d}{d x} x^{2}"
        );
        assert_eq!(
            c("<apply><in/><ci>x</ci><reals/></apply>"),
            r"x \in \mathbb{R}"
        );
        assert_eq!(
            c("<apply><union/><set><ci>a</ci><ci>b</ci></set><emptyset/></apply>"),
            r"\left\{ a, b \right\} \cup \emptyset"
        );
        assert_eq!(
            c(r#"<interval closure="open-closed"><cn>0</cn><cn>1</cn></interval>"#),
            r"\left( 0, 1 \right]"
        );
        assert_eq!(
            c("<apply><frobnicate/><ci>x</ci></apply>"),
            r"\operatorname{frobnicate}\left( x \right)"
        );
    }

    #[test]
    fn mml_files_are_one_formula() {
        let m = read_file(
            r#"<?xml version="1.0"?><m:math xmlns:m="http://www.w3.org/1998/Math/MathML"><m:apply><m:eq/><m:ci>E</m:ci><m:apply><m:times/><m:ci>m</m:ci><m:apply><m:power/><m:ci>c</m:ci><m:cn>2</m:cn></m:apply></m:apply></m:apply></m:math>"#,
        )
        .unwrap();
        assert_eq!(m.latex.as_deref(), Some(r"E = m \times c^{2}"));
        assert!(read_file("<p>no math</p>").is_none());
        let d = crate::Loader::load(
            &MathMlLoader,
            &crate::Source::Bytes {
                data: b"<math><mi>x</mi></math>".to_vec(),
                hint: "mml".into(),
            },
            &crate::LoadOptions::default(),
        )
        .unwrap();
        assert_eq!(d.text().to_string(), "$$x$$");
        assert_eq!(
            d.marker_index()
                .count(textweaver_core::MarkerKind::Math, Some(1)),
            1
        );
    }

    #[test]
    fn deep_math_is_flattened_not_overflowed() {
        let deep = format!(
            "{}<mi>x</mi>{}",
            "<mrow>".repeat(5000),
            "</mrow>".repeat(5000)
        );
        let l = latex(&deep);
        assert!(l.contains('x'), "{l}");
    }
}
