//! Markdown loader: CommonMark (plus tables, footnotes, strikethrough, and
//! front matter) parsed with pulldown-cmark into canonical text and markers.
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
//!   definitions go after the text under a "Footnotes" heading, or, with
//!   [`LoadOptions::footnotes_inline`], replace the reference as
//!   `(footnote: text)`;
//! - code blocks are text under a `Code` marker (level 1, label = language),
//!   dropped entirely with [`LoadOptions::skip_code`]; inline code is kept;
//! - raw HTML is dropped, except `<br>`, which breaks the line.

use std::collections::BTreeMap;

use pulldown_cmark::{CodeBlockKind, Event, MetadataBlockKind, Options, Parser, Tag, TagEnd};
use ropey::Rope;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, DocumentMeta, HEADER_ROW_LABEL, Marker};

use crate::builder::{Builder, OpenId};
use crate::{LoadError, LoadOptions, Loader, Source, meta_for, source_text, title_from_path};

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
        let text = source_text(source)?;
        let mut meta = meta_for(source, self.id());
        let (canonical, markers) = convert(&text, options, &mut meta);
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
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
}

/// Converts Markdown source to canonical text and markers, filling `meta`
/// from front matter and the first level-1 heading.
pub fn convert(
    source: &str,
    options: &LoadOptions,
    meta: &mut DocumentMeta,
) -> (String, Vec<Marker>) {
    let footnotes = collect_footnotes(source);
    let mut c = Converter {
        b: Builder::new(),
        options,
        meta,
        footnotes: &footnotes,
        stack: Vec::new(),
        lists: Vec::new(),
        cell_index: 0,
        skip_depth: 0,
        code: None,
        meta_text: None,
        heading_text: None,
        image: Vec::new(),
    };
    for event in Parser::new_ext(source, parser_options()) {
        c.event(event);
    }
    c.deferred_footnotes();
    c.b.finish()
}

/// Footnote definitions as plain text, in source order.
fn collect_footnotes(source: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut current: Option<(String, String)> = None;
    let mut depth = 0usize;
    for event in Parser::new_ext(source, parser_options()) {
        match event {
            Event::Start(Tag::FootnoteDefinition(label)) => {
                depth += 1;
                if depth == 1 {
                    current = Some((label.to_string(), String::new()));
                }
            }
            Event::End(TagEnd::FootnoteDefinition) => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    if let Some((label, text)) = current.take() {
                        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
                        out.push((label, text));
                    }
                }
            }
            Event::Text(t) | Event::Code(t) => {
                if let Some((_, text)) = current.as_mut() {
                    text.push_str(&t);
                }
            }
            Event::SoftBreak | Event::HardBreak | Event::End(TagEnd::Paragraph) => {
                if let Some((_, text)) = current.as_mut() {
                    text.push(' ');
                }
            }
            _ => {}
        }
    }
    out
}

struct ListState {
    depth: u8,
    next_number: Option<u64>,
}

struct Converter<'a> {
    b: Builder,
    options: &'a LoadOptions,
    meta: &'a mut DocumentMeta,
    footnotes: &'a [(String, String)],
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
}

impl Converter<'_> {
    fn in_list(&self) -> bool {
        !self.lists.is_empty()
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
        if let Some(h) = self.heading_text.as_mut() {
            h.push_str(t);
        }
        if let Some((_, written)) = self.image.last_mut() {
            *written += t.trim().len();
        }
        self.b.text(t);
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
            Event::Text(t) => self.text(&t),
            Event::Code(t) => {
                let id = self.b.open(Self::marker(MarkerKind::Code));
                self.text(&t);
                self.b.close(id);
            }
            Event::SoftBreak => self.b.space(),
            Event::HardBreak => self.b.line_break(),
            Event::Rule => self.b.paragraph_break(),
            Event::InlineHtml(h) | Event::Html(h) => {
                let tag = h.trim().to_ascii_lowercase();
                if tag.starts_with("<br") {
                    self.b.line_break();
                }
            }
            Event::FootnoteReference(label) => self.footnote_reference(&label),
            Event::TaskListMarker(done) => {
                self.b.text(if done { "[x]" } else { "[ ]" });
                self.b.space();
            }
            Event::InlineMath(t) | Event::DisplayMath(t) => self.text(&t),
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => {
                self.block_break();
                let marker = (!self.in_list()).then(|| Self::marker(MarkerKind::Paragraph));
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
            Tag::BlockQuote(_) => {
                self.block_break();
                self.push(Some(Self::marker(MarkerKind::Quote)));
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
                    self.b.literal(crate::CELL_SEPARATOR);
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
                dest_url, title, ..
            } => {
                self.image.push((title.to_string(), 0));
                self.push(Some(
                    Self::marker(MarkerKind::Image).with_reference(dest_url.to_string()),
                ));
            }
            Tag::MetadataBlock(kind) => {
                self.meta_text = Some((kind, String::new()));
            }
            Tag::Strikethrough
            | Tag::Superscript
            | Tag::Subscript
            | Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition => self.push(None),
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
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
                self.pop();
                self.block_break();
            }
            TagEnd::List(_) => {
                self.pop();
                self.lists.pop();
                self.block_break();
            }
            TagEnd::Image => {
                if let Some((title, written)) = self.image.pop() {
                    if written == 0 && !title.trim().is_empty() {
                        self.b.text(&title);
                    }
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

    fn definition(&self, label: &str) -> Option<&str> {
        self.footnotes
            .iter()
            .find(|(l, _)| l == label)
            .map(|(_, t)| t.as_str())
    }

    fn footnote_reference(&mut self, label: &str) {
        if self.options.footnotes_inline {
            if let Some(def) = self.definition(label).map(str::to_owned) {
                self.b.space();
                let m = Self::marker(MarkerKind::Footnote)
                    .with_level(1)
                    .with_reference(label);
                let id = self.b.open(m);
                self.b.text(&format!("(footnote: {def})"));
                self.b.close(id);
                return;
            }
        }
        let id = self
            .b
            .open(Self::marker(MarkerKind::Footnote).with_reference(label));
        self.b.literal(&format!("[{label}]"));
        self.b.close(id);
    }

    /// With deferred footnotes, the definitions after the text under a
    /// "Footnotes" heading, one per line.
    fn deferred_footnotes(&mut self) {
        if self.options.footnotes_inline || self.footnotes.is_empty() {
            return;
        }
        self.b.paragraph_break();
        let h = self.b.open(Self::marker(MarkerKind::Heading).with_level(2));
        self.b.text("Footnotes");
        self.b.close(h);
        self.b.paragraph_break();
        for (label, text) in self.footnotes {
            self.b.line_break();
            let body = self.b.open(
                Self::marker(MarkerKind::Footnote)
                    .with_level(1)
                    .with_reference(label.as_str()),
            );
            let r = self
                .b
                .open(Self::marker(MarkerKind::Footnote).with_reference(label.as_str()));
            self.b.literal(&format!("[{label}]"));
            self.b.close(r);
            self.b.space();
            self.b.text(text);
            self.b.close(body);
        }
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
                footnotes_inline: true,
                ..LoadOptions::default()
            },
        );
        assert_eq!(
            inline.text().to_string(),
            "Text. (footnote: The note.) More."
        );
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
