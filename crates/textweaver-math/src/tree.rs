//! The math tree shared by both parsers, the MathML writer, the speech
//! builder, and the navigator.
//!
//! Every [`Node`] carries the char range of the source text it came from, so
//! spoken words map back to the source (ADR-0005) and navigation can
//! highlight the part of the expression being read.

use serde::{Deserialize, Serialize};
use textweaver_core::CharRange;

use crate::MathError;

/// The input notation of a math expression.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Notation {
    /// LaTeX math (the body between `$…$`, `\(…\)`, and similar).
    #[default]
    Latex,
    /// ASCIIMath (asciimath.org), usually written between backticks.
    AsciiMath,
}

impl Notation {
    /// A lower-case name, for messages.
    pub fn name(self) -> &'static str {
        match self {
            Notation::Latex => "LaTeX",
            Notation::AsciiMath => "ASCIIMath",
        }
    }
}

/// A problem the lenient parsers recovered from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// What went wrong, in plain English.
    pub message: String,
    /// Where, in chars of the source.
    pub span: CharRange,
}

/// A parsed math expression: the tree plus the source it came from.
///
/// Parsing is lenient and total: every input produces a tree (unknown
/// commands and stray braces become [`NodeKind::Error`] nodes or are
/// skipped) and the problems are listed in [`Math::diagnostics`]. Use
/// [`Math::strict`] to turn diagnostics into an error.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Math {
    /// The notation the source was written in.
    pub notation: Notation,
    /// The source text (without delimiters such as `$`).
    pub source: String,
    /// The root node; its span covers the whole source.
    pub root: Node,
    /// Problems the parser recovered from, in source order.
    pub diagnostics: Vec<Diagnostic>,
}

impl Math {
    /// True when the parser reported no problems.
    pub fn is_clean(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Returns the expression, or the first diagnostic as an error.
    pub fn strict(self) -> Result<Math, MathError> {
        match self.diagnostics.first() {
            None => Ok(self),
            Some(d) => Err(MathError::Parse {
                notation: self.notation,
                message: d.message.clone(),
                span: d.span,
            }),
        }
    }

    /// The source as chars, indexed like every [`Node::span`].
    pub fn source_chars(&self) -> Vec<char> {
        self.source.chars().collect()
    }

    /// The source text covered by `span`.
    pub fn source_text(&self, span: CharRange) -> String {
        self.source
            .chars()
            .skip(span.start.0)
            .take(span.len())
            .collect()
    }
}

/// One node of the math tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// What the node is.
    pub kind: NodeKind,
    /// The source chars it came from.
    pub span: CharRange,
}

/// The class of an operator, which decides spacing in MathML and wording in
/// speech (a leading minus is "negative", a relation is spoken between its
/// operands, a large operator takes limits).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpClass {
    /// Binary operator such as `+` or `×`.
    Binary,
    /// Relation such as `=`, `≤`, or an arrow.
    Relation,
    /// Large operator that takes limits (`∑`, `∫`).
    Large,
    /// Opening bracket.
    Open,
    /// Closing bracket.
    Close,
    /// A bracket that opens and closes with the same character (`|`, `‖`).
    Fence,
    /// Punctuation such as `,` or `;`.
    Punctuation,
    /// Postfix operator such as `!` or `′`.
    Postfix,
    /// Prefix operator such as `¬` or `∀`.
    Prefix,
    /// Anything else.
    Other,
}

/// A decoration above or below a base.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccentKind {
    /// `\hat`, `\widehat`, ASCIIMath `hat`.
    Hat,
    /// `\bar`.
    Bar,
    /// `\overline`, ASCIIMath `bar`.
    Overline,
    /// `\tilde`, `\widetilde`.
    Tilde,
    /// `\vec`, `\overrightarrow`, ASCIIMath `vec`.
    Vector,
    /// `\overleftarrow`.
    LeftArrow,
    /// `\overleftrightarrow`.
    LeftRightArrow,
    /// `\dot`.
    Dot,
    /// `\ddot`.
    DoubleDot,
    /// `\dddot`.
    TripleDot,
    /// `\acute`.
    Acute,
    /// `\grave`.
    Grave,
    /// `\breve`.
    Breve,
    /// `\check`.
    Check,
    /// `\mathring`.
    Ring,
    /// `\overarc`, ASCIIMath `overarc`.
    Arc,
    /// `\underline`, ASCIIMath `ul`.
    Underline,
    /// `\overbrace`, ASCIIMath `obrace`.
    Overbrace,
    /// `\underbrace`, ASCIIMath `ubrace`.
    Underbrace,
}

impl AccentKind {
    /// The MathML operator character drawn for the accent.
    pub fn mark(self) -> &'static str {
        match self {
            AccentKind::Hat => "^",
            AccentKind::Bar | AccentKind::Overline => "\u{00AF}",
            AccentKind::Tilde => "~",
            AccentKind::Vector => "\u{2192}",
            AccentKind::LeftArrow => "\u{2190}",
            AccentKind::LeftRightArrow => "\u{2194}",
            AccentKind::Dot => "\u{02D9}",
            AccentKind::DoubleDot => "\u{00A8}",
            AccentKind::TripleDot => "\u{20DB}",
            AccentKind::Acute => "\u{00B4}",
            AccentKind::Grave => "`",
            AccentKind::Breve => "\u{02D8}",
            AccentKind::Check => "\u{02C7}",
            AccentKind::Ring => "\u{02DA}",
            AccentKind::Arc => "\u{2322}",
            AccentKind::Underline => "_",
            AccentKind::Overbrace => "\u{23DE}",
            AccentKind::Underbrace => "\u{23DF}",
        }
    }

    /// True for decorations drawn below the base.
    pub fn is_under(self) -> bool {
        matches!(self, AccentKind::Underline | AccentKind::Underbrace)
    }

    /// True for decorations that stretch over a wide base.
    pub fn is_stretchy(self) -> bool {
        matches!(
            self,
            AccentKind::Overline
                | AccentKind::Underline
                | AccentKind::Overbrace
                | AccentKind::Underbrace
                | AccentKind::LeftArrow
                | AccentKind::LeftRightArrow
                | AccentKind::Arc
        )
    }
}

/// A math font (`\mathbf`, ASCIIMath `bb`, and so on).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Variant {
    /// Upright (`\mathrm`).
    Normal,
    /// Bold (`\mathbf`, `\boldsymbol`).
    Bold,
    /// Italic (`\mathit`).
    Italic,
    /// Bold italic.
    BoldItalic,
    /// Double-struck (`\mathbb`).
    DoubleStruck,
    /// Script or calligraphic (`\mathcal`, `\mathscr`).
    Script,
    /// Bold script.
    BoldScript,
    /// Fraktur (`\mathfrak`).
    Fraktur,
    /// Bold Fraktur.
    BoldFraktur,
    /// Sans-serif (`\mathsf`).
    SansSerif,
    /// Bold sans-serif.
    BoldSansSerif,
    /// Sans-serif italic.
    SansSerifItalic,
    /// Sans-serif bold italic.
    SansSerifBoldItalic,
    /// Monospace (`\mathtt`).
    Monospace,
}

impl Variant {
    /// The MathML `mathvariant` value.
    pub fn mathml_name(self) -> &'static str {
        match self {
            Variant::Normal => "normal",
            Variant::Bold => "bold",
            Variant::Italic => "italic",
            Variant::BoldItalic => "bold-italic",
            Variant::DoubleStruck => "double-struck",
            Variant::Script => "script",
            Variant::BoldScript => "bold-script",
            Variant::Fraktur => "fraktur",
            Variant::BoldFraktur => "bold-fraktur",
            Variant::SansSerif => "sans-serif",
            Variant::BoldSansSerif => "bold-sans-serif",
            Variant::SansSerifItalic => "sans-serif-italic",
            Variant::SansSerifBoldItalic => "sans-serif-bold-italic",
            Variant::Monospace => "monospace",
        }
    }

    /// The name spoken at high verbosity ("bold"), or `None` for fonts that
    /// carry no meaning worth announcing (upright, italic).
    pub fn spoken_name(self) -> Option<&'static str> {
        match self {
            Variant::Normal | Variant::Italic => None,
            Variant::Bold | Variant::BoldItalic => Some("bold"),
            Variant::DoubleStruck => Some("double-struck"),
            Variant::Script => Some("script"),
            Variant::BoldScript => Some("bold script"),
            Variant::Fraktur => Some("fraktur"),
            Variant::BoldFraktur => Some("bold fraktur"),
            Variant::SansSerif | Variant::SansSerifItalic => Some("sans-serif"),
            Variant::BoldSansSerif | Variant::SansSerifBoldItalic => Some("bold sans-serif"),
            Variant::Monospace => Some("monospace"),
        }
    }
}

/// What a [`NodeKind::Table`] is laid out as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TableKind {
    /// A matrix (its brackets are the enclosing [`NodeKind::Fenced`]).
    Matrix,
    /// Piecewise cases (`cases`, ASCIIMath `{(…),(…):}`).
    Cases,
    /// Aligned equations or stacked lines (`aligned`, `\\` line breaks,
    /// `\substack`).
    Aligned,
    /// A general `array`.
    Array,
}

/// An enclosure drawn around its body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Enclosure {
    /// Struck through (`\cancel`, ASCIIMath `cancel`).
    Cancel,
    /// Boxed (`\boxed`).
    Box,
}

/// The kinds of math nodes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// A sequence of nodes (a group, a row, the whole expression).
    Row(Vec<Node>),
    /// A number such as `3` or `2.5`.
    Number(String),
    /// An identifier: a letter, a Greek letter, or a symbol used as a
    /// variable or constant (`∞`, `ℝ`).
    Ident(String),
    /// A named function or operator written upright (`sin`, `log`, `lim`).
    Function(String),
    /// An operator, relation, bracket, or punctuation mark.
    Operator {
        /// The operator as Unicode text.
        text: String,
        /// Its class.
        class: OpClass,
    },
    /// Ordinary text inside math (`\text{if }`).
    Text(String),
    /// White space; carries its MathML width (for example `0.167em`).
    Space(String),
    /// A fraction or, without a bar, a binomial coefficient's stack.
    Fraction {
        /// Numerator.
        num: Box<Node>,
        /// Denominator.
        den: Box<Node>,
        /// False for `\binom`-style stacks drawn without a fraction bar.
        bar: bool,
    },
    /// A square root or, with an index, an n-th root.
    Root {
        /// The index (`3` in a cube root), if any.
        index: Option<Box<Node>>,
        /// The radicand.
        radicand: Box<Node>,
    },
    /// A base with a subscript, a superscript, or both.
    Scripts {
        /// The base.
        base: Box<Node>,
        /// Subscript, or the lower limit when `limits` is set.
        sub: Option<Box<Node>>,
        /// Superscript, or the upper limit when `limits` is set.
        sup: Option<Box<Node>>,
        /// Scripts drawn directly below and above (`\lim_{x\to0}`,
        /// `\overset`), not to the side.
        limits: bool,
    },
    /// A bracketed body. Empty `open` or `close` means an invisible bracket
    /// (`\left.`, ASCIIMath `{:`).
    Fenced {
        /// Opening bracket text.
        open: String,
        /// Closing bracket text.
        close: String,
        /// The contents.
        body: Box<Node>,
    },
    /// A base with an accent or a brace above or below it.
    Accent {
        /// The base.
        base: Box<Node>,
        /// The accent.
        accent: AccentKind,
    },
    /// Rows of cells: matrices, cases, aligned equations.
    Table {
        /// How the table is laid out.
        kind: TableKind,
        /// The rows, each a list of cells.
        rows: Vec<Vec<Node>>,
    },
    /// A body in a math font.
    Style {
        /// The font.
        variant: Variant,
        /// The contents.
        body: Box<Node>,
    },
    /// A body with an enclosure.
    Enclose {
        /// The enclosure.
        notation: Enclosure,
        /// The contents.
        body: Box<Node>,
    },
    /// Source the parser could not interpret, kept as text.
    Error(String),
}

impl Node {
    /// Creates a node.
    pub fn new(kind: NodeKind, span: CharRange) -> Self {
        Node { kind, span }
    }

    /// An empty row at `at`.
    pub fn empty(at: usize) -> Self {
        Node::new(NodeKind::Row(Vec::new()), CharRange::empty(at))
    }

    /// True for a row with no children.
    pub fn is_empty_row(&self) -> bool {
        matches!(&self.kind, NodeKind::Row(items) if items.is_empty())
    }

    /// The text of a token node (number, identifier, function, operator,
    /// text), or `None` for structures.
    pub fn token_text(&self) -> Option<&str> {
        match &self.kind {
            NodeKind::Number(s)
            | NodeKind::Ident(s)
            | NodeKind::Function(s)
            | NodeKind::Text(s)
            | NodeKind::Error(s) => Some(s),
            NodeKind::Operator { text, .. } => Some(text),
            _ => None,
        }
    }

    /// True when this node is the operator `text`.
    pub fn is_op(&self, text: &str) -> bool {
        matches!(&self.kind, NodeKind::Operator { text: t, .. } if t == text)
    }

    /// The operator class, for operator nodes.
    pub fn op_class(&self) -> Option<OpClass> {
        match &self.kind {
            NodeKind::Operator { class, .. } => Some(*class),
            _ => None,
        }
    }

    /// The direct children in source order.
    pub fn children(&self) -> Vec<&Node> {
        let mut out: Vec<&Node> = match &self.kind {
            NodeKind::Row(items) => items.iter().collect(),
            NodeKind::Fraction { num, den, .. } => vec![num, den],
            NodeKind::Root { index, radicand } => {
                index.iter().map(|b| &**b).chain([&**radicand]).collect()
            }
            NodeKind::Scripts { base, sub, sup, .. } => [Some(base), sub.as_ref(), sup.as_ref()]
                .into_iter()
                .flatten()
                .map(|b| &**b)
                .collect(),
            NodeKind::Fenced { body, .. }
            | NodeKind::Accent { base: body, .. }
            | NodeKind::Style { body, .. }
            | NodeKind::Enclose { body, .. } => vec![body],
            NodeKind::Table { rows, .. } => rows.iter().flatten().collect(),
            _ => Vec::new(),
        };
        out.sort_by_key(|n| n.span.start);
        out
    }

    /// Nesting depth of the tree below this node (a token is 1).
    pub fn depth(&self) -> usize {
        1 + self.children().iter().map(|c| c.depth()).max().unwrap_or(0)
    }
}
