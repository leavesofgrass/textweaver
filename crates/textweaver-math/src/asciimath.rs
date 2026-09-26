//! ASCIIMath parser (asciimath.org), following the grammar and symbol table
//! of the reference implementation `ASCIIMathML.js`:
//!
//! ```text
//! v ::= [A-Za-z] | greek letters | numbers | other constant symbols
//! u ::= sqrt | text | bb | other unary symbols for font commands
//! b ::= frac | root | stackrel | other binary symbols
//! l ::= ( | [ | { | (: | {: | other left brackets
//! r ::= ) | ] | } | :) | :} | other right brackets
//! S ::= v | lEr | uS | bSS             simple expression
//! I ::= S_S | S^S | S_S^S | S          intermediate expression
//! E ::= IE | I/I                       expression
//! ```
//!
//! Symbols are matched longest first, the LaTeX names the reference
//! implementation also accepts (`\alpha` spelled `alpha`, `leq`, `times`)
//! are included, brackets around fraction parts, script arguments, and
//! unary arguments are removed, `[[a,b],[c,d]]` and `((a,b),(c,d))` become
//! matrices, and `{(…),(…):}` becomes cases.

use textweaver_core::CharRange;

use crate::build::{MAX_DEPTH, group_fences, items_span, row};
use crate::latex::{double_struck, token_for_char};
use crate::symbols::takes_limits;
use crate::{
    AccentKind, Diagnostic, Enclosure, Math, Node, NodeKind, Notation, OpClass, TableKind, Variant,
};

/// What an ASCIIMath symbol does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Am {
    /// A constant identifier (`alpha`, `oo`, `RR`).
    Ident(&'static str),
    /// A constant operator.
    Op(&'static str, OpClass),
    /// A large operator with limits below and above (`sum`).
    Large(&'static str),
    /// A function with limits below (`lim`, `max`).
    LimFunc(&'static str),
    /// A function applied to the following simple expression (`sin`).
    Func(&'static str),
    /// A letter used as a function (`f`, `g`).
    FuncIdent(&'static str),
    /// A function written as brackets (`abs`, `floor`).
    FenceFn(&'static str, &'static str),
    /// `sqrt`.
    Sqrt,
    /// `root`.
    Root,
    /// `frac`.
    Frac,
    /// `stackrel`, `overset`.
    Over,
    /// `underset`.
    Under,
    /// `color`, `id`, `class`: the first argument is not math.
    Skip2,
    /// An accent.
    Accent(AccentKind),
    /// `cancel`.
    Cancel,
    /// A font.
    Font(Variant),
    /// `text(…)`, `mbox(…)`.
    Text,
    /// `"…"`.
    Quote,
    /// A left bracket.
    Left(&'static str),
    /// A right bracket.
    Right(&'static str),
    /// `|`, which opens or closes.
    LeftRight(&'static str),
    /// `_`.
    Sub,
    /// `^`.
    Sup,
    /// `/`.
    Slash,
    /// `and`, `or`, `if`: words with space around them.
    Word(&'static str),
    /// Horizontal space.
    Space(&'static str),
    /// A definition that expands to another input.
    Def(&'static str),
}

use AccentKind as AK;
use OpClass::{
    Binary as B, Fence as F, Large as L, Other as Q, Postfix as X, Prefix as P, Punctuation as U,
    Relation as R,
};

/// The ASCIIMath symbol table: every input of `AMsymbols` in the reference
/// implementation, plus the LaTeX names it copies in `AMinitSymbols`.
pub(crate) static SYMBOLS: &[(&str, Am)] = &[
    // Greek
    ("alpha", Am::Ident("α")),
    ("beta", Am::Ident("β")),
    ("chi", Am::Ident("χ")),
    ("delta", Am::Ident("δ")),
    ("Delta", Am::Ident("Δ")),
    ("epsi", Am::Ident("ε")),
    ("epsilon", Am::Ident("ε")),
    ("varepsilon", Am::Ident("ɛ")),
    ("eta", Am::Ident("η")),
    ("gamma", Am::Ident("γ")),
    ("Gamma", Am::Ident("Γ")),
    ("iota", Am::Ident("ι")),
    ("kappa", Am::Ident("κ")),
    ("lambda", Am::Ident("λ")),
    ("Lambda", Am::Ident("Λ")),
    ("lamda", Am::Ident("λ")),
    ("Lamda", Am::Ident("Λ")),
    ("mu", Am::Ident("μ")),
    ("nu", Am::Ident("ν")),
    ("omega", Am::Ident("ω")),
    ("Omega", Am::Ident("Ω")),
    ("phi", Am::Ident("φ")),
    ("varphi", Am::Ident("ϕ")),
    ("Phi", Am::Ident("Φ")),
    ("pi", Am::Ident("π")),
    ("Pi", Am::Ident("Π")),
    ("psi", Am::Ident("ψ")),
    ("Psi", Am::Ident("Ψ")),
    ("rho", Am::Ident("ρ")),
    ("sigma", Am::Ident("σ")),
    ("Sigma", Am::Ident("Σ")),
    ("tau", Am::Ident("τ")),
    ("theta", Am::Ident("θ")),
    ("vartheta", Am::Ident("ϑ")),
    ("Theta", Am::Ident("Θ")),
    ("upsilon", Am::Ident("υ")),
    ("xi", Am::Ident("ξ")),
    ("Xi", Am::Ident("Ξ")),
    ("zeta", Am::Ident("ζ")),
    // operation symbols
    ("*", Am::Op("⋅", B)),
    ("cdot", Am::Op("⋅", B)),
    ("**", Am::Op("∗", B)),
    ("ast", Am::Op("∗", B)),
    ("***", Am::Op("⋆", B)),
    ("star", Am::Op("⋆", B)),
    ("//", Am::Op("/", B)),
    ("\\\\", Am::Op("\\", B)),
    ("backslash", Am::Op("\\", B)),
    ("setminus", Am::Op("\\", B)),
    ("xx", Am::Op("×", B)),
    ("times", Am::Op("×", B)),
    ("|><", Am::Op("⋉", B)),
    ("ltimes", Am::Op("⋉", B)),
    ("><|", Am::Op("⋊", B)),
    ("rtimes", Am::Op("⋊", B)),
    ("|><|", Am::Op("⋈", B)),
    ("bowtie", Am::Op("⋈", B)),
    ("-:", Am::Op("÷", B)),
    ("div", Am::Op("÷", B)),
    ("divide", Am::Def("-:")),
    ("@", Am::Op("∘", B)),
    ("circ", Am::Op("∘", B)),
    ("o+", Am::Op("⊕", B)),
    ("oplus", Am::Op("⊕", B)),
    ("o-", Am::Op("⊖", B)),
    ("ominus", Am::Op("⊖", B)),
    ("ox", Am::Op("⊗", B)),
    ("otimes", Am::Op("⊗", B)),
    ("o.", Am::Op("⊙", B)),
    ("odot", Am::Op("⊙", B)),
    ("sum", Am::Large("∑")),
    ("prod", Am::Large("∏")),
    ("^^", Am::Op("∧", B)),
    ("wedge", Am::Op("∧", B)),
    ("^^^", Am::Large("⋀")),
    ("bigwedge", Am::Large("⋀")),
    ("vv", Am::Op("∨", B)),
    ("vee", Am::Op("∨", B)),
    ("vvv", Am::Large("⋁")),
    ("bigvee", Am::Large("⋁")),
    ("nn", Am::Op("∩", B)),
    ("cap", Am::Op("∩", B)),
    ("nnn", Am::Large("⋂")),
    ("bigcap", Am::Large("⋂")),
    ("uu", Am::Op("∪", B)),
    ("cup", Am::Op("∪", B)),
    ("uuu", Am::Large("⋃")),
    ("bigcup", Am::Large("⋃")),
    ("dag", Am::Op("†", B)),
    ("dagger", Am::Op("†", B)),
    ("ddag", Am::Op("‡", B)),
    ("ddagger", Am::Op("‡", B)),
    // relation symbols
    ("!=", Am::Op("≠", R)),
    ("ne", Am::Op("≠", R)),
    (":=", Am::Op(":=", R)),
    ("lt", Am::Op("<", R)),
    ("<=", Am::Op("≤", R)),
    ("le", Am::Op("≤", R)),
    ("lt=", Am::Op("≤", R)),
    ("leq", Am::Op("≤", R)),
    ("gt", Am::Op(">", R)),
    ("mlt", Am::Op("≪", R)),
    ("ll", Am::Op("≪", R)),
    (">=", Am::Op("≥", R)),
    ("ge", Am::Op("≥", R)),
    ("gt=", Am::Op("≥", R)),
    ("geq", Am::Op("≥", R)),
    ("mgt", Am::Op("≫", R)),
    ("gg", Am::Op("≫", R)),
    ("-<", Am::Op("≺", R)),
    ("prec", Am::Op("≺", R)),
    ("-lt", Am::Op("≺", R)),
    (">-", Am::Op("≻", R)),
    ("succ", Am::Op("≻", R)),
    ("-<=", Am::Op("⪯", R)),
    ("preceq", Am::Op("⪯", R)),
    (">-=", Am::Op("⪰", R)),
    ("succeq", Am::Op("⪰", R)),
    ("in", Am::Op("∈", R)),
    ("!in", Am::Op("∉", R)),
    ("notin", Am::Op("∉", R)),
    ("sub", Am::Op("⊂", R)),
    ("subset", Am::Op("⊂", R)),
    ("!sub", Am::Op("⊄", R)),
    ("notsubset", Am::Def("!sub")),
    ("sup", Am::Op("⊃", R)),
    ("supset", Am::Op("⊃", R)),
    ("!sup", Am::Op("⊅", R)),
    ("notsupset", Am::Def("!sup")),
    ("sube", Am::Op("⊆", R)),
    ("subseteq", Am::Op("⊆", R)),
    ("!sube", Am::Op("⊈", R)),
    ("notsubseteq", Am::Def("!sube")),
    ("supe", Am::Op("⊇", R)),
    ("supseteq", Am::Op("⊇", R)),
    ("!supe", Am::Op("⊉", R)),
    ("notsupseteq", Am::Def("!supe")),
    ("-=", Am::Op("≡", R)),
    ("equiv", Am::Op("≡", R)),
    ("!-=", Am::Op("≢", R)),
    ("notequiv", Am::Def("!-=")),
    ("~=", Am::Op("≅", R)),
    ("cong", Am::Op("≅", R)),
    ("~~", Am::Op("≈", R)),
    ("approx", Am::Op("≈", R)),
    ("~", Am::Op("∼", R)),
    ("sim", Am::Op("∼", R)),
    ("prop", Am::Op("∝", R)),
    ("propto", Am::Op("∝", R)),
    // logical symbols
    ("and", Am::Word("and")),
    ("or", Am::Word("or")),
    ("not", Am::Op("¬", P)),
    ("neg", Am::Op("¬", P)),
    ("=>", Am::Op("⇒", R)),
    ("implies", Am::Op("⇒", R)),
    ("if", Am::Word("if")),
    ("<=>", Am::Op("⇔", R)),
    ("iff", Am::Op("⇔", R)),
    ("AA", Am::Op("∀", P)),
    ("forall", Am::Op("∀", P)),
    ("EE", Am::Op("∃", P)),
    ("exists", Am::Op("∃", P)),
    ("_|_", Am::Ident("⊥")),
    ("bot", Am::Ident("⊥")),
    ("TT", Am::Ident("⊤")),
    ("top", Am::Ident("⊤")),
    ("|--", Am::Op("⊢", R)),
    ("vdash", Am::Op("⊢", R)),
    ("|==", Am::Op("⊨", R)),
    ("models", Am::Op("⊨", R)),
    // grouping brackets
    ("(", Am::Left("(")),
    (")", Am::Right(")")),
    ("[", Am::Left("[")),
    ("]", Am::Right("]")),
    ("{", Am::Left("{")),
    ("}", Am::Right("}")),
    ("|", Am::LeftRight("|")),
    (":|:", Am::Op("|", R)),
    ("|:", Am::Left("|")),
    (":|", Am::Right("|")),
    ("(:", Am::Left("⟨")),
    (":)", Am::Right("⟩")),
    ("<<", Am::Left("⟨")),
    (">>", Am::Right("⟩")),
    ("langle", Am::Left("⟨")),
    ("rangle", Am::Right("⟩")),
    ("{:", Am::Left("")),
    (":}", Am::Right("")),
    // miscellaneous symbols
    ("int", Am::Op("∫", L)),
    ("dx", Am::Def("{:d x:}")),
    ("dy", Am::Def("{:d y:}")),
    ("dz", Am::Def("{:d z:}")),
    ("dt", Am::Def("{:d t:}")),
    ("oint", Am::Op("∮", L)),
    ("del", Am::Ident("∂")),
    ("partial", Am::Ident("∂")),
    ("grad", Am::Ident("∇")),
    ("nabla", Am::Ident("∇")),
    ("+-", Am::Op("±", B)),
    ("pm", Am::Op("±", B)),
    ("-+", Am::Op("∓", B)),
    ("mp", Am::Op("∓", B)),
    ("O/", Am::Ident("∅")),
    ("emptyset", Am::Ident("∅")),
    ("oo", Am::Ident("∞")),
    ("infty", Am::Ident("∞")),
    ("aleph", Am::Ident("ℵ")),
    ("...", Am::Op("…", U)),
    ("ldots", Am::Op("…", U)),
    (":.", Am::Op("∴", R)),
    ("therefore", Am::Op("∴", R)),
    (":'", Am::Op("∵", R)),
    ("because", Am::Op("∵", R)),
    ("/_", Am::Ident("∠")),
    ("angle", Am::Ident("∠")),
    ("/_\\", Am::Ident("△")),
    ("triangle", Am::Ident("△")),
    ("'", Am::Op("′", X)),
    ("tilde", Am::Accent(AK::Tilde)),
    ("\\ ", Am::Space("0.333em")),
    ("frown", Am::Op("⌢", R)),
    ("quad", Am::Space("1em")),
    ("qquad", Am::Space("2em")),
    ("enspace", Am::Space("0.5em")),
    ("thinspace", Am::Space("0.167em")),
    ("mspace", Am::Space("0.278em")),
    ("cdots", Am::Op("⋯", U)),
    ("vdots", Am::Op("⋮", U)),
    ("ddots", Am::Op("⋱", U)),
    ("diamond", Am::Op("⋄", B)),
    ("square", Am::Ident("□")),
    ("|__", Am::Op("⌊", OpClass::Open)),
    ("lfloor", Am::Op("⌊", OpClass::Open)),
    ("__|", Am::Op("⌋", OpClass::Close)),
    ("rfloor", Am::Op("⌋", OpClass::Close)),
    ("|~", Am::Op("⌈", OpClass::Open)),
    ("lceiling", Am::Op("⌈", OpClass::Open)),
    ("lceil", Am::Op("⌈", OpClass::Open)),
    ("~|", Am::Op("⌉", OpClass::Close)),
    ("rceiling", Am::Op("⌉", OpClass::Close)),
    ("rceil", Am::Op("⌉", OpClass::Close)),
    ("CC", Am::Ident("ℂ")),
    ("NN", Am::Ident("ℕ")),
    ("QQ", Am::Ident("ℚ")),
    ("RR", Am::Ident("ℝ")),
    ("ZZ", Am::Ident("ℤ")),
    ("f", Am::FuncIdent("f")),
    ("g", Am::FuncIdent("g")),
    ("hbar", Am::Ident("ℏ")),
    // standard functions
    ("lim", Am::LimFunc("lim")),
    ("Lim", Am::LimFunc("Lim")),
    ("sin", Am::Func("sin")),
    ("cos", Am::Func("cos")),
    ("tan", Am::Func("tan")),
    ("sinh", Am::Func("sinh")),
    ("cosh", Am::Func("cosh")),
    ("tanh", Am::Func("tanh")),
    ("cot", Am::Func("cot")),
    ("sec", Am::Func("sec")),
    ("csc", Am::Func("csc")),
    ("arcsin", Am::Func("arcsin")),
    ("arccos", Am::Func("arccos")),
    ("arctan", Am::Func("arctan")),
    ("arcsec", Am::Func("arcsec")),
    ("arccsc", Am::Func("arccsc")),
    ("arccot", Am::Func("arccot")),
    ("coth", Am::Func("coth")),
    ("sech", Am::Func("sech")),
    ("csch", Am::Func("csch")),
    ("exp", Am::Func("exp")),
    ("abs", Am::FenceFn("|", "|")),
    ("Abs", Am::FenceFn("|", "|")),
    ("norm", Am::FenceFn("‖", "‖")),
    ("floor", Am::FenceFn("⌊", "⌋")),
    ("ceil", Am::FenceFn("⌈", "⌉")),
    ("log", Am::Func("log")),
    ("ln", Am::Func("ln")),
    ("det", Am::Func("det")),
    ("dim", Am::Func("dim")),
    ("mod", Am::Func("mod")),
    ("gcd", Am::Func("gcd")),
    ("lcm", Am::Func("lcm")),
    ("lub", Am::Func("lub")),
    ("glb", Am::Func("glb")),
    ("min", Am::LimFunc("min")),
    ("max", Am::LimFunc("max")),
    ("Sin", Am::Func("Sin")),
    ("Cos", Am::Func("Cos")),
    ("Tan", Am::Func("Tan")),
    ("Arcsin", Am::Func("Arcsin")),
    ("Arccos", Am::Func("Arccos")),
    ("Arctan", Am::Func("Arctan")),
    ("Sinh", Am::Func("Sinh")),
    ("Cosh", Am::Func("Cosh")),
    ("Tanh", Am::Func("Tanh")),
    ("Cot", Am::Func("Cot")),
    ("Sec", Am::Func("Sec")),
    ("Csc", Am::Func("Csc")),
    ("Log", Am::Func("Log")),
    ("Ln", Am::Func("Ln")),
    // arrows
    ("uarr", Am::Op("↑", R)),
    ("uparrow", Am::Op("↑", R)),
    ("darr", Am::Op("↓", R)),
    ("downarrow", Am::Op("↓", R)),
    ("rarr", Am::Op("→", R)),
    ("rightarrow", Am::Op("→", R)),
    ("->", Am::Op("→", R)),
    ("to", Am::Op("→", R)),
    (">->", Am::Op("↣", R)),
    ("rightarrowtail", Am::Op("↣", R)),
    ("->>", Am::Op("↠", R)),
    ("twoheadrightarrow", Am::Op("↠", R)),
    (">->>", Am::Op("⤖", R)),
    ("twoheadrightarrowtail", Am::Op("⤖", R)),
    ("|->", Am::Op("↦", R)),
    ("mapsto", Am::Op("↦", R)),
    ("larr", Am::Op("←", R)),
    ("leftarrow", Am::Op("←", R)),
    ("harr", Am::Op("↔", R)),
    ("leftrightarrow", Am::Op("↔", R)),
    ("rArr", Am::Op("⇒", R)),
    ("Rightarrow", Am::Op("⇒", R)),
    ("lArr", Am::Op("⇐", R)),
    ("Leftarrow", Am::Op("⇐", R)),
    ("dArr", Am::Op("⇓", R)),
    ("hArr", Am::Op("⇔", R)),
    ("Leftrightarrow", Am::Op("⇔", R)),
    ("rightleftharpoons", Am::Op("⇌", R)),
    // commands with argument
    ("sqrt", Am::Sqrt),
    ("root", Am::Root),
    ("frac", Am::Frac),
    ("/", Am::Slash),
    ("stackrel", Am::Over),
    ("overset", Am::Over),
    ("underset", Am::Under),
    ("_", Am::Sub),
    ("^", Am::Sup),
    ("hat", Am::Accent(AK::Hat)),
    ("bar", Am::Accent(AK::Overline)),
    ("overline", Am::Accent(AK::Overline)),
    ("vec", Am::Accent(AK::Vector)),
    ("dot", Am::Accent(AK::Dot)),
    ("ddot", Am::Accent(AK::DoubleDot)),
    ("overarc", Am::Accent(AK::Arc)),
    ("overparen", Am::Accent(AK::Arc)),
    ("ul", Am::Accent(AK::Underline)),
    ("underline", Am::Accent(AK::Underline)),
    ("ubrace", Am::Accent(AK::Underbrace)),
    ("underbrace", Am::Accent(AK::Underbrace)),
    ("obrace", Am::Accent(AK::Overbrace)),
    ("overbrace", Am::Accent(AK::Overbrace)),
    ("text", Am::Text),
    ("mbox", Am::Text),
    ("color", Am::Skip2),
    ("id", Am::Skip2),
    ("class", Am::Skip2),
    ("cancel", Am::Cancel),
    ("\"", Am::Quote),
    // fonts
    ("bb", Am::Font(Variant::Bold)),
    ("mathbf", Am::Font(Variant::Bold)),
    ("sf", Am::Font(Variant::SansSerif)),
    ("mathsf", Am::Font(Variant::SansSerif)),
    ("sfit", Am::Font(Variant::SansSerifItalic)),
    ("bbsf", Am::Font(Variant::BoldSansSerif)),
    ("bbb", Am::Font(Variant::DoubleStruck)),
    ("mathbb", Am::Font(Variant::DoubleStruck)),
    ("cc", Am::Font(Variant::Script)),
    ("mathcal", Am::Font(Variant::Script)),
    ("bbcc", Am::Font(Variant::BoldScript)),
    ("tt", Am::Font(Variant::Monospace)),
    ("mathtt", Am::Font(Variant::Monospace)),
    ("fr", Am::Font(Variant::Fraktur)),
    ("mathfrak", Am::Font(Variant::Fraktur)),
    ("bbfr", Am::Font(Variant::BoldFraktur)),
    ("bbit", Am::Font(Variant::BoldItalic)),
    ("bbsfit", Am::Font(Variant::SansSerifBoldItalic)),
    ("bold", Am::Font(Variant::Bold)),
    ("italic", Am::Font(Variant::Italic)),
];

/// Parses ASCIIMath (without its backtick delimiters).
///
/// Never fails and never panics; see [`Math::diagnostics`].
pub fn parse_asciimath(src: &str) -> Math {
    let chars: Vec<char> = src.chars().collect();
    let len = chars.len();
    let mut p = Parser {
        s: chars,
        i: 0,
        depth: 0,
        diags: Vec::new(),
    };
    let items = p.parse_expr(false);
    Math {
        notation: Notation::AsciiMath,
        source: src.to_owned(),
        root: row(items, CharRange::new(0, len)),
        diagnostics: p.diags,
    }
}

#[derive(Clone, Debug)]
enum Kind {
    Sym(Am),
    Number,
    Char(char),
}

#[derive(Clone, Debug)]
struct Tok {
    kind: Kind,
    start: usize,
    end: usize,
}

struct Parser {
    s: Vec<char>,
    i: usize,
    depth: usize,
    diags: Vec<Diagnostic>,
}

/// The longest symbol whose input starts at `j` in `s`.
fn longest_symbol(s: &[char], j: usize) -> Option<(Am, usize)> {
    let mut best: Option<(Am, usize)> = None;
    for (input, am) in SYMBOLS {
        let n = input.chars().count();
        if n == 0 || j + n > s.len() || best.is_some_and(|(_, m)| m >= n) {
            continue;
        }
        if input.chars().zip(&s[j..j + n]).all(|(a, b)| a == *b) {
            best = Some((*am, n));
        }
    }
    best
}

impl Parser {
    fn diag(&mut self, message: impl Into<String>, a: usize, b: usize) {
        self.diags.push(Diagnostic {
            message: message.into(),
            span: CharRange::new(a, b),
        });
    }

    fn peek(&self) -> Option<Tok> {
        let mut j = self.i;
        while j < self.s.len() && self.s[j].is_whitespace() {
            j += 1;
        }
        let c = *self.s.get(j)?;
        if c.is_ascii_digit() {
            let mut k = j;
            while k < self.s.len() && self.s[k].is_ascii_digit() {
                k += 1;
            }
            if k + 1 < self.s.len() && self.s[k] == '.' && self.s[k + 1].is_ascii_digit() {
                k += 1;
                while k < self.s.len() && self.s[k].is_ascii_digit() {
                    k += 1;
                }
            }
            return Some(Tok {
                kind: Kind::Number,
                start: j,
                end: k,
            });
        }
        if let Some((am, n)) = longest_symbol(&self.s, j) {
            return Some(Tok {
                kind: Kind::Sym(am),
                start: j,
                end: j + n,
            });
        }
        Some(Tok {
            kind: Kind::Char(c),
            start: j,
            end: j + 1,
        })
    }

    fn text(&self, a: usize, b: usize) -> String {
        self.s[a..b].iter().collect()
    }

    /// E: a sequence of intermediate expressions and `I/I` fractions.
    fn parse_expr(&mut self, in_bracket: bool) -> Vec<Node> {
        let mut items: Vec<Node> = Vec::new();
        while let Some(t) = self.peek() {
            if let Kind::Sym(Am::Right(text)) = t.kind {
                if in_bracket {
                    break;
                }
                self.i = t.end;
                self.diag("unmatched closing bracket", t.start, t.end);
                if !text.is_empty() {
                    items.push(Node::new(
                        NodeKind::Operator {
                            text: text.into(),
                            class: OpClass::Close,
                        },
                        CharRange::new(t.start, t.end),
                    ));
                }
                continue;
            }
            let before = self.i;
            let Some(mut node) = self.parse_intermediate() else {
                if self.i == before {
                    self.i = t.end;
                }
                continue;
            };
            if let Some(t2) = self.peek()
                && matches!(t2.kind, Kind::Sym(Am::Slash))
            {
                self.i = t2.end;
                let den = self
                    .parse_intermediate()
                    .unwrap_or_else(|| Node::empty(t2.end));
                let span = node.span.cover(den.span);
                node = Node::new(
                    NodeKind::Fraction {
                        num: Box::new(strip(node)),
                        den: Box::new(strip(den)),
                        bar: true,
                    },
                    span,
                );
            }
            items.push(node);
        }
        group_fences(items)
    }

    /// I: a simple expression with optional `_` and `^`.
    fn parse_intermediate(&mut self) -> Option<Node> {
        let base = self.parse_simple()?;
        let mut sub = None;
        let mut sup = None;
        if let Some(t) = self.peek()
            && matches!(t.kind, Kind::Sym(Am::Sub))
        {
            self.i = t.end;
            sub = Some(strip(self.script_arg(t.end)));
        }
        if let Some(t) = self.peek()
            && matches!(t.kind, Kind::Sym(Am::Sup))
        {
            self.i = t.end;
            sup = Some(strip(self.script_arg(t.end)));
        }
        if sub.is_none() && sup.is_none() {
            return Some(base);
        }
        let limits = match &base.kind {
            NodeKind::Operator {
                text,
                class: OpClass::Large,
            } => !matches!(text.as_str(), "∫" | "∮"),
            NodeKind::Function(f) => takes_limits(f),
            NodeKind::Accent {
                accent: AccentKind::Overbrace | AccentKind::Underbrace,
                ..
            } => true,
            _ => false,
        };
        let mut span = base.span;
        for s in [&sub, &sup].into_iter().flatten() {
            span = span.cover(s.span);
        }
        Some(Node::new(
            NodeKind::Scripts {
                base: Box::new(base),
                sub: sub.map(Box::new),
                sup: sup.map(Box::new),
                limits,
            },
            span,
        ))
    }

    /// A script argument; a leading minus joins the following simple
    /// expression (`x^-1`, `sin^-1`).
    fn script_arg(&mut self, at: usize) -> Node {
        if let Some(t) = self.peek()
            && matches!(t.kind, Kind::Char('-'))
        {
            self.i = t.end;
            let minus = Node::new(
                NodeKind::Operator {
                    text: "−".into(),
                    class: OpClass::Binary,
                },
                CharRange::new(t.start, t.end),
            );
            let Some(arg) = self.parse_simple() else {
                return minus;
            };
            let span = minus.span.cover(arg.span);
            return row(vec![minus, arg], span);
        }
        match self.parse_simple() {
            Some(n) => n,
            None => {
                self.diag("missing script", at, at);
                Node::empty(at)
            }
        }
    }

    fn arg(&mut self, what: &str, at: usize) -> Node {
        match self.parse_simple() {
            Some(n) => strip(n),
            None => {
                self.diag(format!("missing {what}"), at, at);
                Node::empty(at)
            }
        }
    }

    /// S: one simple expression.
    fn parse_simple(&mut self) -> Option<Node> {
        let t = self.peek()?;
        if matches!(t.kind, Kind::Sym(Am::Right(_))) {
            return None;
        }
        if self.depth >= MAX_DEPTH {
            self.i = t.end;
            self.diag("nested too deeply", t.start, t.end);
            return Some(Node::new(
                NodeKind::Error(self.text(t.start, t.end)),
                CharRange::new(t.start, t.end),
            ));
        }
        self.depth += 1;
        let r = self.simple_inner(t);
        self.depth -= 1;
        Some(r)
    }

    fn simple_inner(&mut self, t: Tok) -> Node {
        let span = CharRange::new(t.start, t.end);
        self.i = t.end;
        match t.kind {
            Kind::Number => Node::new(NodeKind::Number(self.text(t.start, t.end)), span),
            Kind::Char(c) => token_for_char(c, span),
            Kind::Sym(am) => self.symbol(am, t.start, t.end),
        }
    }

    fn symbol(&mut self, am: Am, start: usize, end: usize) -> Node {
        let span = CharRange::new(start, end);
        let from = |p: &Parser| CharRange::new(start, p.i);
        match am {
            Am::Ident(s) => Node::new(NodeKind::Ident(s.into()), span),
            Am::Op(s, class) => Node::new(
                NodeKind::Operator {
                    text: s.into(),
                    class,
                },
                span,
            ),
            Am::Large(s) => Node::new(
                NodeKind::Operator {
                    text: s.into(),
                    class: OpClass::Large,
                },
                span,
            ),
            Am::LimFunc(s) => Node::new(NodeKind::Function(s.into()), span),
            Am::Func(name) | Am::FuncIdent(name) => {
                let head = if matches!(am, Am::Func(_)) {
                    Node::new(NodeKind::Function(name.into()), span)
                } else {
                    Node::new(NodeKind::Ident(name.into()), span)
                };
                let takes_arg = match self.peek() {
                    None => false,
                    Some(n) => !matches!(
                        n.kind,
                        Kind::Sym(
                            Am::Sub
                                | Am::Sup
                                | Am::Slash
                                | Am::Right(_)
                                | Am::LeftRight(_)
                                | Am::Op(_, B | R | U)
                        ) | Kind::Char(',' | '+' | '-' | '=' | '<' | '>')
                    ),
                };
                if !takes_arg {
                    return head;
                }
                match self.parse_simple() {
                    Some(arg) => {
                        let s = head.span.cover(arg.span);
                        row(vec![head, arg], s)
                    }
                    None => head,
                }
            }
            Am::FenceFn(open, close) => {
                let body = self.arg("argument", end);
                Node::new(
                    NodeKind::Fenced {
                        open: open.into(),
                        close: close.into(),
                        body: Box::new(body),
                    },
                    from(self),
                )
            }
            Am::Sqrt => {
                let rad = self.arg("radicand", end);
                Node::new(
                    NodeKind::Root {
                        index: None,
                        radicand: Box::new(rad),
                    },
                    from(self),
                )
            }
            Am::Root => {
                let index = self.arg("root index", end);
                let rad = self.arg("radicand", self.i);
                Node::new(
                    NodeKind::Root {
                        index: Some(Box::new(index)),
                        radicand: Box::new(rad),
                    },
                    from(self),
                )
            }
            Am::Frac => {
                let num = self.arg("numerator", end);
                let den = self.arg("denominator", self.i);
                Node::new(
                    NodeKind::Fraction {
                        num: Box::new(num),
                        den: Box::new(den),
                        bar: true,
                    },
                    from(self),
                )
            }
            Am::Over | Am::Under => {
                let script = self.arg("script", end);
                let base = self.arg("base", self.i);
                let (sub, sup) = if am == Am::Under {
                    (Some(Box::new(script)), None)
                } else {
                    (None, Some(Box::new(script)))
                };
                Node::new(
                    NodeKind::Scripts {
                        base: Box::new(base),
                        sub,
                        sup,
                        limits: true,
                    },
                    from(self),
                )
            }
            Am::Skip2 => {
                let _ = self.raw_bracketed();
                self.arg("argument", self.i)
            }
            Am::Accent(accent) => {
                let base = self.arg("accented part", end);
                Node::new(
                    NodeKind::Accent {
                        base: Box::new(base),
                        accent,
                    },
                    from(self),
                )
            }
            Am::Cancel => {
                let body = self.arg("crossed-out part", end);
                Node::new(
                    NodeKind::Enclose {
                        notation: Enclosure::Cancel,
                        body: Box::new(body),
                    },
                    from(self),
                )
            }
            Am::Font(variant) => {
                let body = self.arg("styled part", end);
                if variant == Variant::DoubleStruck
                    && let NodeKind::Ident(s) = &body.kind
                    && let Some(set) = double_struck(s)
                {
                    return Node::new(NodeKind::Ident(set.into()), from(self));
                }
                Node::new(
                    NodeKind::Style {
                        variant,
                        body: Box::new(body),
                    },
                    from(self),
                )
            }
            Am::Text => {
                let text = self.raw_bracketed().unwrap_or_default();
                Node::new(NodeKind::Text(text), from(self))
            }
            Am::Quote => {
                let close = self.s[end..].iter().position(|&c| c == '"');
                let text = match close {
                    Some(k) => {
                        self.i = end + k + 1;
                        self.text(end, end + k)
                    }
                    None => {
                        self.i = self.s.len();
                        self.diag("missing closing quote", start, self.i);
                        self.text(end, self.s.len())
                    }
                };
                Node::new(NodeKind::Text(text), from(self))
            }
            Am::Word(w) => Node::new(NodeKind::Text(w.into()), span),
            Am::Space(w) => Node::new(NodeKind::Space(w.into()), span),
            Am::Def(target) => self.definition(target, start, end),
            Am::Left(open) => self.bracket(open, start),
            Am::Right(close) => Node::new(
                NodeKind::Operator {
                    text: close.into(),
                    class: OpClass::Close,
                },
                span,
            ),
            Am::LeftRight(s) => Node::new(
                NodeKind::Operator {
                    text: s.into(),
                    class: F,
                },
                span,
            ),
            Am::Sub => op_node("_", span),
            Am::Sup => op_node("^", span),
            Am::Slash => op_node("/", span),
        }
    }

    /// A definition such as `dx` (`{:d x:}`) or `divide` (`-:`).
    fn definition(&mut self, target: &str, start: usize, end: usize) -> Node {
        let span = CharRange::new(start, end);
        if let Some(inner) = target.strip_prefix("{:").and_then(|t| t.strip_suffix(":}")) {
            // `dx`: the letters in order, each with its own char.
            let letters: Vec<char> = inner.chars().filter(|c| !c.is_whitespace()).collect();
            let items: Vec<Node> = letters
                .iter()
                .enumerate()
                .map(|(k, c)| {
                    let a = (start + k).min(end.saturating_sub(1));
                    Node::new(NodeKind::Ident(c.to_string()), CharRange::new(a, a + 1))
                })
                .collect();
            return row(items, span);
        }
        match SYMBOLS.iter().find(|(input, _)| *input == target) {
            Some((_, Am::Def(_))) | None => Node::new(NodeKind::Error(target.into()), span),
            Some((_, am)) => self.symbol(*am, start, end),
        }
    }

    /// A bracketed expression; a list of bracketed rows becomes a matrix.
    fn bracket(&mut self, open: &str, start: usize) -> Node {
        let body = self.parse_expr(true);
        let close = match self.peek() {
            Some(Tok {
                kind: Kind::Sym(Am::Right(c)),
                end,
                ..
            }) => {
                self.i = end;
                c
            }
            _ => {
                self.diag("missing closing bracket", start, self.i);
                ""
            }
        };
        let span = CharRange::new(start, self.i);
        if let Some(rows) = matrix_rows(&body) {
            let kind = if open == "{" && close.is_empty() {
                TableKind::Cases
            } else {
                TableKind::Matrix
            };
            let tspan = items_span(&body).unwrap_or(span);
            let table = Node::new(NodeKind::Table { kind, rows }, tspan);
            return Node::new(
                NodeKind::Fenced {
                    open: open.into(),
                    close: close.into(),
                    body: Box::new(table),
                },
                span,
            );
        }
        let bspan = items_span(&body).unwrap_or(CharRange::empty(start + 1));
        Node::new(
            NodeKind::Fenced {
                open: open.into(),
                close: close.into(),
                body: Box::new(row(body, bspan)),
            },
            span,
        )
    }

    /// The raw text inside the bracket that follows (for `text(…)`).
    fn raw_bracketed(&mut self) -> Option<String> {
        let mut j = self.i;
        while j < self.s.len() && self.s[j].is_whitespace() {
            j += 1;
        }
        let open = *self.s.get(j)?;
        let close = match open {
            '(' => ')',
            '[' => ']',
            '{' => '}',
            _ => return None,
        };
        let k = self.s[j + 1..].iter().position(|&c| c == close);
        match k {
            Some(k) => {
                self.i = j + 1 + k + 1;
                Some(self.text(j + 1, j + 1 + k))
            }
            None => {
                self.i = self.s.len();
                self.diag("missing closing bracket", j, self.i);
                Some(self.text(j + 1, self.s.len()))
            }
        }
    }
}

fn op_node(text: &str, span: CharRange) -> Node {
    Node::new(
        NodeKind::Operator {
            text: text.into(),
            class: Q,
        },
        span,
    )
}

/// Removes the brackets ASCIIMath drops around arguments: `(x+1)/2`,
/// `sqrt(x)`, `x^(n+1)`. The node keeps the brackets' span.
fn strip(n: Node) -> Node {
    match n.kind {
        NodeKind::Fenced { open, close, body }
            if matches!(open.as_str(), "(" | "[" | "{" | "")
                && matches!(close.as_str(), ")" | "]" | "}" | "")
                && !matches!(body.kind, NodeKind::Table { .. }) =>
        {
            let mut body = *body;
            if let NodeKind::Row(items) = &mut body.kind
                && items.len() == 1
            {
                let mut one = items.remove(0);
                one.span = n.span;
                return one;
            }
            body.span = n.span;
            body
        }
        kind => Node::new(kind, n.span),
    }
}

/// `[(a,b),(c,d)]`: two or more bracketed rows, separated by commas, with
/// equal numbers of comma-separated cells.
fn matrix_rows(items: &[Node]) -> Option<Vec<Vec<Node>>> {
    if items.len() < 3 {
        return None;
    }
    let mut rows = Vec::new();
    for (k, it) in items.iter().enumerate() {
        if k % 2 == 1 {
            if !it.is_op(",") {
                return None;
            }
            continue;
        }
        let NodeKind::Fenced { open, close, body } = &it.kind else {
            return None;
        };
        if !matches!(open.as_str(), "(" | "[") || !matches!(close.as_str(), ")" | "]") {
            return None;
        }
        let cells_src: Vec<Node> = match &body.kind {
            NodeKind::Row(v) => v.clone(),
            _ => vec![(**body).clone()],
        };
        let mut cells: Vec<Node> = Vec::new();
        let mut cur: Vec<Node> = Vec::new();
        let mut cur_start = body.span.start.0;
        for c in cells_src {
            if c.is_op(",") {
                cells.push(cell(std::mem::take(&mut cur), cur_start));
                cur_start = c.span.end.0;
            } else {
                cur.push(c);
            }
        }
        cells.push(cell(cur, cur_start));
        rows.push(cells);
    }
    if items.len().is_multiple_of(2) {
        return None;
    }
    let cols = rows.first()?.len();
    if rows.len() < 2 || rows.iter().any(|r| r.len() != cols) {
        return None;
    }
    Some(rows)
}

fn cell(mut items: Vec<Node>, at: usize) -> Node {
    if items.len() == 1 {
        return items.remove(0);
    }
    let span = items_span(&items).unwrap_or(CharRange::empty(at));
    row(items, span)
}

#[cfg(test)]
#[path = "asciimath_tests.rs"]
mod tests;
