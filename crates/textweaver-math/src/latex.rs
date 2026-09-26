//! LaTeX math parser: the common subset used in course material and papers.
//!
//! Covered: groups, `^`/`_`/primes, `\frac` and friends, `\binom`,
//! `\sqrt[n]{}`, Greek and symbol commands, operators, relations, arrows,
//! big operators with limits (`\limits`, `\nolimits`), named functions and
//! `\operatorname`, `\left…\right` with `\middle`, `\big` sizes,
//! environments (`matrix`, `pmatrix`, `bmatrix`, `Bmatrix`, `vmatrix`,
//! `Vmatrix`, `smallmatrix`, `array`, `cases`, `rcases`, `aligned`, `align`,
//! `gather`, `split`, `equation`), `\\` line breaks, `\text`, font commands,
//! accents, `\overset`/`\underset`/`\stackrel`, braces, `\not`, `\pmod`,
//! `\cancel`, `\boxed`, spacing, and `\over`/`\choose`.
//!
//! The parser is total: unknown commands become [`NodeKind::Error`] nodes,
//! missing or stray braces are recovered from, and each problem is recorded
//! as a [`Diagnostic`].

use textweaver_core::CharRange;

use crate::build::{MAX_DEPTH, append, group_fences, items_span, row, row_or_single};
use crate::symbols::{Sym, char_symbol, latex_function, latex_symbol, negate, takes_limits};
use crate::{
    AccentKind, Diagnostic, Enclosure, Math, Node, NodeKind, Notation, OpClass, TableKind, Variant,
};

/// Parses LaTeX math (without its `$` or `\(` delimiters).
///
/// Never fails and never panics; see [`Math::diagnostics`] for anything the
/// parser had to recover from.
pub fn parse_latex(src: &str) -> Math {
    let chars: Vec<char> = src.chars().collect();
    let len = chars.len();
    let mut p = Parser {
        s: chars,
        i: 0,
        end: len,
        depth: 0,
        diags: Vec::new(),
    };
    let rows = p.parse_lines(Ctx::Top);
    let items = if rows.len() == 1 && rows[0].len() == 1 {
        rows.into_iter()
            .next()
            .and_then(|r| r.into_iter().next())
            .map(|c| c.items)
            .unwrap_or_default()
    } else {
        let span = CharRange::new(0, len);
        vec![Node::new(
            NodeKind::Table {
                kind: TableKind::Aligned,
                rows: cells_to_nodes(rows),
            },
            span,
        )]
    };
    Math {
        notation: Notation::Latex,
        source: src.to_owned(),
        root: row(items, CharRange::new(0, len)),
        diagnostics: p.diags,
    }
}

/// Where a list of lines ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ctx {
    /// The whole input (or a bracketed optional argument).
    Top,
    /// A `{…}` group.
    Group,
    /// An environment body, ended by `\end`.
    Env,
    /// A `\left…\right` body.
    Left,
}

/// What stopped a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stop {
    Eof,
    RBrace,
    Amp,
    NewRow,
    End,
    Right,
}

struct Cell {
    items: Vec<Node>,
    start: usize,
}

fn cells_to_nodes(rows: Vec<Vec<Cell>>) -> Vec<Vec<Node>> {
    rows.into_iter()
        .map(|r| {
            r.into_iter()
                .map(|c| row_or_single(c.items, c.start))
                .collect()
        })
        .collect()
}

struct Parser {
    s: Vec<char>,
    i: usize,
    end: usize,
    depth: usize,
    diags: Vec<Diagnostic>,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        (self.i < self.end).then(|| self.s[self.i])
    }

    fn peek_at(&self, k: usize) -> Option<char> {
        let j = self.i + k;
        (j < self.end).then(|| self.s[j])
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.i += 1;
            } else if c == '%' {
                // A comment runs to the end of the line.
                while let Some(c) = self.peek() {
                    self.i += 1;
                    if c == '\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn diag(&mut self, message: impl Into<String>, a: usize, b: usize) {
        self.diags.push(Diagnostic {
            message: message.into(),
            span: CharRange::new(a, b),
        });
    }

    /// The command name at `at` (which must be a backslash) and the index
    /// after it. Letters form a name; otherwise one character does.
    fn command_at(&self, at: usize) -> (String, usize) {
        let mut j = at + 1;
        if j >= self.end {
            return (String::new(), j.min(self.end));
        }
        if self.s[j].is_ascii_alphabetic() {
            let st = j;
            while j < self.end && self.s[j].is_ascii_alphabetic() {
                j += 1;
            }
            (self.s[st..j].iter().collect(), j)
        } else {
            (self.s[j].to_string(), j + 1)
        }
    }

    fn peek_command(&self) -> Option<String> {
        (self.peek() == Some('\\')).then(|| self.command_at(self.i).0)
    }

    /// Parses rows of cells until the context's end (not consumed).
    fn parse_lines(&mut self, ctx: Ctx) -> Vec<Vec<Cell>> {
        let mut rows: Vec<Vec<Cell>> = Vec::new();
        let mut cells: Vec<Cell> = Vec::new();
        let mut items: Vec<Node> = Vec::new();
        let mut start = self.i;
        loop {
            let stop = self.parse_row(&mut items);
            match stop {
                Stop::Amp => {
                    cells.push(Cell {
                        items: std::mem::take(&mut items),
                        start,
                    });
                    self.i += 1;
                    start = self.i;
                }
                Stop::NewRow => {
                    cells.push(Cell {
                        items: std::mem::take(&mut items),
                        start,
                    });
                    rows.push(std::mem::take(&mut cells));
                    let (_, next) = self.command_at(self.i);
                    self.i = next;
                    self.skip_optional_dimension();
                    start = self.i;
                }
                Stop::Eof => break,
                Stop::RBrace if ctx == Ctx::Group => break,
                Stop::End if ctx == Ctx::Env => break,
                Stop::Right if ctx == Ctx::Left => break,
                stray if ctx == Ctx::Top => self.consume_stray(stray),
                // A closer that belongs to an enclosing context ends this one.
                _ => break,
            }
        }
        cells.push(Cell { items, start });
        rows.push(cells);
        // A final `\\` leaves an empty last row.
        if rows.len() > 1
            && rows
                .last()
                .is_some_and(|r| r.len() == 1 && r[0].items.is_empty())
        {
            rows.pop();
        }
        rows
    }

    fn skip_optional_dimension(&mut self) {
        let save = self.i;
        self.skip_ws();
        if self.peek() == Some('[')
            && let Some(close) = self.find_close_bracket(self.i)
        {
            self.i = close + 1;
            return;
        }
        self.i = save;
    }

    fn consume_stray(&mut self, stop: Stop) {
        let at = self.i;
        match stop {
            Stop::RBrace => {
                self.i += 1;
                self.diag("unmatched closing brace", at, self.i);
            }
            Stop::End => {
                let (_, next) = self.command_at(self.i);
                self.i = next;
                let _ = self.read_group_raw();
                self.diag("\\end without \\begin", at, self.i);
            }
            Stop::Right => {
                let (_, next) = self.command_at(self.i);
                self.i = next;
                let _ = self.parse_delim();
                self.diag("\\right without \\left", at, self.i);
            }
            Stop::Eof | Stop::Amp | Stop::NewRow => {}
        }
    }

    /// Parses items until a stop, which is left unconsumed.
    fn parse_row(&mut self, items: &mut Vec<Node>) -> Stop {
        let mut infix: Option<(String, Vec<Node>, CharRange)> = None;
        let stop = loop {
            self.skip_ws();
            let Some(c) = self.peek() else {
                break Stop::Eof;
            };
            match c {
                '}' => break Stop::RBrace,
                '&' => break Stop::Amp,
                '\\' => {
                    let (name, next) = self.command_at(self.i);
                    match name.as_str() {
                        "\\" | "cr" | "newline" => break Stop::NewRow,
                        "end" => break Stop::End,
                        "right" => break Stop::Right,
                        "over" | "choose" | "atop" | "brack" | "brace" if infix.is_none() => {
                            let at = CharRange::new(self.i, next);
                            self.i = next;
                            let left = group_fences(std::mem::take(items));
                            infix = Some((name, left, at));
                            continue;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            let before = self.i;
            if let Some(node) = self.parse_atom() {
                items.push(node);
            }
            if self.i == before {
                // Never loop without progress.
                self.i += 1;
            }
        };
        let grouped = group_fences(std::mem::take(items));
        match infix {
            None => *items = grouped,
            Some((name, left, at)) => {
                let num = row_or_single(left, at.start.0);
                let den = row_or_single(grouped, at.end.0);
                let span = num.span.cover(den.span).cover(at);
                let frac = Node::new(
                    NodeKind::Fraction {
                        num: Box::new(num),
                        den: Box::new(den),
                        bar: name == "over",
                    },
                    span,
                );
                let node = match name.as_str() {
                    "choose" => fenced("(", ")", frac, span),
                    "brack" => fenced("[", "]", frac, span),
                    "brace" => fenced("{", "}", frac, span),
                    _ => frac,
                };
                *items = vec![node];
            }
        }
        stop
    }

    /// One atom with its scripts.
    fn parse_atom(&mut self) -> Option<Node> {
        let base = self.parse_primary()?;
        Some(self.parse_scripts(base))
    }

    fn parse_scripts(&mut self, base: Node) -> Node {
        let mut sub: Option<Node> = None;
        let mut sup: Option<Node> = None;
        let mut limits: Option<bool> = None;
        loop {
            let save = self.i;
            self.skip_ws();
            match self.peek() {
                Some(c @ ('^' | '_')) => {
                    let at = self.i;
                    self.i += 1;
                    let what = if c == '^' { "superscript" } else { "subscript" };
                    let arg = self.parse_arg(what);
                    let slot = if c == '^' { &mut sup } else { &mut sub };
                    *slot = Some(match slot.take() {
                        None => arg,
                        Some(prev) => {
                            self.diag(format!("double {what}"), at, self.i);
                            append(prev, arg)
                        }
                    });
                }
                Some('\'') => {
                    let at = self.i;
                    while self.peek() == Some('\'') {
                        self.i += 1;
                    }
                    let text = match self.i - at {
                        1 => "′",
                        2 => "″",
                        3 => "‴",
                        _ => "⁗",
                    };
                    let primes = Node::new(
                        NodeKind::Operator {
                            text: text.to_owned(),
                            class: OpClass::Postfix,
                        },
                        CharRange::new(at, self.i),
                    );
                    sup = Some(match sup.take() {
                        None => primes,
                        Some(prev) => append(prev, primes),
                    });
                }
                Some('\\') => match self.peek_command().as_deref() {
                    Some("limits") => {
                        self.i = self.command_at(self.i).1;
                        limits = Some(true);
                    }
                    Some("nolimits") => {
                        self.i = self.command_at(self.i).1;
                        limits = Some(false);
                    }
                    _ => {
                        self.i = save;
                        break;
                    }
                },
                _ => {
                    self.i = save;
                    break;
                }
            }
        }
        if sub.is_none() && sup.is_none() {
            return base;
        }
        let limits = limits.unwrap_or_else(|| default_limits(&base));
        let mut span = base.span;
        for s in [&sub, &sup].into_iter().flatten() {
            span = span.cover(s.span);
        }
        Node::new(
            NodeKind::Scripts {
                base: Box::new(base),
                sub: sub.map(Box::new),
                sup: sup.map(Box::new),
                limits,
            },
            span,
        )
    }

    /// A macro argument: a group, one command, or one character.
    fn parse_arg(&mut self, what: &str) -> Node {
        self.skip_ws();
        let at = self.i;
        let missing = |p: &mut Parser| {
            p.diag(format!("missing {what}"), at, at);
            Node::empty(at)
        };
        match self.peek() {
            None | Some('}' | '&' | '^' | '_') => missing(self),
            Some('{') => self.parse_group(),
            Some('\\') => {
                let (name, _) = self.command_at(self.i);
                if matches!(name.as_str(), "\\" | "end" | "right" | "cr" | "newline") {
                    return missing(self);
                }
                self.parse_primary().unwrap_or_else(|| Node::empty(at))
            }
            Some(c) if c.is_ascii_digit() => {
                self.i += 1;
                Node::new(NodeKind::Number(c.to_string()), CharRange::new(at, self.i))
            }
            Some(_) => self.parse_primary().unwrap_or_else(|| Node::empty(at)),
        }
    }

    /// A `{…}` group. One child is returned as itself; several become a row
    /// whose span includes the braces; `\\` inside makes a stacked table.
    fn parse_group(&mut self) -> Node {
        let start = self.i;
        self.i += 1;
        if self.depth >= MAX_DEPTH {
            return self.too_deep(start);
        }
        self.depth += 1;
        let rows = self.parse_lines(Ctx::Group);
        self.depth -= 1;
        if self.peek() == Some('}') {
            self.i += 1;
        } else {
            self.diag("missing closing brace", start, self.i);
        }
        let span = CharRange::new(start, self.i);
        if rows.len() == 1 && rows[0].len() == 1 {
            let mut rows = rows;
            let cell = rows.remove(0).remove(0);
            let mut items = cell.items;
            if items.len() == 1 {
                return items.remove(0);
            }
            return row(std::mem::take(&mut items), span);
        }
        Node::new(
            NodeKind::Table {
                kind: TableKind::Aligned,
                rows: cells_to_nodes(rows),
            },
            span,
        )
    }

    fn too_deep(&mut self, start: usize) -> Node {
        // Skip the rest of this group without recursing.
        let mut level = 1usize;
        while let Some(c) = self.peek() {
            self.i += 1;
            match c {
                '\\' => self.i = (self.i + 1).min(self.end),
                '{' => level += 1,
                '}' => {
                    level -= 1;
                    if level == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
        self.diag("nested too deeply", start, self.i);
        let text: String = self.s[start..self.i].iter().collect();
        Node::new(NodeKind::Error(text), CharRange::new(start, self.i))
    }

    fn parse_primary(&mut self) -> Option<Node> {
        if self.depth >= MAX_DEPTH {
            let start = self.i;
            if self.peek() == Some('{') {
                self.i += 1;
                return Some(self.too_deep(start));
            }
            self.i = (self.i + 1).min(self.end);
            self.diag("nested too deeply", start, self.i);
            let text: String = self.s[start..self.i].iter().collect();
            return Some(Node::new(
                NodeKind::Error(text),
                CharRange::new(start, self.i),
            ));
        }
        self.depth += 1;
        let r = self.primary_inner();
        self.depth -= 1;
        r
    }

    fn primary_inner(&mut self) -> Option<Node> {
        let start = self.i;
        let c = self.peek()?;
        match c {
            '{' => Some(self.parse_group()),
            '\\' => self.parse_command(),
            '^' | '_' => Some(Node::empty(start)),
            '~' => {
                self.i += 1;
                Some(space("0.333em", start, self.i))
            }
            '#' => {
                self.i += 1;
                self.diag("macro parameter in math", start, self.i);
                Some(Node::new(
                    NodeKind::Error("#".into()),
                    CharRange::new(start, self.i),
                ))
            }
            c if c.is_ascii_digit() => {
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    self.i += 1;
                }
                if self.peek() == Some('.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
                    self.i += 1;
                    while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                        self.i += 1;
                    }
                }
                let text: String = self.s[start..self.i].iter().collect();
                Some(Node::new(
                    NodeKind::Number(text),
                    CharRange::new(start, self.i),
                ))
            }
            c => {
                self.i += 1;
                let span = CharRange::new(start, self.i);
                Some(token_for_char(c, span))
            }
        }
    }

    fn parse_command(&mut self) -> Option<Node> {
        let start = self.i;
        let (name, next) = self.command_at(self.i);
        self.i = next;
        if name.is_empty() {
            self.diag("backslash at the end", start, self.i);
            return Some(Node::new(
                NodeKind::Error("\\".into()),
                CharRange::new(start, self.i),
            ));
        }
        let span_here = |p: &Parser| CharRange::new(start, p.i);
        let node = match name.as_str() {
            "frac" | "dfrac" | "tfrac" | "cfrac" => {
                let num = self.parse_arg("numerator");
                let den = self.parse_arg("denominator");
                Node::new(
                    NodeKind::Fraction {
                        num: Box::new(num),
                        den: Box::new(den),
                        bar: true,
                    },
                    span_here(self),
                )
            }
            "binom" | "dbinom" | "tbinom" => {
                let num = self.parse_arg("top of the binomial");
                let den = self.parse_arg("bottom of the binomial");
                let inner = CharRange::new(num.span.start, den.span.end.max(num.span.end));
                let frac = Node::new(
                    NodeKind::Fraction {
                        num: Box::new(num),
                        den: Box::new(den),
                        bar: false,
                    },
                    inner,
                );
                fenced("(", ")", frac, span_here(self))
            }
            "sqrt" => {
                self.skip_ws();
                let index = if self.peek() == Some('[') {
                    self.parse_bracket_arg()
                } else {
                    None
                };
                let radicand = self.parse_arg("radicand");
                Node::new(
                    NodeKind::Root {
                        index: index.map(Box::new),
                        radicand: Box::new(radicand),
                    },
                    span_here(self),
                )
            }
            "left" => self.parse_left(start),
            "middle" => {
                let text = self.parse_delim().unwrap_or_default();
                Node::new(
                    NodeKind::Operator {
                        text,
                        class: OpClass::Relation,
                    },
                    span_here(self),
                )
            }
            "big" | "Big" | "bigg" | "Bigg" | "bigl" | "Bigl" | "biggl" | "Biggl" | "bigr"
            | "Bigr" | "biggr" | "Biggr" | "bigm" | "Bigm" | "biggm" | "Biggm" => {
                let text = self.parse_delim()?;
                let class = if name.ends_with('l') {
                    OpClass::Open
                } else if name.ends_with('r') {
                    OpClass::Close
                } else {
                    delim_class(&text)
                };
                Node::new(NodeKind::Operator { text, class }, span_here(self))
            }
            "begin" => self.parse_env(start),
            "text" | "textrm" | "textit" | "textbf" | "textsf" | "texttt" | "textnormal"
            | "textup" | "mbox" | "hbox" | "emph" => {
                let raw = self.read_group_raw().unwrap_or_default();
                Node::new(NodeKind::Text(raw), span_here(self))
            }
            "operatorname" => {
                if self.peek() == Some('*') {
                    self.i += 1;
                }
                let raw = self.read_group_raw().unwrap_or_default();
                let name: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
                Node::new(NodeKind::Function(name), span_here(self))
            }
            "mathrm" | "mathup" | "mathnormal" => self.style(Variant::Normal, start),
            "mathbf" => self.style(Variant::Bold, start),
            "boldsymbol" | "bm" | "pmb" | "mathbfit" => self.style(Variant::BoldItalic, start),
            "mathit" => self.style(Variant::Italic, start),
            "mathbb" | "Bbb" => self.style(Variant::DoubleStruck, start),
            "mathcal" | "mathscr" => self.style(Variant::Script, start),
            "mathfrak" => self.style(Variant::Fraktur, start),
            "mathsf" => self.style(Variant::SansSerif, start),
            "mathtt" => self.style(Variant::Monospace, start),
            "hat" | "widehat" => self.accent(AccentKind::Hat, start),
            "bar" => self.accent(AccentKind::Bar, start),
            "overline" => self.accent(AccentKind::Overline, start),
            "tilde" | "widetilde" => self.accent(AccentKind::Tilde, start),
            "vec" | "overrightarrow" => self.accent(AccentKind::Vector, start),
            "overleftarrow" => self.accent(AccentKind::LeftArrow, start),
            "overleftrightarrow" => self.accent(AccentKind::LeftRightArrow, start),
            "dot" => self.accent(AccentKind::Dot, start),
            "ddot" => self.accent(AccentKind::DoubleDot, start),
            "dddot" => self.accent(AccentKind::TripleDot, start),
            "acute" => self.accent(AccentKind::Acute, start),
            "grave" => self.accent(AccentKind::Grave, start),
            "breve" => self.accent(AccentKind::Breve, start),
            "check" | "widecheck" => self.accent(AccentKind::Check, start),
            "mathring" => self.accent(AccentKind::Ring, start),
            "overarc" | "overparen" | "wideparen" => self.accent(AccentKind::Arc, start),
            "underline" => self.accent(AccentKind::Underline, start),
            "overbrace" => self.accent(AccentKind::Overbrace, start),
            "underbrace" => self.accent(AccentKind::Underbrace, start),
            "overset" | "stackrel" | "underset" => {
                let script = self.parse_arg("script");
                let base = self.parse_arg("base");
                let (sub, sup) = if name == "underset" {
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
                    span_here(self),
                )
            }
            "xrightarrow" | "xleftarrow" => {
                let arrow_span = span_here(self);
                self.skip_ws();
                let below = if self.peek() == Some('[') {
                    self.parse_bracket_arg()
                } else {
                    None
                };
                let above = self.parse_arg("arrow label");
                let text = if name == "xrightarrow" { "→" } else { "←" };
                let arrow = Node::new(
                    NodeKind::Operator {
                        text: text.into(),
                        class: OpClass::Relation,
                    },
                    arrow_span,
                );
                Node::new(
                    NodeKind::Scripts {
                        base: Box::new(arrow),
                        sub: below.map(Box::new),
                        sup: Some(Box::new(above)),
                        limits: true,
                    },
                    span_here(self),
                )
            }
            "not" => {
                self.skip_ws();
                let target = self.parse_primary();
                match target {
                    Some(Node {
                        kind: NodeKind::Operator { text, class },
                        ..
                    }) => Node::new(
                        NodeKind::Operator {
                            text: negate(&text),
                            class,
                        },
                        span_here(self),
                    ),
                    Some(other) => {
                        let not = Node::new(
                            NodeKind::Operator {
                                text: "¬".into(),
                                class: OpClass::Prefix,
                            },
                            CharRange::new(start, other.span.start.0.max(start)),
                        );
                        let span = span_here(self);
                        row(vec![not, other], span)
                    }
                    None => Node::new(
                        NodeKind::Operator {
                            text: "¬".into(),
                            class: OpClass::Prefix,
                        },
                        span_here(self),
                    ),
                }
            }
            "pmod" => {
                let arg = self.parse_arg("modulus");
                let modw = Node::new(
                    NodeKind::Function("mod".into()),
                    CharRange::new(start, arg.span.start.0.max(start)),
                );
                let inner = CharRange::new(start, arg.span.end.0.max(start));
                let body = row(vec![modw, arg], inner);
                fenced("(", ")", body, span_here(self))
            }
            "color" => {
                let _ = self.read_group_raw();
                return None;
            }
            "textcolor" | "colorbox" => {
                let _ = self.read_group_raw();
                self.parse_arg("colored text")
            }
            "cancel" | "bcancel" | "xcancel" | "sout" => {
                let body = self.parse_arg("crossed-out part");
                Node::new(
                    NodeKind::Enclose {
                        notation: Enclosure::Cancel,
                        body: Box::new(body),
                    },
                    span_here(self),
                )
            }
            "boxed" | "fbox" => {
                let body = self.parse_arg("boxed part");
                Node::new(
                    NodeKind::Enclose {
                        notation: Enclosure::Box,
                        body: Box::new(body),
                    },
                    span_here(self),
                )
            }
            "phantom" | "hphantom" | "vphantom" | "hspace" | "hspace*" | "mspace" => {
                let _ = self.read_group_raw();
                space("0.278em", start, self.i)
            }
            "quad" => space("1em", start, self.i),
            "qquad" => space("2em", start, self.i),
            "," | "thinspace" => space("0.167em", start, self.i),
            ":" | ">" | "medspace" => space("0.222em", start, self.i),
            ";" | "thickspace" => space("0.278em", start, self.i),
            " " | "enspace" => space("0.5em", start, self.i),
            "!" | "negthinspace" | "negmedspace" | "negthickspace" => return None,
            "displaystyle" | "textstyle" | "scriptstyle" | "scriptscriptstyle" | "limits"
            | "nolimits" | "nonumber" | "notag" | "hline" | "mathstrut" | "strut"
            | "allowbreak" | "nobreak" | "relax" | "rm" | "bf" | "it" | "cal" | "sf" | "tt"
            | "vline" => return None,
            "label" | "tag" | "ref" | "eqref" => {
                if self.peek() == Some('*') {
                    self.i += 1;
                }
                let _ = self.read_group_raw();
                return None;
            }
            _ => {
                if let Some(f) = latex_function(&name) {
                    Node::new(NodeKind::Function(f.into()), span_here(self))
                } else if let Some(sym) = latex_symbol(&name) {
                    sym_node(sym, span_here(self))
                } else {
                    self.diag(format!("unknown command \\{name}"), start, self.i);
                    Node::new(NodeKind::Error(format!("\\{name}")), span_here(self))
                }
            }
        };
        Some(node)
    }

    fn style(&mut self, variant: Variant, start: usize) -> Node {
        self.skip_ws();
        let arg_start = self.i;
        // Upright words (`\mathrm{kg}`) stay one identifier.
        if matches!(variant, Variant::Normal | Variant::DoubleStruck)
            && self.peek() == Some('{')
            && let Some(close) = self.find_close_brace(self.i)
        {
            let inner: String = self.s[self.i + 1..close].iter().collect();
            let word = inner.trim();
            if !word.is_empty() && word.chars().all(|c| c.is_ascii_alphabetic()) {
                let lead = inner.len() - inner.trim_start().len();
                let a = self.i + 1 + inner[..lead].chars().count();
                let b = a + word.chars().count();
                self.i = close + 1;
                let span = CharRange::new(start, self.i);
                if variant == Variant::DoubleStruck
                    && let Some(set) = double_struck(word)
                {
                    return Node::new(NodeKind::Ident(set.into()), span);
                }
                let ident = Node::new(NodeKind::Ident(word.into()), CharRange::new(a, b));
                return Node::new(
                    NodeKind::Style {
                        variant,
                        body: Box::new(ident),
                    },
                    span,
                );
            }
        }
        self.i = arg_start;
        let body = self.parse_arg("styled part");
        Node::new(
            NodeKind::Style {
                variant,
                body: Box::new(body),
            },
            CharRange::new(start, self.i),
        )
    }

    fn accent(&mut self, accent: AccentKind, start: usize) -> Node {
        let base = self.parse_arg("accented part");
        Node::new(
            NodeKind::Accent {
                base: Box::new(base),
                accent,
            },
            CharRange::new(start, self.i),
        )
    }

    /// `[…]` optional argument parsed as math (the root index).
    fn parse_bracket_arg(&mut self) -> Option<Node> {
        let open = self.i;
        let close = self.find_close_bracket(open)?;
        self.i = open + 1;
        let saved_end = self.end;
        self.end = close;
        self.depth += 1;
        let rows = self.parse_lines(Ctx::Top);
        self.depth -= 1;
        self.end = saved_end;
        self.i = close + 1;
        let node = if rows.len() == 1 && rows[0].len() == 1 {
            let mut rows = rows;
            let cell = rows.remove(0).remove(0);
            row_or_single(cell.items, open + 1)
        } else {
            Node::new(
                NodeKind::Table {
                    kind: TableKind::Aligned,
                    rows: cells_to_nodes(rows),
                },
                CharRange::new(open, close + 1),
            )
        };
        Some(node)
    }

    /// Index of the `]` matching the `[` at `open`, skipping braced groups.
    fn find_close_bracket(&self, open: usize) -> Option<usize> {
        let mut depth = 0usize;
        let mut j = open + 1;
        while j < self.end {
            match self.s[j] {
                '\\' => j += 1,
                '{' => depth += 1,
                '}' => depth = depth.checked_sub(1)?,
                ']' if depth == 0 => return Some(j),
                _ => {}
            }
            j += 1;
        }
        None
    }

    /// Index of the `}` matching the `{` at `open`.
    fn find_close_brace(&self, open: usize) -> Option<usize> {
        let mut depth = 0usize;
        let mut j = open;
        while j < self.end {
            match self.s[j] {
                '\\' => j += 1,
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(j);
                    }
                }
                _ => {}
            }
            j += 1;
        }
        None
    }

    /// Reads a `{…}` group as raw text (for `\text`, `\operatorname`,
    /// environment names). A single character without braces also counts.
    fn read_group_raw(&mut self) -> Option<String> {
        self.skip_ws();
        let open = self.i;
        match self.peek()? {
            '{' => match self.find_close_brace(open) {
                Some(close) => {
                    self.i = close + 1;
                    Some(self.s[open + 1..close].iter().collect())
                }
                None => {
                    self.i = self.end;
                    self.diag("missing closing brace", open, self.i);
                    Some(self.s[open + 1..self.end].iter().collect())
                }
            },
            '}' | '\\' => None,
            c => {
                self.i += 1;
                Some(c.to_string())
            }
        }
    }

    /// A delimiter after `\left`, `\right`, `\big`, or `\middle`. `.` is the
    /// empty delimiter.
    fn parse_delim(&mut self) -> Option<String> {
        self.skip_ws();
        let c = self.peek()?;
        let text = match c {
            '.' => "",
            '(' => "(",
            ')' => ")",
            '[' => "[",
            ']' => "]",
            '|' => "|",
            '/' => "/",
            '<' => "⟨",
            '>' => "⟩",
            '\\' => {
                let (name, next) = self.command_at(self.i);
                let t = match name.as_str() {
                    "{" | "lbrace" => "{",
                    "}" | "rbrace" => "}",
                    "|" | "Vert" | "lVert" | "rVert" => "‖",
                    "vert" | "lvert" | "rvert" => "|",
                    "langle" => "⟨",
                    "rangle" => "⟩",
                    "lfloor" => "⌊",
                    "rfloor" => "⌋",
                    "lceil" => "⌈",
                    "rceil" => "⌉",
                    "lbrack" => "[",
                    "rbrack" => "]",
                    "backslash" => "\\",
                    "uparrow" => "↑",
                    "downarrow" => "↓",
                    "updownarrow" => "↕",
                    "Uparrow" => "⇑",
                    "Downarrow" => "⇓",
                    "lgroup" => "⟮",
                    "rgroup" => "⟯",
                    "llbracket" => "⟦",
                    "rrbracket" => "⟧",
                    _ => return None,
                };
                self.i = next;
                return Some(t.to_owned());
            }
            _ => return None,
        };
        self.i += 1;
        Some(text.to_owned())
    }

    fn parse_left(&mut self, start: usize) -> Node {
        let open = match self.parse_delim() {
            Some(t) => t,
            None => {
                self.diag("\\left without a delimiter", start, self.i);
                String::new()
            }
        };
        let body_start = self.i;
        self.depth += 1;
        let rows = self.parse_lines(Ctx::Left);
        self.depth -= 1;
        let body = lines_node(rows, body_start, self.i);
        let close = if self.peek_command().as_deref() == Some("right") {
            self.i = self.command_at(self.i).1;
            self.parse_delim().unwrap_or_default()
        } else {
            self.diag("\\left without \\right", start, self.i);
            String::new()
        };
        Node::new(
            NodeKind::Fenced {
                open,
                close,
                body: Box::new(body),
            },
            CharRange::new(start, self.i),
        )
    }

    fn parse_env(&mut self, start: usize) -> Node {
        let Some(raw) = self.read_group_raw() else {
            self.diag("\\begin without an environment name", start, self.i);
            return Node::new(
                NodeKind::Error("\\begin".into()),
                CharRange::new(start, self.i),
            );
        };
        let name = raw.trim().to_owned();
        if matches!(
            name.as_str(),
            "array" | "subarray" | "alignat" | "alignat*" | "alignedat" | "tabular"
        ) {
            let _ = self.read_group_raw();
        }
        // Optional column alignment of starred matrices (`pmatrix*` `[r]`).
        if name.ends_with('*') && name.contains("matrix") {
            self.skip_optional_dimension();
        }
        let body_start = self.i;
        self.depth += 1;
        let rows = self.parse_lines(Ctx::Env);
        self.depth -= 1;
        let body_end = self.i;
        if self.peek_command().as_deref() == Some("end") {
            self.i = self.command_at(self.i).1;
            let end_name = self.read_group_raw().unwrap_or_default();
            if end_name.trim() != name {
                self.diag(
                    format!("\\begin{{{name}}} ended by \\end{{{}}}", end_name.trim()),
                    start,
                    self.i,
                );
            }
        } else {
            self.diag(format!("missing \\end{{{name}}}"), start, self.i);
        }
        let span = CharRange::new(start, self.i);
        let base = name.trim_end_matches('*');
        let fence: Option<(&str, &str)> = match base {
            "pmatrix" => Some(("(", ")")),
            "bmatrix" => Some(("[", "]")),
            "Bmatrix" => Some(("{", "}")),
            "vmatrix" => Some(("|", "|")),
            "Vmatrix" => Some(("‖", "‖")),
            "cases" | "dcases" => Some(("{", "")),
            "rcases" => Some(("", "}")),
            _ => None,
        };
        let kind = match base {
            "matrix" | "smallmatrix" | "pmatrix" | "bmatrix" | "Bmatrix" | "vmatrix"
            | "Vmatrix" => TableKind::Matrix,
            "cases" | "dcases" | "rcases" => TableKind::Cases,
            "aligned" | "align" | "alignat" | "alignedat" | "split" | "gather" | "gathered"
            | "multline" | "eqnarray" | "flalign" => TableKind::Aligned,
            "equation" | "displaymath" | "math" => {
                let mut body = lines_node(rows, body_start, body_end);
                body.span = span;
                return body;
            }
            "array" | "subarray" | "tabular" => TableKind::Array,
            _ => {
                self.diag(format!("unknown environment {name}"), start, self.i);
                TableKind::Array
            }
        };
        let table = Node::new(
            NodeKind::Table {
                kind,
                rows: cells_to_nodes(rows),
            },
            CharRange::new(body_start, body_end),
        );
        match fence {
            Some((o, c)) => fenced(o, c, table, span),
            None => Node::new(table.kind, span),
        }
    }
}

/// A node for parsed lines: a single cell's items, or a stacked table.
fn lines_node(rows: Vec<Vec<Cell>>, start: usize, end: usize) -> Node {
    if rows.len() == 1 && rows[0].len() == 1 {
        let mut rows = rows;
        let cell = rows.remove(0).remove(0);
        let span = items_span(&cell.items).unwrap_or(CharRange::empty(start));
        return row(cell.items, span);
    }
    Node::new(
        NodeKind::Table {
            kind: TableKind::Aligned,
            rows: cells_to_nodes(rows),
        },
        CharRange::new(start, end),
    )
}

fn fenced(open: &str, close: &str, body: Node, span: CharRange) -> Node {
    Node::new(
        NodeKind::Fenced {
            open: open.into(),
            close: close.into(),
            body: Box::new(body),
        },
        span,
    )
}

fn space(width: &str, a: usize, b: usize) -> Node {
    Node::new(NodeKind::Space(width.into()), CharRange::new(a, b))
}

fn sym_node(sym: Sym, span: CharRange) -> Node {
    match sym {
        Sym::Ident(t) => Node::new(NodeKind::Ident(t.into()), span),
        Sym::Op(t, class) => Node::new(
            NodeKind::Operator {
                text: t.into(),
                class,
            },
            span,
        ),
    }
}

/// The node for a character typed directly in math.
pub(crate) fn token_for_char(c: char, span: CharRange) -> Node {
    if let Some(sym) = char_symbol(c) {
        return sym_node(sym, span);
    }
    if c.is_alphabetic() {
        Node::new(NodeKind::Ident(c.to_string()), span)
    } else if c.is_numeric() {
        Node::new(NodeKind::Number(c.to_string()), span)
    } else {
        Node::new(
            NodeKind::Operator {
                text: c.to_string(),
                class: OpClass::Other,
            },
            span,
        )
    }
}

fn delim_class(text: &str) -> OpClass {
    match text {
        "(" | "[" | "{" | "⟨" | "⌊" | "⌈" | "⟮" | "⟦" => OpClass::Open,
        ")" | "]" | "}" | "⟩" | "⌋" | "⌉" | "⟯" | "⟧" => OpClass::Close,
        "|" | "‖" => OpClass::Fence,
        _ => OpClass::Other,
    }
}

/// `\mathbb{R}` and friends as the set symbols.
pub(crate) fn double_struck(word: &str) -> Option<&'static str> {
    Some(match word {
        "R" => "ℝ",
        "N" => "ℕ",
        "Z" => "ℤ",
        "Q" => "ℚ",
        "C" => "ℂ",
        "P" => "ℙ",
        "H" => "ℍ",
        _ => return None,
    })
}

fn default_limits(base: &Node) -> bool {
    match &base.kind {
        NodeKind::Function(f) => takes_limits(f),
        NodeKind::Accent {
            accent: AccentKind::Overbrace | AccentKind::Underbrace,
            ..
        } => true,
        _ => false,
    }
}
