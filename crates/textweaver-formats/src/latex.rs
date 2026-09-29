//! LaTeX loader: a subset of LaTeX documents read natively, without Pandoc
//! (ADR-0035).
//!
//! The source is cut into tokens by a small lexer, then read by an
//! iterative parser with an explicit stack of groups, environments, and
//! arguments, so no input can overflow the thread's stack. The parser runs
//! twice: the first pass learns what a reader needs before it gets there
//! (the numbers `\label` names, captions, which table rows are headers),
//! and the second writes the canonical text.
//!
//! What is read:
//!
//! - **Structure**: `\part` to `\subparagraph` as headings, numbered as
//!   LaTeX numbers them (starred ones are not); the title block from
//!   `\title`, `\author`, and `\date` at `\maketitle`; `abstract`;
//!   `itemize`, `enumerate` (labeled `1.`, `(a)`, `i.`, `A.` by depth), and
//!   `description` lists; `tabular` and its relatives as tables (a first
//!   row followed by a rule is the header row); `table` and `figure` floats
//!   with their captions ("Table 1: ..."; a figure's caption is its
//!   picture's description); `quote`, `quotation`, and `verse`; theorems
//!   declared with `\newtheorem`, and `proof`; `thebibliography`.
//! - **Math**: `$...$`, `\(...\)`, `$$...$$`, `\[...\]`, and the
//!   `equation`, `align`, `gather`, `multline`, `alignat`, `flalign`, and
//!   `eqnarray` environments become `Math` markers over LaTeX with its
//!   delimiters, as the Markdown loader writes math, so speech reads it
//!   with the math engine and writers typeset it. Numbered environments
//!   carry their numbers ("(1)").
//! - **References**: `\label` and `\ref` (also `\eqref`, `\autoref`,
//!   `\cref`, `\pageref`) read the number; a reference to a section is a
//!   link to its heading. `\cite` and its natbib and biblatex relatives
//!   become Pandoc citations (`[@key, p. 12]`, or `@key` for `\citet`),
//!   which reading and the renderer already know.
//! - **Inline**: `\footnote` as the other loaders place footnotes;
//!   `\emph`, `\textbf`, `\underline`, `\texttt`, and the switches `\bf`,
//!   `\itshape`, and their kin as markers; `\url` and `\href` as links;
//!   `\verb`, `verbatim`, `lstlisting`, and `minted` as code; accents
//!   (`\'e`), special characters, and `--`, `---`, and TeX quotes.
//! - **Macros**: `\newcommand`, `\renewcommand`, `\providecommand`, and
//!   `\def` without arguments are expanded, in text and in math;
//!   `\DeclareMathOperator` too. Macros with arguments are not expanded.
//! - **Files**: `\input`, `\include`, `\subfile`, and `\import` read `.tex`
//!   files from the document's own folder or below it, never outside it.
//!
//! Commands textweaver does not know are left out, and their arguments
//! read as text; the document's warnings name them, and with
//! [`LoadOptions::name_skipped_commands`] each is said in place ("(command
//! hl)"). The preamble's text is never read.
//!
//! Limits (see the constants): the file size, the tokens read and made,
//! the parser's steps, macro expansions, nesting, and included files.
//! Past a limit the rest is cut off, or kept without structure, and the
//! document says so.

use std::collections::{BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};

use ropey::Rope;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, DocumentMeta, HEADER_ROW_LABEL, Marker};

use crate::builder::{Builder, OpenId};
use crate::{
    FootnoteMode, LoadError, LoadOptions, Loader, Source, add_warning, decode_bytes, encoding,
    meta_for, note_encoding, title_from_path,
};

/// Largest LaTeX file read, the document or one it includes.
pub const MAX_SOURCE_BYTES: usize = 16 << 20;

/// Most bytes read from included files in all.
pub const MAX_INCLUDE_BYTES: usize = 64 << 20;

/// Most tokens read from files and made by macros and arguments in one
/// pass.
pub const MAX_TOKENS: usize = 2_000_000;

/// Most parser steps in one pass: every token taken counts, so a hostile
/// document costs bounded time.
pub const MAX_STEPS: usize = 8_000_000;

/// Most macro expansions in one pass.
pub const MAX_EXPANSIONS: usize = 10_000;

/// How deep `\input` and `\include` may nest.
pub const MAX_INCLUDE_DEPTH: usize = 8;

/// Most files included in one pass.
pub const MAX_INCLUDES: usize = 64;

/// Longest formula read as math; a longer one is read as text.
const MAX_MATH_BYTES: usize = 64 << 10;

/// Longest environment name, and most tokens in an optional argument.
const MAX_NAME: usize = 64;
const MAX_OPTIONAL: usize = 4096;

/// Most unknown command names the warning lists.
const MAX_NAMED: usize = 20;

/// The warning a document carries when a limit cut it short.
pub const CUT_WARNING: &str = "This LaTeX document is too long or too complex to read in full, so the rest of it was left out.";

/// Loads LaTeX documents (`.tex`, `.latex`, `.ltx`).
#[derive(Clone, Copy, Debug, Default)]
pub struct LatexLoader;

impl Loader for LatexLoader {
    fn id(&self) -> &'static str {
        "latex"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["tex", "latex", "ltx"]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        let bytes = source.read_head(MAX_SOURCE_BYTES + 1)?;
        if bytes.len() > MAX_SOURCE_BYTES {
            return Err(LoadError::Parse(format!(
                "the LaTeX file is larger than {} MB",
                MAX_SOURCE_BYTES >> 20
            )));
        }
        let head = &bytes[..bytes.len().min(encoding::SNIFF_BYTES)];
        if let Some(kind) = encoding::binary_kind(head) {
            let name = title_from_path(source).unwrap_or_else(|| "This document".into());
            return Err(LoadError::Binary(name, kind));
        }
        let decoded = decode_bytes(&bytes, None);
        let mut meta = meta_for(source, self.id());
        note_encoding(&mut meta, &decoded);
        let folder = match source {
            Source::Path(p) if p.is_file() => p.parent().map(Path::to_path_buf),
            _ => None,
        };
        let (text, markers) = convert(&decoded.text, folder.as_deref(), options, &mut meta);
        if meta.title.is_none() {
            meta.title = markers
                .iter()
                .filter(|m| m.kind == MarkerKind::Heading && m.level == 1)
                .min_by_key(|m| m.range.start)
                .map(|m| {
                    text.chars()
                        .skip(m.range.start.0)
                        .take(m.range.len())
                        .collect()
                })
                .or_else(|| title_from_path(source));
        }
        Ok(Document::new(meta, Rope::from_str(&text), markers))
    }
}

/// Converts LaTeX source to canonical text and markers. `folder` is where
/// `\input` looks (none: included files are not read). Fills `meta` with
/// the title, author, language, and warnings.
pub fn convert(
    source: &str,
    folder: Option<&Path>,
    options: &LoadOptions,
    meta: &mut DocumentMeta,
) -> (String, Vec<Marker>) {
    let folder = folder.and_then(|f| f.canonicalize().ok());
    // The first pass learns labels, captions, and header rows.
    let mut first = Parser::new(source, folder.clone(), options, Knowledge::default(), true);
    first.run();
    let known = std::mem::take(&mut first.learn);
    let mut second = Parser::new(source, folder, options, known, false);
    second.run();
    second.finish(meta)
}

// ---------------------------------------------------------------------
// Tokens

/// What an argument does when it starts and ends.
#[derive(Clone, Debug, PartialEq)]
enum Act {
    /// A group, nothing more.
    Plain,
    /// A marker over the argument.
    Mark(Marker),
    /// The argument is read into its own text and handled at its end.
    Capture(Cap),
    /// A heading, with its number said first.
    Heading { level: u8, number: Option<String> },
    /// A caption line ("Table 1: ..."), under an `Image` marker for a
    /// figure with a picture.
    Caption {
        prefix: String,
        image: Option<String>,
    },
}

/// What a captured argument is for.
#[derive(Clone, Debug, PartialEq)]
enum Cap {
    Discard,
    Title,
    Author,
    Footnote,
    /// A heading's text, for the link a `\ref` to it follows.
    HeadingTarget {
        number: String,
        kind: &'static str,
    },
    /// A float's caption, for its table's label.
    Caption(usize),
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    /// `\name`.
    Cmd(String),
    /// A control symbol, `\%`.
    Sym(char),
    Open,
    Close,
    Amp,
    LBrack,
    RBrack,
    /// A run of ordinary characters.
    Text(String),
    Space,
    /// A blank line.
    Par,
    Math {
        display: bool,
        body: String,
    },
    /// A math environment, `\begin{align}...\end{align}`.
    MathEnv {
        name: String,
        body: String,
    },
    Verbatim {
        lang: Option<String>,
        body: String,
        inline: bool,
    },
    /// The raw group after `\url` and `\href`.
    Raw(String),
    /// A skipped environment (`comment`, `tikzpicture`).
    Skip,
    // Made by the parser, never by the lexer:
    Begin(usize, Box<Act>),
    EndArg(usize),
    EndInput,
}

impl Tok {
    fn is_sentinel(&self) -> bool {
        matches!(self, Tok::Begin(..) | Tok::EndArg(_) | Tok::EndInput)
    }
}

const MATH_ENVS: &[&str] = &[
    "equation",
    "equation*",
    "align",
    "align*",
    "alignat",
    "alignat*",
    "flalign",
    "flalign*",
    "gather",
    "gather*",
    "multline",
    "multline*",
    "eqnarray",
    "eqnarray*",
    "displaymath",
    "math",
];

const CODE_ENVS: &[&str] = &["verbatim", "verbatim*", "Verbatim", "lstlisting", "minted"];

/// Environments left out whole: comments and drawings.
const SKIP_ENVS: &[&str] = &[
    "comment",
    "tikzpicture",
    "pgfpicture",
    "picture",
    "tikzcd",
    "forest",
];

/// Cuts LaTeX source into tokens: at most `budget`, and true when it had to
/// stop there.
fn lex(s: &str, budget: usize) -> (Vec<Tok>, bool) {
    let mut l = Lexer {
        s,
        pos: 0,
        out: Vec::new(),
        budget,
        truncated: false,
    };
    l.run();
    (l.out, l.truncated)
}

struct Lexer<'s> {
    s: &'s str,
    pos: usize,
    out: Vec<Tok>,
    budget: usize,
    truncated: bool,
}

impl<'s> Lexer<'s> {
    fn rest(&self) -> &'s str {
        &self.s[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn push(&mut self, t: Tok) {
        if self.out.len() >= self.budget {
            self.truncated = true;
        } else {
            self.out.push(t);
        }
    }

    fn flush(&mut self, text: &mut String) {
        if !text.is_empty() {
            let t = std::mem::take(text);
            self.push(Tok::Text(t));
        }
    }

    fn run(&mut self) {
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if self.truncated {
                return;
            }
            match c {
                '\\' | '{' | '}' | '$' | '&' | '%' | '~' | '[' | ']' => {
                    self.flush(&mut text);
                    self.special(c);
                }
                c if c.is_whitespace() => {
                    self.flush(&mut text);
                    self.whitespace();
                }
                _ => {
                    text.push(c);
                    self.pos += c.len_utf8();
                }
            }
        }
        self.flush(&mut text);
    }

    fn whitespace(&mut self) {
        let mut newlines = 0;
        while let Some(c) = self.peek() {
            if !c.is_whitespace() {
                break;
            }
            if c == '\n' {
                newlines += 1;
            }
            self.pos += c.len_utf8();
        }
        self.push(if newlines >= 2 { Tok::Par } else { Tok::Space });
    }

    fn special(&mut self, c: char) {
        self.pos += 1;
        match c {
            '{' => self.push(Tok::Open),
            '}' => self.push(Tok::Close),
            '&' => self.push(Tok::Amp),
            '[' => self.push(Tok::LBrack),
            ']' => self.push(Tok::RBrack),
            '~' => self.push(Tok::Space),
            '%' => self.comment(),
            '$' => self.dollar(),
            _ => self.backslash(),
        }
    }

    /// A comment runs to the end of the line and takes the line break and
    /// the next line's indentation with it.
    fn comment(&mut self) {
        match self.rest().find('\n') {
            Some(i) => self.pos += i + 1,
            None => self.pos = self.s.len(),
        }
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn dollar(&mut self) {
        let display = self.rest().starts_with('$');
        if display {
            self.pos += 1;
        }
        let close = if display {
            Close::Dollars
        } else {
            Close::Dollar
        };
        match scan_math(self.rest(), close) {
            Some((body, used)) => {
                let body = body.to_owned();
                self.pos += used;
                self.push(Tok::Math { display, body });
            }
            None => self.push(Tok::Text(if display { "$$" } else { "$" }.into())),
        }
    }

    fn backslash(&mut self) {
        let Some(c) = self.peek() else {
            self.push(Tok::Text("\\".into()));
            return;
        };
        if c.is_ascii_alphabetic() {
            let start = self.pos;
            let end = self
                .rest()
                .find(|c: char| !c.is_ascii_alphabetic())
                .map_or(self.s.len(), |i| self.pos + i);
            self.pos = end;
            let name = &self.s[start..end];
            self.control_word(name);
            return;
        }
        self.pos += c.len_utf8();
        let math = match c {
            '(' => Some((false, Close::Escaped(')'))),
            '[' => Some((true, Close::Escaped(']'))),
            _ => None,
        };
        if let Some((display, close)) = math
            && let Some((body, used)) = scan_math(self.rest(), close)
        {
            let body = body.to_owned();
            self.pos += used;
            self.push(Tok::Math { display, body });
            return;
        }
        self.push(Tok::Sym(c));
    }

    fn control_word(&mut self, name: &str) {
        match name {
            "begin" => {
                if !self.raw_environment() {
                    self.push(Tok::Cmd(name.into()));
                }
            }
            "verb" => {
                if !self.inline_verbatim(false) {
                    self.push(Tok::Cmd(name.into()));
                }
            }
            "lstinline" | "mintinline" => {
                if !self.inline_verbatim(true) {
                    self.push(Tok::Cmd(name.into()));
                }
            }
            "url" | "href" | "path" => {
                self.push(Tok::Cmd(name.into()));
                let save = self.pos;
                self.skip_blanks();
                match self.raw_group() {
                    Some(raw) => self.push(Tok::Raw(raw)),
                    None => self.pos = save,
                }
            }
            _ => self.push(Tok::Cmd(name.into())),
        }
    }

    fn skip_blanks(&mut self) {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// A `{...}` group read without tokenizing (braces inside counted).
    fn raw_group(&mut self) -> Option<String> {
        if self.peek() != Some('{') {
            return None;
        }
        let rest = self.rest();
        let mut depth = 0usize;
        for (i, c) in rest.char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        let body = rest[1..i].to_owned();
                        self.pos += i + 1;
                        return Some(body);
                    }
                }
                '\n' if rest[..i].ends_with('\n') => return None,
                _ => {}
            }
            if i > MAX_MATH_BYTES {
                return None;
            }
        }
        None
    }

    /// `\verb|x|`, `\lstinline[opts]{x}`: code on one line.
    fn inline_verbatim(&mut self, braces: bool) -> bool {
        let save = self.pos;
        let mut lang = None;
        if braces && self.peek() == Some('[') {
            let Some(end) = self.rest().find(']') else {
                return false;
            };
            lang = language_option(&self.rest()[1..end]);
            self.pos += end + 1;
        }
        if braces && self.peek() == Some('{') {
            // mintinline{lang}{code}: the first group is the language.
            let first = self.raw_group();
            let second = if self.peek() == Some('{') {
                self.raw_group()
            } else {
                None
            };
            let (l, body) = match (first, second) {
                (Some(l), Some(b)) => (Some(l), b),
                (Some(b), None) => (lang.clone(), b),
                _ => {
                    self.pos = save;
                    return false;
                }
            };
            self.push(Tok::Verbatim {
                lang: l.or(lang),
                body,
                inline: true,
            });
            return true;
        }
        if !braces && self.peek() == Some('*') {
            self.pos += 1;
        }
        let Some(delim) = self.peek() else {
            self.pos = save;
            return false;
        };
        if delim.is_ascii_alphabetic() || delim.is_whitespace() {
            self.pos = save;
            return false;
        }
        let body_start = self.pos + delim.len_utf8();
        let rest = &self.s[body_start..];
        match rest.find([delim, '\n']) {
            Some(i) if rest[i..].starts_with(delim) => {
                self.push(Tok::Verbatim {
                    lang,
                    body: rest[..i].to_owned(),
                    inline: true,
                });
                self.pos = body_start + i + delim.len_utf8();
                true
            }
            _ => {
                self.pos = save;
                false
            }
        }
    }

    /// `\begin{name}` of a math, code, or skipped environment: read raw up
    /// to its `\end{name}`. False (nothing consumed) for any other.
    fn raw_environment(&mut self) -> bool {
        let save = self.pos;
        self.skip_blanks();
        let rest = self.rest();
        let name = rest
            .strip_prefix('{')
            .and_then(|r| r.find('}').filter(|&i| i <= MAX_NAME).map(|i| &r[..i]));
        let Some(name) = name else {
            self.pos = save;
            return false;
        };
        let math = MATH_ENVS.contains(&name);
        let code = CODE_ENVS.contains(&name);
        let skip = SKIP_ENVS.contains(&name);
        if !(math || code || skip) {
            self.pos = save;
            return false;
        }
        let body_start = self.pos + name.len() + 2;
        let end = format!("\\end{{{name}}}");
        let Some(i) = self.s[body_start..].find(&end) else {
            self.pos = save;
            return false;
        };
        let body = &self.s[body_start..body_start + i];
        self.pos = body_start + i + end.len();
        let name = name.to_owned();
        if skip {
            self.push(Tok::Skip);
        } else if math {
            if body.len() > MAX_MATH_BYTES {
                self.push(Tok::Text(body.to_owned()));
            } else {
                self.push(Tok::MathEnv {
                    name,
                    body: body.to_owned(),
                });
            }
        } else {
            let (lang, body) = code_options(&name, body);
            self.push(Tok::Verbatim {
                lang,
                body,
                inline: false,
            });
        }
        true
    }
}

/// How a math region ends.
#[derive(Clone, Copy)]
enum Close {
    Dollar,
    Dollars,
    Escaped(char),
}

/// The body of a math region at the start of `s` and the bytes it used
/// with its closing delimiter, or `None` when it does not close (within
/// [`MAX_MATH_BYTES`], and for `$`, before a blank line).
fn scan_math(s: &str, close: Close) -> Option<(&str, usize)> {
    let mut it = s.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        if i > MAX_MATH_BYTES {
            return None;
        }
        match c {
            '\\' => {
                if let Some(&(_, n)) = it.peek() {
                    if let Close::Escaped(e) = close
                        && n == e
                    {
                        return Some((&s[..i], i + 1 + n.len_utf8()));
                    }
                    it.next();
                }
            }
            '$' => match close {
                Close::Dollar => return (i > 0).then(|| (&s[..i], i + 1)),
                Close::Dollars if s[i + 1..].starts_with('$') => return Some((&s[..i], i + 2)),
                _ => {}
            },
            '\n' if matches!(close, Close::Dollar) => {
                // A blank line ends a paragraph, and inline math with it.
                let after = s[i + 1..].trim_start_matches([' ', '\t']);
                if after.starts_with('\n') {
                    return None;
                }
            }
            _ => {}
        }
    }
    None
}

/// The language a listing's options name (`language=Python`).
fn language_option(opts: &str) -> Option<String> {
    opts.split(',').find_map(|o| {
        let (k, v) = o.split_once('=')?;
        (k.trim() == "language")
            .then(|| v.trim().trim_matches(['{', '}']).to_owned())
            .filter(|v| !v.is_empty())
    })
}

/// A code environment's language and its body without its options.
fn code_options(name: &str, body: &str) -> (Option<String>, String) {
    let mut rest = body;
    let mut lang = None;
    if matches!(name, "lstlisting" | "minted" | "Verbatim")
        && let Some(r) = rest.strip_prefix('[')
        && let Some(end) = r.find(']')
    {
        lang = language_option(&r[..end]);
        rest = &r[end + 1..];
    }
    if name == "minted"
        && let Some(r) = rest.strip_prefix('{')
        && let Some(end) = r.find('}')
    {
        lang = Some(r[..end].trim().to_owned()).filter(|l| !l.is_empty());
        rest = &r[end + 1..];
    }
    let rest = rest.strip_prefix('\r').unwrap_or(rest);
    (lang, rest.strip_prefix('\n').unwrap_or(rest).to_owned())
}

/// Tokens as plain text: for labels, keys, names, and notes.
fn plain(toks: &[Tok]) -> String {
    let mut s = String::new();
    for t in toks {
        match t {
            Tok::Text(x) | Tok::Raw(x) => s.push_str(x),
            Tok::Space | Tok::Par => s.push(' '),
            Tok::Sym(c) if "%&$#_{}".contains(*c) => s.push(*c),
            Tok::LBrack => s.push('['),
            Tok::RBrack => s.push(']'),
            Tok::Math { body, .. } => s.push_str(body),
            _ => {}
        }
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Tokens written back as LaTeX: for macro bodies used in math.
fn to_source(toks: &[Tok]) -> String {
    let mut s = String::new();
    for t in toks {
        match t {
            Tok::Cmd(n) => {
                s.push('\\');
                s.push_str(n);
                s.push(' ');
            }
            Tok::Sym(c) => {
                s.push('\\');
                s.push(*c);
            }
            Tok::Open => s.push('{'),
            Tok::Close => s.push('}'),
            Tok::Amp => s.push('&'),
            Tok::LBrack => s.push('['),
            Tok::RBrack => s.push(']'),
            Tok::Text(x) => s.push_str(x),
            Tok::Raw(x) => {
                s.push('{');
                s.push_str(x);
                s.push('}');
            }
            Tok::Space | Tok::Par => s.push(' '),
            Tok::Math { body, .. } => s.push_str(body),
            Tok::MathEnv { name, body } => {
                s.push_str(&format!("\\begin{{{name}}}{body}\\end{{{name}}}"));
            }
            Tok::Verbatim { body, .. } => s.push_str(body),
            _ => {}
        }
    }
    s.trim().to_owned()
}

/// TeX's ligatures in running text: dashes and quotes.
fn ligatures(s: &str) -> String {
    if !s.contains(['-', '`', '\'']) {
        return s.to_owned();
    }
    s.replace("---", "\u{2014}")
        .replace("--", "\u{2013}")
        .replace("``", "\u{201c}")
        .replace("''", "\u{201d}")
        .replace('`', "\u{2018}")
}

// ---------------------------------------------------------------------
// What the first pass learns

#[derive(Clone, Debug, Default)]
struct Label {
    number: String,
    /// The link a reference follows (a heading's), when there is one.
    target: Option<String>,
    /// "Section", "Table", ... for `\autoref`.
    kind: String,
}

#[derive(Debug, Default)]
struct Knowledge {
    labels: HashMap<String, Label>,
    /// Per table, in order: its first row is a header row.
    header_rows: Vec<bool>,
    /// Per float, in order: its caption.
    captions: Vec<Option<String>>,
    /// Per float, in order: its first picture.
    graphics: Vec<Option<String>>,
    chapters: bool,
}

// ---------------------------------------------------------------------
// The parser

/// A marker opened in one of the parser's builders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Opened {
    ctx: usize,
    id: OpenId,
}

#[derive(Debug)]
enum FrameKind {
    Brace,
    Env(String),
    Arg(usize, Act),
}

#[derive(Debug)]
struct Frame {
    kind: FrameKind,
    opened: Vec<Opened>,
}

#[derive(Debug)]
struct ListState {
    frame: usize,
    kind: ListKind,
    marker: Option<Opened>,
    item: Option<Opened>,
    next: i64,
    /// Depth among enumerate lists, for the label style.
    enum_depth: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ListKind {
    Itemize,
    Enumerate,
    Description,
    Bibliography,
}

#[derive(Debug)]
struct TableState {
    frame: usize,
    /// Inside another table's cell: read as running text.
    nested: bool,
    index: usize,
    marker: Option<Opened>,
    row: Option<Opened>,
    cell: Option<Opened>,
    /// Switch markers opened in the current cell.
    cell_opened: Vec<Opened>,
    rows_done: usize,
    /// The first row has ended and nothing but rules has come since.
    after_first_row: bool,
}

#[derive(Debug)]
struct FloatState {
    frame: usize,
    index: usize,
    kind: &'static str,
    number: usize,
}

struct Capture {
    ctx: usize,
    builder: Builder,
}

struct Theorem {
    display: String,
    counter: String,
    numbered: bool,
}

struct Parser<'a> {
    options: &'a LoadOptions,
    folder: Option<PathBuf>,
    learning: bool,
    known: Knowledge,
    learn: Knowledge,

    pending: Vec<Tok>,
    budget: usize,
    steps: usize,
    expansions: usize,
    next_id: usize,
    done: bool,
    cut: bool,
    flattened: bool,

    main: Builder,
    sink: Builder,
    captures: Vec<Capture>,
    next_ctx: usize,
    preamble: bool,

    frames: Vec<Frame>,
    overflow: usize,
    root_opened: Vec<Opened>,
    lists: Vec<ListState>,
    tables: Vec<TableState>,
    floats: Vec<FloatState>,
    in_heading: usize,
    para: Option<Opened>,

    // Document state.
    counters: [u32; 7],
    appendix: bool,
    chapters: bool,
    equations: u32,
    table_count: usize,
    float_count: usize,
    figure_number: usize,
    table_number: usize,
    theorem_counts: HashMap<String, u32>,
    theorems: HashMap<String, Theorem>,
    footnote_count: usize,
    notes: Vec<(String, String)>,
    target: Option<Label>,

    macros: HashMap<String, Vec<Tok>>,
    math_macros: HashMap<String, String>,
    title: Option<Vec<Tok>>,
    author: Option<Vec<Tok>>,
    date: Option<Vec<Tok>>,
    meta_title: Option<String>,
    meta_author: Option<String>,
    language: Option<String>,

    include_depth: usize,
    includes: usize,
    include_bytes: usize,
    unknown: BTreeSet<String>,
    unknown_envs: BTreeSet<String>,
    skipped_includes: BTreeSet<String>,
    arg_macros: BTreeSet<String>,
}

impl<'a> Parser<'a> {
    fn new(
        source: &str,
        folder: Option<PathBuf>,
        options: &'a LoadOptions,
        known: Knowledge,
        learning: bool,
    ) -> Self {
        let (tokens, truncated) = lex(source, MAX_TOKENS);
        let budget = MAX_TOKENS.saturating_sub(tokens.len());
        let preamble = source.contains("\\begin{document}");
        let chapters = known.chapters || source.contains("\\chapter");
        let mut pending = tokens;
        pending.reverse();
        Parser {
            options,
            folder,
            learning,
            known,
            learn: Knowledge::default(),
            pending,
            budget,
            steps: 0,
            expansions: 0,
            next_id: 0,
            done: false,
            cut: truncated,
            flattened: false,
            main: Builder::new(),
            sink: Builder::new(),
            captures: Vec::new(),
            next_ctx: 2,
            preamble,
            frames: Vec::new(),
            overflow: 0,
            root_opened: Vec::new(),
            lists: Vec::new(),
            tables: Vec::new(),
            floats: Vec::new(),
            in_heading: 0,
            para: None,
            counters: [0; 7],
            appendix: false,
            chapters,
            equations: 0,
            table_count: 0,
            float_count: 0,
            figure_number: 0,
            table_number: 0,
            theorem_counts: HashMap::new(),
            theorems: HashMap::new(),
            footnote_count: 0,
            notes: Vec::new(),
            target: None,
            macros: HashMap::new(),
            math_macros: HashMap::new(),
            title: None,
            author: None,
            date: None,
            meta_title: None,
            meta_author: None,
            language: None,
            include_depth: 0,
            includes: 0,
            include_bytes: 0,
            unknown: BTreeSet::new(),
            unknown_envs: BTreeSet::new(),
            skipped_includes: BTreeSet::new(),
            arg_macros: BTreeSet::new(),
        }
    }

    // --- builders -----------------------------------------------------

    fn ctx(&self) -> usize {
        match self.captures.last() {
            Some(c) => c.ctx,
            None if self.preamble => 1,
            None => 0,
        }
    }

    fn b(&mut self) -> &mut Builder {
        if let Some(c) = self.captures.last_mut() {
            return &mut c.builder;
        }
        if self.preamble {
            &mut self.sink
        } else {
            &mut self.main
        }
    }

    fn builder_for(&mut self, ctx: usize) -> Option<&mut Builder> {
        match ctx {
            0 => Some(&mut self.main),
            1 => Some(&mut self.sink),
            _ => self
                .captures
                .iter_mut()
                .find(|c| c.ctx == ctx)
                .map(|c| &mut c.builder),
        }
    }

    fn open(&mut self, m: Marker) -> Opened {
        let ctx = self.ctx();
        Opened {
            ctx,
            id: self.b().open(m),
        }
    }

    fn open_here(&mut self, m: Marker) -> Opened {
        let ctx = self.ctx();
        Opened {
            ctx,
            id: self.b().open_here(m),
        }
    }

    fn close(&mut self, o: Opened) {
        if let Some(b) = self.builder_for(o.ctx) {
            b.close(o.id);
        }
    }

    // --- tokens -------------------------------------------------------

    fn pop(&mut self) -> Option<Tok> {
        if self.steps >= MAX_STEPS {
            self.cut = true;
            return None;
        }
        self.steps += 1;
        self.pending.pop()
    }

    fn peek(&self) -> Option<&Tok> {
        self.pending.last()
    }

    fn inject(&mut self, mut seq: Vec<Tok>) {
        if seq.len() > self.budget {
            seq.truncate(self.budget);
            self.cut = true;
        }
        self.budget -= seq.len();
        self.pending.extend(seq.into_iter().rev());
    }

    /// Queues `toks` as an argument that does `act`.
    fn inject_arg(&mut self, act: Act, toks: Vec<Tok>) {
        let seq = self.arg_seq(act, toks);
        self.inject(seq);
    }

    fn arg_seq(&mut self, act: Act, toks: Vec<Tok>) -> Vec<Tok> {
        let id = self.next_id;
        self.next_id += 1;
        let mut seq = Vec::with_capacity(toks.len() + 2);
        seq.push(Tok::Begin(id, Box::new(act)));
        seq.extend(toks);
        seq.push(Tok::EndArg(id));
        seq
    }

    fn skip_spaces(&mut self) {
        while matches!(self.peek(), Some(Tok::Space)) {
            self.pop();
        }
    }

    /// A `*` right after a command.
    fn star(&mut self) -> bool {
        let Some(Tok::Text(t)) = self.pending.last_mut() else {
            return false;
        };
        let Some(rest) = t.strip_prefix('*') else {
            return false;
        };
        if rest.is_empty() {
            self.pending.pop();
        } else {
            *t = rest.to_owned();
        }
        true
    }

    /// A mandatory argument: a braced group, else the next token (the
    /// first character of a text run). Spaces before it are skipped.
    fn arg(&mut self) -> Vec<Tok> {
        self.skip_spaces();
        match self.peek() {
            Some(Tok::Open) => {
                self.pop();
                self.group_body()
            }
            Some(Tok::Text(_)) => {
                let Some(Tok::Text(t)) = self.pop() else {
                    return Vec::new();
                };
                let mut chars = t.chars();
                let first = chars.next().map(String::from).unwrap_or_default();
                let rest: String = chars.collect();
                if !rest.is_empty() {
                    self.pending.push(Tok::Text(rest));
                }
                vec![Tok::Text(first)]
            }
            Some(t) if t.is_sentinel() => Vec::new(),
            Some(Tok::Close | Tok::Par) | None => Vec::new(),
            Some(_) => self.pop().into_iter().collect(),
        }
    }

    /// The tokens up to the `}` that closes an open `{` (taken), stopping
    /// early at a parser sentinel.
    fn group_body(&mut self) -> Vec<Tok> {
        let mut depth = 0usize;
        let mut out = Vec::new();
        loop {
            match self.peek() {
                None => break,
                Some(t) if t.is_sentinel() => break,
                _ => {}
            }
            let Some(t) = self.pop() else { break };
            match t {
                Tok::Open => depth += 1,
                Tok::Close if depth == 0 => break,
                Tok::Close => depth -= 1,
                _ => {}
            }
            out.push(t);
        }
        out
    }

    /// An optional `[...]` argument, when one follows (after spaces when
    /// `spaces`).
    fn opt(&mut self, spaces: bool) -> Option<Vec<Tok>> {
        if spaces {
            self.skip_spaces();
        }
        if self.peek() != Some(&Tok::LBrack) {
            return None;
        }
        self.pop();
        let mut depth = 0usize;
        let mut out = Vec::new();
        loop {
            if out.len() > MAX_OPTIONAL {
                // Not an optional argument after all: put it back.
                out.reverse();
                self.pending.extend(out);
                self.pending.push(Tok::Text("[".into()));
                return None;
            }
            match self.peek() {
                None => break,
                Some(t) if t.is_sentinel() => break,
                _ => {}
            }
            let Some(t) = self.pop() else { break };
            match t {
                Tok::Open => depth += 1,
                Tok::Close => depth = depth.saturating_sub(1),
                Tok::RBrack if depth == 0 => break,
                _ => {}
            }
            out.push(t);
        }
        Some(out)
    }

    fn skip_args(&mut self, n: usize) {
        for _ in 0..n {
            self.arg();
        }
    }

    // --- frames -------------------------------------------------------

    fn push_frame(&mut self, kind: FrameKind) -> bool {
        if self.frames.len() >= crate::MAX_NESTING {
            self.flattened = true;
            return false;
        }
        self.frames.push(Frame {
            kind,
            opened: Vec::new(),
        });
        true
    }

    /// Records a marker to close when the innermost scope ends: a table
    /// cell, else the innermost frame.
    fn attach(&mut self, o: Opened) {
        let top = self.frames.len().checked_sub(1);
        if let Some(t) = self.tables.last_mut()
            && Some(t.frame) == top
            && !t.nested
            && t.cell.is_some()
        {
            t.cell_opened.push(o);
            return;
        }
        match self.frames.last_mut() {
            Some(f) => f.opened.push(o),
            None => self.root_opened.push(o),
        }
    }

    /// Pops the top frame and ends what it began.
    fn pop_frame(&mut self) {
        let Some(frame) = self.frames.pop() else {
            return;
        };
        let index = self.frames.len();
        for o in frame.opened.into_iter().rev() {
            self.close(o);
        }
        if self.tables.last().is_some_and(|t| t.frame == index) {
            self.end_table();
        }
        if self.lists.last().is_some_and(|l| l.frame == index) {
            self.end_list();
        }
        if self.floats.last().is_some_and(|f| f.frame == index) {
            self.floats.pop();
            self.b().paragraph_break();
        }
        match frame.kind {
            FrameKind::Arg(_, act) => self.end_act(act),
            FrameKind::Env(name) => self.end_env_effects(&name),
            FrameKind::Brace => {}
        }
    }

    // --- the loop -----------------------------------------------------

    fn run(&mut self) {
        while !self.done {
            let Some(t) = self.pop() else { break };
            self.step(t);
        }
        while !self.frames.is_empty() {
            self.pop_frame();
        }
    }

    fn step(&mut self, t: Tok) {
        match t {
            Tok::Text(s) => self.emit_text(&ligatures(&s)),
            Tok::Space => self.b().space(),
            Tok::Par => self.par(),
            Tok::Open => {
                if !self.push_frame(FrameKind::Brace) {
                    self.overflow += 1;
                }
            }
            Tok::Close => {
                if self.overflow > 0 {
                    self.overflow -= 1;
                } else if matches!(
                    self.frames.last(),
                    Some(Frame {
                        kind: FrameKind::Brace,
                        ..
                    })
                ) {
                    self.pop_frame();
                }
            }
            Tok::Amp => self.amp(),
            Tok::LBrack => self.emit_text("["),
            Tok::RBrack => self.emit_text("]"),
            Tok::Sym(c) => self.symbol(c),
            Tok::Cmd(name) => self.command(&name),
            Tok::Math { display, body } => self.math(display, None, &body),
            Tok::MathEnv { name, body } => self.math(true, Some(&name), &body),
            Tok::Verbatim { lang, body, inline } => self.verbatim(lang, &body, inline),
            Tok::Raw(s) => self.emit_text(&s),
            Tok::Skip => {}
            Tok::Begin(id, act) => self.begin_act(id, *act),
            Tok::EndArg(id) => self.end_arg(id),
            Tok::EndInput => self.include_depth = self.include_depth.saturating_sub(1),
        }
    }

    fn begin_act(&mut self, id: usize, act: Act) {
        if !self.push_frame(FrameKind::Arg(id, Act::Plain)) {
            return;
        }
        match &act {
            Act::Plain => {}
            Act::Mark(m) => {
                self.text_start();
                let o = self.open(m.clone());
                self.attach(o);
            }
            Act::Capture(_) => {
                let ctx = self.next_ctx;
                self.next_ctx += 1;
                self.captures.push(Capture {
                    ctx,
                    builder: Builder::new(),
                });
            }
            Act::Heading { level, number } => {
                self.end_para();
                self.block_break();
                self.in_heading += 1;
                let o = self
                    .open(Marker::new(MarkerKind::Heading, CharRange::empty(0)).with_level(*level));
                self.attach(o);
                if let Some(n) = number {
                    let n = n.clone();
                    self.b().text(&n);
                    self.b().space();
                }
            }
            Act::Caption { prefix, image } => {
                self.end_para();
                self.block_break();
                if let Some(img) = image {
                    let o = self.open(
                        Marker::new(MarkerKind::Image, CharRange::empty(0))
                            .with_reference(img.clone()),
                    );
                    self.attach(o);
                }
                let prefix = prefix.clone();
                self.b().text(&prefix);
                self.b().space();
            }
        }
        if let Some(Frame {
            kind: FrameKind::Arg(_, a),
            ..
        }) = self.frames.last_mut()
        {
            *a = act;
        }
    }

    fn end_arg(&mut self, id: usize) {
        let Some(i) = self
            .frames
            .iter()
            .rposition(|f| matches!(f.kind, FrameKind::Arg(fid, _) if fid == id))
        else {
            return;
        };
        while self.frames.len() > i {
            self.pop_frame();
        }
    }

    /// What an argument does at its end (its markers are closed already).
    fn end_act(&mut self, act: Act) {
        match act {
            Act::Plain | Act::Mark(_) => {}
            Act::Heading { .. } => {
                self.in_heading = self.in_heading.saturating_sub(1);
                self.block_break();
            }
            Act::Caption { .. } => self.block_break(),
            Act::Capture(cap) => {
                let Some(c) = self.captures.pop() else {
                    return;
                };
                let (text, _) = c.builder.finish();
                let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
                self.captured(cap, text);
            }
        }
    }

    fn captured(&mut self, cap: Cap, text: String) {
        match cap {
            Cap::Discard => {}
            Cap::Title => self.meta_title = Some(text).filter(|t| !t.is_empty()),
            Cap::Author => self.meta_author = Some(text).filter(|t| !t.is_empty()),
            Cap::Footnote => self.footnote_text(text),
            Cap::HeadingTarget { number, kind } => {
                let full = if number.is_empty() {
                    text
                } else {
                    format!("{number} {text}")
                };
                self.target = Some(Label {
                    number,
                    target: Some(format!("#{}", textweaver_text::slug::slugify(&full))),
                    kind: kind.into(),
                });
            }
            Cap::Caption(index) => {
                if let Some(slot) = self.learn.captions.get_mut(index) {
                    *slot = Some(text);
                }
            }
        }
    }

    // --- text and breaks ----------------------------------------------

    /// Before content: a table cell to put it in, or a paragraph.
    fn text_start(&mut self) {
        if let Some(i) = self.tables.iter().rposition(|t| !t.nested) {
            self.ensure_cell(i);
            return;
        }
        if self.para.is_none()
            && self.captures.is_empty()
            && !self.preamble
            && self.lists.is_empty()
            && self.in_heading == 0
        {
            let o = self.open(Marker::new(MarkerKind::Paragraph, CharRange::empty(0)));
            self.para = Some(o);
        }
    }

    fn end_para(&mut self) {
        if let Some(p) = self.para.take() {
            self.close(p);
        }
    }

    fn emit_text(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        self.text_start();
        self.b().text(s);
    }

    fn in_cell(&self) -> bool {
        !self.tables.is_empty()
    }

    /// A paragraph break, as its context allows: a space in a table cell
    /// or a heading, a line break in a list.
    fn par(&mut self) {
        self.end_para();
        if self.in_cell() || self.in_heading > 0 || !self.captures.is_empty() {
            self.b().space();
        } else if !self.lists.is_empty() {
            self.b().line_break();
        } else {
            self.b().paragraph_break();
        }
    }

    /// The break around a block (heading, list, display): a paragraph
    /// break, or less inside a cell or list.
    fn block_break(&mut self) {
        if self.in_cell() || !self.captures.is_empty() {
            self.b().space();
        } else if !self.lists.is_empty() {
            self.b().line_break();
        } else {
            self.b().paragraph_break();
        }
    }

    fn line_break(&mut self) {
        if self.in_cell() || self.in_heading > 0 || !self.captures.is_empty() {
            self.b().space();
        } else {
            self.b().line_break();
        }
    }

    fn symbol(&mut self, c: char) {
        match c {
            '\\' => self.newline(),
            '%' | '&' | '$' | '#' | '_' | '{' | '}' => self.emit_text(&c.to_string()),
            ' ' | ',' | ';' | ':' | '>' | '\n' | '\t' => self.b().space(),
            '!' | '/' | '-' | '@' | '*' => {}
            '\'' | '`' | '^' | '"' | '~' | '=' | '.' => self.accent(c),
            '(' | '[' | ')' | ']' => self.emit_text(&c.to_string()),
            other => self.emit_text(&other.to_string()),
        }
    }

    /// `\\`: the end of a table row, else a line break.
    fn newline(&mut self) {
        self.star();
        // `\\[2pt]`: the space is not read.
        if self.peek() == Some(&Tok::LBrack) {
            self.opt(false);
        }
        if let Some(i) = self.tables.iter().rposition(|t| !t.nested) {
            if i == self.tables.len() - 1 {
                self.end_row(i);
            } else {
                self.b().space();
            }
            return;
        }
        self.line_break();
    }

    fn accent(&mut self, c: char) {
        let arg = self.arg();
        let base = plain(&arg);
        let mut chars = base.chars();
        let Some(letter) = chars.next() else {
            return;
        };
        let rest: String = chars.collect();
        let text = format!("{}{rest}", accented(c, letter));
        self.emit_text(&text);
    }

    // --- tables -------------------------------------------------------

    fn ensure_cell(&mut self, i: usize) {
        if self.tables[i].row.is_none() {
            self.b().line_break();
            let header = !self.learning
                && self.tables[i].rows_done == 0
                && self.known.header_rows.get(self.tables[i].index) == Some(&true);
            let mut m = Marker::new(MarkerKind::TableRow, CharRange::empty(0));
            if header {
                m = m.with_label(HEADER_ROW_LABEL);
            }
            let o = self.open(m);
            let t = &mut self.tables[i];
            t.row = Some(o);
            t.after_first_row = false;
        }
        if self.tables[i].cell.is_none() {
            let o = self.open_here(Marker::new(MarkerKind::TableCell, CharRange::empty(0)));
            self.tables[i].cell = Some(o);
        }
    }

    fn end_cell(&mut self, i: usize) {
        let opened = std::mem::take(&mut self.tables[i].cell_opened);
        for o in opened.into_iter().rev() {
            self.close(o);
        }
        if let Some(c) = self.tables[i].cell.take() {
            self.close(c);
        }
    }

    fn end_row(&mut self, i: usize) {
        if self.tables[i].row.is_none() {
            return;
        }
        self.end_cell(i);
        if let Some(r) = self.tables[i].row.take() {
            self.close(r);
        }
        let t = &mut self.tables[i];
        t.rows_done += 1;
        t.after_first_row = t.rows_done == 1;
    }

    fn amp(&mut self) {
        let Some(i) = self.tables.iter().rposition(|t| !t.nested) else {
            self.emit_text("&");
            return;
        };
        if i != self.tables.len() - 1 {
            self.b().space();
            return;
        }
        self.ensure_cell(i);
        self.end_cell(i);
        self.b().separator(crate::CELL_SEPARATOR);
        let o = self.open_here(Marker::new(MarkerKind::TableCell, CharRange::empty(0)));
        self.tables[i].cell = Some(o);
    }

    /// A horizontal rule in a table: after the first row, it makes that
    /// row the header.
    fn rule(&mut self) {
        if let Some(t) = self.tables.last()
            && t.after_first_row
            && !t.nested
            && self.learning
        {
            let index = t.index;
            if let Some(slot) = self.learn.header_rows.get_mut(index) {
                *slot = true;
            }
        }
    }

    fn begin_table(&mut self, name: &str) {
        match name {
            "tabular*" | "tabularx" | "tabulary" | "tabu" => {
                self.opt(true);
                self.skip_args(2);
            }
            _ => {
                self.opt(true);
                self.arg();
            }
        }
        if !self.push_frame(FrameKind::Env(name.into())) {
            return;
        }
        let nested = !self.tables.is_empty();
        let index = self.table_count;
        self.table_count += 1;
        if self.learning {
            self.learn.header_rows.push(false);
        }
        let mut marker = None;
        if !nested {
            self.end_para();
            self.block_break();
            let mut m = Marker::new(MarkerKind::Table, CharRange::empty(0));
            if let Some(label) = self.float_caption_line() {
                m = m.with_label(label);
            }
            marker = Some(self.open(m));
        } else {
            self.b().space();
        }
        self.tables.push(TableState {
            frame: self.frames.len() - 1,
            nested,
            index,
            marker,
            row: None,
            cell: None,
            cell_opened: Vec::new(),
            rows_done: 0,
            after_first_row: false,
        });
    }

    fn end_table(&mut self) {
        let i = self.tables.len() - 1;
        self.end_row(i);
        self.end_cell(i);
        let Some(t) = self.tables.pop() else {
            return;
        };
        if let Some(m) = t.marker {
            self.close(m);
        }
        if t.nested {
            self.b().space();
        } else {
            self.block_break();
        }
    }

    /// "Table 1: caption" for the float being read, from the first pass.
    fn float_caption_line(&self) -> Option<String> {
        let f = self.floats.last()?;
        let caption = self.known.captions.get(f.index)?.as_ref()?;
        Some(format!("{} {}: {caption}", f.kind, f.number))
    }

    // --- lists --------------------------------------------------------

    fn begin_list(&mut self, name: &str, kind: ListKind) {
        self.opt(true);
        if !self.push_frame(FrameKind::Env(name.into())) {
            return;
        }
        self.end_para();
        let enum_depth = self
            .lists
            .iter()
            .filter(|l| l.kind == ListKind::Enumerate)
            .count()
            + usize::from(kind == ListKind::Enumerate);
        let depth = u8::try_from(self.lists.len() + 1).unwrap_or(u8::MAX);
        if self.lists.is_empty() {
            self.block_break();
        } else {
            self.b().line_break();
        }
        let marker = (!self.in_cell()).then(|| {
            self.open(Marker::new(MarkerKind::List, CharRange::empty(0)).with_level(depth))
        });
        self.lists.push(ListState {
            frame: self.frames.len() - 1,
            kind,
            marker,
            item: None,
            next: 1,
            enum_depth,
        });
    }

    fn end_list(&mut self) {
        let Some(l) = self.lists.pop() else {
            return;
        };
        if let Some(item) = l.item {
            self.close(item);
        }
        if let Some(m) = l.marker {
            self.close(m);
        }
        self.block_break();
    }

    fn item(&mut self) {
        let opt = self.opt(true);
        if self.lists.is_empty() {
            // `\item` outside a list: a new line.
            self.par();
            if let Some(o) = opt {
                self.emit_text(&plain(&o));
                self.b().space();
            }
            return;
        }
        let depth = u8::try_from(self.lists.len()).unwrap_or(u8::MAX);
        let li = self.lists.len() - 1;
        let list = &mut self.lists[li];
        let old = list.item.take();
        let label = match (&opt, list.kind) {
            (Some(o), ListKind::Itemize | ListKind::Enumerate) => Some(plain(o)),
            (_, ListKind::Enumerate) => {
                let n = list.next;
                list.next += 1;
                Some(enumerate_label(n, list.enum_depth))
            }
            (_, ListKind::Bibliography) => {
                let n = list.next;
                list.next += 1;
                Some(format!("[{n}]"))
            }
            _ => None,
        };
        let kind = list.kind;
        if let Some(o) = old {
            self.close(o);
        }
        self.b().line_break();
        let mut m = Marker::new(MarkerKind::ListItem, CharRange::empty(0)).with_level(depth);
        if let Some(l) = &label {
            m = m.with_label(l.clone());
            if kind == ListKind::Enumerate {
                let number = l.trim_matches(['.', '(', ')']).to_owned();
                self.target = Some(Label {
                    number,
                    target: None,
                    kind: "Item".into(),
                });
            }
        }
        let o = if self.in_cell() {
            None
        } else {
            Some(self.open(m))
        };
        if let Some(l) = self.lists.last_mut() {
            l.item = o;
        }
        if let (Some(term), ListKind::Description) = (opt, kind) {
            let mut seq = self.arg_seq(
                Act::Mark(Marker::new(MarkerKind::Bold, CharRange::empty(0))),
                term,
            );
            seq.push(Tok::Space);
            self.inject(seq);
        }
    }

    fn bibitem(&mut self) {
        let opt = self.opt(true);
        self.arg();
        let depth = u8::try_from(self.lists.len()).unwrap_or(u8::MAX);
        let Some(list) = self.lists.last_mut() else {
            self.par();
            return;
        };
        let old = list.item.take();
        let label = match &opt {
            Some(o) => format!("[{}]", plain(o)),
            None => {
                let n = list.next;
                list.next += 1;
                format!("[{n}]")
            }
        };
        if let Some(o) = old {
            self.close(o);
        }
        self.target = Some(Label {
            number: label.trim_matches(['[', ']']).to_owned(),
            target: None,
            kind: "Reference".into(),
        });
        self.b().line_break();
        let m = Marker::new(MarkerKind::ListItem, CharRange::empty(0))
            .with_level(depth)
            .with_label(label);
        let o = self.open(m);
        if let Some(l) = self.lists.last_mut() {
            l.item = Some(o);
        }
    }

    // --- environments ---------------------------------------------------

    fn begin_env(&mut self) {
        let name = plain(&self.arg());
        let base = name.trim_end_matches('*');
        if let Some(th) = self.theorems.get(base) {
            let (display, counter, numbered) =
                (th.display.clone(), th.counter.clone(), th.numbered);
            self.theorem(&name, &display, &counter, numbered);
            return;
        }
        match name.as_str() {
            "document" => {
                self.preamble = false;
                self.push_frame(FrameKind::Env(name));
            }
            "itemize" | "compactitem" | "asparaitem" | "inparaitem" => {
                self.begin_list(&name, ListKind::Itemize);
            }
            "enumerate" | "compactenum" | "asparaenum" | "inparaenum" => {
                self.begin_list(&name, ListKind::Enumerate);
            }
            "description" | "compactdesc" => self.begin_list(&name, ListKind::Description),
            "thebibliography" => {
                self.arg();
                self.unnumbered_heading("References");
                self.begin_list(&name, ListKind::Bibliography);
            }
            "tabular" | "tabular*" | "tabularx" | "tabulary" | "longtable" | "longtable*"
            | "array" | "supertabular" | "tabu" => self.begin_table(&name),
            "table" | "table*" | "figure" | "figure*" | "sidewaystable" | "sidewaysfigure"
            | "wrapfigure" | "wraptable" => self.begin_float(&name),
            "quote" | "quotation" | "verse" => {
                if self.push_frame(FrameKind::Env(name)) {
                    self.end_para();
                    self.block_break();
                    let o = self.open(Marker::new(MarkerKind::Quote, CharRange::empty(0)));
                    self.attach(o);
                }
            }
            "abstract" => {
                if self.push_frame(FrameKind::Env(name)) {
                    self.unnumbered_heading("Abstract");
                }
            }
            "proof" => {
                let opt = self.opt(true);
                if self.push_frame(FrameKind::Env(name)) {
                    self.end_para();
                    self.block_break();
                    let head = opt.map_or_else(|| "Proof".to_owned(), |o| plain(&o));
                    self.text_start();
                    let o = self.open(Marker::new(MarkerKind::Italic, CharRange::empty(0)));
                    self.b().text(&format!("{head}."));
                    self.close(o);
                    self.b().space();
                }
            }
            _ => self.transparent(name),
        }
    }

    /// An environment read as its content: known layout ones (their
    /// arguments skipped), and unknown ones (named in the warnings).
    fn transparent(&mut self, name: String) {
        let base = name.trim_end_matches('*');
        let known = match base {
            "minipage" => {
                self.opt(true);
                self.opt(true);
                self.opt(true);
                self.arg();
                true
            }
            "multicols" => {
                self.arg();
                self.opt(true);
                true
            }
            "adjustbox" | "spacing" | "otherlanguage" | "column" | "subfigure" | "subtable" => {
                self.opt(true);
                self.arg();
                true
            }
            "center" | "flushleft" | "flushright" | "raggedright" | "raggedleft" | "titlepage"
            | "landscape" | "singlespace" | "onehalfspace" | "doublespace" | "small"
            | "footnotesize" | "scriptsize" | "tiny" | "normalsize" | "large" | "Large"
            | "LARGE" | "huge" | "Huge" | "frame" | "block" | "columns" | "subequations"
            | "samepage" | "sloppypar" | "document" | "enumerate" | "itemize" => true,
            _ => false,
        };
        if !known && !self.preamble {
            self.unknown_envs.insert(name.clone());
        }
        if self.push_frame(FrameKind::Env(name)) {
            self.end_para();
            self.block_break();
        }
    }

    fn end_env(&mut self) {
        let name = plain(&self.arg());
        let stop = self
            .frames
            .iter()
            .rposition(|f| matches!(f.kind, FrameKind::Arg(..)))
            .map_or(0, |i| i + 1);
        let Some(i) = self.frames[stop..]
            .iter()
            .rposition(|f| matches!(&f.kind, FrameKind::Env(n) if *n == name))
            .map(|i| i + stop)
        else {
            if name == "document" {
                self.done = true;
            }
            return;
        };
        while self.frames.len() > i {
            self.pop_frame();
        }
    }

    fn end_env_effects(&mut self, name: &str) {
        match name {
            "document" => {
                self.end_para();
                self.done = true;
            }
            _ => {
                self.end_para();
                self.block_break();
            }
        }
    }

    fn begin_float(&mut self, name: &str) {
        match name {
            "wrapfigure" | "wraptable" => {
                self.opt(true);
                self.arg();
                self.opt(true);
                self.arg();
            }
            _ => {
                self.opt(true);
            }
        }
        if !self.push_frame(FrameKind::Env(name.into())) {
            return;
        }
        self.end_para();
        let kind = if name.contains("table") {
            "Table"
        } else {
            "Figure"
        };
        let number = if kind == "Table" {
            self.table_number += 1;
            self.table_number
        } else {
            self.figure_number += 1;
            self.figure_number
        };
        let index = self.float_count;
        self.float_count += 1;
        if self.learning {
            self.learn.captions.push(None);
            self.learn.graphics.push(None);
        }
        self.floats.push(FloatState {
            frame: self.frames.len() - 1,
            index,
            kind,
            number,
        });
    }

    fn theorem(&mut self, name: &str, display: &str, counter: &str, numbered: bool) {
        let note = self.opt(true);
        if !self.push_frame(FrameKind::Env(name.into())) {
            return;
        }
        self.end_para();
        self.block_break();
        let starred = name.ends_with('*');
        let mut head = display.to_owned();
        if numbered && !starred {
            let n = self.theorem_counts.entry(counter.to_owned()).or_insert(0);
            *n += 1;
            let number = n.to_string();
            head = format!("{head} {number}");
            self.target = Some(Label {
                number,
                target: None,
                kind: display.to_owned(),
            });
        }
        if let Some(n) = note {
            head = format!("{head} ({})", plain(&n));
        }
        self.text_start();
        let o = self.open(Marker::new(MarkerKind::Bold, CharRange::empty(0)));
        self.b().text(&format!("{head}."));
        self.close(o);
        self.b().space();
    }

    fn section_level(&self) -> u8 {
        if self.chapters { 2 } else { 1 }
    }

    fn unnumbered_heading(&mut self, text: &str) {
        let level = self.section_level();
        self.inject_arg(
            Act::Heading {
                level,
                number: None,
            },
            vec![Tok::Text(text.into())],
        );
    }

    // --- headings -----------------------------------------------------

    fn heading(&mut self, name: &str) {
        let starred = self.star();
        self.opt(true);
        let title = self.arg();
        let index = match name {
            "part" => 0,
            "chapter" => 1,
            "section" => 2,
            "subsection" => 3,
            "subsubsection" => 4,
            "paragraph" => 5,
            _ => 6,
        };
        if index == 1 {
            self.chapters = true;
            self.learn.chapters = true;
        }
        let level: u8 = match (index, self.chapters) {
            (0 | 1, _) => 1,
            (i, true) => u8::try_from(i).unwrap_or(6),
            (i, false) => u8::try_from(i - 1).unwrap_or(6),
        };
        let numbered = !starred && (1..=4).contains(&index) && (index > 1 || self.chapters);
        let number = numbered.then(|| self.step_counter(index));
        let kind = match index {
            0 => "Part",
            1 => "Chapter",
            _ => "Section",
        };
        let mut seq = Vec::new();
        if self.learning {
            seq = self.arg_seq(
                Act::Capture(Cap::HeadingTarget {
                    number: number.clone().unwrap_or_default(),
                    kind,
                }),
                title.clone(),
            );
        }
        let visible = self.arg_seq(Act::Heading { level, number }, title);
        seq.extend(visible);
        self.inject(seq);
    }

    /// Steps the counter at `index` (1 chapter, 2 section, ...) and gives
    /// the number, "2.1".
    fn step_counter(&mut self, index: usize) -> String {
        self.counters[index] = self.counters[index].saturating_add(1);
        for c in &mut self.counters[index + 1..] {
            *c = 0;
        }
        if index == 1 {
            self.equations = 0;
        }
        let top = if self.chapters { 1 } else { 2 };
        (top..=index)
            .map(|i| {
                let n = self.counters[i];
                if i == top && self.appendix {
                    letter(n)
                } else {
                    n.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(".")
    }

    fn maketitle(&mut self) {
        let mut seq = Vec::new();
        if let Some(t) = self.title.clone() {
            seq.extend(self.arg_seq(
                Act::Heading {
                    level: 1,
                    number: None,
                },
                t,
            ));
        }
        for part in [self.author.clone(), self.date.clone()]
            .into_iter()
            .flatten()
        {
            seq.push(Tok::Par);
            seq.extend(self.arg_seq(Act::Plain, part));
            seq.push(Tok::Par);
        }
        self.inject(seq);
    }

    // --- math ---------------------------------------------------------

    fn math(&mut self, display: bool, env: Option<&str>, body: &str) {
        let body = self.expand_math(body);
        let (body, lines) = strip_math_labels(&body);
        let base = env.map(|e| e.trim_end_matches('*'));
        let numbered = env.is_some_and(|e| !e.ends_with('*'))
            && matches!(
                base,
                Some(
                    "equation"
                        | "align"
                        | "alignat"
                        | "flalign"
                        | "gather"
                        | "multline"
                        | "eqnarray"
                )
            );
        // Numbers for the lines that have them, and the labels in them.
        let mut numbers = Vec::new();
        for line in &lines {
            let number = match (&line.tag, numbered && !line.unnumbered) {
                (Some(tag), _) => Some(tag.clone()),
                (None, true) => {
                    self.equations += 1;
                    Some(self.equation_number())
                }
                _ => None,
            };
            for key in &line.labels {
                self.learn_label(
                    key,
                    Label {
                        number: number.clone().unwrap_or_default(),
                        target: None,
                        kind: "Equation".into(),
                    },
                );
            }
            numbers.extend(number);
        }
        let body = body.trim();
        let latex = match base {
            None | Some("equation" | "displaymath" | "math") => body.to_owned(),
            Some("align" | "flalign" | "eqnarray") => {
                format!("\\begin{{align}}{body}\\end{{align}}")
            }
            Some("alignat") => {
                let inner = body
                    .strip_prefix('{')
                    .and_then(|r| r.split_once('}').map(|(_, rest)| rest))
                    .unwrap_or(body);
                format!("\\begin{{align}}{inner}\\end{{align}}")
            }
            Some(_) => format!("\\begin{{gather}}{body}\\end{{gather}}"),
        };
        if latex.trim().is_empty() {
            return;
        }
        let display = display && base != Some("math");
        if display {
            self.text_start();
            self.line_break();
        } else {
            self.text_start();
        }
        let text = if display {
            format!("$${latex}$$")
        } else {
            format!("${latex}$")
        };
        let o = self
            .open(Marker::new(MarkerKind::Math, CharRange::empty(0)).with_level(u8::from(display)));
        self.b().literal(&text);
        self.close(o);
        if !numbers.is_empty() {
            let list = numbers
                .iter()
                .map(|n| format!("({n})"))
                .collect::<Vec<_>>()
                .join(", ");
            self.b().space();
            self.b().text(&list);
        }
        if display {
            self.line_break();
        }
    }

    fn equation_number(&self) -> String {
        if self.chapters && self.counters[1] > 0 {
            format!("{}.{}", self.counters[1], self.equations)
        } else {
            self.equations.to_string()
        }
    }

    /// Macros without arguments expanded inside a formula.
    fn expand_math(&mut self, body: &str) -> String {
        if self.math_macros.is_empty() || !body.contains('\\') {
            return body.to_owned();
        }
        let mut out = body.to_owned();
        for _ in 0..4 {
            let mut changed = false;
            let mut next = String::with_capacity(out.len());
            let mut rest = out.as_str();
            while let Some(i) = rest.find('\\') {
                next.push_str(&rest[..i]);
                let after = &rest[i + 1..];
                let len = after
                    .find(|c: char| !c.is_ascii_alphabetic())
                    .unwrap_or(after.len());
                let name = &after[..len];
                match self.math_macros.get(name) {
                    Some(body)
                        if len > 0
                            && self.expansions < MAX_EXPANSIONS
                            && next.len() + body.len() < MAX_MATH_BYTES =>
                    {
                        self.expansions += 1;
                        next.push('{');
                        next.push_str(body);
                        next.push('}');
                        changed = true;
                        rest = &after[len..];
                    }
                    _ => {
                        let take = if len == 0 {
                            after.chars().next().map_or(0, char::len_utf8)
                        } else {
                            len
                        };
                        next.push('\\');
                        next.push_str(&after[..take]);
                        rest = &after[take..];
                    }
                }
            }
            next.push_str(rest);
            out = next;
            if !changed {
                break;
            }
        }
        out
    }

    fn verbatim(&mut self, lang: Option<String>, body: &str, inline: bool) {
        if inline {
            self.text_start();
            let o = self.open(Marker::new(MarkerKind::Code, CharRange::empty(0)));
            self.b().text(body);
            self.close(o);
            return;
        }
        self.end_para();
        self.block_break();
        if self.options.skip_code {
            return;
        }
        if self.in_cell() || !self.captures.is_empty() {
            self.b().text(body);
            return;
        }
        let mut m = Marker::new(MarkerKind::Code, CharRange::empty(0)).with_level(1);
        if let Some(l) = lang {
            m = m.with_label(l);
        }
        let o = self.open(m);
        self.b().verbatim(body);
        self.close(o);
        self.block_break();
    }

    // --- references, citations, notes -----------------------------------

    fn learn_label(&mut self, key: &str, label: Label) {
        if self.learning && !key.is_empty() {
            self.learn.labels.entry(key.to_owned()).or_insert(label);
        }
    }

    fn label(&mut self) {
        let key = plain(&self.arg());
        let label = self.target.clone().unwrap_or_default();
        self.learn_label(&key, label);
    }

    fn reference(&mut self, name: &str) {
        self.star();
        let keys = plain(&self.arg());
        let mut first = true;
        for key in keys.split(',').map(str::trim).filter(|k| !k.is_empty()) {
            if !first {
                self.emit_text(" and");
                self.b().space();
            }
            first = false;
            let label = self.known.labels.get(key).cloned();
            let Some(label) = label else {
                self.emit_text(key);
                continue;
            };
            let number = if name == "eqref" {
                format!("({})", label.number)
            } else {
                label.number.clone()
            };
            let text = match name {
                "autoref" | "Cref" | "cref" if !label.kind.is_empty() => {
                    let kind = if name == "cref" {
                        label.kind.to_lowercase()
                    } else {
                        label.kind.clone()
                    };
                    format!("{kind} {number}")
                }
                _ => number,
            };
            if text.is_empty() {
                continue;
            }
            self.text_start();
            match label.target {
                Some(t) => {
                    let o = self
                        .open(Marker::new(MarkerKind::Link, CharRange::empty(0)).with_reference(t));
                    self.b().text(&text);
                    self.close(o);
                }
                None => self.b().text(&text),
            }
        }
    }

    /// `\cite[pre][post]{a,b}` as a Pandoc citation, `[pre @a, post; @b]`;
    /// `\citet{a}` as `@a`.
    fn cite(&mut self, name: &str) {
        self.star();
        let first = self.opt(true);
        let second = self.opt(true);
        let keys = plain(&self.arg());
        let (pre, post) = match (first, second) {
            (Some(a), Some(b)) => (plain(&a), plain(&b)),
            (Some(a), None) => (String::new(), plain(&a)),
            _ => (String::new(), String::new()),
        };
        let keys: Vec<&str> = keys
            .split(',')
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .collect();
        if keys.is_empty() || name == "nocite" {
            return;
        }
        let narrative = matches!(
            name,
            "citet" | "textcite" | "Textcite" | "citeauthor" | "Citet" | "Citeauthor"
        );
        let text = if narrative {
            let mut s = keys
                .iter()
                .map(|k| format!("@{k}"))
                .collect::<Vec<_>>()
                .join(" and ");
            if !post.is_empty() {
                s = format!("{s} [{post}]");
            }
            s
        } else {
            let suppress = if name == "citeyear" { "-" } else { "" };
            let mut items: Vec<String> = keys.iter().map(|k| format!("{suppress}@{k}")).collect();
            if !post.is_empty()
                && let Some(last) = items.last_mut()
            {
                last.push_str(&format!(", {post}"));
            }
            if !pre.is_empty()
                && let Some(first) = items.first_mut()
            {
                *first = format!("{pre} {first}");
            }
            format!("[{}]", items.join("; "))
        };
        self.emit_text(&text);
    }

    fn footnote(&mut self) {
        self.opt(false);
        let body = self.arg();
        self.inject_arg(Act::Capture(Cap::Footnote), body);
    }

    fn footnote_text(&mut self, text: String) {
        self.footnote_count += 1;
        let label = self.footnote_count.to_string();
        match self.options.footnotes {
            FootnoteMode::Skip => {}
            FootnoteMode::Inline => {
                self.text_start();
                self.b().inline_footnote(&label, &text);
            }
            FootnoteMode::Deferred => {
                self.text_start();
                self.b().footnote_reference(&label);
                self.notes.push((label, text));
            }
        }
    }

    fn caption(&mut self) {
        self.star();
        self.opt(true);
        let text = self.arg();
        let Some(f) = self.floats.last() else {
            // A caption outside a float (longtable, captionof): a line.
            self.inject_arg(Act::Plain, text);
            return;
        };
        let (index, kind, number) = (f.index, f.kind, f.number);
        self.target = Some(Label {
            number: number.to_string(),
            target: None,
            kind: kind.into(),
        });
        let mut seq = Vec::new();
        if self.learning {
            seq = self.arg_seq(Act::Capture(Cap::Caption(index)), text.clone());
        }
        let image = if kind == "Figure" && !self.learning {
            self.known.graphics.get(index).cloned().flatten()
        } else {
            None
        };
        if !self.tables.is_empty() {
            // Inside a table (longtable): not a cell of its own.
            let visible = self.arg_seq(Act::Capture(Cap::Discard), text);
            seq.extend(visible);
        } else {
            let visible = self.arg_seq(
                Act::Caption {
                    prefix: format!("{kind} {number}:"),
                    image,
                },
                text,
            );
            seq.extend(visible);
        }
        self.inject(seq);
    }

    fn includegraphics(&mut self) {
        self.star();
        self.opt(true);
        let file = plain(&self.arg());
        if let Some(f) = self.floats.last() {
            let index = f.index;
            if self.learning {
                if let Some(slot) = self.learn.graphics.get_mut(index)
                    && slot.is_none()
                {
                    *slot = Some(file);
                }
                return;
            }
            // A figure with a caption: the caption describes it.
            if self.known.captions.get(index).is_some_and(Option::is_some) {
                return;
            }
        }
        if file.is_empty() {
            return;
        }
        self.text_start();
        let o = self
            .open(Marker::new(MarkerKind::Image, CharRange::empty(0)).with_reference(file.clone()));
        let name = Path::new(&file)
            .file_name()
            .map_or_else(|| file.clone(), |n| n.to_string_lossy().into_owned());
        self.b().text(&name);
        self.close(o);
    }

    // --- definitions ------------------------------------------------------

    fn define(&mut self) {
        self.star();
        let name_toks = self.arg();
        let Some(Tok::Cmd(name)) = name_toks.into_iter().find(|t| matches!(t, Tok::Cmd(_))) else {
            return;
        };
        let args = self.opt(true);
        let default = self.opt(true);
        let body = self.arg();
        let n = args
            .map(|a| plain(&a).trim().parse::<u32>().unwrap_or(1))
            .unwrap_or(0);
        if n > 0 || default.is_some() {
            self.arg_macros.insert(name);
            return;
        }
        self.math_macros.insert(name.clone(), to_source(&body));
        self.macros.insert(name, body);
    }

    fn def(&mut self) {
        let Some(Tok::Cmd(name)) = self.pop() else {
            return;
        };
        // Parameters (`#1`) come before the body.
        if !matches!(self.peek(), Some(Tok::Open)) {
            while let Some(t) = self.peek() {
                if matches!(t, Tok::Open) || t.is_sentinel() {
                    break;
                }
                self.pop();
            }
            self.arg();
            self.arg_macros.insert(name);
            return;
        }
        let body = self.arg();
        self.math_macros.insert(name.clone(), to_source(&body));
        self.macros.insert(name, body);
    }

    fn math_operator(&mut self) {
        self.star();
        let name = self.arg();
        let text = plain(&self.arg());
        if let Some(Tok::Cmd(n)) = name.into_iter().find(|t| matches!(t, Tok::Cmd(_))) {
            self.math_macros
                .insert(n, format!("\\operatorname{{{text}}}"));
        }
    }

    fn newtheorem(&mut self) {
        let starred = self.star();
        let name = plain(&self.arg());
        let shared = self.opt(true).map(|o| plain(&o));
        let display = plain(&self.arg());
        self.opt(true);
        if name.is_empty() {
            return;
        }
        let counter = shared.unwrap_or_else(|| name.clone());
        self.theorems.insert(
            name,
            Theorem {
                display,
                counter,
                numbered: !starred,
            },
        );
    }

    fn usepackage(&mut self) {
        let opts = self.opt(true);
        let names = plain(&self.arg());
        if names.split(',').any(|n| n.trim() == "babel")
            && let Some(o) = opts
            && let Some(lang) = plain(&o).split(',').map(str::trim).rfind(|s| !s.is_empty())
        {
            self.language = language_tag(lang).map(str::to_owned);
        }
    }

    // --- includes -------------------------------------------------------

    fn include(&mut self, name: &str) {
        let file = match name {
            "import" | "subimport" | "inputfrom" | "subinputfrom" => {
                let dir = plain(&self.arg());
                let file = plain(&self.arg());
                format!("{}/{file}", dir.trim_end_matches('/'))
            }
            _ => {
                self.skip_spaces();
                if matches!(self.peek(), Some(Tok::Open)) {
                    plain(&self.arg())
                } else {
                    // TeX's `\input chapter`: the name runs to a space.
                    match self.pop() {
                        Some(Tok::Text(t)) => t,
                        Some(other) => {
                            self.pending.push(other);
                            String::new()
                        }
                        None => String::new(),
                    }
                }
            }
        };
        let file = file.trim().to_owned();
        if file.is_empty() {
            return;
        }
        match self.read_include(&file) {
            Ok(text) => {
                let (mut toks, truncated) = lex(&text, self.budget);
                if truncated {
                    self.cut = true;
                }
                toks.push(Tok::EndInput);
                self.include_depth += 1;
                self.includes += 1;
                self.inject(toks);
            }
            Err(why) => {
                if !self.learning {
                    self.skipped_includes.insert(format!("{file} ({why})"));
                }
            }
        }
    }

    /// The text of an included file, or why it was not read.
    fn read_include(&mut self, file: &str) -> Result<String, &'static str> {
        let Some(folder) = self.folder.clone() else {
            return Err("the document has no folder");
        };
        if self.include_depth >= MAX_INCLUDE_DEPTH || self.includes >= MAX_INCLUDES {
            return Err("too many files included");
        }
        let mut rel = PathBuf::from(file.replace('\\', "/"));
        if rel.extension().is_none() {
            rel.set_extension("tex");
        }
        if !rel.components().all(|c| matches!(c, Component::Normal(_))) {
            return Err("it is outside the document's folder");
        }
        let path = folder.join(&rel);
        let real = path.canonicalize().map_err(|_| "it was not found")?;
        if !real.starts_with(&folder) {
            return Err("it is outside the document's folder");
        }
        let meta = std::fs::metadata(&real).map_err(|_| "it could not be read")?;
        let len = usize::try_from(meta.len()).unwrap_or(usize::MAX);
        if len > MAX_SOURCE_BYTES || self.include_bytes.saturating_add(len) > MAX_INCLUDE_BYTES {
            return Err("it is too large");
        }
        let bytes = std::fs::read(&real).map_err(|_| "it could not be read")?;
        if encoding::binary_kind(&bytes[..bytes.len().min(encoding::SNIFF_BYTES)]).is_some() {
            return Err("it is not a text file");
        }
        self.include_bytes += bytes.len();
        Ok(decode_bytes(&bytes, None).text)
    }

    // --- commands ---------------------------------------------------------

    fn marked(&mut self, kind: MarkerKind) {
        let body = self.arg();
        self.inject_arg(Act::Mark(Marker::new(kind, CharRange::empty(0))), body);
    }

    fn switch(&mut self, kind: MarkerKind) {
        self.text_start();
        let o = self.open(Marker::new(kind, CharRange::empty(0)));
        self.attach(o);
    }

    fn plain_arg(&mut self) {
        let body = self.arg();
        self.inject_arg(Act::Plain, body);
    }

    fn command(&mut self, name: &str) {
        use MarkerKind as K;
        match name {
            "part" | "chapter" | "section" | "subsection" | "subsubsection" | "paragraph"
            | "subparagraph" => self.heading(name),
            "begin" => self.begin_env(),
            "end" => self.end_env(),
            "item" => self.item(),
            "bibitem" => self.bibitem(),
            "maketitle" => self.maketitle(),
            "title" | "author" | "date" => {
                self.opt(true);
                let toks = self.arg();
                match name {
                    "title" => {
                        self.title = Some(toks.clone());
                        self.inject_arg(Act::Capture(Cap::Title), toks);
                    }
                    "author" => {
                        self.author = Some(toks.clone());
                        self.inject_arg(Act::Capture(Cap::Author), toks);
                    }
                    _ => self.date = Some(toks),
                }
            }
            "and" => self.emit_text(","),
            "thanks" => {
                let body = self.arg();
                self.inject_arg(Act::Capture(Cap::Discard), body);
            }
            "footnote" | "marginpar" | "footnotetext" => self.footnote(),
            "footnotemark" => {
                self.opt(false);
            }
            "caption" | "captionof" => {
                if name == "captionof" {
                    self.arg();
                }
                self.caption();
            }
            "label" => self.label(),
            "ref" | "eqref" | "autoref" | "cref" | "Cref" | "pageref" | "vref" | "nameref"
            | "cpageref" => self.reference(name),
            "cite" | "citep" | "citet" | "parencite" | "Parencite" | "autocite" | "Autocite"
            | "textcite" | "Textcite" | "citealp" | "citealt" | "citeauthor" | "Citeauthor"
            | "citeyear" | "citeyearpar" | "footcite" | "supercite" | "Cite" | "Citet"
            | "Citep" | "nocite" | "smartcite" => self.cite(name),
            "input" | "include" | "subfile" | "import" | "subimport" | "inputfrom"
            | "subinputfrom" => self.include(name),
            "newcommand"
            | "renewcommand"
            | "providecommand"
            | "DeclareRobustCommand"
            | "newcommandx" => self.define(),
            "def" | "gdef" | "edef" | "xdef" => self.def(),
            "let" => {
                self.pop();
                if matches!(self.peek(), Some(Tok::Text(t)) if t.starts_with('=')) {
                    self.star_eq();
                }
                self.pop();
            }
            "DeclareMathOperator" => self.math_operator(),
            "newtheorem" => self.newtheorem(),
            "newenvironment" | "renewenvironment" => {
                self.star();
                self.arg();
                self.opt(true);
                self.opt(true);
                self.skip_args(2);
            }
            "usepackage" | "RequirePackage" => self.usepackage(),
            "documentclass" | "documentstyle" => {
                self.opt(true);
                self.arg();
                self.opt(true);
            }
            "setmainlanguage" | "setdefaultlanguage" | "selectlanguage" => {
                self.opt(true);
                let lang = plain(&self.arg());
                if let Some(tag) = language_tag(&lang) {
                    self.language = Some(tag.into());
                }
            }
            "appendix" => {
                self.appendix = true;
                let top = if self.chapters { 1 } else { 2 };
                self.counters[top] = 0;
            }
            "url" => {
                if let Some(Tok::Raw(u)) = self.peek().cloned() {
                    self.pop();
                    self.text_start();
                    let o = self
                        .open(Marker::new(K::Link, CharRange::empty(0)).with_reference(u.clone()));
                    self.b().text(&u);
                    self.close(o);
                }
            }
            "path" => {
                if let Some(Tok::Raw(u)) = self.peek().cloned() {
                    self.pop();
                    self.emit_text(&u);
                }
            }
            "href" => {
                let url = match self.peek().cloned() {
                    Some(Tok::Raw(u)) => {
                        self.pop();
                        u
                    }
                    _ => plain(&self.arg()),
                };
                let body = self.arg();
                self.inject_arg(
                    Act::Mark(Marker::new(K::Link, CharRange::empty(0)).with_reference(url)),
                    body,
                );
            }
            "hyperref" => {
                let label = self.opt(true).map(|o| plain(&o));
                let body = self.arg();
                let target =
                    label.and_then(|l| self.known.labels.get(&l).and_then(|l| l.target.clone()));
                match target {
                    Some(t) => self.inject_arg(
                        Act::Mark(Marker::new(K::Link, CharRange::empty(0)).with_reference(t)),
                        body,
                    ),
                    None => self.inject_arg(Act::Plain, body),
                }
            }
            "includegraphics" => self.includegraphics(),
            "multicolumn" => {
                self.skip_args(2);
                self.plain_arg();
            }
            "multirow" => {
                self.arg();
                self.opt(true);
                self.arg();
                self.plain_arg();
            }
            "hline" | "toprule" | "midrule" | "bottomrule" | "specialrule" | "hhline" => {
                if name == "specialrule" {
                    self.skip_args(3);
                }
                if name == "hhline" {
                    self.arg();
                }
                self.rule();
            }
            "cline" => {
                self.arg();
                self.rule();
            }
            "cmidrule" => {
                self.skip_spaces();
                if let Some(Tok::Text(t)) = self.peek()
                    && t.starts_with('(')
                    && let Some(Tok::Text(t)) = self.pop()
                    && let Some(end) = t.find(')')
                    && end + 1 < t.len()
                {
                    self.pending.push(Tok::Text(t[end + 1..].to_owned()));
                }
                self.arg();
                self.rule();
            }
            "endhead" | "endfirsthead" | "endfoot" | "endlastfoot" | "tabularnewline" => {
                if name == "tabularnewline" {
                    self.newline();
                }
            }
            // Text styles over an argument.
            "emph" | "textit" | "textsl" | "mathit" => self.marked(K::Italic),
            "textbf" | "mathbf" | "bm" | "boldsymbol" => self.marked(K::Bold),
            "underline" | "uline" | "uuline" => self.marked(K::Underline),
            "sout" | "st" | "xout" => self.marked(K::Strikethrough),
            "texttt" | "code" => self.marked(K::Code),
            "textsc" | "textrm" | "textsf" | "textup" | "textmd" | "textnormal" | "mbox"
            | "fbox" | "text" | "hbox" | "textsuperscript" | "textsubscript" | "ensuremath"
            | "enquote" | "mathrm" | "mathsf" | "mathtt" | "emphasize" | "NoCaseChange" => {
                if name == "enquote" {
                    let body = self.arg();
                    let mut seq = vec![Tok::Text("\u{201c}".into())];
                    seq.extend(self.arg_seq(Act::Plain, body));
                    seq.push(Tok::Text("\u{201d}".into()));
                    self.inject(seq);
                } else {
                    self.plain_arg();
                }
            }
            "makebox" | "framebox" => {
                self.opt(true);
                self.opt(true);
                self.plain_arg();
            }
            "parbox" | "raisebox" => {
                self.opt(true);
                self.opt(true);
                self.opt(true);
                self.arg();
                self.plain_arg();
            }
            "textcolor" | "colorbox" | "foreignlanguage" | "resizebox" | "scalebox"
            | "rotatebox" => {
                self.opt(true);
                if name == "resizebox" {
                    self.arg();
                }
                self.arg();
                self.plain_arg();
            }
            "fcolorbox" => {
                self.skip_args(2);
                self.plain_arg();
            }
            // Switches: to the end of the group.
            "bf" | "bfseries" => self.switch(K::Bold),
            "it" | "itshape" | "em" | "sl" | "slshape" => self.switch(K::Italic),
            "tt" | "ttfamily" => self.switch(K::Code),
            // Text.
            "LaTeX" => self.emit_text("LaTeX"),
            "LaTeXe" => self.emit_text("LaTeX2e"),
            "TeX" => self.emit_text("TeX"),
            "BibTeX" => self.emit_text("BibTeX"),
            "ldots" | "dots" | "textellipsis" | "dotsc" => self.emit_text("\u{2026}"),
            "textendash" => self.emit_text("\u{2013}"),
            "textemdash" => self.emit_text("\u{2014}"),
            "textquoteleft" => self.emit_text("\u{2018}"),
            "textquoteright" => self.emit_text("\u{2019}"),
            "textquotedblleft" => self.emit_text("\u{201c}"),
            "textquotedblright" => self.emit_text("\u{201d}"),
            "ss" => self.emit_text("\u{df}"),
            "ae" => self.emit_text("\u{e6}"),
            "AE" => self.emit_text("\u{c6}"),
            "oe" => self.emit_text("\u{153}"),
            "OE" => self.emit_text("\u{152}"),
            "o" => self.emit_text("\u{f8}"),
            "O" => self.emit_text("\u{d8}"),
            "aa" => self.emit_text("\u{e5}"),
            "AA" => self.emit_text("\u{c5}"),
            "l" => self.emit_text("\u{142}"),
            "L" => self.emit_text("\u{141}"),
            "i" => self.emit_text("\u{131}"),
            "S" => self.emit_text("\u{a7}"),
            "P" => self.emit_text("\u{b6}"),
            "dag" => self.emit_text("\u{2020}"),
            "ddag" => self.emit_text("\u{2021}"),
            "copyright" | "textcopyright" => self.emit_text("\u{a9}"),
            "textregistered" => self.emit_text("\u{ae}"),
            "texttrademark" => self.emit_text("\u{2122}"),
            "pounds" | "textsterling" => self.emit_text("\u{a3}"),
            "euro" | "texteuro" => self.emit_text("\u{20ac}"),
            "textdegree" | "degree" => self.emit_text("\u{b0}"),
            "textbackslash" => self.emit_text("\\"),
            "textasciitilde" => self.emit_text("~"),
            "textasciicircum" => self.emit_text("^"),
            "textbar" => self.emit_text("|"),
            "textless" => self.emit_text("<"),
            "textgreater" => self.emit_text(">"),
            "textunderscore" => self.emit_text("_"),
            "textbullet" => self.emit_text("\u{2022}"),
            "slash" => self.emit_text("/"),
            "c" | "v" | "u" | "H" | "k" | "r" | "d" | "b" | "t" => {
                let mark = match name {
                    "c" => 'c',
                    "v" => 'v',
                    "u" => 'u',
                    "H" => 'H',
                    "k" => 'k',
                    "r" => 'r',
                    _ => '.',
                };
                self.accent(mark);
            }
            // Breaks and spaces.
            "par" => self.par(),
            "newline" => self.line_break(),
            "linebreak" => {
                self.opt(false);
                self.line_break();
            }
            "quad" | "qquad" | "enspace" | "thinspace" | "hfill" | "vfill" | "hfil" | "space"
            | "nobreakspace" => self.b().space(),
            "newblock" => self.b().space(),
            "vspace" | "hspace" | "addvspace" | "phantom" | "hphantom" | "vphantom" | "index"
            | "glossary" | "pagestyle" | "thispagestyle" | "pagenumbering"
            | "bibliographystyle" | "bibliography" | "addbibresource" | "graphicspath"
            | "geometry" | "hypersetup" | "stepcounter" | "refstepcounter" | "newlength"
            | "lstset" | "usetikzlibrary" | "tikzset" | "pgfplotsset" | "setlist"
            | "linespread" | "includeonly" | "color" | "pagecolor" | "newcounter" | "urlstyle"
            | "setmainfont" | "setsansfont" | "setmonofont" | "setotherlanguage"
            | "setotherlanguages" | "captionsetup" | "hyphenation" | "enlargethispage"
            | "restylefloat" | "floatstyle" | "theoremstyle" | "newpagestyle" | "fontsize"
            | "selectfont" | "definecolor" | "setlength" | "addtolength" | "setcounter"
            | "addtocounter" | "numberwithin" | "nomenclature" | "renewcommandx" => {
                self.star();
                self.opt(true);
                let n = match name {
                    "definecolor" => 3,
                    "setlength" | "addtolength" | "setcounter" | "addtocounter"
                    | "numberwithin" | "nomenclature" | "fontsize" => 2,
                    "selectfont" => 0,
                    _ => 1,
                };
                self.skip_args(n);
                if name == "newcounter" {
                    self.opt(true);
                }
            }
            "noindent" | "indent" | "centering" | "raggedright" | "raggedleft" | "newpage"
            | "clearpage" | "cleardoublepage" | "pagebreak" | "nopagebreak" | "smallskip"
            | "medskip" | "bigskip" | "protect" | "relax" | "makeatletter" | "makeatother"
            | "tableofcontents" | "listoffigures" | "listoftables" | "frontmatter"
            | "mainmatter" | "backmatter" | "normalfont" | "rmfamily" | "sffamily" | "upshape"
            | "mdseries" | "scshape" | "normalsize" | "small" | "footnotesize" | "scriptsize"
            | "tiny" | "large" | "Large" | "LARGE" | "huge" | "Huge" | "onehalfspacing"
            | "doublespacing" | "singlespacing" | "sloppy" | "fussy" | "noalign"
            | "printbibliography" | "today" | "null" | "hrule" | "vrule" | "hrulefill"
            | "dotfill" | "leavevmode" | "strut" | "allowbreak" | "nobreak" | "clubpenalty"
            | "widowpenalty" | "maketitlepage" | "FloatBarrier" | "printindex" | "makeindex"
            | "rm" | "sf" | "sc" | "footnotesep" | "centerline" => {
                if matches!(name, "pagebreak") {
                    self.opt(false);
                }
                if name == "printbibliography" {
                    self.opt(true);
                }
            }
            _ => self.unknown_command(name),
        }
    }

    /// `\let\a=\b`: the `=`.
    fn star_eq(&mut self) {
        if let Some(Tok::Text(t)) = self.pending.last_mut() {
            let rest = t.trim_start_matches('=').to_owned();
            if rest.is_empty() {
                self.pending.pop();
            } else {
                *t = rest;
            }
        }
    }

    fn unknown_command(&mut self, name: &str) {
        if let Some(body) = self.macros.get(name) {
            if self.expansions < MAX_EXPANSIONS {
                self.expansions += 1;
                let body = body.clone();
                self.inject(body);
            } else {
                self.cut = true;
            }
            return;
        }
        if self.preamble {
            return;
        }
        if self.unknown.len() < 1000 {
            self.unknown.insert(name.to_owned());
        }
        if self.options.name_skipped_commands {
            self.emit_text(&format!("(command {name})"));
            self.b().space();
        }
        // An optional argument right after it is an option, not text.
        if self.peek() == Some(&Tok::LBrack) {
            self.opt(false);
        }
    }

    // --- the end --------------------------------------------------------

    fn finish(mut self, meta: &mut DocumentMeta) -> (String, Vec<Marker>) {
        self.end_para();
        for o in std::mem::take(&mut self.root_opened).into_iter().rev() {
            self.close(o);
        }
        let notes = std::mem::take(&mut self.notes);
        self.main.footnotes_section(&notes);
        if meta.title.is_none() {
            meta.title = self.meta_title.clone();
        }
        if meta.author.is_none() {
            meta.author = self.meta_author.clone();
        }
        if meta.language.is_none() {
            meta.language = self.language.clone();
        }
        if self.cut {
            add_warning(meta, CUT_WARNING);
        }
        if self.flattened || self.overflow > 0 {
            add_warning(meta, crate::NESTING_WARNING);
        }
        let mut unknown: Vec<String> = self.unknown.iter().map(|n| format!("\\{n}")).collect();
        unknown.extend(self.arg_macros.iter().map(|n| format!("\\{n}")));
        unknown.sort();
        unknown.dedup();
        if !unknown.is_empty() {
            let more = unknown.len().saturating_sub(MAX_NAMED);
            unknown.truncate(MAX_NAMED);
            let mut list = unknown.join(", ");
            if more > 0 {
                list.push_str(&format!(", and {more} more"));
            }
            add_warning(
                meta,
                &format!(
                    "Some LaTeX commands are not supported, so only their text is read: {list}."
                ),
            );
        }
        if !self.unknown_envs.is_empty() {
            let list: Vec<&str> = self
                .unknown_envs
                .iter()
                .take(MAX_NAMED)
                .map(String::as_str)
                .collect();
            add_warning(
                meta,
                &format!(
                    "Some LaTeX environments are not supported, so only their text is read: {}.",
                    list.join(", ")
                ),
            );
        }
        for skipped in &self.skipped_includes {
            add_warning(meta, &format!("An included file was not read: {skipped}."));
        }
        self.main.finish()
    }
}

/// One line of a math environment: its labels, and whether it is numbered.
#[derive(Debug, Default)]
struct MathLine {
    labels: Vec<String>,
    tag: Option<String>,
    unnumbered: bool,
}

/// The formula without `\label`, `\tag`, `\nonumber`, and `\notag`, and
/// what each line (split at `\\` outside braces) had.
fn strip_math_labels(body: &str) -> (String, Vec<MathLine>) {
    let mut out = String::with_capacity(body.len());
    let mut lines = vec![MathLine::default()];
    let mut depth = 0usize;
    let mut rest = body;
    while let Some(c) = rest.chars().next() {
        if c == '\\' {
            let after = &rest[1..];
            let len = after
                .find(|c: char| !c.is_ascii_alphabetic())
                .unwrap_or(after.len());
            let name = &after[..len];
            let brace = |s: &str| -> Option<(String, usize)> {
                let s2 = s.trim_start();
                let skipped = s.len() - s2.len();
                let inner = s2.strip_prefix('{')?;
                let end = inner.find('}')?;
                Some((inner[..end].trim().to_owned(), skipped + end + 2))
            };
            match name {
                "label" | "tag" => {
                    if let Some((arg, used)) = brace(&after[len..])
                        && let Some(line) = lines.last_mut()
                    {
                        if name == "label" {
                            line.labels.push(arg);
                        } else {
                            line.tag = Some(arg);
                        }
                        rest = &after[len + used..];
                        continue;
                    }
                }
                "nonumber" | "notag" => {
                    if let Some(line) = lines.last_mut() {
                        line.unnumbered = true;
                    }
                    rest = &after[len..];
                    continue;
                }
                "" if after.starts_with('\\') && depth == 0 => {
                    out.push_str("\\\\");
                    lines.push(MathLine::default());
                    rest = &after[1..];
                    continue;
                }
                _ => {}
            }
            let take = if len == 0 {
                after.chars().next().map_or(0, char::len_utf8)
            } else {
                len
            };
            out.push('\\');
            out.push_str(&after[..take]);
            rest = &after[take..];
            continue;
        }
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    // A last line with nothing on it (after a final `\\`) has no number.
    if lines.len() > 1
        && let Some(last) = out.rsplit("\\\\").next()
        && last.trim().is_empty()
        && let Some(line) = lines.last()
        && line.labels.is_empty()
        && line.tag.is_none()
    {
        lines.pop();
    }
    (out, lines)
}

/// An enumerate label for item `n` at enumerate depth `depth`: `1.`,
/// `(a)`, `i.`, `A.`.
fn enumerate_label(n: i64, depth: usize) -> String {
    let n32 = u32::try_from(n.clamp(1, 100_000)).unwrap_or(1);
    match depth {
        2 => format!("({})", letter(n32).to_lowercase()),
        3 => format!("{}.", roman(n32)),
        4 => format!("{}.", letter(n32)),
        _ => format!("{n}."),
    }
}

/// 1 is A, 26 is Z, 27 is AA.
fn letter(n: u32) -> String {
    let mut n = n.max(1);
    let mut out = Vec::new();
    while n > 0 {
        n -= 1;
        out.push(char::from(b'A' + u8::try_from(n % 26).unwrap_or(0)));
        n /= 26;
    }
    out.iter().rev().collect()
}

/// Lower-case roman numerals (up to 3999, then the number).
fn roman(n: u32) -> String {
    if !(1..4000).contains(&n) {
        return n.to_string();
    }
    const TABLE: &[(u32, &str)] = &[
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut n = n;
    let mut s = String::new();
    for &(v, r) in TABLE {
        while n >= v {
            s.push_str(r);
            n -= v;
        }
    }
    s
}

/// A letter with a TeX accent: precomposed when there is one, else the
/// letter and a combining mark.
fn accented(accent: char, letter: char) -> String {
    let (bases, composed, combining): (&str, &str, u32) = match accent {
        '\'' => ("aeiouyAEIOUYcnszCNSZ", "áéíóúýÁÉÍÓÚÝćńśźĆŃŚŹ", 0x301),
        '`' => ("aeiouAEIOU", "àèìòùÀÈÌÒÙ", 0x300),
        '^' => ("aeiouAEIOU", "âêîôûÂÊÎÔÛ", 0x302),
        '"' => ("aeiouyAEIOU", "äëïöüÿÄËÏÖÜ", 0x308),
        '~' => ("anoANO", "ãñõÃÑÕ", 0x303),
        'c' => ("cCsS", "çÇşŞ", 0x327),
        'v' => ("csznrCSZNR", "čšžňřČŠŽŇŘ", 0x30c),
        '=' => ("aeiouAEIOU", "āēīōūĀĒĪŌŪ", 0x304),
        'u' => ("agAG", "ăğĂĞ", 0x306),
        'H' => ("ouOU", "őűŐŰ", 0x30b),
        'k' => ("aeAE", "ąęĄĘ", 0x328),
        'r' => ("auAU", "åůÅŮ", 0x30a),
        '.' => ("zeZEI", "żėŻĖİ", 0x307),
        _ => ("", "", 0),
    };
    if let Some(i) = bases.chars().position(|b| b == letter)
        && let Some(c) = composed.chars().nth(i)
    {
        return c.to_string();
    }
    let mut s = letter.to_string();
    if let Some(m) = char::from_u32(combining).filter(|_| combining != 0) {
        s.push(m);
    }
    s
}

/// A BCP 47 tag for a babel or polyglossia language name.
fn language_tag(name: &str) -> Option<&'static str> {
    Some(match name.trim() {
        "english" | "american" | "USenglish" | "canadian" => "en",
        "british" | "UKenglish" => "en-GB",
        "french" | "francais" | "acadian" | "canadien" => "fr",
        "german" | "ngerman" | "austrian" | "naustrian" => "de",
        "spanish" | "mexican" => "es",
        "portuguese" | "portuges" => "pt",
        "brazil" | "brazilian" => "pt-BR",
        "arabic" => "ar",
        "italian" => "it",
        "dutch" => "nl",
        "russian" => "ru",
        "greek" => "el",
        "polish" => "pl",
        "swedish" => "sv",
        "norsk" | "norwegian" => "nb",
        "finnish" => "fi",
        "danish" => "da",
        "czech" => "cs",
        "turkish" => "tr",
        "hebrew" => "he",
        "japanese" => "ja",
        "chinese" => "zh",
        "korean" => "ko",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use textweaver_core::MarkerKind;

    use super::*;

    fn load_with(src: &str, options: &LoadOptions) -> Document {
        LatexLoader
            .load(
                &Source::Bytes {
                    data: src.as_bytes().to_vec(),
                    hint: "tex".into(),
                },
                options,
            )
            .expect("the LaTeX loads")
    }

    fn load(src: &str) -> Document {
        load_with(src, &LoadOptions::default())
    }

    fn kinds(d: &Document, kind: MarkerKind) -> Vec<String> {
        d.marker_index()
            .iter(kind, None)
            .map(|m| d.slice(m.range))
            .collect()
    }

    #[test]
    fn sections_are_numbered_headings() {
        let d = load(
            "\\documentclass{article}\n\\usepackage{amsmath}\nPreamble text.\n\\begin{document}\n\\section{Introduction}\nHello \\emph{world}.\n\n\\subsection{Why}\nBecause.\n\\section*{Notes}\nEnd.\n\\end{document}\nAfter the end.",
        );
        assert_eq!(
            d.text().to_string(),
            "1 Introduction\n\nHello world.\n\n1.1 Why\n\nBecause.\n\nNotes\n\nEnd."
        );
        let levels: Vec<u8> = d
            .marker_index()
            .iter(MarkerKind::Heading, None)
            .map(|m| m.level)
            .collect();
        assert_eq!(levels, [1, 2, 1]);
        assert_eq!(kinds(&d, MarkerKind::Italic), ["world"]);
        assert_eq!(
            kinds(&d, MarkerKind::Paragraph),
            ["Hello world.", "Because.", "End."]
        );
    }

    #[test]
    fn math_goes_to_math_markers() {
        let d = load(
            "Area $\\pi r^2$ here.\n\\begin{equation}\\label{eq:e}E = mc^2\\end{equation}\nSee \\eqref{eq:e} and \\[ a+b \\].",
        );
        let text = d.text().to_string();
        assert_eq!(
            kinds(&d, MarkerKind::Math),
            ["$\\pi r^2$", "$$E = mc^2$$", "$$a+b$$"]
        );
        assert!(text.contains("$$E = mc^2$$ (1)"), "{text}");
        assert!(text.contains("See (1) and"), "{text}");
        let levels: Vec<u8> = d
            .marker_index()
            .iter(MarkerKind::Math, None)
            .map(|m| m.level)
            .collect();
        assert_eq!(levels, [0, 1, 1]);
    }

    #[test]
    fn align_lines_are_numbered_and_labels_resolve() {
        let d = load(
            "\\begin{align}\na &= b \\label{one}\\\\\nc &= d \\nonumber\\\\\ne &= f \\label{three}\n\\end{align}\nBy \\ref{one} and \\ref{three}.",
        );
        let text = d.text().to_string();
        assert!(text.contains("(1), (2)"), "{text}");
        assert!(text.contains("By 1 and 2."), "{text}");
        let math = kinds(&d, MarkerKind::Math);
        assert!(math[0].starts_with("$$\\begin{align}"), "{math:?}");
        assert!(!math[0].contains("label"), "{math:?}");
    }

    #[test]
    fn lists_nest_with_labels() {
        let d = load(
            "\\begin{enumerate}\n\\item One\n\\item Two\n\\begin{enumerate}\\item Inner\\end{enumerate}\n\\end{enumerate}\n\\begin{itemize}\\item[--] Dash\\end{itemize}\n\\begin{description}\\item[Term] Meaning\\end{description}",
        );
        assert_eq!(
            d.text().to_string(),
            "One\nTwo\nInner\n\nDash\n\nTerm Meaning"
        );
        let labels: Vec<Option<String>> = d
            .marker_index()
            .iter(MarkerKind::ListItem, None)
            .map(|m| m.label.clone())
            .collect();
        assert_eq!(
            labels,
            [
                Some("1.".into()),
                Some("2.".into()),
                Some("(a)".into()),
                Some("--".into()),
                None
            ]
        );
        assert_eq!(kinds(&d, MarkerKind::Bold), ["Term"]);
    }

    #[test]
    fn tables_with_a_rule_under_the_first_row_have_a_header() {
        let d = load(
            "\\begin{table}\\centering\n\\caption{Scores}\\label{t:s}\n\\begin{tabular}{|l|r|}\\hline\nName & Score \\\\ \\hline\nAda & 10 \\\\\n & 3 \\\\ \\hline\n\\end{tabular}\\end{table}\nSee Table~\\ref{t:s}.",
        );
        let text = d.text().to_string();
        assert!(
            text.starts_with("Table 1: Scores\n\nName | Score\nAda | 10\n | 3\n\nSee Table 1."),
            "{text}"
        );
        let rows: Vec<Option<String>> = d
            .marker_index()
            .iter(MarkerKind::TableRow, None)
            .map(|m| m.label.clone())
            .collect();
        assert_eq!(rows, [Some(HEADER_ROW_LABEL.into()), None, None]);
        let table = d.marker_index().nth(MarkerKind::Table, None, 0).cloned();
        assert_eq!(
            table.and_then(|t| t.label).as_deref(),
            Some("Table 1: Scores")
        );
        assert_eq!(kinds(&d, MarkerKind::TableCell).len(), 6);
    }

    #[test]
    fn refs_to_sections_link_to_their_headings() {
        let d = load(
            "\\section{Methods}\\label{sec:m}\nText.\n\\section{Results}\nAs in Section~\\ref{sec:m} and \\autoref{sec:m}; \\ref{missing}.",
        );
        let links: Vec<(String, Option<String>)> = d
            .marker_index()
            .iter(MarkerKind::Link, None)
            .map(|m| (d.slice(m.range), m.reference.clone()))
            .collect();
        assert_eq!(
            links,
            [
                ("1".into(), Some("#1-methods".into())),
                ("Section 1".into(), Some("#1-methods".into()))
            ]
        );
        assert!(d.text().to_string().contains("; missing."));
    }

    #[test]
    fn citations_become_pandoc_citations() {
        let d = load(
            "As \\citet{doe2020} shows \\citep[see][p.~12]{doe2020,roe99}. Also \\cite{a}.\\nocite{b}",
        );
        assert_eq!(
            d.text().to_string(),
            "As @doe2020 shows [see @doe2020; @roe99, p. 12]. Also [@a]."
        );
    }

    #[test]
    fn footnotes_follow_the_footnote_mode() {
        let src = "Text\\footnote{A \\emph{note}.} here.";
        let d = load(src);
        assert_eq!(
            d.text().to_string(),
            "Text[1] here.\n\nFootnotes\n\n[1] A note."
        );
        let inline = load_with(
            src,
            &LoadOptions {
                footnotes: FootnoteMode::Inline,
                ..LoadOptions::default()
            },
        );
        assert_eq!(inline.text().to_string(), "Text (footnote: A note.) here.");
    }

    #[test]
    fn title_block_and_metadata() {
        let d = load(
            "\\documentclass{article}\\usepackage[french]{babel}\n\\title{On \\LaTeX}\\author{Ada Example \\and Bo Example}\\date{Spring}\n\\begin{document}\\maketitle\nBody.\\end{document}",
        );
        assert_eq!(d.meta.title.as_deref(), Some("On LaTeX"));
        assert_eq!(d.meta.author.as_deref(), Some("Ada Example , Bo Example"));
        assert_eq!(d.meta.language.as_deref(), Some("fr"));
        assert_eq!(
            d.text().to_string(),
            "On LaTeX\n\nAda Example , Bo Example\n\nSpring\n\nBody."
        );
    }

    #[test]
    fn macros_without_arguments_expand_in_text_and_math() {
        let d = load(
            "\\newcommand{\\course}{Biology 101}\\newcommand{\\R}{\\mathbb{R}}\\newcommand{\\pair}[2]{(#1,#2)}\nIn \\course, $x \\in \\R$. \\pair{a}{b}",
        );
        let text = d.text().to_string();
        assert!(
            text.starts_with("In Biology 101, $x \\in {\\mathbb {R}}$. ab"),
            "{text}"
        );
        assert!(
            crate::warnings(&d.meta)
                .iter()
                .any(|w| w.contains("\\pair")),
            "{:?}",
            crate::warnings(&d.meta)
        );
    }

    #[test]
    fn unknown_commands_read_their_arguments_and_can_be_named() {
        let src = "Some \\hl[yellow]{marked} words.";
        let d = load(src);
        assert_eq!(d.text().to_string(), "Some marked words.");
        assert!(crate::warnings(&d.meta).iter().any(|w| w.contains("\\hl")));
        let named = load_with(
            src,
            &LoadOptions {
                name_skipped_commands: true,
                ..LoadOptions::default()
            },
        );
        assert_eq!(named.text().to_string(), "Some (command hl) marked words.");
    }

    #[test]
    fn verbatim_and_code() {
        let d = load(
            "Use \\verb|x_1 % y| now.\n\\begin{lstlisting}[language=Python]\nprint(1)  # hi\n\\end{lstlisting}\nAfter.",
        );
        assert_eq!(
            d.text().to_string(),
            "Use x_1 % y now.\n\nprint(1)  # hi\n\nAfter."
        );
        let code = d.marker_index().nth(MarkerKind::Code, Some(1), 0).cloned();
        assert_eq!(code.and_then(|c| c.label).as_deref(), Some("Python"));
    }

    #[test]
    fn special_characters_accents_and_ligatures() {
        let d = load("Caf\\'e, na\\\"ive, 50\\% \\& more --- ``quoted'' 1--2 \\ss{} \\c{c}a.");
        assert_eq!(
            d.text().to_string(),
            "Café, naïve, 50% & more \u{2014} \u{201c}quoted\u{201d} 1\u{2013}2 ß ça."
        );
    }

    #[test]
    fn links_and_urls() {
        let d = load(
            "See \\url{https://example.org/a_b%20c} and \\href{https://example.org}{the site}.",
        );
        let links: Vec<(String, Option<String>)> = d
            .marker_index()
            .iter(MarkerKind::Link, None)
            .map(|m| (d.slice(m.range), m.reference.clone()))
            .collect();
        assert_eq!(
            links,
            [
                (
                    "https://example.org/a_b%20c".into(),
                    Some("https://example.org/a_b%20c".into())
                ),
                ("the site".into(), Some("https://example.org".into()))
            ]
        );
    }

    #[test]
    fn figures_are_described_by_their_captions() {
        let d = load(
            "\\begin{figure}[h]\\centering\\includegraphics[width=3cm]{cell.png}\\caption{A cell.}\\label{f}\\end{figure}\nFigure \\ref{f}. \\includegraphics{loose.jpg}",
        );
        let images: Vec<(String, Option<String>)> = d
            .marker_index()
            .iter(MarkerKind::Image, None)
            .map(|m| (d.slice(m.range), m.reference.clone()))
            .collect();
        assert_eq!(
            images,
            [
                ("Figure 1: A cell.".into(), Some("cell.png".into())),
                ("loose.jpg".into(), Some("loose.jpg".into()))
            ]
        );
        assert!(d.text().to_string().contains("Figure 1. loose.jpg"));
    }

    #[test]
    fn theorems_and_proofs() {
        let d = load(
            "\\newtheorem{thm}{Theorem}\n\\begin{thm}[Euclid]\\label{t}Primes are infinite.\\end{thm}\n\\begin{proof}Suppose not.\\end{proof}\nBy \\autoref{t}.",
        );
        assert_eq!(
            d.text().to_string(),
            "Theorem 1 (Euclid). Primes are infinite.\n\nProof. Suppose not.\n\nBy Theorem 1."
        );
    }

    #[test]
    fn braces_and_switches_scope_markers() {
        let d = load("A {\\bf bold \\it both} plain \\textbf{b} end.");
        assert_eq!(d.text().to_string(), "A bold both plain b end.");
        assert_eq!(kinds(&d, MarkerKind::Bold), ["bold both", "b"]);
        assert_eq!(kinds(&d, MarkerKind::Italic), ["both"]);
    }

    #[test]
    fn hostile_nesting_and_macro_bombs_are_bounded() {
        let deep = format!("{}x{}", "{".repeat(100_000), "}".repeat(100_000));
        let d = load(&deep);
        assert_eq!(d.text().to_string(), "x");
        assert!(
            crate::warnings(&d.meta)
                .iter()
                .any(|w| w == crate::NESTING_WARNING)
        );
        let bomb = "\\def\\a{\\a\\a}\\a";
        let d = load(bomb);
        assert!(crate::warnings(&d.meta).iter().any(|w| w == CUT_WARNING));
        let args = "\\textbf{".repeat(50_000);
        let d = load(&args);
        assert!(d.text().to_string().is_empty());
        let envs = "\\begin{itemize}\\item a".repeat(10_000);
        let d = load(&envs);
        assert!(d.text().to_string().starts_with('a'));
    }

    #[test]
    fn unclosed_math_and_groups_read_as_text() {
        let d = load("Costs $5 and\n\nmore } text \\end{itemize} {open");
        assert_eq!(d.text().to_string(), "Costs $5 and\n\nmore text open");
        assert!(kinds(&d, MarkerKind::Math).is_empty());
    }

    #[test]
    fn includes_stay_inside_the_folder() {
        let dir = tempfile::tempdir().expect("temp dir");
        let sub = dir.path().join("book");
        std::fs::create_dir(&sub).expect("folder");
        std::fs::create_dir(sub.join("ch")).expect("folder");
        std::fs::write(sub.join("ch").join("one.tex"), "\\section{One}\nInside.").expect("write");
        std::fs::write(dir.path().join("secret.tex"), "Secret.").expect("write");
        let main = sub.join("main.tex");
        std::fs::write(
            &main,
            "\\input{ch/one}\n\\input{../secret}\n\\include{missing}\n\\input{main}",
        )
        .expect("write");
        let d = crate::Registry::with_builtins()
            .load(&Source::Path(main), &LoadOptions::default())
            .expect("loads");
        let text = d.text().to_string();
        assert!(text.starts_with("1 One\n\nInside."), "{text}");
        assert!(!text.contains("Secret"), "{text}");
        let warnings = crate::warnings(&d.meta);
        assert!(
            warnings
                .iter()
                .any(|w| w.contains("secret") && w.contains("outside")),
            "{warnings:?}"
        );
        assert!(
            warnings.iter().any(|w| w.contains("missing")),
            "{warnings:?}"
        );
        // main.tex includes itself: stopped by the depth limit.
        assert!(
            warnings.iter().any(|w| w.contains("too many")),
            "{warnings:?}"
        );
    }

    #[test]
    fn helpers() {
        assert_eq!(letter(1), "A");
        assert_eq!(letter(27), "AA");
        assert_eq!(roman(14), "xiv");
        assert_eq!(enumerate_label(3, 3), "iii.");
        assert_eq!(accented('\'', 'e'), "é");
        assert_eq!(accented('\'', 'q').chars().count(), 2);
        assert_eq!(
            ligatures("a--b---c``d''"),
            "a\u{2013}b\u{2014}c\u{201c}d\u{201d}"
        );
        assert_eq!(scan_math("x$", Close::Dollar), Some(("x", 2)));
        assert_eq!(scan_math("x\n\ny$", Close::Dollar), None);
        assert_eq!(scan_math("a \\] b", Close::Escaped(']')), Some(("a ", 4)));
    }
}
