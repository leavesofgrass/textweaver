//! Presentation MathML output.
//!
//! The output is one `<math>` element with `alttext`, suitable for HTML:
//! browsers render it natively and screen readers (NVDA with MathCAT,
//! JAWS, VoiceOver) read and navigate it.

use serde::{Deserialize, Serialize};
use textweaver_core::Verbosity;

use crate::speech::{SpeechOptions, speak};
use crate::symbols::takes_limits;
use crate::{Enclosure, Math, Node, NodeKind, Notation, OpClass, TableKind, Variant};

/// What goes into the `alttext` attribute.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AltText {
    /// No `alttext`.
    None,
    /// The source (LaTeX or ASCIIMath), the common convention.
    #[default]
    Source,
    /// The spoken English at the given verbosity.
    Speech(Verbosity),
}

/// Options for [`to_mathml`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MathMlOptions {
    /// Display (block) math rather than inline.
    pub display: bool,
    /// The `alttext` attribute.
    pub alttext: AltText,
    /// Wrap the output in `<semantics>` with the source as an
    /// `<annotation>`, so copying and conversion tools can recover it.
    pub annotation: bool,
}

impl Default for MathMlOptions {
    fn default() -> Self {
        MathMlOptions {
            display: false,
            alttext: AltText::Source,
            annotation: true,
        }
    }
}

impl MathMlOptions {
    /// Default options, inline or display.
    pub fn new(display: bool) -> Self {
        MathMlOptions {
            display,
            ..MathMlOptions::default()
        }
    }
}

/// Writes `math` as a presentation MathML `<math>` element.
pub fn to_mathml(math: &Math, opts: &MathMlOptions) -> String {
    let mut out = String::new();
    out.push_str("<math xmlns=\"http://www.w3.org/1998/Math/MathML\"");
    if opts.display {
        out.push_str(" display=\"block\"");
    }
    let alt = match opts.alttext {
        AltText::None => None,
        AltText::Source => Some(math.source.trim().to_owned()),
        AltText::Speech(v) => Some(speak(math, &SpeechOptions::new(v)).text),
    };
    if let Some(alt) = alt {
        out.push_str(" alttext=\"");
        escape_into(&mut out, &alt);
        out.push('"');
    }
    out.push('>');
    let mut w = Writer {
        out: String::new(),
        display: opts.display,
    };
    w.as_one(&math.root, None);
    if opts.annotation {
        out.push_str("<semantics>");
        out.push_str(&w.out);
        let encoding = match math.notation {
            Notation::Latex => "application/x-tex",
            Notation::AsciiMath => "text/x-asciimath",
        };
        out.push_str("<annotation encoding=\"");
        out.push_str(encoding);
        out.push_str("\">");
        escape_into(&mut out, &math.source);
        out.push_str("</annotation></semantics>");
    } else {
        out.push_str(&w.out);
    }
    out.push_str("</math>");
    out
}

/// Escapes text for XML content and attribute values.
pub(crate) fn escape_into(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c if (c as u32) < 0x20 && c != '\n' && c != '\t' && c != '\r' => {}
            c => out.push(c),
        }
    }
}

struct Writer {
    out: String,
    display: bool,
}

impl Writer {
    fn open(&mut self, tag: &str, attrs: &[(&str, &str)]) {
        self.out.push('<');
        self.out.push_str(tag);
        for (k, v) in attrs {
            self.out.push(' ');
            self.out.push_str(k);
            self.out.push_str("=\"");
            escape_into(&mut self.out, v);
            self.out.push('"');
        }
        self.out.push('>');
    }

    fn close(&mut self, tag: &str) {
        self.out.push_str("</");
        self.out.push_str(tag);
        self.out.push('>');
    }

    fn token(&mut self, tag: &str, attrs: &[(&str, &str)], text: &str) {
        self.open(tag, attrs);
        escape_into(&mut self.out, text);
        self.close(tag);
    }

    /// Writes `n` as exactly one MathML element (wrapping rows in `mrow`),
    /// as `mfrac`, `msup`, and the others require.
    fn as_one(&mut self, n: &Node, variant: Option<Variant>) {
        match &n.kind {
            NodeKind::Row(items) if items.len() == 1 => self.as_one(&items[0], variant),
            NodeKind::Row(items) => {
                self.open("mrow", &[]);
                self.items(items, variant);
                self.close("mrow");
            }
            _ => self.node(n, variant),
        }
    }

    fn items(&mut self, items: &[Node], variant: Option<Variant>) {
        for (i, it) in items.iter().enumerate() {
            self.node(it, variant);
            if applies(it)
                && let Some(next) = items.get(i + 1)
                && !matches!(
                    next.kind,
                    NodeKind::Operator {
                        class: OpClass::Binary
                            | OpClass::Relation
                            | OpClass::Punctuation
                            | OpClass::Close,
                        ..
                    }
                )
            {
                self.token("mo", &[], "\u{2061}");
            }
        }
    }

    fn node(&mut self, n: &Node, variant: Option<Variant>) {
        let mv = variant.map(Variant::mathml_name);
        match &n.kind {
            NodeKind::Row(items) => {
                self.open("mrow", &[]);
                self.items(items, variant);
                self.close("mrow");
            }
            NodeKind::Number(s) => match mv {
                Some(v) => self.token("mn", &[("mathvariant", v)], s),
                None => self.token("mn", &[], s),
            },
            NodeKind::Ident(s) => {
                let upright_greek = s.chars().count() == 1
                    && s.chars().all(|c| ('\u{0391}'..='\u{03A9}').contains(&c));
                match mv {
                    Some(v) => self.token("mi", &[("mathvariant", v)], s),
                    None if upright_greek => self.token("mi", &[("mathvariant", "normal")], s),
                    None => self.token("mi", &[], s),
                }
            }
            NodeKind::Function(s) => self.token("mi", &[], s),
            NodeKind::Operator { text, class } => {
                let mut attrs: Vec<(&str, &str)> = Vec::new();
                match class {
                    OpClass::Open => attrs.extend([("form", "prefix"), ("stretchy", "false")]),
                    OpClass::Close => attrs.extend([("form", "postfix"), ("stretchy", "false")]),
                    OpClass::Fence => attrs.push(("stretchy", "false")),
                    _ => {}
                }
                self.token("mo", &attrs, text);
            }
            NodeKind::Text(t) => {
                // Keep edge spaces visible (`\text{ if }`).
                let t = t.replace(' ', "\u{00A0}");
                self.token("mtext", &[], &t);
            }
            NodeKind::Space(width) => {
                self.open("mspace", &[("width", width)]);
                self.close("mspace");
            }
            NodeKind::Fraction { num, den, bar } => {
                if *bar {
                    self.open("mfrac", &[]);
                } else {
                    self.open("mfrac", &[("linethickness", "0")]);
                }
                self.as_one(num, variant);
                self.as_one(den, variant);
                self.close("mfrac");
            }
            NodeKind::Root { index, radicand } => match index {
                None => {
                    self.open("msqrt", &[]);
                    self.as_one(radicand, variant);
                    self.close("msqrt");
                }
                Some(ix) => {
                    self.open("mroot", &[]);
                    self.as_one(radicand, variant);
                    self.as_one(ix, variant);
                    self.close("mroot");
                }
            },
            NodeKind::Scripts {
                base,
                sub,
                sup,
                limits,
            } => {
                let under_over = *limits || (self.display && display_limits(base));
                let tag = match (under_over, sub.is_some(), sup.is_some()) {
                    (true, true, true) => "munderover",
                    (true, true, false) => "munder",
                    (true, false, _) => "mover",
                    (false, true, true) => "msubsup",
                    (false, true, false) => "msub",
                    (false, false, _) => "msup",
                };
                self.open(tag, &[]);
                self.as_one(base, variant);
                if let Some(s) = sub {
                    self.as_one(s, variant);
                }
                if let Some(p) = sup {
                    self.as_one(p, variant);
                }
                self.close(tag);
            }
            NodeKind::Fenced { open, close, body } => {
                self.open("mrow", &[]);
                if !open.is_empty() {
                    self.token("mo", &[("fence", "true"), ("form", "prefix")], open);
                }
                self.as_one(body, variant);
                if !close.is_empty() {
                    self.token("mo", &[("fence", "true"), ("form", "postfix")], close);
                }
                self.close("mrow");
            }
            NodeKind::Accent { base, accent } => {
                let stretchy = if accent.is_stretchy() {
                    "true"
                } else {
                    "false"
                };
                if accent.is_under() {
                    self.open("munder", &[("accentunder", "true")]);
                } else {
                    self.open("mover", &[("accent", "true")]);
                }
                self.as_one(base, variant);
                self.token("mo", &[("stretchy", stretchy)], accent.mark());
                self.close(if accent.is_under() { "munder" } else { "mover" });
            }
            NodeKind::Table { kind, rows } => {
                let align = match kind {
                    TableKind::Cases => Some("left left"),
                    TableKind::Aligned => Some("right left"),
                    _ => None,
                };
                match align {
                    Some(a) => self.open("mtable", &[("columnalign", a)]),
                    None => self.open("mtable", &[]),
                }
                for r in rows {
                    self.open("mtr", &[]);
                    for cell in r {
                        self.open("mtd", &[]);
                        self.as_one(cell, variant);
                        self.close("mtd");
                    }
                    self.close("mtr");
                }
                self.close("mtable");
            }
            NodeKind::Style { variant: v, body } => self.as_one(body, Some(*v)),
            NodeKind::Enclose { notation, body } => {
                let n = match notation {
                    Enclosure::Cancel => "updiagonalstrike",
                    Enclosure::Box => "box",
                };
                self.open("menclose", &[("notation", n)]);
                self.as_one(body, variant);
                self.close("menclose");
            }
            NodeKind::Error(s) => {
                self.open("merror", &[]);
                self.token("mtext", &[], s);
                self.close("merror");
            }
        }
    }
}

/// True for nodes followed by an invisible function application.
fn applies(n: &Node) -> bool {
    match &n.kind {
        NodeKind::Function(f) => !takes_limits(f) && f != "mod" && f != "bmod",
        NodeKind::Scripts {
            base,
            limits: false,
            ..
        } => matches!(&base.kind, NodeKind::Function(f) if !takes_limits(f)),
        _ => false,
    }
}

/// Large operators whose limits go below and above in display math (sums,
/// products, unions; not integrals).
fn display_limits(base: &Node) -> bool {
    match &base.kind {
        NodeKind::Operator {
            text,
            class: OpClass::Large,
        } => !matches!(text.as_str(), "∫" | "∬" | "∭" | "⨌" | "∮" | "∯"),
        _ => false,
    }
}
