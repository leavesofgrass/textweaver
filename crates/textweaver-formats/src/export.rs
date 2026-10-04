//! Exports: a [`Document`] as Markdown, HTML, or plain text.
//!
//! Every exporter takes a document and its options and returns a string;
//! nothing here touches the file system, so batch converters can call them
//! from any thread ([`ExportFormat`] and [`export`] pick one by name).
//!
//! - **Markdown** ([`to_markdown_with`]): `#` headings, `-` or numbered list
//!   items indented under their parent item's marker, GFM pipe tables (the
//!   first row is always the header row, as GFM requires), fenced code
//!   blocks (the fence outgrows any backtick run inside), `>` quotes,
//!   `**bold**`, `*italic*`, `<u>underline</u>`, `` `code` ``, links, and
//!   images. Text is escaped so it reads back as the same text: Markdown
//!   punctuation (`\`, `` ` ``, `*`, `[`, `]`, `_` at word edges, `<` before
//!   a tag-like character, `&` before an entity-like run, `~~`, and `|` in
//!   table cells) and line starts that would become structure (`#`, `>`,
//!   `-`/`+` bullets, `1.` numbers, `===`/`---` underlines). Lines inside one
//!   paragraph end with a hard break (two spaces). Loading the output with
//!   [`MarkdownLoader`](crate::MarkdownLoader) gives back the same headings,
//!   lists, tables, and text.
//! - **HTML** ([`to_html`]): semantic HTML5 (`h1`–`h6`, `p`, nested
//!   `ul`/`ol`/`li` with `start`, `table` with `thead`/`th scope="col"`,
//!   `pre`/`code`, `blockquote`, `strong`, `em`, `u`, `a`, `img alt`),
//!   page breaks as DPUB-ARIA `doc-pagebreak` markers so assistive
//!   technology can go to a page, footnote references linked to their notes
//!   (`doc-noteref`, `doc-footnote`); optionally a standalone page with
//!   `lang`, `title`, and author metadata.
//! - **Text** ([`to_text`]): the canonical text (headings, list items, and
//!   table rows on lines of their own), optionally wrapped to a line width.

use std::fmt::Write as _;

use textweaver_core::{CharPos, CharRange, MarkerKind};
use textweaver_text::{Document, Marker};

/// An export format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExportFormat {
    /// Markdown (CommonMark with GFM tables).
    Markdown,
    /// HTML5.
    Html,
    /// Plain text.
    Text,
}

impl ExportFormat {
    /// The format for a name or extension: `md`/`markdown`, `html`/`htm`,
    /// `txt`/`text` (any case).
    pub fn from_name(name: &str) -> Option<Self> {
        match name
            .trim()
            .trim_start_matches('.')
            .to_ascii_lowercase()
            .as_str()
        {
            "md" | "markdown" => Some(ExportFormat::Markdown),
            "html" | "htm" => Some(ExportFormat::Html),
            "txt" | "text" | "plain" => Some(ExportFormat::Text),
            _ => None,
        }
    }

    /// The usual file extension, without a dot.
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Markdown => "md",
            ExportFormat::Html => "html",
            ExportFormat::Text => "txt",
        }
    }
}

/// Exports `doc` in `format` with default options (a standalone page for
/// HTML).
pub fn export(doc: &Document, format: ExportFormat) -> String {
    match format {
        ExportFormat::Markdown => to_markdown(doc),
        ExportFormat::Html => to_html(
            doc,
            &HtmlOptions {
                standalone: true,
                ..HtmlOptions::default()
            },
        ),
        ExportFormat::Text => to_text(doc, &TextOptions::default()),
    }
}

/// Options for [`to_markdown_with`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MarkdownOptions {
    /// Start with YAML front matter holding the title, author, and language.
    pub front_matter: bool,
    /// Mark page starts (from paginated sources such as PDF) with
    /// `<!-- page N -->` comments, which render as nothing.
    pub page_comments: bool,
}

/// Options for [`to_html`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlOptions {
    /// A complete page (`<!DOCTYPE html>`, `lang`, `title`, author, a
    /// `main` landmark) rather than a fragment.
    pub standalone: bool,
    /// Page starts as `doc-pagebreak` markers.
    pub page_breaks: bool,
}

impl Default for HtmlOptions {
    fn default() -> Self {
        HtmlOptions {
            standalone: false,
            page_breaks: true,
        }
    }
}

/// Options for [`to_text`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextOptions {
    /// Wrap lines longer than this many chars at spaces (no wrapping when
    /// `None`; code lines are never wrapped).
    pub wrap: Option<usize>,
}

/// Inline markup inserted at a position: closings sort before openings.
#[derive(Clone, Debug)]
struct Insert {
    at: usize,
    closing: bool,
    order: usize,
    text: String,
    /// For HTML images: the chars of the marker are the `alt` attribute and
    /// are not written as text.
    skip_to: Option<usize>,
}

/// Markdown or HTML flavor of the shared walk.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Flavor {
    Markdown,
    Html,
}

fn inserts(doc: &Document, flavor: Flavor, page_marks: bool) -> Vec<Insert> {
    let mut out: Vec<Insert> = Vec::new();
    for (order, m) in doc.markers().iter().enumerate() {
        let r = m.reference.as_deref().unwrap_or("");
        if m.kind == MarkerKind::PageBreak {
            if page_marks {
                let label = m.label.as_deref().unwrap_or("");
                let text = match flavor {
                    Flavor::Markdown => format!("<!-- page {label} -->"),
                    Flavor::Html => format!(
                        "<span class=\"pagebreak\" role=\"doc-pagebreak\" id=\"page-{}\" aria-label=\"Page {}\"></span>",
                        attr(label),
                        attr(label)
                    ),
                };
                out.push(Insert {
                    at: m.range.start.0,
                    closing: false,
                    order,
                    text,
                    skip_to: None,
                });
            }
            continue;
        }
        let (open, close) = match (flavor, m.kind) {
            (Flavor::Markdown, MarkerKind::Bold) => ("**".to_owned(), "**".to_owned()),
            (Flavor::Markdown, MarkerKind::Italic) => ("*".to_owned(), "*".to_owned()),
            (Flavor::Markdown, MarkerKind::Underline) => ("<u>".to_owned(), "</u>".to_owned()),
            (Flavor::Markdown, MarkerKind::Code) if m.level == 0 => {
                // Backslash escapes are literal inside a code span, so the
                // code is written as it is.
                let code = doc.slice(m.range);
                let ticks = "`".repeat(longest_run(&code, '`') + 1);
                let pad = if code.starts_with('`') || code.ends_with('`') {
                    " "
                } else {
                    ""
                };
                raw(
                    &mut out,
                    m,
                    order,
                    format!("{ticks}{pad}{code}{pad}{ticks}"),
                );
                continue;
            }
            // Math keeps its delimiters in the text and is written as it is.
            (Flavor::Markdown, MarkerKind::Math) => {
                raw(&mut out, m, order, doc.slice(m.range));
                continue;
            }
            (Flavor::Html, MarkerKind::Math) => {
                let class = if m.level == 1 {
                    "math display"
                } else {
                    "math inline"
                };
                let html = format!(
                    "<span class=\"{class}\">{}</span>",
                    text(&doc.slice(m.range))
                );
                raw(&mut out, m, order, html);
                continue;
            }
            (Flavor::Markdown, MarkerKind::Strikethrough) => ("~~".to_owned(), "~~".to_owned()),
            (Flavor::Html, MarkerKind::Strikethrough) => ("<del>".to_owned(), "</del>".to_owned()),
            (Flavor::Markdown, MarkerKind::Link) => {
                ("[".to_owned(), format!("]({})", md_destination(r)))
            }
            (Flavor::Markdown, MarkerKind::Image) => {
                ("![".to_owned(), format!("]({})", md_destination(r)))
            }
            (Flavor::Html, MarkerKind::Bold) => ("<strong>".to_owned(), "</strong>".to_owned()),
            (Flavor::Html, MarkerKind::Italic) => ("<em>".to_owned(), "</em>".to_owned()),
            (Flavor::Html, MarkerKind::Underline) => ("<u>".to_owned(), "</u>".to_owned()),
            (Flavor::Html, MarkerKind::Code) if m.level == 0 => {
                ("<code>".to_owned(), "</code>".to_owned())
            }
            (Flavor::Html, MarkerKind::Link) => {
                (format!("<a href=\"{}\">", attr(r)), "</a>".to_owned())
            }
            (Flavor::Html, MarkerKind::Footnote) if m.level == 0 => (
                format!("<a href=\"#fn-{}\" role=\"doc-noteref\">", attr(r)),
                "</a>".to_owned(),
            ),
            (Flavor::Html, MarkerKind::Image) => {
                let alt = doc.slice(m.range);
                let tag = if m.range.is_empty() {
                    // No description: no `alt`, which would say decorative.
                    format!("<img src=\"{}\">", attr(r))
                } else if r.is_empty() {
                    format!(
                        "<span role=\"img\" aria-label=\"{}\">{}</span>",
                        attr(&alt),
                        text(&alt)
                    )
                } else {
                    format!("<img src=\"{}\" alt=\"{}\">", attr(r), attr(&alt))
                };
                out.push(Insert {
                    at: m.range.start.0,
                    closing: false,
                    order,
                    text: tag,
                    skip_to: Some(m.range.end.0),
                });
                continue;
            }
            _ => continue,
        };
        if m.range.is_empty() {
            continue;
        }
        out.push(Insert {
            at: m.range.start.0,
            closing: false,
            order,
            text: open,
            skip_to: None,
        });
        out.push(Insert {
            at: m.range.end.0,
            closing: true,
            order: usize::MAX - order,
            text: close,
            skip_to: None,
        });
    }
    out.sort_by_key(|i| (i.at, !i.closing, i.order));
    out
}

/// An insert that replaces a marker's text with `text`, written as it is.
fn raw(out: &mut Vec<Insert>, m: &Marker, order: usize, text: String) {
    out.push(Insert {
        at: m.range.start.0,
        closing: false,
        order,
        text,
        skip_to: Some(m.range.end.0),
    });
}

/// True when a horizontal rule stands before the block starting at `at`.
fn rule_at(doc: &Document, at: CharPos) -> bool {
    doc.marker_index()
        .starting_at(at)
        .iter()
        .any(|m| m.kind == MarkerKind::Rule)
}

/// True when a horizontal rule ends the document.
fn rule_at_end(doc: &Document) -> bool {
    let end = CharPos(doc.len_chars());
    doc.markers()
        .iter()
        .rev()
        .take_while(|m| m.range.start == end)
        .any(|m| m.kind == MarkerKind::Rule)
}

fn longest_run(s: &str, c: char) -> usize {
    let mut best = 0;
    let mut cur = 0;
    for ch in s.chars() {
        if ch == c {
            cur += 1;
            best = best.max(cur);
        } else {
            cur = 0;
        }
    }
    best
}

/// A link destination that survives Markdown: wrapped in `<>` when it has
/// spaces or parentheses.
fn md_destination(r: &str) -> String {
    if r.contains([' ', '(', ')', '<', '>']) {
        format!("<{}>", r.replace('<', "%3C").replace('>', "%3E"))
    } else {
        r.to_owned()
    }
}

/// Escapes text for HTML content.
fn text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Escapes text for an HTML attribute value.
fn attr(s: &str) -> String {
    text(s).replace('"', "&quot;")
}

/// Renders `range` (within one line) with inline markup and escaping.
struct Inline<'a> {
    doc: &'a Document,
    inserts: Vec<Insert>,
    flavor: Flavor,
}

impl Inline<'_> {
    fn render(&self, range: CharRange, in_cell: bool) -> String {
        let mut out = String::new();
        let chars: Vec<char> = self.doc.slice(range).chars().collect();
        let first = self.inserts.partition_point(|i| i.at < range.start.0);
        let mut k = first;
        let mut skip_until = 0usize;
        for (i, &c) in chars.iter().enumerate() {
            let pos = range.start.0 + i;
            while k < self.inserts.len() && self.inserts[k].at <= pos {
                let ins = &self.inserts[k];
                if ins.at == pos || (ins.at >= range.start.0 && ins.at <= pos) {
                    out.push_str(&ins.text);
                    if let Some(to) = ins.skip_to {
                        skip_until = skip_until.max(to);
                    }
                }
                k += 1;
            }
            if pos < skip_until {
                continue;
            }
            let prev = if i > 0 { chars[i - 1] } else { ' ' };
            let next = chars.get(i + 1).copied().unwrap_or(' ');
            match self.flavor {
                Flavor::Markdown => md_escape(&mut out, c, prev, next, in_cell),
                Flavor::Html => match c {
                    '&' => out.push_str("&amp;"),
                    '<' => out.push_str("&lt;"),
                    '>' => out.push_str("&gt;"),
                    _ => out.push(c),
                },
            }
        }
        // Closings (and openings of empty-content spans) at the range end.
        while k < self.inserts.len() && self.inserts[k].at <= range.end.0 {
            let ins = &self.inserts[k];
            if ins.closing || ins.skip_to.is_some() {
                out.push_str(&ins.text);
            }
            k += 1;
        }
        out
    }
}

fn md_escape(out: &mut String, c: char, prev: char, next: char, in_cell: bool) {
    let escape = match c {
        '\\' | '`' | '*' | '[' | ']' => true,
        '_' => !(prev.is_alphanumeric() && next.is_alphanumeric()),
        '<' => next.is_ascii_alphabetic() || matches!(next, '/' | '!' | '?'),
        '&' => next.is_ascii_alphanumeric() || next == '#',
        '~' => next == '~' || prev == '~',
        '|' => in_cell,
        _ => false,
    };
    if escape {
        out.push('\\');
    }
    out.push(c);
}

/// Escapes a line start that Markdown would read as structure.
fn md_line_start(s: String) -> String {
    let t = s.trim_start();
    if t.len() != s.len() {
        return md_line_start(t.to_owned());
    }
    let first = s.chars().next().unwrap_or(' ');
    let second = s.chars().nth(1).unwrap_or(' ');
    if matches!(first, '#' | '>') || (matches!(first, '-' | '+') && second.is_whitespace()) {
        return format!("\\{s}");
    }
    if !s.is_empty() && (s.chars().all(|c| c == '=') || s.chars().all(|c| c == '-')) {
        return format!("\\{s}");
    }
    let digits = s.chars().take_while(char::is_ascii_digit).count();
    if (1..=9).contains(&digits) {
        let rest = &s[digits..];
        let mut it = rest.chars();
        if let Some(p @ ('.' | ')')) = it.next()
            && it.next().is_none_or(char::is_whitespace)
        {
            return format!("{}\\{p}{}", &s[..digits], &rest[1..]);
        }
    }
    s
}

/// Walks the document line by line with the block markers each line starts
/// or sits in.
struct Walk<'a> {
    /// Block code markers, quotes, sorted by start.
    codes: Vec<&'a Marker>,
    quotes: Vec<&'a Marker>,
}

impl<'a> Walk<'a> {
    fn new(doc: &'a Document) -> Self {
        let pick = |k: MarkerKind, lvl: Option<u8>| {
            doc.markers()
                .iter()
                .filter(move |m| m.kind == k && lvl.is_none_or(|l| m.level == l))
                .collect::<Vec<_>>()
        };
        Walk {
            codes: pick(MarkerKind::Code, Some(1)),
            quotes: pick(MarkerKind::Quote, None),
        }
    }

    fn code_at(&self, at: CharPos) -> Option<&'a Marker> {
        let i = self.codes.partition_point(|m| m.range.start <= at);
        self.codes[..i]
            .iter()
            .rev()
            .find(|m| m.range.contains(at) || m.range.start == at)
            .copied()
    }

    fn quote_depth(&self, range: CharRange) -> usize {
        let at = range.start;
        let i = self.quotes.partition_point(|m| m.range.start <= at);
        self.quotes[..i]
            .iter()
            .filter(|m| m.range.contains(at) || (m.range.start == at && !range.is_empty()))
            .count()
    }
}

/// The document as Markdown with default options.
pub fn to_markdown(doc: &Document) -> String {
    to_markdown_with(doc, &MarkdownOptions::default())
}

fn yaml(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// The document as Markdown.
pub fn to_markdown_with(doc: &Document, options: &MarkdownOptions) -> String {
    let mut out = String::new();
    if options.front_matter {
        out.push_str("---\n");
        if let Some(t) = &doc.meta.title {
            let _ = writeln!(out, "title: {}", yaml(t));
        }
        if let Some(a) = &doc.meta.author {
            let _ = writeln!(out, "author: {}", yaml(a));
        }
        if let Some(l) = &doc.meta.language {
            let _ = writeln!(out, "lang: {}", yaml(l));
        }
        out.push_str("---\n\n");
    }
    let walk = Walk::new(doc);
    let index = doc.marker_index();
    let inline = Inline {
        doc,
        inserts: inserts(doc, Flavor::Markdown, options.page_comments),
        flavor: Flavor::Markdown,
    };
    // Width of the marker (plus its space) of the last item at each depth.
    let mut widths: Vec<usize> = Vec::new();
    let lines = doc.line_count();
    let mut in_table: Option<CharRange> = None;
    for line in 0..lines {
        let range = doc.line_range(line);
        let at = range.start;
        let quote = "> ".repeat(walk.quote_depth(range));
        let starting = index.starting_at(at);
        if !range.is_empty() && rule_at(doc, at) {
            out.push_str(&quote);
            out.push_str("---\n");
            out.push_str(quote.trim_end());
            out.push('\n');
        }
        if let Some(code) = walk.code_at(at) {
            let fence = "`".repeat(longest_run(&doc.slice(code.range), '`').max(2) + 1);
            if code.range.start == at {
                out.push_str(&quote);
                out.push_str(&fence);
                out.push_str(code.label.as_deref().unwrap_or(""));
                out.push('\n');
            }
            out.push_str(&quote);
            out.push_str(&doc.slice(range));
            if code.range.end <= range.end {
                out.push('\n');
                out.push_str(&quote);
                out.push_str(&fence);
            }
        } else if range.is_empty() || doc.line_is_blank(line) {
            out.push_str(quote.trim_end());
            widths.clear();
        } else if let Some(row) = starting.iter().find(|m| m.kind == MarkerKind::TableRow) {
            out.push_str(&quote);
            let cells: Vec<&Marker> = index
                .starting_in(row.range)
                .iter()
                .filter(|m| m.kind == MarkerKind::TableCell && row.range.contains_range(m.range))
                .collect();
            let rendered: Vec<String> = if cells.is_empty() {
                vec![inline.render(range, true)]
            } else {
                cells.iter().map(|c| inline.render(c.range, true)).collect()
            };
            let _ = write!(out, "| {} |", rendered.join(" | "));
            let first_row = index
                .enclosing(MarkerKind::Table, at)
                .is_some_and(|t| in_table != Some(t.range) && t.range.start <= at);
            if let Some(t) = index.enclosing(MarkerKind::Table, at)
                && first_row
            {
                in_table = Some(t.range);
                out.push('\n');
                out.push_str(&quote);
                out.push_str(&"|---".repeat(rendered.len().max(1)));
                out.push('|');
            }
        } else {
            out.push_str(&quote);
            let mut content = range;
            if let Some(h) = starting.iter().find(|m| m.kind == MarkerKind::Heading) {
                let _ = write!(out, "{} ", "#".repeat(usize::from(h.level.clamp(1, 6))));
                let mut t = inline.render(content, false);
                if t.ends_with('#') {
                    t.insert(t.len() - 1, '\\');
                }
                out.push_str(&t);
            } else if let Some(item) = starting.iter().find(|m| m.kind == MarkerKind::ListItem) {
                let depth = usize::from(item.level.max(1));
                let bullet = match item.label.as_deref() {
                    Some(l) if is_md_number(l) => l.to_owned(),
                    _ => "-".to_owned(),
                };
                widths.truncate(depth - 1);
                while widths.len() < depth - 1 {
                    widths.push(2);
                }
                let indent: usize = widths.iter().sum();
                widths.push(bullet.chars().count() + 1);
                let _ = write!(out, "{}{} ", " ".repeat(indent), bullet);
                if let Some(l) = item.label.as_deref().filter(|l| !is_md_number(l)) {
                    let mut esc = String::new();
                    for c in l.chars() {
                        md_escape(&mut esc, c, ' ', ' ', false);
                    }
                    let _ = write!(out, "{esc} ");
                }
                content = CharRange::new(content.start.0, content.end.0);
                out.push_str(&md_line_start(inline.render(content, false)));
            } else {
                out.push_str(&md_line_start(inline.render(content, false)));
                // A line break inside one paragraph is a hard break.
                if line + 1 < lines && !doc.line_is_blank(line + 1) && !starts_block(doc, line + 1)
                {
                    out.push_str("  ");
                }
            }
        }
        if line + 1 < lines {
            out.push('\n');
        }
    }
    if rule_at_end(doc) {
        out.push_str("\n\n---");
    }
    out.push('\n');
    out
}

/// A Markdown ordered-list marker: digits then `.` or `)`.
fn is_md_number(l: &str) -> bool {
    let digits = l.chars().take_while(char::is_ascii_digit).count();
    (1..=9).contains(&digits) && matches!(&l[digits..], "." | ")")
}

/// True when line `line` starts a block of its own (heading, item, row,
/// code, quote, paragraph), so the line before it needs no hard break.
fn starts_block(doc: &Document, line: usize) -> bool {
    let at = doc.line_range(line).start;
    doc.marker_index()
        .starting_at(at)
        .iter()
        .any(|m| m.is_block() || m.kind == MarkerKind::Image)
}

/// The document as HTML.
pub fn to_html(doc: &Document, options: &HtmlOptions) -> String {
    let walk = Walk::new(doc);
    let index = doc.marker_index();
    let inline = Inline {
        doc,
        inserts: inserts(doc, Flavor::Html, options.page_breaks),
        flavor: Flavor::Html,
    };
    let mut body = String::new();
    // Open lists: (depth, ordered, list range), each with an open <li>.
    let mut lists: Vec<(u8, bool, Option<CharRange>)> = Vec::new();
    let mut quote_depth = 0usize;
    let mut in_para = false;
    let mut table: Option<(CharRange, bool)> = None;
    let mut code_open = false;
    let lines = doc.line_count();
    let close_lists =
        |body: &mut String, lists: &mut Vec<(u8, bool, Option<CharRange>)>, to: u8| {
            while lists.last().is_some_and(|(d, _, _)| *d > to) {
                if let Some((_, ordered, _)) = lists.pop() {
                    body.push_str(if ordered {
                        "</li></ol>\n"
                    } else {
                        "</li></ul>\n"
                    });
                }
            }
        };
    for line in 0..lines {
        let range = doc.line_range(line);
        let at = range.start;
        let blank = range.is_empty() || doc.line_is_blank(line);
        let starting = index.starting_at(at);
        let code = walk.code_at(at);
        let item = starting.iter().find(|m| m.kind == MarkerKind::ListItem);
        let row = starting.iter().find(|m| m.kind == MarkerKind::TableRow);
        let heading = starting.iter().find(|m| m.kind == MarkerKind::Heading);
        let note = starting
            .iter()
            .find(|m| m.kind == MarkerKind::Footnote && m.level == 1);

        // Close what this line does not continue.
        let continues_para = in_para
            && !blank
            && item.is_none()
            && row.is_none()
            && heading.is_none()
            && code.is_none()
            && !starts_block(doc, line);
        if in_para && !continues_para {
            body.push_str("</p>\n");
            in_para = false;
        }
        if item.is_none() && !(blank && !lists.is_empty() && next_is_item(doc, line)) {
            close_lists(&mut body, &mut lists, 0);
        }
        if let Some((t, _)) = table
            && (row.is_none() || !t.contains(at))
        {
            body.push_str("</tbody></table>\n");
            table = None;
        }
        if code_open && code.is_none() {
            body.push_str("</code></pre>\n");
            code_open = false;
        }
        let depth = if blank {
            quote_depth
        } else {
            walk.quote_depth(range)
        };
        while quote_depth < depth {
            body.push_str("<blockquote>\n");
            quote_depth += 1;
        }
        while quote_depth > depth {
            body.push_str("</blockquote>\n");
            quote_depth -= 1;
        }
        if blank {
            continue;
        }
        if rule_at(doc, at) {
            body.push_str("<hr>\n");
        }

        if let Some(c) = code {
            if !code_open {
                match c.label.as_deref() {
                    Some(lang) => {
                        let _ = write!(body, "<pre><code class=\"language-{}\">", attr(lang));
                    }
                    None => body.push_str("<pre><code>"),
                }
                code_open = true;
            } else {
                body.push('\n');
            }
            body.push_str(&text(&doc.slice(range)));
        } else if let Some(h) = heading {
            let l = h.level.clamp(1, 6);
            let _ = writeln!(body, "<h{l}>{}</h{l}>", inline.render(range, false));
        } else if let Some(item) = item {
            let depth = item.level.max(1);
            let ordered = item
                .label
                .as_deref()
                .is_some_and(|l| l.chars().any(|c| c.is_ascii_digit()));
            let list = index.enclosing(MarkerKind::List, at).map(|m| m.range);
            close_lists(&mut body, &mut lists, depth);
            // Another list (or another kind) at this depth closes the open one.
            if lists.last().is_some_and(|(d, o, r)| {
                *d == depth && (*o != ordered || (r.is_some() && *r != list))
            }) {
                close_lists(&mut body, &mut lists, depth - 1);
            }
            if lists.last().is_some_and(|(d, _, _)| *d == depth) {
                body.push_str("</li>\n<li>");
            } else {
                while lists.last().is_none_or(|(d, _, _)| *d < depth) {
                    let d = lists.last().map_or(1, |(d, _, _)| d + 1);
                    let tag = if ordered && d == depth {
                        let start: String = item
                            .label
                            .as_deref()
                            .unwrap_or("")
                            .chars()
                            .filter(char::is_ascii_digit)
                            .collect();
                        match start.parse::<u64>() {
                            Ok(n) if n != 1 => format!("<ol start=\"{n}\">"),
                            _ => "<ol>".to_owned(),
                        }
                    } else {
                        "<ul>".to_owned()
                    };
                    let _ = write!(body, "{}\n<li>", tag);
                    lists.push((
                        d,
                        ordered && d == depth,
                        if d == depth { list } else { None },
                    ));
                }
            }
            body.push_str(&inline.render(range, false));
        } else if let Some(row) = row {
            let header = row.is_header_row();
            if table.is_none() {
                body.push_str("<table>\n");
                if header {
                    body.push_str("<thead>");
                } else {
                    body.push_str("<tbody>");
                }
                table = index
                    .enclosing(MarkerKind::Table, at)
                    .map(|t| (t.range, header));
                if table.is_none() {
                    table = Some((row.range, header));
                }
            } else if let Some((_, true)) = table
                && !header
            {
                body.push_str("</thead>\n<tbody>");
                if let Some(t) = table.as_mut() {
                    t.1 = false;
                }
            }
            let cells: Vec<&Marker> = index
                .starting_in(row.range)
                .iter()
                .filter(|m| m.kind == MarkerKind::TableCell && row.range.contains_range(m.range))
                .collect();
            body.push_str("<tr>");
            let tag = if header { "th scope=\"col\"" } else { "td" };
            let close = if header { "th" } else { "td" };
            if cells.is_empty() {
                let _ = write!(body, "<{tag}>{}</{close}>", inline.render(range, false));
            }
            for c in cells {
                let _ = write!(body, "<{tag}>{}</{close}>", inline.render(c.range, false));
            }
            body.push_str("</tr>\n");
        } else if let Some(n) = note {
            let id = n.reference.as_deref().unwrap_or("");
            let _ = writeln!(
                body,
                "<p id=\"fn-{}\" role=\"doc-footnote\">{}</p>",
                attr(id),
                inline.render(range, false)
            );
        } else if continues_para {
            body.push_str("<br>\n");
            body.push_str(&inline.render(range, false));
        } else {
            body.push_str("<p>");
            body.push_str(&inline.render(range, false));
            in_para = true;
        }
    }
    if in_para {
        body.push_str("</p>\n");
    }
    if rule_at_end(doc) {
        body.push_str("<hr>\n");
    }
    close_lists(&mut body, &mut lists, 0);
    if table.is_some() {
        body.push_str("</tbody></table>\n");
    }
    if code_open {
        body.push_str("</code></pre>\n");
    }
    for _ in 0..quote_depth {
        body.push_str("</blockquote>\n");
    }
    if !options.standalone {
        return body;
    }
    let mut out = String::from("<!DOCTYPE html>\n");
    match &doc.meta.language {
        Some(l) => {
            let _ = writeln!(out, "<html lang=\"{}\">", attr(l));
        }
        None => out.push_str("<html>\n"),
    }
    out.push_str("<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    let title = doc.meta.title.as_deref().unwrap_or("Untitled document");
    let _ = writeln!(out, "<title>{}</title>", text(title));
    if let Some(a) = &doc.meta.author {
        let _ = writeln!(out, "<meta name=\"author\" content=\"{}\">", attr(a));
    }
    out.push_str("</head>\n<body>\n<main>\n");
    out.push_str(&body);
    out.push_str("</main>\n</body>\n</html>\n");
    out
}

/// True when the next non-blank line after `line` starts a list item.
fn next_is_item(doc: &Document, line: usize) -> bool {
    let n = doc.line_count();
    let mut l = line + 1;
    while l < n && doc.line_is_blank(l) {
        l += 1;
    }
    l < n
        && doc
            .marker_index()
            .starting_at(doc.line_range(l).start)
            .iter()
            .any(|m| m.kind == MarkerKind::ListItem)
}

/// The document as plain text.
pub fn to_text(doc: &Document, options: &TextOptions) -> String {
    let mut out = String::with_capacity(doc.len_chars() + 1);
    let walk = Walk::new(doc);
    for line in 0..doc.line_count() {
        let range = doc.line_range(line);
        let s = doc.slice(range);
        match options.wrap {
            Some(w) if w > 0 && s.chars().count() > w && walk.code_at(range.start).is_none() => {
                wrap_into(&mut out, &s, w);
            }
            _ => out.push_str(&s),
        }
        out.push('\n');
    }
    out
}

fn wrap_into(out: &mut String, s: &str, width: usize) {
    let mut len = 0;
    for (i, word) in s.split_whitespace().enumerate() {
        let n = word.chars().count();
        if i > 0 && len + 1 + n > width {
            out.push('\n');
            len = 0;
        } else if i > 0 {
            out.push(' ');
            len += 1;
        }
        out.push_str(word);
        len += n;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LoadOptions, Loader, MarkdownLoader, Source};

    fn md(src: &str) -> Document {
        MarkdownLoader
            .load(
                &Source::Bytes {
                    data: src.as_bytes().to_vec(),
                    hint: "md".into(),
                },
                &LoadOptions::default(),
            )
            .unwrap()
    }

    #[test]
    fn round_trips_common_structure() {
        let src = "# Title\n\nSome **bold** and [a link](https://x.org).\n\n- one\n  - two\n\n1. first\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n> Quoted.\n\n```rust\nlet x = 1;\n```\n";
        let doc = md(src);
        assert_eq!(
            to_markdown(&doc),
            "# Title\n\nSome **bold** and [a link](https://x.org).\n\n- one\n  - two\n\n1. first\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n> Quoted.\n\n```rust\nlet x = 1;\n```\n"
        );
    }

    #[test]
    fn math_strikethrough_rules_and_code_round_trip() {
        let src =
            "Area $\\pi r_1^2$ and ~~old~~ new; `a*b_c` code.\n\n---\n\n$$\\frac{a}{b}$$\n\n***\n";
        let doc = md(src);
        let out = to_markdown(&doc);
        assert_eq!(
            out,
            "Area $\\pi r_1^2$ and ~~old~~ new; `a*b_c` code.\n\n---\n\n$$\\frac{a}{b}$$\n\n---\n"
        );
        // Reading the export back gives the same text and markers.
        let again = md(&out);
        assert_eq!(again.text().to_string(), doc.text().to_string());
        assert_eq!(again.markers(), doc.markers());
        let html = to_html(&doc, &HtmlOptions::default());
        assert!(
            html.contains("<span class=\"math inline\">$\\pi r_1^2$</span>"),
            "{html}"
        );
        assert!(html.contains("<del>old</del>"), "{html}");
        assert_eq!(html.matches("<hr>").count(), 2, "{html}");
        assert!(
            html.contains("<span class=\"math display\">$$\\frac{a}{b}$$</span>"),
            "{html}"
        );
    }

    #[test]
    fn escapes_text_that_would_become_markup() {
        let doc = Document::from_plain_text(
            "# not a heading\n1. not a list\n- nor this\nstars *here* and snake_case, _edge_, [brackets], a<b>, x & y, &amp; ~~no~~\n---",
        );
        let out = to_markdown(&doc);
        assert_eq!(
            out,
            "\\# not a heading  \n1\\. not a list  \n\\- nor this  \nstars \\*here\\* and snake_case, \\_edge\\_, \\[brackets\\], a\\<b>, x & y, \\&amp; \\~\\~no\\~\\~  \n\\---\n"
        );
        // Loading the Markdown gives back the text.
        assert_eq!(md(&out).text().to_string(), doc.text().to_string());
    }

    #[test]
    fn nested_ordered_items_indent_under_their_marker() {
        let doc = md("1. one\n   1. inner\n2. two\n");
        let out = to_markdown(&doc);
        assert_eq!(out, "1. one\n   1. inner\n2. two\n");
        let back = md(&out);
        assert_eq!(back.text().to_string(), doc.text().to_string());
    }

    #[test]
    fn html_is_semantic_and_escaped() {
        let doc = md(
            "# T & co\n\nA <b>b</b> **bold** [l](https://x.org?a=1&b=2).\n\n- a\n  - b\n- c\n\n3. x\n4. y\n\n| H | I |\n|---|---|\n| 1 | 2 |\n\n```\n<code>\n```\n\n![alt \"q\"](p.png)\n",
        );
        let html = to_html(&doc, &HtmlOptions::default());
        assert!(html.contains("<h1>T &amp; co</h1>"), "{html}");
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<a href=\"https://x.org?a=1&amp;b=2\">l</a>"));
        assert!(
            html.contains("<ul>\n<li>a<ul>\n<li>b</li></ul>\n</li>\n<li>c</li></ul>"),
            "{html}"
        );
        assert!(
            html.contains("<ol start=\"3\">\n<li>x</li>\n<li>y</li></ol>"),
            "{html}"
        );
        assert!(html.contains("<thead><tr><th scope=\"col\">H</th><th scope=\"col\">I</th></tr>\n</thead>\n<tbody><tr><td>1</td><td>2</td></tr>\n</tbody></table>"), "{html}");
        assert!(html.contains("<pre><code>&lt;code&gt;</code></pre>"));
        assert!(
            html.contains("<img src=\"p.png\" alt=\"alt &quot;q&quot;\">"),
            "{html}"
        );
        let page = to_html(
            &doc,
            &HtmlOptions {
                standalone: true,
                ..HtmlOptions::default()
            },
        );
        assert!(page.starts_with("<!DOCTYPE html>\n<html>\n<head>"));
        assert!(page.contains("<title>T &amp; co</title>"));
        assert!(page.contains("<main>"));
    }

    #[test]
    fn text_wraps_but_not_code() {
        let doc = Document::from_plain_text("one two three four five");
        assert_eq!(
            to_text(&doc, &TextOptions { wrap: Some(9) }),
            "one two\nthree\nfour five\n"
        );
        assert_eq!(ExportFormat::from_name(".MD"), Some(ExportFormat::Markdown));
        assert_eq!(ExportFormat::Html.extension(), "html");
    }
}
