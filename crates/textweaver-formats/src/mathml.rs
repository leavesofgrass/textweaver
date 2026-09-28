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
    accent_command, delimiter, function_name, is_atom, math_text, nary_command, push, takes_limits,
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
            let mut c = Conv { depth: 0 };
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
}

impl Conv {
    /// The element children of `el`, in order.
    fn kids<'a>(el: ElementRef<'a>) -> Vec<ElementRef<'a>> {
        el.child_elements().collect()
    }

    /// The LaTeX of `el`'s children, one after another.
    fn children(&mut self, el: ElementRef<'_>) -> String {
        let mut out = String::new();
        for k in Self::kids(el) {
            let piece = self.node(k);
            if !piece.is_empty() {
                push(&mut out, &piece);
            }
        }
        out
    }

    fn node(&mut self, el: ElementRef<'_>) -> String {
        if self.depth >= MAX_DEPTH {
            return token(&el.text().collect::<String>());
        }
        self.depth += 1;
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
            let (mut subs, mut sups) = (String::new(), String::new());
            let mut i = from;
            while i < to {
                push(&mut subs, &this.nth(kids, i));
                if i + 1 < to {
                    push(&mut sups, &this.nth(kids, i + 1));
                }
                i += 2;
            }
            (subs, sups)
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
        let mut inner = String::new();
        for (i, k) in kids.iter().enumerate() {
            if i > 0
                && let Some(sep) = seps.get(i - 1).or(seps.last())
            {
                inner.push_str(&token(&sep.to_string()));
                inner.push(' ');
            }
            push(&mut inner, &self.node(*k));
        }
        match (delimiter(open), delimiter(close)) {
            (Some(l), Some(r)) => format!("\\left{l} {inner} \\right{r}"),
            _ => format!("{} {inner} {}", token(open), token(close)),
        }
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
