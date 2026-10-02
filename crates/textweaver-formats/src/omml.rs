//! Office Math (OMML, the `m:` elements of a Word document) to LaTeX.
//!
//! Word stores equations as `m:oMath` trees. The DOCX loader turns each one
//! into LaTeX with its delimiters (`$…$`, or `$$…$$` for display math)
//! under a `Math` marker, as the Markdown loader does, so speech reads it as
//! math rather than as a run of letters.
//!
//! Covered: runs (`m:r`, letters, digits, and operators, with Unicode math
//! symbols mapped to commands such as `\alpha`, `\leq`, and `\to`; plain
//! text runs as `\text{…}`), fractions (`m:f`), scripts (`m:sSup`,
//! `m:sSub`, `m:sSubSup`, `m:sPre`), radicals (`m:rad`), n-ary operators
//! (`m:nary`), delimiters (`m:d`), matrices (`m:m`), functions (`m:func`),
//! accents (`m:acc`), bars (`m:bar`), limits (`m:limLow`, `m:limUpp`),
//! boxes (`m:box`, `m:borderBox`), grouping characters (`m:groupChr`),
//! equation arrays (`m:eqArr`), and phantoms (`m:phant`). Any other element
//! reads as the LaTeX of its children, so no text is lost.

use roxmltree::Node;

use crate::package::child;

/// The OMML namespace.
const MATH_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/math";

/// The LaTeX of an OMML element (usually `m:oMath`), without delimiters.
/// Braces in the result balance.
pub(crate) fn to_latex(node: Node<'_, '_>) -> String {
    let mut c = Conv {
        depth: 0,
        aligned: false,
    };
    let raw = c.node(node);
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

struct Conv {
    /// Nesting of `node` calls, bounded by [`MAX_NESTING`](crate::MAX_NESTING).
    depth: usize,
    /// Inside an equation array, where `&` in a run marks an alignment
    /// point.
    aligned: bool,
}

/// The value of attribute `local` (any namespace).
fn attr<'a>(n: Node<'a, '_>, local: &str) -> Option<&'a str> {
    n.attributes()
        .find(|a| a.name() == local)
        .map(|a| a.value())
}

/// `m:val` of property `name` in properties `pr`.
fn prop<'a>(pr: Option<Node<'a, '_>>, name: &str) -> Option<&'a str> {
    pr.and_then(|p| child(p, name)).and_then(|c| attr(c, "val"))
}

/// A toggle property (`<m:degHide/>`, `<m:degHide m:val="1"/>`): on when
/// present, unless its value turns it off.
fn toggle(pr: Option<Node<'_, '_>>, name: &str) -> bool {
    pr.and_then(|p| child(p, name))
        .is_some_and(|c| !matches!(attr(c, "val"), Some("0" | "false" | "off" | "none")))
}

/// LaTeX built piece by piece, kept apart where a piece would run into
/// the command before it (`\alpha` then `x` gives `\alpha x`).
///
/// It remembers how its text ends, so each piece costs its own length.
/// Scanning back from the end instead (W8b-ml) cost the whole run of
/// letters before it on every piece: a 512 KB token read letter by letter
/// was quadratic, and took minutes.
#[derive(Debug, Default)]
pub(crate) struct Tex {
    s: String,
    /// The text ends with ASCII letters.
    in_word: bool,
    /// Those letters follow an odd number of backslashes: a control word.
    command: bool,
    /// The text ends with an odd number of backslashes.
    odd_slashes: bool,
}

impl Tex {
    /// Empty LaTeX.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Appends `piece`, with a space first when it starts with a letter
    /// and the text ends with a control word.
    pub(crate) fn push(&mut self, piece: &str) {
        if self.in_word && self.command && piece.starts_with(|c: char| c.is_ascii_alphabetic()) {
            self.push_raw(" ");
        }
        self.push_raw(piece);
    }

    /// Appends `piece` as it is, never adding a space.
    pub(crate) fn push_raw(&mut self, piece: &str) {
        self.s.push_str(piece);
        for c in piece.chars() {
            if c.is_ascii_alphabetic() {
                if !self.in_word {
                    self.in_word = true;
                    self.command = self.odd_slashes;
                }
                self.odd_slashes = false;
            } else {
                self.in_word = false;
                self.odd_slashes = c == '\\' && !self.odd_slashes;
            }
        }
    }

    /// The LaTeX so far.
    pub(crate) fn into_string(self) -> String {
        self.s
    }
}

/// Appends `piece`, with a space when it would otherwise run into the
/// command `out` ends with: the reference [`Tex::push`] is tested against.
#[cfg(test)]
pub(crate) fn push(out: &mut String, piece: &str) {
    if piece.starts_with(|c: char| c.is_ascii_alphabetic()) && ends_with_command(out) {
        out.push(' ');
    }
    out.push_str(piece);
}

/// Whether `s` ends with a control word (`\alpha`, but not the row break
/// `\\` followed by letters).
#[cfg(test)]
fn ends_with_command(s: &str) -> bool {
    let letters = s
        .chars()
        .rev()
        .take_while(char::is_ascii_alphabetic)
        .count();
    if letters == 0 {
        return false;
    }
    let before = &s[..s.len() - letters];
    let slashes = before.chars().rev().take_while(|&c| c == '\\').count();
    slashes % 2 == 1
}

/// Function names LaTeX has a command for.
const FUNCTIONS: &[&str] = &[
    "arccos", "arcsin", "arctan", "arg", "cos", "cosh", "cot", "coth", "csc", "deg", "det", "dim",
    "exp", "gcd", "hom", "inf", "ker", "lg", "lim", "liminf", "limsup", "ln", "log", "max", "min",
    "Pr", "sec", "sin", "sinh", "sup", "tan", "tanh",
];

/// Commands that take limits below and above (`\lim_{x \to 0}`).
const LIMIT_COMMANDS: &[&str] = &[
    "lim",
    "liminf",
    "limsup",
    "max",
    "min",
    "sup",
    "inf",
    "det",
    "gcd",
    "Pr",
    "sum",
    "prod",
    "coprod",
    "int",
    "iint",
    "iiint",
    "oint",
    "bigcup",
    "bigcap",
    "bigvee",
    "bigwedge",
    "bigoplus",
    "bigotimes",
    "bigodot",
    "biguplus",
    "bigsqcup",
];

/// A function name as LaTeX: `\sin` for a name LaTeX knows,
/// `\operatorname{name}` for another word, anything else as it is.
pub(crate) fn function_name(name: &str) -> String {
    let name = name.trim();
    if name.len() > 1 && name.chars().all(|c| c.is_ascii_alphabetic()) {
        if FUNCTIONS.contains(&name) {
            format!("\\{name}")
        } else {
            format!("\\operatorname{{{name}}}")
        }
    } else {
        name.to_owned()
    }
}

/// Whether `s` is a command that takes limits, maybe with a limit already
/// attached (`\sum_{i=1}` before its upper limit).
pub(crate) fn takes_limits(s: &str) -> bool {
    let Some(rest) = s.strip_prefix('\\') else {
        return false;
    };
    let name: String = rest.chars().take_while(char::is_ascii_alphabetic).collect();
    let after = &rest[name.len()..];
    LIMIT_COMMANDS.contains(&name.as_str()) && (after.is_empty() || after.starts_with("_{"))
}

/// The LaTeX command for an n-ary operator character.
pub(crate) fn nary_command(chr: &str) -> String {
    let cmd = match chr {
        "∑" => "\\sum",
        "∏" => "\\prod",
        "∐" => "\\coprod",
        "∫" => "\\int",
        "∬" => "\\iint",
        "∭" => "\\iiint",
        "∮" => "\\oint",
        "∯" => "\\oiint",
        "∰" => "\\oiiint",
        "⋃" => "\\bigcup",
        "⋂" => "\\bigcap",
        "⋁" => "\\bigvee",
        "⋀" => "\\bigwedge",
        "⨁" => "\\bigoplus",
        "⨂" => "\\bigotimes",
        "⨀" => "\\bigodot",
        "⨄" => "\\biguplus",
        "⨆" => "\\bigsqcup",
        other => return math_text(other, false),
    };
    cmd.to_owned()
}

/// A delimiter after `\left` or `\right`; `None` for one LaTeX cannot
/// stretch.
pub(crate) fn delimiter(chr: &str) -> Option<&'static str> {
    Some(match chr {
        "" => ".",
        "(" => "(",
        ")" => ")",
        "[" => "[",
        "]" => "]",
        "{" => "\\{",
        "}" => "\\}",
        "|" => "|",
        "‖" => "\\|",
        "/" => "/",
        "⟨" | "〈" => "\\langle",
        "⟩" | "〉" => "\\rangle",
        "⌊" => "\\lfloor",
        "⌋" => "\\rfloor",
        "⌈" => "\\lceil",
        "⌉" => "\\rceil",
        _ => return None,
    })
}

/// The accent command for an accent character (combining or spacing).
pub(crate) fn accent_command(chr: &str) -> Option<&'static str> {
    Some(match chr {
        "\u{0302}" | "^" | "ˆ" => "\\hat",
        "\u{0304}" | "\u{0305}" | "¯" | "‾" => "\\bar",
        "\u{0307}" | "˙" => "\\dot",
        "\u{0308}" | "¨" => "\\ddot",
        "\u{0303}" | "~" | "˜" => "\\tilde",
        "\u{20D7}" | "\u{20D1}" | "→" => "\\vec",
        "\u{0301}" | "´" => "\\acute",
        "\u{0300}" | "`" => "\\grave",
        "\u{030C}" | "ˇ" => "\\check",
        "\u{0306}" | "˘" => "\\breve",
        _ => return None,
    })
}

/// The command for a Unicode math symbol.
fn symbol(c: char) -> Option<&'static str> {
    Some(match c {
        // Greek.
        'α' => "\\alpha",
        'β' => "\\beta",
        'γ' => "\\gamma",
        'δ' => "\\delta",
        'ε' | 'ϵ' => "\\epsilon",
        'ζ' => "\\zeta",
        'η' => "\\eta",
        'θ' => "\\theta",
        'ϑ' => "\\vartheta",
        'ι' => "\\iota",
        'κ' => "\\kappa",
        'λ' => "\\lambda",
        'μ' | 'µ' => "\\mu",
        'ν' => "\\nu",
        'ξ' => "\\xi",
        'π' => "\\pi",
        'ϖ' => "\\varpi",
        'ρ' => "\\rho",
        'ϱ' => "\\varrho",
        'σ' => "\\sigma",
        'ς' => "\\varsigma",
        'τ' => "\\tau",
        'υ' => "\\upsilon",
        'φ' | 'ϕ' => "\\phi",
        'χ' => "\\chi",
        'ψ' => "\\psi",
        'ω' => "\\omega",
        'Γ' => "\\Gamma",
        'Δ' => "\\Delta",
        'Θ' => "\\Theta",
        'Λ' => "\\Lambda",
        'Ξ' => "\\Xi",
        'Π' => "\\Pi",
        'Σ' => "\\Sigma",
        'Υ' => "\\Upsilon",
        'Φ' => "\\Phi",
        'Ψ' => "\\Psi",
        'Ω' => "\\Omega",
        // Relations.
        '≤' | '⩽' => "\\leq",
        '≥' | '⩾' => "\\geq",
        '≠' => "\\neq",
        '≈' => "\\approx",
        '≡' => "\\equiv",
        '∼' => "\\sim",
        '≃' => "\\simeq",
        '≅' => "\\cong",
        '∝' => "\\propto",
        '≪' => "\\ll",
        '≫' => "\\gg",
        '∈' => "\\in",
        '∉' => "\\notin",
        '∋' => "\\ni",
        '⊂' => "\\subset",
        '⊃' => "\\supset",
        '⊆' => "\\subseteq",
        '⊇' => "\\supseteq",
        '⊥' => "\\perp",
        '∥' => "\\parallel",
        '∣' => "\\mid",
        '≔' => "\\coloneqq",
        // Operators.
        '×' => "\\times",
        '·' | '⋅' => "\\cdot",
        '÷' => "\\div",
        '±' => "\\pm",
        '∓' => "\\mp",
        '∘' => "\\circ",
        '∙' => "\\bullet",
        '∪' => "\\cup",
        '∩' => "\\cap",
        '∖' => "\\setminus",
        '⊕' => "\\oplus",
        '⊗' => "\\otimes",
        '∧' => "\\wedge",
        '∨' => "\\vee",
        '¬' => "\\neg",
        '√' => "\\surd",
        '∑' => "\\sum",
        '∏' => "\\prod",
        '∐' => "\\coprod",
        '∫' => "\\int",
        '∬' => "\\iint",
        '∭' => "\\iiint",
        '∮' => "\\oint",
        '⋃' => "\\bigcup",
        '⋂' => "\\bigcap",
        // Arrows.
        '→' => "\\to",
        '←' => "\\leftarrow",
        '↔' => "\\leftrightarrow",
        '⇒' => "\\Rightarrow",
        '⇐' => "\\Leftarrow",
        '⇔' => "\\Leftrightarrow",
        '↦' => "\\mapsto",
        '↑' => "\\uparrow",
        '↓' => "\\downarrow",
        '⟶' => "\\longrightarrow",
        '⟹' => "\\Longrightarrow",
        // Other symbols.
        '∞' => "\\infty",
        '∂' => "\\partial",
        '∇' => "\\nabla",
        '∀' => "\\forall",
        '∃' => "\\exists",
        '∄' => "\\nexists",
        '∅' => "\\emptyset",
        '∠' => "\\angle",
        '°' => "^{\\circ}",
        '′' => "'",
        '″' => "''",
        '…' => "\\ldots",
        '⋯' => "\\cdots",
        '⋮' => "\\vdots",
        '⋱' => "\\ddots",
        'ℏ' => "\\hbar",
        'ℓ' => "\\ell",
        'ℵ' => "\\aleph",
        'ℜ' => "\\Re",
        'ℑ' => "\\Im",
        'ℝ' => "\\mathbb{R}",
        'ℕ' => "\\mathbb{N}",
        'ℤ' => "\\mathbb{Z}",
        'ℚ' => "\\mathbb{Q}",
        'ℂ' => "\\mathbb{C}",
        '⟨' | '〈' => "\\langle",
        '⟩' | '〉' => "\\rangle",
        '⌊' => "\\lfloor",
        '⌋' => "\\rfloor",
        '⌈' => "\\lceil",
        '⌉' => "\\rceil",
        '‖' => "\\|",
        '−' => "-",
        '∗' => "*",
        '#' => "\\#",
        '$' => "\\$",
        '%' => "\\%",
        '_' => "\\_",
        '{' => "\\{",
        '}' => "\\}",
        '\\' => "\\backslash",
        '~' => "\\sim",
        '^' => "\\wedge",
        _ => return None,
    })
}

/// Math run text as LaTeX. `aligned`: `&` is an alignment point (in an
/// equation array) rather than an ampersand.
pub(crate) fn math_text(text: &str, aligned: bool) -> String {
    let mut out = Tex::new();
    for c in text.chars() {
        match c {
            // Function application, invisible times and separator and
            // plus, and zero-width space: layout hints with no reading.
            '\u{2061}'..='\u{2064}' | '\u{200B}' => {}
            '&' if aligned => out.push_raw(" & "),
            '&' => out.push_raw("\\&"),
            c if c.is_whitespace() => out.push_raw(" "),
            c => match symbol(c) {
                Some(cmd) => out.push(cmd),
                None => {
                    let mut buf = [0u8; 4];
                    out.push(c.encode_utf8(&mut buf));
                }
            },
        }
    }
    out.into_string()
}

/// Text for `\text{…}`, with text-mode specials escaped.
pub(crate) fn text_mode(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\textbackslash{}"),
            '{' | '}' | '$' | '%' | '&' | '#' | '_' => {
                out.push('\\');
                out.push(c);
            }
            '^' => out.push_str("\\^{}"),
            '~' => out.push_str("\\~{}"),
            c if c.is_whitespace() => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// Whether `s` is one token a script can attach to without braces: one
/// character or one command.
pub(crate) fn is_atom(s: &str) -> bool {
    let mut chars = s.chars();
    match (chars.next(), chars.next()) {
        (Some(_), None) => true,
        (Some('\\'), Some(_)) => s[1..].chars().all(|c| c.is_ascii_alphabetic()),
        _ => false,
    }
}

impl Conv {
    /// The LaTeX of an element.
    fn node(&mut self, n: Node<'_, '_>) -> String {
        if self.depth >= crate::MAX_NESTING {
            return self.flat(n);
        }
        self.depth += 1;
        let out = self.element(n);
        self.depth -= 1;
        out
    }

    /// Past the nesting limit: the run text of everything inside, without
    /// structure. Iterative, so no depth can overflow the stack.
    fn flat(&self, n: Node<'_, '_>) -> String {
        let mut out = Tex::new();
        for d in n.descendants() {
            if d.is_element() && d.tag_name().name() == "t" {
                out.push(&math_text(d.text().unwrap_or(""), self.aligned));
            }
        }
        out.into_string()
    }

    /// The LaTeX of an element's content children, in order; properties,
    /// deleted revisions, and field codes are skipped.
    fn children(&mut self, n: Node<'_, '_>) -> String {
        let mut out = Tex::new();
        for c in n.children().filter(Node::is_element) {
            let name = c.tag_name().name();
            if name.ends_with("Pr") || matches!(name, "del" | "moveFrom" | "delText" | "instrText")
            {
                continue;
            }
            let piece = self.node(c);
            out.push(&piece);
        }
        out.into_string()
    }

    /// The LaTeX of child `name`, empty when there is none.
    fn part(&mut self, n: Node<'_, '_>, name: &str) -> String {
        child(n, name).map_or_else(String::new, |c| self.children(c))
    }

    /// The base (`m:e`) of a script, in braces unless it is one token or
    /// one delimited group.
    fn base(&mut self, n: Node<'_, '_>) -> String {
        let Some(e) = child(n, "e") else {
            return "{}".to_owned();
        };
        let s = self.children(e);
        let mut content = e
            .children()
            .filter(|c| c.is_element() && !c.tag_name().name().ends_with("Pr"));
        let one_group = matches!(
            (content.next(), content.next()),
            (Some(only), None) if only.tag_name().name() == "d"
        );
        if is_atom(&s) || one_group {
            s
        } else {
            format!("{{{s}}}")
        }
    }

    fn element(&mut self, n: Node<'_, '_>) -> String {
        let pr = |name: &str| child(n, name);
        match n.tag_name().name() {
            "r" => self.run(n),
            "t" => math_text(n.text().unwrap_or(""), self.aligned),
            "f" => {
                let num = self.part(n, "num");
                let den = self.part(n, "den");
                if prop(pr("fPr"), "type") == Some("noBar") {
                    format!("{{{num} \\atop {den}}}")
                } else {
                    format!("\\frac{{{num}}}{{{den}}}")
                }
            }
            "sSup" => {
                let base = self.base(n);
                let sup = self.part(n, "sup");
                format!("{base}^{{{sup}}}")
            }
            "sSub" => {
                let base = self.base(n);
                let sub = self.part(n, "sub");
                format!("{base}_{{{sub}}}")
            }
            "sSubSup" => {
                let base = self.base(n);
                let sub = self.part(n, "sub");
                let sup = self.part(n, "sup");
                format!("{base}_{{{sub}}}^{{{sup}}}")
            }
            "sPre" => {
                let sub = self.part(n, "sub");
                let sup = self.part(n, "sup");
                let e = self.part(n, "e");
                format!("{{}}_{{{sub}}}^{{{sup}}}{e}")
            }
            "rad" => {
                let e = self.part(n, "e");
                let deg = self.part(n, "deg");
                if toggle(pr("radPr"), "degHide") || deg.trim().is_empty() {
                    format!("\\sqrt{{{e}}}")
                } else {
                    let deg = if deg.contains(']') {
                        format!("{{{deg}}}")
                    } else {
                        deg
                    };
                    format!("\\sqrt[{deg}]{{{e}}}")
                }
            }
            "nary" => {
                let props = pr("naryPr");
                let chr = prop(props, "chr").filter(|c| !c.is_empty()).unwrap_or("∫");
                let mut out = nary_command(chr);
                let sub = self.part(n, "sub");
                if !toggle(props, "subHide") && !sub.trim().is_empty() {
                    out.push_str(&format!("_{{{sub}}}"));
                }
                let sup = self.part(n, "sup");
                if !toggle(props, "supHide") && !sup.trim().is_empty() {
                    out.push_str(&format!("^{{{sup}}}"));
                }
                let e = self.part(n, "e");
                if !e.trim().is_empty() {
                    out.push(' ');
                    out.push_str(&e);
                }
                out
            }
            "d" => self.delimited(n),
            "m" => {
                let rows: Vec<String> = n
                    .children()
                    .filter(|r| r.tag_name().name() == "mr")
                    .map(|r| {
                        r.children()
                            .filter(|e| e.tag_name().name() == "e")
                            .map(|e| self.children(e))
                            .collect::<Vec<_>>()
                            .join(" & ")
                    })
                    .collect();
                format!("\\begin{{matrix}} {} \\end{{matrix}}", rows.join(" \\\\ "))
            }
            "eqArr" => {
                let was = std::mem::replace(&mut self.aligned, true);
                let rows: Vec<String> = n
                    .children()
                    .filter(|e| e.tag_name().name() == "e")
                    .map(|e| self.children(e))
                    .collect();
                self.aligned = was;
                format!(
                    "\\begin{{aligned}} {} \\end{{aligned}}",
                    rows.join(" \\\\ ")
                )
            }
            "func" => {
                let name = function_name(&self.part(n, "fName"));
                let e = self.part(n, "e");
                format!("{name} {e}")
            }
            "acc" => {
                let chr = prop(pr("accPr"), "chr").unwrap_or("\u{0302}");
                let e = self.part(n, "e");
                match accent_command(chr) {
                    Some(cmd) => format!("{cmd}{{{e}}}"),
                    None => format!("\\overset{{{}}}{{{e}}}", math_text(chr, false)),
                }
            }
            "bar" => {
                let e = self.part(n, "e");
                if prop(pr("barPr"), "pos") == Some("top") {
                    format!("\\overline{{{e}}}")
                } else {
                    format!("\\underline{{{e}}}")
                }
            }
            "limLow" | "limUpp" => {
                let low = n.tag_name().name() == "limLow";
                let e = self.part(n, "e");
                let e = if FUNCTIONS.contains(&e.trim()) {
                    function_name(&e)
                } else {
                    e
                };
                let lim = self.part(n, "lim");
                match (takes_limits(e.trim()), low) {
                    (true, true) => format!("{}_{{{lim}}}", e.trim()),
                    (true, false) => format!("{}^{{{lim}}}", e.trim()),
                    (false, true) => format!("\\underset{{{lim}}}{{{e}}}"),
                    (false, false) => format!("\\overset{{{lim}}}{{{e}}}"),
                }
            }
            "borderBox" => format!("\\boxed{{{}}}", self.part(n, "e")),
            "groupChr" => {
                let props = pr("groupChrPr");
                let chr = prop(props, "chr").unwrap_or("\u{23DF}");
                let top = prop(props, "pos") == Some("top");
                let e = self.part(n, "e");
                match chr {
                    "\u{23DF}" => format!("\\underbrace{{{e}}}"),
                    "\u{23DE}" => format!("\\overbrace{{{e}}}"),
                    other if top => format!("\\overset{{{}}}{{{e}}}", math_text(other, false)),
                    other => format!("\\underset{{{}}}{{{e}}}", math_text(other, false)),
                }
            }
            "phant" => {
                let e = self.part(n, "e");
                let shown =
                    prop(pr("phantPr"), "show").is_none_or(|v| !matches!(v, "0" | "false" | "off"));
                if shown {
                    e
                } else {
                    format!("\\phantom{{{e}}}")
                }
            }
            // `m:oMath`, `m:oMathPara`, `m:box`, the argument elements
            // (`m:e`, `m:num`, ...), and anything unknown: the children.
            _ => self.children(n),
        }
    }

    /// `m:d`: its elements between `\left` and `\right`, separated by the
    /// separator character.
    fn delimited(&mut self, n: Node<'_, '_>) -> String {
        let props = child(n, "dPr");
        let beg = prop(props, "begChr").unwrap_or("(");
        let end = prop(props, "endChr").unwrap_or(")");
        let sep = match prop(props, "sepChr").unwrap_or("|") {
            "|" => "\\middle|".to_owned(),
            "‖" => "\\middle\\|".to_owned(),
            other => math_text(other, false),
        };
        let items: Vec<String> = n
            .children()
            .filter(|e| e.tag_name().name() == "e")
            .map(|e| self.children(e))
            .collect();
        let inner = items.join(&format!(" {sep} "));
        // A delimiter `\left` cannot take is written as a symbol inside
        // invisible ones.
        let (left, open) = match delimiter(beg) {
            Some(d) => (d.to_owned(), String::new()),
            None => (".".to_owned(), format!("{} ", math_text(beg, false))),
        };
        let (right, close) = match delimiter(end) {
            Some(d) => (d.to_owned(), String::new()),
            None => (".".to_owned(), format!(" {}", math_text(end, false))),
        };
        format!("\\left{left} {open}{inner}{close} \\right{right}")
    }

    /// `m:r`: math text, or `\text{…}` for a plain text run (`m:nor`, or a
    /// Word run inside the equation).
    fn run(&mut self, r: Node<'_, '_>) -> String {
        let mut text = String::new();
        for c in r.children().filter(Node::is_element) {
            match c.tag_name().name() {
                "t" => text.push_str(c.text().unwrap_or("")),
                "tab" | "br" | "cr" => text.push(' '),
                _ => {}
            }
        }
        let props: Vec<Node<'_, '_>> = r
            .children()
            .filter(|c| c.tag_name().name() == "rPr")
            .collect();
        let normal = props.iter().any(|p| toggle(Some(*p), "nor"));
        let upright = props.iter().any(|p| prop(Some(*p), "sty") == Some("p"));
        let word_run = r.tag_name().namespace().is_some_and(|ns| ns != MATH_NS);
        if normal || word_run {
            if text.trim().is_empty() {
                return text;
            }
            return format!("\\text{{{}}}", text_mode(&text));
        }
        if upright && FUNCTIONS.contains(&text.trim()) {
            return function_name(&text);
        }
        math_text(&text, self.aligned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const NS: &str = r#"xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math" xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#;

    /// The LaTeX of `body` inside an `m:oMath`.
    fn latex(body: &str) -> String {
        let xml = format!("<m:oMath {NS}>{body}</m:oMath>");
        let doc = roxmltree::Document::parse(&xml).expect("test XML parses");
        to_latex(doc.root_element())
    }

    fn r(text: &str) -> String {
        format!("<m:r><m:t>{text}</m:t></m:r>")
    }

    fn e(body: &str) -> String {
        format!("<m:e>{body}</m:e>")
    }

    /// An accent `chr` over a run of `text`.
    fn acc(chr: &str, text: &str) -> String {
        format!(
            r#"<m:acc><m:accPr><m:chr m:val="{chr}"/></m:accPr>{}</m:acc>"#,
            e(&r(text))
        )
    }

    /// Braces balance, not counting escaped ones (`\{`).
    fn balanced(s: &str) -> bool {
        let mut depth = 0i64;
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            match c {
                '\\' => {
                    chars.next();
                }
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth < 0 {
                        return false;
                    }
                }
                _ => {}
            }
        }
        depth == 0
    }

    /// (OMML, expected LaTeX) for every element the converter knows.
    fn fixtures() -> Vec<(String, &'static str)> {
        vec![
            (r("x+1=2"), "x+1=2"),
            (r("α+β≤π·∞"), "\\alpha+\\beta\\leq\\pi\\cdot\\infty"),
            (r("αx"), "\\alpha x"),
            (r("a≠b×c±d→e≥f"), "a\\neq b\\times c\\pm d\\to e\\geq f"),
            (r("{a}_#%$&amp;"), "\\{a\\}\\_\\#\\%\\$\\&"),
            (
                "<m:r><m:rPr><m:nor/></m:rPr><m:t>if x {big} &amp; $5</m:t></m:r>".into(),
                "\\text{if x \\{big\\} \\& \\$5}",
            ),
            ("<w:r><w:t>where</w:t></w:r>".into(), "\\text{where}"),
            (
                format!(
                    "<m:f><m:num>{}</m:num><m:den>{}</m:den></m:f>",
                    r("a"),
                    r("b")
                ),
                "\\frac{a}{b}",
            ),
            (
                format!(
                    r#"<m:f><m:fPr><m:type m:val="lin"/></m:fPr><m:num>{}</m:num><m:den>{}</m:den></m:f>"#,
                    r("1"),
                    r("2")
                ),
                "\\frac{1}{2}",
            ),
            (
                format!(
                    r#"<m:f><m:fPr><m:type m:val="noBar"/></m:fPr><m:num>{}</m:num><m:den>{}</m:den></m:f>"#,
                    r("n"),
                    r("k")
                ),
                "{n \\atop k}",
            ),
            (
                format!("<m:sSup>{}<m:sup>{}</m:sup></m:sSup>", e(&r("x")), r("2")),
                "x^{2}",
            ),
            (
                format!("<m:sSub>{}<m:sub>{}</m:sub></m:sSub>", e(&r("x")), r("i")),
                "x_{i}",
            ),
            (
                format!(
                    "<m:sSubSup>{}<m:sub>{}</m:sub><m:sup>{}</m:sup></m:sSubSup>",
                    e(&r("x")),
                    r("i"),
                    r("2")
                ),
                "x_{i}^{2}",
            ),
            (
                format!("<m:sSup>{}<m:sup>{}</m:sup></m:sSup>", e(&r("ab")), r("n")),
                "{ab}^{n}",
            ),
            (
                format!(
                    "<m:sPre><m:sub>{}</m:sub><m:sup>{}</m:sup>{}</m:sPre>",
                    r("1"),
                    r("2"),
                    e(&r("X"))
                ),
                "{}_{1}^{2}X",
            ),
            (
                format!(
                    r#"<m:rad><m:radPr><m:degHide m:val="1"/></m:radPr><m:deg/>{}</m:rad>"#,
                    e(&r("x"))
                ),
                "\\sqrt{x}",
            ),
            (
                format!("<m:rad><m:deg>{}</m:deg>{}</m:rad>", r("3"), e(&r("x"))),
                "\\sqrt[3]{x}",
            ),
            (
                format!(
                    r#"<m:nary><m:naryPr><m:chr m:val="∑"/></m:naryPr><m:sub>{}</m:sub><m:sup>{}</m:sup>{}</m:nary>"#,
                    r("i=1"),
                    r("n"),
                    e(&r("i"))
                ),
                "\\sum_{i=1}^{n} i",
            ),
            (
                format!(
                    "<m:nary><m:sub>{}</m:sub><m:sup>{}</m:sup>{}</m:nary>",
                    r("a"),
                    r("b"),
                    e(&r("f(x)dx"))
                ),
                "\\int_{a}^{b} f(x)dx",
            ),
            (
                format!(
                    r#"<m:nary><m:naryPr><m:chr m:val="∏"/><m:supHide m:val="1"/></m:naryPr><m:sub>{}</m:sub><m:sup/>{}</m:nary>"#,
                    r("k"),
                    e(&r("a"))
                ),
                "\\prod_{k} a",
            ),
            (
                format!(
                    r#"<m:nary><m:naryPr><m:chr m:val="∮"/><m:subHide/><m:supHide/></m:naryPr><m:sub/><m:sup/>{}</m:nary>"#,
                    e(&r("F"))
                ),
                "\\oint F",
            ),
            (
                format!("<m:d>{}</m:d>", e(&r("a+b"))),
                "\\left( a+b \\right)",
            ),
            (
                format!(
                    r#"<m:d><m:dPr><m:begChr m:val="["/><m:endChr m:val="}}"/></m:dPr>{}</m:d>"#,
                    e(&r("x"))
                ),
                "\\left[ x \\right\\}",
            ),
            (
                format!("<m:d>{}{}</m:d>", e(&r("a")), e(&r("b"))),
                "\\left( a \\middle| b \\right)",
            ),
            (
                format!(
                    r#"<m:d><m:dPr><m:begChr m:val="⟨"/><m:sepChr m:val=","/><m:endChr m:val="⟩"/></m:dPr>{}{}</m:d>"#,
                    e(&r("u")),
                    e(&r("v"))
                ),
                "\\left\\langle u , v \\right\\rangle",
            ),
            (
                format!(
                    r#"<m:d><m:dPr><m:begChr m:val="|"/><m:endChr m:val=""/></m:dPr>{}</m:d>"#,
                    e(&r("x"))
                ),
                "\\left| x \\right.",
            ),
            (
                format!(
                    "<m:m><m:mr>{}{}</m:mr><m:mr>{}{}</m:mr></m:m>",
                    e(&r("a")),
                    e(&r("b")),
                    e(&r("c")),
                    e(&r("d"))
                ),
                "\\begin{matrix} a & b \\\\ c & d \\end{matrix}",
            ),
            (
                format!(
                    r#"<m:func><m:fName><m:r><m:rPr><m:sty m:val="p"/></m:rPr><m:t>sin</m:t></m:r></m:fName>{}</m:func>"#,
                    e(&r("x"))
                ),
                "\\sin x",
            ),
            (
                format!(
                    "<m:func><m:fName>{}</m:fName>{}</m:func>",
                    r("sgn"),
                    e(&r("x"))
                ),
                "\\operatorname{sgn} x",
            ),
            (acc("\u{0302}", "x"), "\\hat{x}"),
            (format!("<m:acc>{}</m:acc>", e(&r("y"))), "\\hat{y}"),
            (acc("\u{20D7}", "v"), "\\vec{v}"),
            (acc("\u{0303}", "n"), "\\tilde{n}"),
            (acc("\u{0307}", "q"), "\\dot{q}"),
            (acc("\u{0305}", "z"), "\\bar{z}"),
            (
                format!(
                    r#"<m:bar><m:barPr><m:pos m:val="top"/></m:barPr>{}</m:bar>"#,
                    e(&r("AB"))
                ),
                "\\overline{AB}",
            ),
            (format!("<m:bar>{}</m:bar>", e(&r("x"))), "\\underline{x}"),
            (
                format!(
                    "<m:func><m:fName><m:limLow>{}<m:lim>{}</m:lim></m:limLow></m:fName>{}</m:func>",
                    e(&r("lim")),
                    r("x→0"),
                    e(&r("f(x)"))
                ),
                "\\lim_{x\\to0} f(x)",
            ),
            (
                format!(
                    "<m:limUpp>{}<m:lim>{}</m:lim></m:limUpp>",
                    e(&r("x")),
                    r("def")
                ),
                "\\overset{def}{x}",
            ),
            (
                format!(
                    "<m:limUpp>{}<m:lim>{}</m:lim></m:limUpp>",
                    e(&format!(
                        "<m:limLow>{}<m:lim>{}</m:lim></m:limLow>",
                        e(&r("∑")),
                        r("i=1")
                    )),
                    r("n")
                ),
                "\\sum_{i=1}^{n}",
            ),
            (format!("<m:box>{}</m:box>", e(&r("a=b"))), "a=b"),
            (
                format!("<m:borderBox>{}</m:borderBox>", e(&r("E=mc"))),
                "\\boxed{E=mc}",
            ),
            (
                format!("<m:groupChr>{}</m:groupChr>", e(&r("a+b"))),
                "\\underbrace{a+b}",
            ),
            (
                format!(
                    r#"<m:groupChr><m:groupChrPr><m:chr m:val="→"/><m:pos m:val="top"/></m:groupChrPr>{}</m:groupChr>"#,
                    e(&r("f"))
                ),
                "\\overset{\\to}{f}",
            ),
            (
                format!(
                    "<m:eqArr>{}{}</m:eqArr>",
                    e(&r("x&amp;=1")),
                    e(&r("y&amp;=2"))
                ),
                "\\begin{aligned} x & =1 \\\\ y & =2 \\end{aligned}",
            ),
            (
                format!(
                    r#"<m:phant><m:phantPr><m:show m:val="0"/></m:phantPr>{}</m:phant>"#,
                    e(&r("x"))
                ),
                "\\phantom{x}",
            ),
            (
                format!(
                    "<m:unknownThing><m:inner>{}</m:inner></m:unknownThing>",
                    r("q")
                ),
                "q",
            ),
            (
                format!(
                    "<m:sSup>{}<m:sup>{}</m:sup></m:sSup>",
                    e(&format!("<m:d>{}</m:d>", e(&r("a+b")))),
                    r("2")
                ),
                "\\left( a+b \\right)^{2}",
            ),
        ]
    }

    #[test]
    fn every_element_converts() {
        for (omml, want) in fixtures() {
            assert_eq!(latex(&omml), want, "for {omml}");
        }
    }

    #[test]
    fn braces_balance_on_every_fixture() {
        for (omml, _) in fixtures() {
            let got = latex(&omml);
            assert!(balanced(&got), "unbalanced {got:?} for {omml}");
        }
    }

    #[test]
    fn commands_are_kept_apart_from_letters() {
        assert!(ends_with_command("\\alpha"));
        assert!(!ends_with_command("\\\\ab"));
        assert!(!ends_with_command("x"));
        let mut s = String::from("\\pi");
        push(&mut s, "r");
        assert_eq!(s, "\\pi r");
        let mut s = String::from("a \\\\");
        push(&mut s, "b");
        assert_eq!(s, "a \\\\b");
    }

    proptest! {
        /// The tracked builder spaces pieces exactly as the scan back
        /// from the end does, for any pieces of letters, backslashes, and
        /// other characters, pushed spaced or raw.
        #[test]
        fn tex_matches_the_scan(
            pieces in proptest::collection::vec((r"[ab\\{ 1&é]{0,5}", any::<bool>()), 0..40)
        ) {
            let mut tex = Tex::new();
            let mut scan = String::new();
            for (p, raw) in &pieces {
                if *raw {
                    tex.push_raw(p);
                    scan.push_str(p);
                } else {
                    tex.push(p);
                    push(&mut scan, p);
                }
            }
            prop_assert_eq!(tex.into_string(), scan);
        }
    }

    #[test]
    fn a_long_run_of_letters_is_read_in_linear_time() {
        // Each letter of a token is pushed on its own; a scan back over
        // the run before it made this quadratic (W8b-ml).
        let token = "x".repeat(400_000);
        let t = std::time::Instant::now();
        let got = math_text(&token, false);
        assert_eq!(got, token);
        assert!(
            t.elapsed() < std::time::Duration::from_secs(5),
            "took {:?}",
            t.elapsed()
        );
    }

    #[test]
    fn deep_nesting_does_not_overflow() {
        let depth = 2_000;
        let mut xml = String::new();
        for _ in 0..depth {
            xml.push_str("<m:sSup><m:e>");
        }
        xml.push_str(&r("x"));
        for _ in 0..depth {
            xml.push_str("</m:e><m:sup><m:r><m:t>2</m:t></m:r></m:sup></m:sSup>");
        }
        let got = std::thread::Builder::new()
            .stack_size(8 << 20)
            .spawn(move || latex(&xml))
            .expect("thread starts")
            .join()
            .expect("no overflow");
        assert!(got.contains('x'));
        assert!(balanced(&got));
    }

    /// Random OMML: runs with awkward characters inside nested elements.
    fn omml() -> impl Strategy<Value = String> {
        let leaf = prop::sample::select(vec![
            "x", "{", "}", "\\", "α", "&amp;", "$", "2", "sin", " ", "_", "^", "~",
        ])
        .prop_map(|t| format!("<m:r><m:t xml:space=\"preserve\">{t}</m:t></m:r>"));
        leaf.prop_recursive(5, 48, 3, |inner| {
            let kids = prop::collection::vec(inner, 1..3).prop_map(|v| v.concat());
            (0usize..16, kids.clone(), kids).prop_map(|(k, a, b)| {
                let tag = [
                    "f", "sSup", "sSub", "sSubSup", "sPre", "rad", "nary", "d", "m", "func", "acc",
                    "bar", "limLow", "groupChr", "eqArr", "box",
                ][k];
                let e = format!("<m:e>{a}</m:e>");
                match tag {
                    "f" => format!("<m:f><m:num>{a}</m:num><m:den>{b}</m:den></m:f>"),
                    "sSup" => format!("<m:sSup>{e}<m:sup>{b}</m:sup></m:sSup>"),
                    "sSub" => format!("<m:sSub>{e}<m:sub>{b}</m:sub></m:sSub>"),
                    "sSubSup" | "sPre" | "nary" => {
                        format!("<m:{tag}>{e}<m:sub>{b}</m:sub><m:sup>{a}</m:sup></m:{tag}>")
                    }
                    "rad" => format!("<m:rad><m:deg>{b}</m:deg>{e}</m:rad>"),
                    "m" => format!("<m:m><m:mr>{e}<m:e>{b}</m:e></m:mr></m:m>"),
                    "func" => format!("<m:func><m:fName>{b}</m:fName>{e}</m:func>"),
                    "limLow" => format!("<m:limLow>{e}<m:lim>{b}</m:lim></m:limLow>"),
                    "eqArr" => format!("<m:eqArr>{e}<m:e>{b}</m:e></m:eqArr>"),
                    _ => format!("<m:{tag}>{e}<m:e>{b}</m:e></m:{tag}>"),
                }
            })
        })
    }

    proptest! {
        #[test]
        fn braces_always_balance(body in omml()) {
            let got = latex(&body);
            prop_assert!(balanced(&got), "unbalanced {:?} for {}", got, body);
        }
    }
}
