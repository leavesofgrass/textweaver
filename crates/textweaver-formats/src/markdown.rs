//! Markdown loader: CommonMark (plus tables, footnotes, strikethrough, task
//! lists, math, GFM alerts, wiki links, heading attributes, and front
//! matter) parsed with pulldown-cmark into canonical text and markers.
//!
//! Unlike Star, which strips Markdown with regular expressions (and merges
//! headings into lists, speaks front matter, and never processes inline
//! footnotes), the source is parsed, so:
//!
//! - headings, list items, and table rows are lines of their own; ordered
//!   list numbers are the item marker's label, not text;
//! - YAML (`---`) or TOML (`+++`) front matter fills [`DocumentMeta`] and is
//!   not part of the text;
//! - footnote references read `[label]` under a `Footnote` marker; their
//!   definitions go after the text under a "Footnotes" heading
//!   ([`FootnoteMode::Deferred`]), replace the reference as
//!   `(footnote: text)` ([`FootnoteMode::Inline`]), or are left out
//!   ([`FootnoteMode::Skip`]);
//! - code blocks are text under a `Code` marker (level 1, label = language),
//!   dropped entirely with [`LoadOptions::skip_code`]; inline code is kept;
//! - raw HTML is dropped, except `<br>`, which breaks the line;
//! - math (`$…$`, `$$…$$`) keeps its delimiters in the text under a `Math`
//!   marker (level 1 for display math), so speech reads it as math and the
//!   writers typeset it; `$5 and $10` stays prose;
//! - struck-through text is under a `Strikethrough` marker, and a horizontal
//!   rule is an empty `Rule` marker where the next block starts, so both
//!   can be heard;
//! - a GFM alert (`> [!NOTE]`) or an Obsidian callout of any type
//!   (`> [!tip]- Remember`) is a block quote labeled with its type as
//!   written, read "Tip, collapsed: Remember" before its body ("Note:"
//!   when it has no title); the rules are shared with the renderer
//!   ([`crate::callout`]);
//! - wiki links (`[[Page]]`, `[[Page|shown]]`) are links to the page;
//!   heading attributes (`# Title {#id .class}`) are not read;
//! - Obsidian's embeds (`![[note]]`, `![[note#Heading]]`,
//!   `![[note#^id]]`) read the other note in place, from the note's own
//!   folder or below, two levels deep at most; `![[picture.png|300]]` is a
//!   graphic named by its file; tags (`#physics/waves`) are read "tag
//!   physics slash waves"; `==highlights==` are under an `Underline`
//!   marker labeled [`HIGHLIGHT_LABEL`]; `%%comments%%` are not read; and
//!   block ids (`^id`) are not read but kept as link targets (see
//!   [`crate::obsidian`]).

use std::collections::BTreeMap;
use std::path::Path;

use pulldown_cmark::{
    BlockQuoteKind, CodeBlockKind, Event, LinkType, MetadataBlockKind, Options, Parser, Tag, TagEnd,
};
use ropey::Rope;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, DocumentMeta, HEADER_ROW_LABEL, Marker};

use crate::builder::{Builder, OpenId};
use crate::obsidian::{EmbedTarget, Embeds, Piece};
use crate::{
    FootnoteMode, LoadError, LoadOptions, Loader, Source, decode_source, meta_for, note_encoding,
    title_from_path,
};

/// Loads Markdown.
#[derive(Clone, Copy, Debug, Default)]
pub struct MarkdownLoader;

impl Loader for MarkdownLoader {
    fn id(&self) -> &'static str {
        "markdown"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &[
            "md", "markdown", "mdown", "mkd", "mkdn", "mdwn", "mdtxt", "rmd",
        ]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        let decoded = decode_source(source, None)?;
        let mut meta = meta_for(source, self.id());
        note_encoding(&mut meta, &decoded);
        let path = match source {
            Source::Path(p) if p.is_file() => Some(p.as_path()),
            _ => None,
        };
        let (canonical, markers) = convert_in(&decoded.text, path, options, &mut meta);
        if meta.title.is_none() {
            meta.title = title_from_path(source);
        }
        Ok(Document::new(meta, Rope::from_str(&canonical), markers))
    }
}

fn parser_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
        | Options::ENABLE_MATH
        | Options::ENABLE_GFM
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_WIKILINKS
}

/// Math source with its delimiters, as it stays in the canonical text.
fn delimited_math(t: &str, display: bool) -> String {
    if display {
        format!("$${t}$$")
    } else {
        format!("${t}$")
    }
}

/// What a GFM alert's text starts with.
fn alert_label(kind: BlockQuoteKind) -> &'static str {
    match kind {
        BlockQuoteKind::Note => "Note",
        BlockQuoteKind::Tip => "Tip",
        BlockQuoteKind::Important => "Important",
        BlockQuoteKind::Warning => "Warning",
        BlockQuoteKind::Caution => "Caution",
    }
}

/// A callout title as words: the emphasis, code, highlight, and wiki
/// link marks a title may carry are not read.
fn plain_title(title: &str) -> String {
    let mut t = title.to_owned();
    for mark in ["**", "__", "==", "~~", "[[", "]]", "`", "*"] {
        t = t.replace(mark, "");
    }
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Converts Markdown source to canonical text and markers, filling `meta`
/// from front matter and the first level-1 heading. Embedded notes
/// (`![[note]]`) are links: without a folder there is nowhere to find them
/// (see [`convert_in`]).
pub fn convert(
    source: &str,
    options: &LoadOptions,
    meta: &mut DocumentMeta,
) -> (String, Vec<Marker>) {
    convert_in(source, None, options, meta)
}

/// [`convert`] for a note at `path`: notes it embeds are read in place
/// from its folder or below it (see [`crate::obsidian`]).
pub fn convert_in(
    source: &str,
    path: Option<&Path>,
    options: &LoadOptions,
    meta: &mut DocumentMeta,
) -> (String, Vec<Marker>) {
    // Only inline footnotes need every definition before the text that
    // refers to it, so only they parse the source twice. Deferred notes are
    // gathered from the one pass that builds the text, and skipped notes
    // need none.
    let footnotes = if options.footnotes == FootnoteMode::Inline {
        collect_footnotes(source)
    } else {
        Vec::new()
    };
    let embeds = path.and_then(|p| {
        let folder = p.parent().filter(|f| !f.as_os_str().is_empty());
        Embeds::new(folder.unwrap_or(Path::new(".")), Some(p))
    });
    let mut gather = (options.footnotes == FootnoteMode::Deferred).then(FootnoteCollector::default);
    let b = Builder::with_capacity(source.len());
    let mut c = Converter::new(b, options, meta, footnotes, embeds);
    for (event, range) in Parser::new_ext(source, parser_options()).into_offset_iter() {
        if let Some(g) = gather.as_mut() {
            g.event(&event);
        }
        c.event_at(event, range, source);
    }
    if let Some(g) = gather {
        c.footnotes = g.out;
    }
    c.deferred_footnotes();
    let ids = std::mem::take(&mut c.block_ids);
    crate::obsidian::set_block_ids(c.meta, &ids);
    crate::pause_markup::record(c.meta, c.b.take_pauses());
    c.b.finish()
}

/// Converts Markdown into a builder that already holds text (a notebook's
/// markdown cells). Footnotes are read in place (`Deferred` becomes
/// `Inline`), since the part has no end of its own to gather them at.
pub(crate) fn convert_into(
    b: Builder,
    source: &str,
    options: &LoadOptions,
    meta: &mut DocumentMeta,
) -> Builder {
    run_into(b, source, options, meta, None)
}

/// [`convert_into`] with the embeds of the note being read.
fn run_into(
    b: Builder,
    source: &str,
    options: &LoadOptions,
    meta: &mut DocumentMeta,
    embeds: Option<Embeds>,
) -> Builder {
    let options = LoadOptions {
        footnotes: match options.footnotes {
            FootnoteMode::Deferred => FootnoteMode::Inline,
            other => other,
        },
        ..options.clone()
    };
    let footnotes = if options.footnotes == FootnoteMode::Inline {
        collect_footnotes(source)
    } else {
        Vec::new()
    };
    let mut c = Converter::new(b, &options, meta, footnotes, embeds);
    for (event, range) in Parser::new_ext(source, parser_options()).into_offset_iter() {
        c.event_at(event, range, source);
    }
    c.end_highlight();
    while !c.stack.is_empty() {
        c.pop();
    }
    std::mem::take(&mut c.b)
}

/// Footnote definitions as plain text, in source order: a pass of its own,
/// for inline footnotes.
fn collect_footnotes(source: &str) -> Vec<(String, String)> {
    // A definition starts with `[^`; without one there is nothing to find.
    if !source.contains("[^") {
        return Vec::new();
    }
    let mut g = FootnoteCollector::default();
    for event in Parser::new_ext(source, parser_options()) {
        g.event(&event);
    }
    g.out
}

/// Gathers footnote definitions as plain text from a stream of events.
#[derive(Default)]
struct FootnoteCollector {
    out: Vec<(String, String)>,
    current: Option<(String, String)>,
    depth: usize,
}

impl FootnoteCollector {
    fn event(&mut self, event: &Event<'_>) {
        match event {
            Event::Start(Tag::FootnoteDefinition(label)) => {
                self.depth += 1;
                if self.depth == 1 {
                    self.current = Some((label.to_string(), String::new()));
                }
            }
            Event::End(TagEnd::FootnoteDefinition) => {
                self.depth = self.depth.saturating_sub(1);
                if self.depth == 0
                    && let Some((label, text)) = self.current.take()
                {
                    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
                    self.out.push((label, text));
                }
            }
            Event::Text(t) | Event::Code(t) => {
                if let Some((_, text)) = self.current.as_mut() {
                    text.push_str(t);
                }
            }
            Event::InlineMath(t) => {
                if let Some((_, text)) = self.current.as_mut() {
                    text.push_str(&delimited_math(t, false));
                }
            }
            Event::DisplayMath(t) => {
                if let Some((_, text)) = self.current.as_mut() {
                    text.push_str(&delimited_math(t, true));
                }
            }
            Event::SoftBreak | Event::HardBreak | Event::End(TagEnd::Paragraph) => {
                if let Some((_, text)) = self.current.as_mut() {
                    text.push(' ');
                }
            }
            _ => {}
        }
    }
}

struct ListState {
    depth: u8,
    next_number: Option<u64>,
}

struct Converter<'a> {
    b: Builder,
    options: &'a LoadOptions,
    meta: &'a mut DocumentMeta,
    /// Footnote definitions: every one from the start for inline notes,
    /// gathered by the end of the pass for deferred ones.
    footnotes: Vec<(String, String)>,
    /// Open markers, one per open tag that has one (None for tags without).
    stack: Vec<Option<OpenId>>,
    lists: Vec<ListState>,
    cell_index: usize,
    /// Inside a skipped subtree (footnote definition, skipped code block).
    skip_depth: usize,
    /// Text of the code block being read.
    code: Option<String>,
    /// Text of the front matter being read.
    meta_text: Option<(MetadataBlockKind, String)>,
    /// Text of the first level-1 heading, while it is being read.
    heading_text: Option<String>,
    /// For each open image: its title, and whether alt text was written.
    image: Vec<(String, usize)>,
    /// Raw HTML read so far (comments and scripts span events).
    html: HtmlState,
    /// A callout's label ("Note:", "Tip, collapsed:"), written before its
    /// first text.
    alert: Option<String>,
    /// While reading a callout's head line: where the line ends. Its text
    /// (`[!tip] Title`) is not read; the label says it.
    head_end: Option<usize>,
    /// Where embedded notes come from, when the document has a folder.
    embeds: Option<Embeds>,
    /// Inside an Obsidian comment (`%%...%%`), which is not read.
    in_comment: bool,
    /// An open highlight (`==text==`).
    highlight: Option<OpenId>,
    /// How many `==` there are from the text being read to the end of its
    /// block: one opens a highlight only when another follows it.
    marks_ahead: usize,
    /// The text being read starts after white space or at a line start,
    /// where a `#` starts a tag.
    boundary: bool,
    /// Block ids (`^id`) and where their blocks start.
    block_ids: Vec<(String, usize)>,
    /// The stack depth of the open paragraph, if the innermost open tag is
    /// one, so an embed can end it before reading another note.
    paragraph_at: Option<usize>,
}

/// Where the reader is inside raw HTML, carried from one HTML event to the
/// next (a block's lines arrive as separate events).
#[derive(Debug, Default)]
struct HtmlState {
    /// Inside `<!-- ... -->`.
    in_comment: bool,
    /// Inside `<script>` or `<style>`: the closing tag that ends it.
    skip_until: Option<&'static str>,
}

/// HTML tags that start a new line of text.
const HTML_BLOCK_TAGS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "center",
    "dd",
    "details",
    "div",
    "dl",
    "dt",
    "figcaption",
    "figure",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "li",
    "main",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "summary",
    "table",
    "tr",
    "ul",
];

/// The common character references, decoded (`&amp;`, `&lt;`, `&#233;`,
/// `&#xe9;`, `&nbsp;`, ...); anything else is kept as written.
fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let end = rest[1..].find(';').map(|e| e + 1).filter(|&e| e <= 10);
        let decoded = end.and_then(|e| {
            let name = &rest[1..e];
            let c = match name {
                "amp" => '&',
                "lt" => '<',
                "gt" => '>',
                "quot" => '"',
                "apos" => '\'',
                "nbsp" => ' ',
                _ => {
                    let num = name.strip_prefix('#')?;
                    let n = match num.strip_prefix(['x', 'X']) {
                        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                        None => num.parse().ok()?,
                    };
                    char::from_u32(n)?
                }
            };
            Some((c, e))
        });
        match decoded {
            Some((c, e)) => {
                out.push(c);
                rest = &rest[e + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The value of attribute `name` in a tag's inside (`img src="a" alt="b"`).
fn html_attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find(name) {
        let at = from + i;
        from = at + name.len();
        let before = lower[..at].chars().last();
        if !before.is_some_and(char::is_whitespace) {
            continue;
        }
        let rest = tag[from..].trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim_start();
        let value = match rest.chars().next() {
            Some(q @ ('"' | '\'')) => rest[1..].split(q).next().unwrap_or(""),
            _ => rest
                .split(|c: char| c.is_whitespace() || c == '/')
                .next()
                .unwrap_or(""),
        };
        return Some(decode_entities(value));
    }
    None
}

/// The label of the `Underline` marker a highlight (`==text==`) is read
/// under, until the core has a marker kind of its own for it.
pub const HIGHLIGHT_LABEL: &str = "highlight";

impl<'a> Converter<'a> {
    fn new(
        b: Builder,
        options: &'a LoadOptions,
        meta: &'a mut DocumentMeta,
        footnotes: Vec<(String, String)>,
        embeds: Option<Embeds>,
    ) -> Self {
        Converter {
            b,
            options,
            meta,
            footnotes,
            stack: Vec::new(),
            lists: Vec::new(),
            cell_index: 0,
            skip_depth: 0,
            code: None,
            meta_text: None,
            heading_text: None,
            image: Vec::new(),
            html: HtmlState::default(),
            alert: None,
            head_end: None,
            embeds,
            in_comment: false,
            highlight: None,
            marks_ahead: 0,
            boundary: true,
            block_ids: Vec::new(),
            paragraph_at: None,
        }
    }

    fn in_list(&self) -> bool {
        !self.lists.is_empty()
    }

    /// Text from the source: Obsidian's comments left out, tags read as
    /// words, highlights marked, and a trailing block id kept aside.
    fn source_text(&mut self, t: &str) {
        // Most text has none of Obsidian's marks: read it as it is, with
        // no allocation (one byte scan instead of cutting it into pieces).
        if !t.bytes().any(|b| matches!(b, b'#' | b'=' | b'%' | b'^')) {
            if !self.in_comment {
                self.text(t);
            }
            return;
        }
        let pieces = crate::obsidian::pieces(t, self.boundary);
        let last = pieces.len().saturating_sub(1);
        for (i, piece) in pieces.into_iter().enumerate() {
            match piece {
                Piece::Comment => self.in_comment = !self.in_comment,
                _ if self.in_comment => {}
                Piece::Text(s) => match crate::obsidian::trailing_block_id(s) {
                    Some((before, id)) if i == last => {
                        self.text(before);
                        self.block_id(id);
                    }
                    _ => self.text(s),
                },
                Piece::Tag(words) => self.text(&words),
                Piece::Mark => match self.highlight.take() {
                    Some(id) => {
                        self.marks_ahead = self.marks_ahead.saturating_sub(1);
                        self.b.close(id);
                    }
                    None if self.marks_ahead >= 2 => {
                        self.marks_ahead -= 1;
                        let m = Self::marker(MarkerKind::Underline).with_label(HIGHLIGHT_LABEL);
                        self.highlight = Some(self.b.open(m));
                    }
                    None => {
                        self.marks_ahead = self.marks_ahead.saturating_sub(1);
                        self.text("==");
                    }
                },
            }
        }
    }

    /// Records a block id where its block starts: the paragraph, item,
    /// heading, or row it ends, or for an id alone on its line the table,
    /// list, quote, or code block before it.
    fn block_id(&mut self, id: &str) {
        use MarkerKind as K;
        const BLOCKS: [MarkerKind; 4] = [K::ListItem, K::Paragraph, K::Heading, K::TableRow];
        const BEFORE: [MarkerKind; 5] = [K::Table, K::List, K::Quote, K::Code, K::Paragraph];
        // A block with text of its own is the one the id ends; an id alone
        // in its paragraph names the block before.
        let at = self
            .b
            .innermost_open_start(&BLOCKS)
            .or_else(|| self.b.last_closed_start(&BEFORE));
        if let Some(at) = at
            && !self.block_ids.iter().any(|(k, _)| k == id)
        {
            self.block_ids.push((id.to_owned(), at));
        }
    }

    /// Ends an open highlight at the end of its block.
    fn end_highlight(&mut self) {
        if let Some(id) = self.highlight.take() {
            self.b.close(id);
        }
    }

    /// `![[...]]`: a picture by its file name, another note read in place,
    /// or a link when it cannot be read.
    fn embed(&mut self, target: &str) {
        match crate::obsidian::embed_target(target) {
            EmbedTarget::Image => {
                let name = Path::new(target.split('#').next().unwrap_or(target))
                    .file_name()
                    .map_or_else(|| target.to_owned(), |n| n.to_string_lossy().into_owned());
                let id = self
                    .b
                    .open(Self::marker(MarkerKind::Image).with_reference(target));
                self.text(&name);
                self.b.close(id);
            }
            EmbedTarget::Note { name, part } => {
                let read = match &self.embeds {
                    Some(e) => e.read(&name),
                    None => Err(crate::obsidian::Refused::NotFound),
                };
                match read {
                    Ok((path, text)) => self.embed_note(&name, part.as_deref(), path, &text),
                    Err(why) => {
                        let id = self
                            .b
                            .open(Self::marker(MarkerKind::Link).with_reference(target));
                        self.text(&name);
                        self.b.close(id);
                        self.b.space();
                        self.b.text(why.words());
                    }
                }
            }
            EmbedTarget::Other => {
                let id = self
                    .b
                    .open(Self::marker(MarkerKind::Link).with_reference(target));
                self.text(target);
                self.b.close(id);
            }
        }
    }

    /// Reads another note (or its part under a heading or block id) in
    /// place: "Embedded from Name" before it and "End of embed" after, the
    /// whole under a block quote labeled `embed`.
    fn embed_note(&mut self, name: &str, part: Option<&str>, path: std::path::PathBuf, text: &str) {
        let body = match part {
            Some(p) if p.starts_with('^') => {
                crate::obsidian::block_section(text, p.trim_start_matches('^'))
            }
            Some(h) => crate::obsidian::heading_section(text, h).map(str::to_owned),
            None => Some(text.to_owned()),
        };
        let shown = match part {
            Some(p) => format!("{name}, {}", p.trim_start_matches('^')),
            None => name.to_owned(),
        };
        let Some(body) = body else {
            let id = self
                .b
                .open(Self::marker(MarkerKind::Link).with_reference(name));
            self.text(&shown);
            self.b.close(id);
            self.b.space();
            self.b.text("(part not found)");
            return;
        };
        // A paragraph holding only the embed ends before the other note.
        if let Some(at) = self.paragraph_at
            && at + 1 == self.stack.len()
            && let Some(Some(id)) = self.stack.last_mut().map(Option::take)
        {
            self.b.close(id);
        }
        self.end_highlight();
        self.block_break();
        let quote = self
            .b
            .open(Self::marker(MarkerKind::Quote).with_label("embed"));
        self.b.text(&format!("Embedded from {shown}"));
        self.b.paragraph_break();
        let mut scratch = DocumentMeta::default();
        let embeds = self.embeds.as_ref().map(|e| e.child(path));
        let b = std::mem::take(&mut self.b);
        self.b = run_into(b, &body, self.options, &mut scratch, embeds);
        self.b.paragraph_break();
        self.b.text("End of embed");
        self.b.close(quote);
        self.block_break();
    }

    fn block_break(&mut self) {
        if self.in_list() {
            self.b.line_break();
        } else {
            self.b.paragraph_break();
        }
    }

    fn push(&mut self, marker: Option<Marker>) {
        let id = marker.map(|m| self.b.open(m));
        self.stack.push(id);
    }

    fn pop(&mut self) {
        if let Some(Some(id)) = self.stack.pop() {
            self.b.close(id);
        }
    }

    fn marker(kind: MarkerKind) -> Marker {
        Marker::new(kind, CharRange::empty(0))
    }

    fn text(&mut self, t: &str) {
        if let Some(label) = self.alert.take() {
            self.b.text(&label);
            self.b.space();
        }
        if let Some(h) = self.heading_text.as_mut() {
            h.push_str(t);
        }
        if let Some((_, written)) = self.image.last_mut() {
            *written += t.trim().len();
        }
        self.b.text(t);
    }

    /// Reads raw HTML: text kept (entities decoded), tags dropped, with a
    /// line break for `<br>` and block tags and alt text for `<img>`.
    fn html(&mut self, h: &str) {
        let mut rest = h;
        loop {
            if self.html.in_comment {
                match rest.find("-->") {
                    Some(i) => {
                        rest = &rest[i + 3..];
                        self.html.in_comment = false;
                    }
                    None => return,
                }
            }
            if let Some(end) = self.html.skip_until {
                match rest.to_ascii_lowercase().find(end) {
                    Some(i) => {
                        rest = &rest[i..];
                        self.html.skip_until = None;
                    }
                    None => return,
                }
            }
            let Some(i) = rest.find('<') else {
                self.html_text(rest);
                return;
            };
            self.html_text(&rest[..i]);
            rest = &rest[i..];
            if let Some(after) = rest.strip_prefix("<!--") {
                self.html.in_comment = true;
                rest = after;
                continue;
            }
            let Some(close) = rest.find('>') else {
                // Not a tag after all ("a < b").
                self.html_text(rest);
                return;
            };
            let tag = &rest[1..close];
            rest = &rest[close + 1..];
            self.html_tag(tag);
        }
    }

    fn html_text(&mut self, t: &str) {
        if !t.is_empty() {
            self.text(&decode_entities(t));
        }
    }

    fn html_tag(&mut self, tag: &str) {
        let inner = tag.trim();
        let closing = inner.starts_with('/');
        let name: String = inner
            .trim_start_matches('/')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        match name.as_str() {
            "br" => self.b.line_break(),
            "break" if !closing && !self.options.keep_pause_markup => {
                if let Some(ms) = crate::pause_markup::tag_ms(inner) {
                    self.b.pause(ms);
                }
            }
            "img" if !closing => {
                let alt = html_attr(inner, "alt").unwrap_or_default();
                if !alt.trim().is_empty() {
                    let src = html_attr(inner, "src").unwrap_or_default();
                    let id = self
                        .b
                        .open(Self::marker(MarkerKind::Image).with_reference(src));
                    self.text(&alt);
                    self.b.close(id);
                }
            }
            "script" if !closing => self.html.skip_until = Some("</script"),
            "style" if !closing => self.html.skip_until = Some("</style"),
            "td" | "th" => self.b.space(),
            n if HTML_BLOCK_TAGS.contains(&n) => self.b.line_break(),
            _ => {}
        }
    }

    /// An event with where it is in the source: a block quote whose first
    /// line is a callout head (`> [!tip]- Title`) becomes a callout.
    fn event_at(&mut self, event: Event<'_>, range: std::ops::Range<usize>, source: &str) {
        if let Some(end) = self.head_end {
            if range.start < end {
                if matches!(
                    event,
                    Event::Text(_)
                        | Event::Code(_)
                        | Event::InlineHtml(_)
                        | Event::InlineMath(_)
                        | Event::FootnoteReference(_)
                        | Event::SoftBreak
                        | Event::HardBreak
                ) {
                    return;
                }
            } else {
                self.head_end = None;
            }
        }
        if let Event::Text(t) = &event {
            let before = source.get(..range.start).unwrap_or("");
            self.boundary = before.is_empty() || before.ends_with(char::is_whitespace);
            if t.contains("==") {
                let rest = source.get(range.start..).unwrap_or("");
                let block = rest.find("\n\n").map_or(rest, |end| &rest[..end]);
                self.marks_ahead = block.matches("==").count();
            }
        }
        if let Event::Start(Tag::BlockQuote(_)) = &event
            && self.skip_depth == 0
            && self.code.is_none()
            && self.meta_text.is_none()
        {
            let rest = source.get(range.start..).unwrap_or("");
            let line = rest.split('\n').next().unwrap_or(rest);
            if let Some(head) = crate::callout::head(line, true) {
                self.callout(&head);
                self.head_end = Some(range.start + line.len());
                return;
            }
        }
        self.event(event);
    }

    /// Starts a callout: a block quote labeled with its type as written
    /// (`tip`, `hint`), read as "Tip: Title" on a line of its own then its
    /// body, or "Tip:"
    /// before the body when it has no title of its own. A foldable one
    /// says its state once, in the label ("Tip, collapsed:").
    fn callout(&mut self, head: &crate::callout::CalloutHead) {
        self.block_break();
        let m = Self::marker(MarkerKind::Quote).with_label(head.kind.clone());
        self.push(Some(m));
        let fold = match head.fold {
            Some(crate::callout::Fold::Collapsed) => ", collapsed",
            Some(crate::callout::Fold::Expanded) => ", expanded",
            None => "",
        };
        let label = format!("{}{fold}:", head.type_word());
        let title = plain_title(&head.custom_title);
        if title.is_empty() {
            self.alert = Some(label);
        } else {
            self.alert = None;
            self.b.text(&format!("{label} {title}"));
            self.b.line_break();
        }
    }

    fn event(&mut self, event: Event<'_>) {
        if self.skip_depth > 0 {
            match event {
                Event::Start(_) => self.skip_depth += 1,
                Event::End(_) => self.skip_depth -= 1,
                _ => {}
            }
            return;
        }
        if let Some((_, text)) = self.meta_text.as_mut() {
            match event {
                Event::Text(t) => text.push_str(&t),
                Event::End(TagEnd::MetadataBlock(_)) => {
                    if let Some((kind, text)) = self.meta_text.take() {
                        apply_front_matter(kind, &text, self.meta);
                    }
                }
                _ => {}
            }
            return;
        }
        if let Some(code) = self.code.as_mut() {
            match event {
                Event::Text(t) => code.push_str(&t),
                Event::End(TagEnd::CodeBlock) => {
                    let code = self.code.take().unwrap_or_default();
                    self.b.verbatim(&code);
                    self.pop();
                    self.block_break();
                }
                _ => {}
            }
            return;
        }
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(t) => self.source_text(&t),
            // Nothing inside an Obsidian comment is read.
            Event::Code(_)
            | Event::InlineHtml(_)
            | Event::Html(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::FootnoteReference(_)
                if self.in_comment => {}
            Event::Code(t) => {
                let id = self.b.open(Self::marker(MarkerKind::Code));
                self.text(&t);
                self.b.close(id);
            }
            Event::SoftBreak => self.b.space(),
            Event::HardBreak => self.b.line_break(),
            Event::Rule => {
                self.b.paragraph_break();
                self.b.point(Self::marker(MarkerKind::Rule));
            }
            // Raw HTML: its text is read and its tags dropped (README-style
            // `<p align="center">`, `<details>`, `<img alt>`); comments,
            // scripts, and styles are not read (audit finding M2).
            Event::InlineHtml(h) | Event::Html(h) => self.html(&h),
            Event::FootnoteReference(label) => self.footnote_reference(&label),
            // A task list item says whether it is done through its label
            // ("checked Buy milk"); the brackets are not read (finding M1).
            Event::TaskListMarker(done) => {
                let state = if done { "checked" } else { "not checked" };
                if let Some(m) = self.b.innermost_open_mut(MarkerKind::ListItem) {
                    m.label = Some(match m.label.take() {
                        Some(number) => format!("{number} {state}"),
                        None => state.to_owned(),
                    });
                }
            }
            Event::InlineMath(t) => self.math(&t, false),
            Event::DisplayMath(t) => self.math(&t, true),
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => {
                self.block_break();
                let marker = (!self.in_list()).then(|| Self::marker(MarkerKind::Paragraph));
                self.paragraph_at = Some(self.stack.len());
                self.push(marker);
            }
            Tag::Heading { level, .. } => {
                self.block_break();
                let level = level as u8;
                if level == 1 && self.meta.title.is_none() && self.heading_text.is_none() {
                    self.heading_text = Some(String::new());
                }
                self.push(Some(Self::marker(MarkerKind::Heading).with_level(level)));
            }
            Tag::BlockQuote(kind) => {
                self.block_break();
                let mut m = Self::marker(MarkerKind::Quote);
                if let Some(kind) = kind {
                    let label = alert_label(kind);
                    m = m.with_label(label.to_lowercase());
                    self.alert = Some(format!("{label}:"));
                }
                self.push(Some(m));
            }
            Tag::CodeBlock(kind) => {
                self.block_break();
                if self.options.skip_code {
                    self.skip_depth = 1;
                    return;
                }
                let mut m = Self::marker(MarkerKind::Code).with_level(1);
                if let CodeBlockKind::Fenced(lang) = kind {
                    let lang = lang.split_whitespace().next().unwrap_or("");
                    if !lang.is_empty() {
                        m = m.with_label(lang);
                    }
                }
                self.push(Some(m));
                self.code = Some(String::new());
            }
            Tag::HtmlBlock => self.push(None),
            Tag::List(start) => {
                let depth = u8::try_from(self.lists.len() + 1).unwrap_or(u8::MAX);
                if self.lists.is_empty() {
                    self.b.paragraph_break();
                } else {
                    self.b.line_break();
                }
                self.lists.push(ListState {
                    depth,
                    next_number: start,
                });
                self.push(Some(Self::marker(MarkerKind::List).with_level(depth)));
            }
            Tag::Item => {
                self.b.line_break();
                let (depth, label) = match self.lists.last_mut() {
                    Some(l) => {
                        let label = l.next_number.map(|n| format!("{n}."));
                        if let Some(n) = l.next_number.as_mut() {
                            *n += 1;
                        }
                        (l.depth, label)
                    }
                    None => (1, None),
                };
                let mut m = Self::marker(MarkerKind::ListItem).with_level(depth);
                m.label = label;
                self.push(Some(m));
            }
            Tag::FootnoteDefinition(_) => {
                self.skip_depth = 1;
            }
            Tag::Table(_) => {
                self.block_break();
                self.push(Some(Self::marker(MarkerKind::Table)));
            }
            Tag::TableHead => {
                self.b.line_break();
                self.cell_index = 0;
                self.push(Some(
                    Self::marker(MarkerKind::TableRow).with_label(HEADER_ROW_LABEL),
                ));
            }
            Tag::TableRow => {
                self.b.line_break();
                self.cell_index = 0;
                self.push(Some(Self::marker(MarkerKind::TableRow)));
            }
            Tag::TableCell => {
                if self.cell_index > 0 {
                    self.b.separator(crate::CELL_SEPARATOR);
                }
                self.cell_index += 1;
                let id = self.b.open_here(Self::marker(MarkerKind::TableCell));
                self.stack.push(Some(id));
            }
            Tag::Emphasis => self.push(Some(Self::marker(MarkerKind::Italic))),
            Tag::Strong => self.push(Some(Self::marker(MarkerKind::Bold))),
            Tag::Link { dest_url, .. } => {
                self.push(Some(
                    Self::marker(MarkerKind::Link).with_reference(dest_url.to_string()),
                ));
            }
            Tag::Image {
                link_type,
                dest_url,
                title,
                ..
            } => {
                if matches!(link_type, LinkType::WikiLink { .. }) {
                    // `![[...]]`: an Obsidian embed; its text (a size or
                    // an alias) is not read.
                    if !self.in_comment {
                        self.embed(&dest_url);
                    }
                    self.skip_depth = 1;
                    return;
                }
                self.image.push((title.to_string(), 0));
                self.push(Some(
                    Self::marker(MarkerKind::Image).with_reference(dest_url.to_string()),
                ));
            }
            Tag::MetadataBlock(kind) => {
                self.meta_text = Some((kind, String::new()));
            }
            Tag::Strikethrough => self.push(Some(Self::marker(MarkerKind::Strikethrough))),
            Tag::Superscript
            | Tag::Subscript
            | Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition => self.push(None),
        }
    }

    fn end(&mut self, tag: TagEnd) {
        if matches!(
            tag,
            TagEnd::Paragraph
                | TagEnd::Heading(_)
                | TagEnd::Item
                | TagEnd::TableCell
                | TagEnd::BlockQuote(_)
        ) {
            self.end_highlight();
        }
        match tag {
            TagEnd::Paragraph => {
                self.paragraph_at = None;
                self.pop();
                self.block_break();
            }
            TagEnd::Heading(_) => {
                self.pop();
                if let Some(h) = self.heading_text.take() {
                    let h = h.split_whitespace().collect::<Vec<_>>().join(" ");
                    if !h.is_empty() {
                        self.meta.title = Some(h);
                    }
                }
                self.block_break();
            }
            TagEnd::BlockQuote(_) | TagEnd::Table => {
                // A callout with no body still says its type.
                if let Some(label) = self.alert.take() {
                    self.b.text(label.trim_end_matches(':'));
                }
                self.pop();
                self.block_break();
            }
            TagEnd::List(_) => {
                self.pop();
                self.lists.pop();
                self.block_break();
            }
            TagEnd::Image => {
                if let Some((title, written)) = self.image.pop()
                    && written == 0
                    && !title.trim().is_empty()
                {
                    self.b.text(&title);
                }
                self.pop();
            }
            TagEnd::Item
            | TagEnd::TableHead
            | TagEnd::TableRow
            | TagEnd::TableCell
            | TagEnd::Emphasis
            | TagEnd::Strong
            | TagEnd::Link
            | TagEnd::HtmlBlock
            | TagEnd::Strikethrough
            | TagEnd::Superscript
            | TagEnd::Subscript
            | TagEnd::DefinitionList
            | TagEnd::DefinitionListTitle
            | TagEnd::DefinitionListDefinition => self.pop(),
            TagEnd::CodeBlock | TagEnd::FootnoteDefinition | TagEnd::MetadataBlock(_) => {}
        }
    }

    /// Math with its delimiters under a `Math` marker (level 1 for display
    /// math).
    fn math(&mut self, t: &str, display: bool) {
        let id = self
            .b
            .open(Self::marker(MarkerKind::Math).with_level(u8::from(display)));
        self.text(&delimited_math(t, display));
        self.b.close(id);
    }

    fn definition(&self, label: &str) -> Option<&str> {
        self.footnotes
            .iter()
            .find(|(l, _)| l == label)
            .map(|(_, t)| t.as_str())
    }

    fn footnote_reference(&mut self, label: &str) {
        if self.options.footnotes == FootnoteMode::Skip {
            return;
        }
        if self.options.footnotes == FootnoteMode::Inline
            && let Some(def) = self.definition(label).map(str::to_owned)
        {
            self.b.inline_footnote(label, &def);
            return;
        }
        self.b.footnote_reference(label);
    }

    /// With deferred footnotes, the definitions after the text under a
    /// "Footnotes" heading, one per line.
    fn deferred_footnotes(&mut self) {
        if self.options.footnotes != FootnoteMode::Deferred || self.footnotes.is_empty() {
            return;
        }
        self.b.footnotes_section(&self.footnotes);
    }
}

/// Fills `meta` from YAML-style (`key: value` lines) or TOML front matter.
fn apply_front_matter(kind: MetadataBlockKind, text: &str, meta: &mut DocumentMeta) {
    let mut fields: BTreeMap<String, String> = BTreeMap::new();
    match kind {
        MetadataBlockKind::YamlStyle => {
            for line in text.lines() {
                if line.starts_with([' ', '\t', '-', '#']) {
                    continue;
                }
                if let Some((k, v)) = line.split_once(':') {
                    let v = v.trim().trim_matches(|c| c == '"' || c == '\'').trim();
                    if !k.trim().is_empty() && !v.is_empty() {
                        fields.insert(k.trim().to_lowercase(), v.to_owned());
                    }
                }
            }
        }
        MetadataBlockKind::PlusesStyle => {
            if let Ok(table) = text.parse::<toml::Table>() {
                for (k, v) in table {
                    let v = match v {
                        toml::Value::String(s) => s,
                        toml::Value::Array(a) => a
                            .iter()
                            .map(|x| x.as_str().map_or_else(|| x.to_string(), str::to_owned))
                            .collect::<Vec<_>>()
                            .join(", "),
                        other => other.to_string(),
                    };
                    fields.insert(k.to_lowercase(), v);
                }
            }
        }
    }
    for (k, v) in fields {
        match k.as_str() {
            "title" => meta.title = Some(v),
            "author" | "authors" => meta.author = Some(v),
            "lang" | "language" => meta.language = Some(v),
            _ => {
                meta.properties.insert(k, v);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use textweaver_core::{CharPos, MarkerKind};

    use super::*;

    fn load(src: &str, options: &LoadOptions) -> Document {
        MarkdownLoader
            .load(
                &Source::Bytes {
                    data: src.as_bytes().to_vec(),
                    hint: "md".into(),
                },
                options,
            )
            .unwrap()
    }

    fn kinds(d: &Document, kind: MarkerKind) -> Vec<String> {
        d.marker_index()
            .iter(kind, None)
            .map(|m| d.slice(m.range))
            .collect()
    }

    #[test]
    fn math_keeps_its_delimiters_under_a_math_marker() {
        let d = load(
            "The area is $\\pi r^2$, not $5 and $10.\n\n$$\\frac{a}{b}$$\n\nA $x_1 * y_2$ product.\n",
            &LoadOptions::default(),
        );
        let text = d.text().to_string();
        assert!(
            text.contains("The area is $\\pi r^2$, not $5 and $10."),
            "{text}"
        );
        let math: Vec<(String, u8)> = d
            .marker_index()
            .iter(MarkerKind::Math, None)
            .map(|m| (d.slice(m.range), m.level))
            .collect();
        assert_eq!(
            math,
            vec![
                ("$\\pi r^2$".to_owned(), 0),
                ("$$\\frac{a}{b}$$".to_owned(), 1),
                // Emphasis is not parsed inside math.
                ("$x_1 * y_2$".to_owned(), 0),
            ]
        );
        assert_eq!(d.marker_index().count(MarkerKind::Italic, None), 0);
    }

    #[test]
    fn strikethrough_and_rules_have_markers() {
        let d = load(
            "Keep ~~drop this~~ here.\n\n---\n\nAfter the rule.\n\n***\n",
            &LoadOptions::default(),
        );
        assert_eq!(kinds(&d, MarkerKind::Strikethrough), vec!["drop this"]);
        let rules: Vec<CharPos> = d
            .marker_index()
            .iter(MarkerKind::Rule, None)
            .map(|m| {
                assert!(m.range.is_empty());
                m.range.start
            })
            .collect();
        let after = d.text().to_string().find("After").unwrap();
        // The first rule is where the next paragraph starts; the last, with
        // nothing after it, at the end.
        assert_eq!(rules, vec![CharPos(after), CharPos(d.len_chars())]);
        assert!(!d.text().to_string().contains("---"));
    }

    #[test]
    fn gfm_alerts_wiki_links_and_heading_attributes() {
        let d = load(
            "# Title {#intro .big}\n\n> [!NOTE]\n> Remember to save.\n\n> [!warning]\n> Hot.\n\nSee [[Other page]] and [[Page#Part|that part]].\n",
            &LoadOptions::default(),
        );
        let text = d.text().to_string();
        assert!(text.starts_with("Title\n"), "{text}");
        assert!(text.contains("Note: Remember to save."), "{text}");
        assert!(text.contains("Warning: Hot."), "{text}");
        assert!(!text.contains("[!"), "{text}");
        let quotes: Vec<Option<String>> = d
            .marker_index()
            .iter(MarkerKind::Quote, None)
            .map(|m| m.label.clone())
            .collect();
        assert_eq!(quotes, vec![Some("note".into()), Some("warning".into())]);
        assert!(text.contains("See Other page and that part."), "{text}");
        let links: Vec<Option<String>> = d
            .marker_index()
            .iter(MarkerKind::Link, None)
            .map(|m| m.reference.clone())
            .collect();
        assert_eq!(
            links,
            vec![Some("Other page".into()), Some("Page#Part".into())]
        );
        assert_eq!(d.meta.title.as_deref(), Some("Title"));
    }

    #[test]
    fn obsidian_callouts_of_any_type_say_their_type_first() {
        let d = load(
            "> [!tip] Remember\n> Drink water.\n\n> [!warning]- Hot **stove**\n> Careful.\n\n> [!my-box]\n> Custom.\n\n> [!faq]+\n\n> [!IMPORTANT]\n> GitHub kind.\n",
            &LoadOptions::default(),
        );
        let text = d.text().to_string();
        assert!(text.starts_with("Tip: Remember\n"), "{text}");
        assert!(text.contains("Drink water."), "{text}");
        assert!(text.contains("Warning, collapsed: Hot stove\n"), "{text}");
        assert!(text.contains("My box: Custom."), "{text}");
        assert!(text.contains("FAQ, expanded"), "{text}");
        assert!(text.contains("Important: GitHub kind."), "{text}");
        assert!(!text.contains("[!"), "{text}");
        assert!(!text.contains("Remember Drink"), "{text}");
        let labels: Vec<Option<String>> = d
            .marker_index()
            .iter(MarkerKind::Quote, None)
            .map(|m| m.label.clone())
            .collect();
        assert_eq!(
            labels,
            ["tip", "warning", "my-box", "faq", "important"].map(|l| Some(l.to_owned()))
        );
        // A quote that only looks like one is still a quote.
        let plain = load("> [not a callout]\n> text\n", &LoadOptions::default());
        assert_eq!(plain.text().to_string(), "[not a callout] text");
    }

    #[test]
    fn obsidian_tags_highlights_comments_and_block_ids() {
        let d = load(
            "Waves #physics/waves and issue #12. Some ==bright **bold** words== here, a == b.\n\nHidden %%secret `code`%% gone. Next ^para-1\n\n- item one ^li\n- item two\n\n| a |\n|---|\n| 1 |\n\n^tbl\n",
            &LoadOptions::default(),
        );
        let text = d.text().to_string();
        assert!(
            text.contains("Waves tag physics slash waves and issue #12."),
            "{text}"
        );
        assert!(
            text.contains("Some bright bold words here, a == b."),
            "{text}"
        );
        assert!(text.contains("Hidden gone. Next\n"), "{text}");
        for gone in ["secret", "code", "^", "%%"] {
            assert!(!text.contains(gone), "{gone} in {text}");
        }
        let highlights: Vec<String> = d
            .marker_index()
            .iter(MarkerKind::Underline, None)
            .filter(|m| m.label.as_deref() == Some(HIGHLIGHT_LABEL))
            .map(|m| d.slice(m.range))
            .collect();
        assert_eq!(highlights, ["bright bold words"]);
        let at = |id: &str| {
            crate::obsidian::block_position(&d.meta, id)
                .map(|p| d.slice(CharRange::new(p.0, p.0 + 4)))
        };
        assert_eq!(at("para-1").as_deref(), Some("Hidd"));
        assert_eq!(at("^li").as_deref(), Some("item"));
        assert_eq!(at("tbl").as_deref(), Some("a\n1"));
        assert_eq!(at("nope"), None);
    }

    #[test]
    fn obsidian_embeds_read_notes_in_place_inside_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(
            root.join("main.md"),
            "# Main\n\n![[Other]]\n\n![[sub/Deep#Part B]]\n\n![[Deep#^blk]]\n\n![[pic.png|300]]\n\n![[Missing]]\n\n![[main]]\n",
        )
        .unwrap();
        std::fs::write(root.join("Other.md"), "Other text.\n\n![[Third]]\n").unwrap();
        std::fs::write(root.join("Third.md"), "Third text.\n\n![[Fourth]]\n").unwrap();
        std::fs::write(root.join("Fourth.md"), "Fourth text.\n").unwrap();
        std::fs::write(
            root.join("sub").join("Deep.md"),
            "## Part A\n\nno\n\n## Part B\n\nyes\n\nA block ^blk\n",
        )
        .unwrap();
        let d = MarkdownLoader
            .load(&Source::Path(root.join("main.md")), &LoadOptions::default())
            .unwrap();
        let text = d.text().to_string();
        let expect = [
            "Embedded from Other\n\nOther text.",
            "Embedded from Third\n\nThird text.",
            // Two levels deep at most.
            "Fourth (embedded too deeply to read here)",
            "End of embed",
            "Embedded from sub/Deep, Part B\n\nPart B\n\nyes",
            "Embedded from Deep, blk\n\nA block\n\nEnd of embed",
            "pic.png",
            "Missing (embedded note not found)",
            "main (embedded above, not repeated)",
        ];
        for e in expect {
            assert!(text.contains(e), "{e:?} missing from {text:?}");
        }
        assert!(!text.contains("no\n"), "{text}");
        assert!(!text.contains("300"), "{text}");
        assert_eq!(text.matches("Embedded from").count(), 4, "{text}");
        assert_eq!(text.matches("End of embed").count(), 4, "{text}");
        let embeds = d
            .marker_index()
            .iter(MarkerKind::Quote, None)
            .filter(|m| m.label.as_deref() == Some("embed"))
            .count();
        assert_eq!(embeds, 4);
        assert_eq!(kinds(&d, MarkerKind::Image), ["pic.png"]);
        // Outside the folder: never read.
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("Secret.md"), "secret").unwrap();
        let rel = format!(
            "![[{}]]",
            outside
                .path()
                .join("Secret")
                .to_string_lossy()
                .replace('\\', "/")
        );
        std::fs::write(root.join("escape.md"), format!("{rel}\n\n![[../Secret]]\n")).unwrap();
        let e = MarkdownLoader
            .load(
                &Source::Path(root.join("escape.md")),
                &LoadOptions::default(),
            )
            .unwrap();
        assert!(!e.text().to_string().contains("secret"), "{}", e.text());
    }

    #[test]
    fn task_list_items_say_checked_or_not_checked() {
        let d = load(
            "- [ ] Buy milk\n- [x] Walk the dog\n- plain\n\n1. [X] Numbered done\n",
            &LoadOptions::default(),
        );
        let text = d.text().to_string();
        assert!(!text.contains('['), "{text}");
        assert_eq!(
            kinds(&d, MarkerKind::ListItem),
            vec!["Buy milk", "Walk the dog", "plain", "Numbered done"]
        );
        let labels: Vec<Option<String>> = d
            .marker_index()
            .iter(MarkerKind::ListItem, None)
            .map(|m| m.label.clone())
            .collect();
        assert_eq!(
            labels,
            vec![
                Some("not checked".into()),
                Some("checked".into()),
                None,
                Some("1. checked".into())
            ]
        );
    }

    #[test]
    fn html_blocks_and_inline_html_keep_their_text() {
        let d = load(
            "<p align=\"center\">Welcome to the <b>project</b> &amp; friends</p>\n\n\
             <!-- a comment\nacross lines -->\n\n\
             <details>\n<summary>More &#233;tails</summary>\n\nHidden text.\n\n</details>\n\n\
             Inline <img src=\"logo.png\" alt=\"The logo\"> and <kbd>Ctrl</kbd>.\n\n\
             <script>var x = 1;</script>\n\n<style>p { color: red }</style>\n\nEnd.\n",
            &LoadOptions::default(),
        );
        let text = d.text().to_string();
        for kept in [
            "Welcome to the project & friends",
            "More \u{e9}tails",
            "Hidden text.",
            "Inline The logo and Ctrl.",
            "End.",
        ] {
            assert!(text.contains(kept), "{kept:?} missing from {text:?}");
        }
        for dropped in ["comment", "across", "var x", "color", "<", "align"] {
            assert!(!text.contains(dropped), "{dropped:?} in {text:?}");
        }
        assert_eq!(kinds(&d, MarkerKind::Image), vec!["The logo"]);
        let img = d
            .marker_index()
            .iter(MarkerKind::Image, None)
            .next()
            .cloned();
        assert_eq!(img.and_then(|m| m.reference), Some("logo.png".into()));
    }

    #[test]
    fn entities_and_attributes() {
        assert_eq!(
            decode_entities("a &lt;b&gt; &#x41;&#66; &bogus; &"),
            "a <b> AB &bogus; &"
        );
        assert_eq!(
            html_attr("img data-alt=\"no\" alt='Yes it is' src=x.png", "alt").as_deref(),
            Some("Yes it is")
        );
        assert_eq!(html_attr("img src=x.png", "alt"), None);
    }

    #[test]
    fn headings_lists_and_paragraphs() {
        let d = load(
            "# Title\n\nSome *text* and **bold**.\n\n## Lists\n\n- one\n- two\n  - nested\n\n1. first\n2. second\n",
            &LoadOptions::default(),
        );
        assert_eq!(
            d.text().to_string(),
            "Title\n\nSome text and bold.\n\nLists\n\none\ntwo\nnested\n\nfirst\nsecond"
        );
        assert_eq!(d.meta.title.as_deref(), Some("Title"));
        assert_eq!(kinds(&d, MarkerKind::Heading), ["Title", "Lists"]);
        assert_eq!(kinds(&d, MarkerKind::Italic), ["text"]);
        assert_eq!(
            kinds(&d, MarkerKind::ListItem),
            ["one", "two\nnested", "nested", "first", "second"]
        );
        let labels: Vec<Option<String>> = d
            .marker_index()
            .iter(MarkerKind::ListItem, None)
            .map(|m| m.label.clone())
            .collect();
        assert_eq!(
            labels,
            [None, None, None, Some("1.".into()), Some("2.".into())]
        );
        assert_eq!(kinds(&d, MarkerKind::List).len(), 3);
    }

    #[test]
    fn front_matter_is_metadata_not_text() {
        let d = load(
            "---\ntitle: My Doc\nauthor: \"A. Writer\"\ntags: x\n---\n\nBody.",
            &LoadOptions::default(),
        );
        assert_eq!(d.text().to_string(), "Body.");
        assert_eq!(d.meta.title.as_deref(), Some("My Doc"));
        assert_eq!(d.meta.author.as_deref(), Some("A. Writer"));
        assert_eq!(d.meta.properties.get("tags").map(String::as_str), Some("x"));
        let toml = load(
            "+++\ntitle = \"T\"\nlang = \"fr\"\n+++\n\nCorps.",
            &LoadOptions::default(),
        );
        assert_eq!(toml.meta.title.as_deref(), Some("T"));
        assert_eq!(toml.meta.language.as_deref(), Some("fr"));
    }

    #[test]
    fn tables_are_rows_of_cells() {
        let d = load(
            "| A | B | C |\n|---|---|---|\n| 1 |   | 3 |\n",
            &LoadOptions::default(),
        );
        assert_eq!(d.text().to_string(), "A | B | C\n1 |  | 3");
        let cells: Vec<String> = kinds(&d, MarkerKind::TableCell);
        assert_eq!(cells, ["A", "B", "C", "1", "", "3"]);
        let rows: Vec<bool> = d
            .marker_index()
            .iter(MarkerKind::TableRow, None)
            .map(Marker::is_header_row)
            .collect();
        assert_eq!(rows, [true, false]);
    }

    #[test]
    fn code_blocks_and_skip_code() {
        let src = "Before.\n\n```rust\nfn main() {\n    x();\n}\n```\n\nAfter `code`.";
        let d = load(src, &LoadOptions::default());
        assert_eq!(
            d.text().to_string(),
            "Before.\n\nfn main() {\n    x();\n}\n\nAfter code."
        );
        let code = d
            .marker_index()
            .iter(MarkerKind::Code, Some(1))
            .next()
            .unwrap()
            .clone();
        assert_eq!(code.label.as_deref(), Some("rust"));
        let skipped = load(
            src,
            &LoadOptions {
                skip_code: true,
                ..LoadOptions::default()
            },
        );
        assert_eq!(skipped.text().to_string(), "Before.\n\nAfter code.");
    }

    #[test]
    fn footnotes_deferred_and_inline() {
        let src = "Text.[^1] More.\n\n[^1]: The note.";
        let d = load(src, &LoadOptions::default());
        assert_eq!(
            d.text().to_string(),
            "Text.[1] More.\n\nFootnotes\n\n[1] The note."
        );
        let refs = d.marker_index().count(MarkerKind::Footnote, Some(0));
        assert_eq!(refs, 2);
        let inline = load(
            src,
            &LoadOptions {
                footnotes: FootnoteMode::Inline,
                ..LoadOptions::default()
            },
        );
        assert_eq!(
            inline.text().to_string(),
            "Text. (footnote: The note.) More."
        );
        let skip = load(
            src,
            &LoadOptions {
                footnotes: FootnoteMode::Skip,
                ..LoadOptions::default()
            },
        );
        assert_eq!(skip.text().to_string(), "Text. More.");
        assert_eq!(skip.marker_index().count(MarkerKind::Footnote, None), 0);
    }

    #[test]
    fn links_images_and_breaks() {
        let d = load(
            "See [the site](https://example.org).\nNext line  \nhard.\n\n![Alt text](a.png)\n\n![](spacer.gif)\n\nA<br>B",
            &LoadOptions::default(),
        );
        assert_eq!(
            d.text().to_string(),
            "See the site. Next line\nhard.\n\nAlt text\n\nA\nB"
        );
        let link = d
            .marker_index()
            .nth(MarkerKind::Link, None, 0)
            .unwrap()
            .clone();
        assert_eq!(link.reference.as_deref(), Some("https://example.org"));
        assert_eq!(d.slice(link.range), "the site");
        assert_eq!(kinds(&d, MarkerKind::Image), ["Alt text"]);
        assert_eq!(
            d.marker_index()
                .ordinal(MarkerKind::Image, None, CharPos(99)),
            Some((1, 1))
        );
    }
}
