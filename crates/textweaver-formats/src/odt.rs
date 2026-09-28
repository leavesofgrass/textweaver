//! OpenDocument text loader (ODF 1.4): `.odt` and `.ott` packages, and flat
//! `.fodt` XML, read natively on roxmltree and zip, without Pandoc.
//!
//! - **Headings**: `text:h` with its outline level (above 6 reads as 6),
//!   and paragraphs in the `Title` style as level 1.
//! - **Lists**: `text:list` nested to any depth, one `ListItem` per item
//!   at its depth; numbered levels get their label from the list style
//!   (`text:list-level-style-number`: format `1`, `a`, `A`, `i`, `I`,
//!   prefix, suffix, start value, and `text:display-levels`), bullets have
//!   none.
//! - **Tables**: one row per line, cells separated by
//!   [`CELL_SEPARATOR`](crate::CELL_SEPARATOR); rows under
//!   `table:table-header-rows` are header rows. Repeated rows and cells
//!   are capped, and empty repeats read once, so a hostile repeat count
//!   costs nothing. A table in a cell reads as running text.
//! - **Runs**: bold, italic, and underline from paragraph and text styles
//!   (automatic styles included, with `style:parent-style-name`
//!   inheritance); `Preformatted Text` paragraphs are code blocks and
//!   `Source Text` spans inline code; links (`text:a`); `text:s`, `text:tab`,
//!   and `text:line-break`.
//! - **Images**: `draw:frame` reads as its alt text (`svg:title`, else
//!   `svg:desc`) under an `Image` marker; frames without one are
//!   decorative. A frame's text box reads in place.
//! - **Footnotes and endnotes** (`text:note`) follow
//!   [`LoadOptions::footnotes`].
//! - **Comments** (`office:annotation`, ranged to its
//!   `office:annotation-end`, resolved and replies as LibreOffice writes
//!   them) become [`DocumentComment`]s.
//! - **Tracked changes** (`text:tracked-changes`, `text:change-start`,
//!   `text:change-end`, `text:change`) follow [`LoadOptions::revisions`].
//! - **Metadata** from `meta.xml`: title, creator, and language.

use std::collections::HashMap;

use ropey::Rope;
use roxmltree::Node;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, DocumentMeta, HEADER_ROW_LABEL, Marker};

use crate::annotations::{CommentReply, DocumentComment, clean_text};
use crate::builder::{Builder, OpenId};
use crate::package::{Package, child, parse_xml};
use crate::revision::{self, ChangeKind};
use crate::xmldepth::FLAT_ELEMENT;
use crate::{
    FootnoteMode, LoadError, LoadOptions, Loader, RevisionMode, Source, meta_for, title_from_path,
};

/// Most times a table row or cell with text is repeated
/// (`table:number-rows-repeated`, `table:number-columns-repeated`).
pub const MAX_REPEAT: usize = 32;

/// Loads OpenDocument text documents (`.odt`, templates `.ott`, and flat
/// XML `.fodt`).
#[derive(Clone, Copy, Debug, Default)]
pub struct OdtLoader;

impl Loader for OdtLoader {
    fn id(&self) -> &'static str {
        "odt"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["odt", "ott", "fodt"]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        let bytes = source.read()?;
        let mut meta = meta_for(source, self.id());
        let mut flattened = false;
        let (content, styles, meta_xml) = if bytes.starts_with(b"PK") {
            let mut pkg = Package::open(bytes, "OpenDocument")?;
            let content = pkg.read_text("content.xml")?.ok_or_else(|| {
                LoadError::Parse("not an OpenDocument text: no content.xml".into())
            })?;
            let styles = pkg.read_text("styles.xml")?;
            let meta_xml = pkg.read_text("meta.xml")?;
            flattened |= pkg.flattened();
            (content, styles, meta_xml)
        } else {
            // Flat XML: one document holds everything.
            let declared = crate::encoding::sniff_xml_encoding(&bytes);
            let text = crate::decode_bytes(&bytes, declared.as_deref()).text;
            let text = match crate::xmldepth::limit_depth(&text, crate::MAX_NESTING) {
                Some(flat) => {
                    flattened = true;
                    flat
                }
                None => text,
            };
            (text, None, None)
        };
        let xml = parse_xml(&content)?;
        let styles_xml = styles.as_deref().map(parse_xml).transpose()?;
        let meta_doc = meta_xml.as_deref().map(parse_xml).transpose()?;

        let mut st = Styles::default();
        if let Some(s) = &styles_xml {
            st.read(s.root());
        }
        st.read(xml.root());
        let office_meta = meta_doc
            .as_ref()
            .map_or(xml.root(), roxmltree::Document::root);
        read_meta(office_meta, &mut meta);

        let body = xml
            .descendants()
            .find(|n| {
                n.tag_name().name() == "text"
                    && n.parent().is_some_and(|p| p.tag_name().name() == "body")
            })
            .ok_or_else(|| LoadError::Parse("not an OpenDocument text: no office:text".into()))?;
        let changes = tracked_changes(body);
        let mut c = Conv {
            b: Builder::new(),
            options,
            styles: &st,
            changes: &changes,
            open_changes: HashMap::new(),
            lists: Vec::new(),
            counters: Vec::new(),
            in_cell: 0,
            fmt: [None; 4],
            want: Vec::new(),
            code: None,
            deferred: Vec::new(),
            note_count: 0,
            depth: 0,
            flattened: false,
            comments: Vec::new(),
            revisions: 0,
        };
        c.blocks(body);
        c.close_code();
        c.close_lists();
        let deferred = std::mem::take(&mut c.deferred);
        c.b.footnotes_section(&deferred);
        flattened |= c.flattened;
        let revisions = c.revisions;
        let mut comments = std::mem::take(&mut c.comments);
        let (text, markers, anchors) = c.b.finish_with_anchors();
        let ranges: HashMap<String, CharRange> = anchors.into_iter().collect();
        for cm in &mut comments {
            if let Some(r) = ranges.get(&cm.c.id) {
                cm.c.range = *r;
            }
        }
        let comments = attach_replies(comments);
        if flattened {
            crate::add_warning(&mut meta, crate::NESTING_WARNING);
        }
        crate::annotations::record(&mut meta, comments, revisions);
        if meta.title.is_none() {
            meta.title = markers
                .iter()
                .find(|m| m.kind == MarkerKind::Heading && m.level == 1)
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

/// The value of attribute `local` (any namespace).
fn attr<'a>(n: Node<'a, '_>, local: &str) -> Option<&'a str> {
    n.attributes()
        .find(|a| a.name() == local)
        .map(|a| a.value())
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// All the text under `n`, paragraphs separated by spaces.
fn plain_text(n: Node<'_, '_>) -> String {
    let mut out = String::new();
    for d in n.descendants() {
        if d.is_text() {
            out.push_str(d.text().unwrap_or(""));
        } else if matches!(d.tag_name().name(), "p" | "h" | "s" | "tab" | "line-break") {
            out.push(' ');
        }
    }
    collapse(&out)
}

/// Title, author, and language from `office:meta`.
fn read_meta(root: Node<'_, '_>, meta: &mut DocumentMeta) {
    let Some(m) = root.descendants().find(|n| {
        n.tag_name().name() == "meta"
            && n.is_element()
            && n.parent().is_some_and(|p| {
                p.tag_name().name() == "document-meta" || p.tag_name().name() == "document"
            })
    }) else {
        return;
    };
    let mut initial = None;
    for n in m.children().filter(Node::is_element) {
        let t = plain_text(n);
        if t.is_empty() {
            continue;
        }
        match n.tag_name().name() {
            "title" => meta.title = Some(t),
            "creator" => meta.author = Some(t),
            "initial-creator" => initial = Some(t),
            "language" => meta.language = Some(t),
            _ => {}
        }
    }
    if meta.author.is_none() {
        meta.author = initial;
    }
}

/// A paragraph or text style.
#[derive(Clone, Debug, Default)]
struct StyleInfo {
    /// Lowercased, with `_20_` read as a space.
    name: String,
    parent: Option<String>,
    /// Bold, italic, underline, inline code.
    fmt: [Option<bool>; 4],
    outline: Option<u8>,
}

/// One level of a list style.
#[derive(Clone, Debug, Default)]
struct ListLevel {
    /// `None` for bullets and images.
    format: Option<String>,
    prefix: String,
    suffix: String,
    start: u64,
    display: usize,
}

#[derive(Debug, Default)]
struct Styles {
    map: HashMap<String, StyleInfo>,
    lists: HashMap<String, HashMap<u8, ListLevel>>,
}

fn style_name(s: &str) -> String {
    s.replace("_20_", " ").to_lowercase()
}

impl Styles {
    fn read(&mut self, root: Node<'_, '_>) {
        for s in root.descendants().filter(Node::is_element) {
            match s.tag_name().name() {
                "style" => {
                    let Some(name) = attr(s, "name") else {
                        continue;
                    };
                    let mut info = StyleInfo {
                        name: style_name(attr(s, "display-name").unwrap_or(name)),
                        parent: attr(s, "parent-style-name").map(str::to_owned),
                        ..StyleInfo::default()
                    };
                    if let Some(tp) = child(s, "text-properties") {
                        info.fmt = text_fmt(tp);
                    }
                    if attr(s, "family") == Some("text")
                        && (info.name == "source text" || info.name.contains("code"))
                    {
                        info.fmt[3] = Some(true);
                    }
                    info.outline = attr(s, "default-outline-level")
                        .and_then(|v| v.parse::<u8>().ok())
                        .filter(|&l| l > 0);
                    self.map.insert(name.to_owned(), info);
                }
                "list-style" => {
                    let Some(name) = attr(s, "name") else {
                        continue;
                    };
                    let mut levels = HashMap::new();
                    for l in s.children().filter(Node::is_element) {
                        let Some(level) = attr(l, "level").and_then(|v| v.parse::<u8>().ok())
                        else {
                            continue;
                        };
                        let numbered = l.tag_name().name() == "list-level-style-number";
                        let format = attr(l, "num-format").filter(|f| numbered && !f.is_empty());
                        levels.insert(
                            level,
                            ListLevel {
                                format: format.map(str::to_owned),
                                prefix: attr(l, "num-prefix").unwrap_or("").to_owned(),
                                suffix: attr(l, "num-suffix").unwrap_or("").to_owned(),
                                start: attr(l, "start-value")
                                    .and_then(|v| v.trim().parse::<i128>().ok())
                                    .map_or(1, crate::counter::clamp),
                                display: attr(l, "display-levels")
                                    .and_then(|v| v.parse().ok())
                                    .unwrap_or(1)
                                    .clamp(1, 10),
                            },
                        );
                    }
                    self.lists.insert(name.to_owned(), levels);
                }
                _ => {}
            }
        }
    }

    /// The style and its ancestors (bounded, in case of a cycle).
    fn chain(&self, id: Option<&str>) -> Vec<&StyleInfo> {
        let mut out = Vec::new();
        let mut cur = id;
        while let Some(i) = cur {
            let Some(s) = self.map.get(i) else { break };
            out.push(s);
            if out.len() > 16 {
                break;
            }
            cur = s.parent.as_deref();
        }
        out
    }

    fn fmt(&self, id: Option<&str>) -> [Option<bool>; 4] {
        let mut out = [None; 4];
        for s in self.chain(id).into_iter().rev() {
            for (o, v) in out.iter_mut().zip(s.fmt) {
                if v.is_some() {
                    *o = v;
                }
            }
        }
        out
    }

    fn is_code(&self, id: Option<&str>) -> bool {
        self.chain(id)
            .iter()
            .any(|s| s.name.contains("preformatted") || s.name == "source code" || s.name == "code")
    }

    fn is_title(&self, id: Option<&str>) -> bool {
        self.chain(id).first().is_some_and(|s| s.name == "title")
    }
}

fn text_fmt(tp: Node<'_, '_>) -> [Option<bool>; 4] {
    let bold =
        attr(tp, "font-weight").map(|w| w == "bold" || w.parse::<u32>().is_ok_and(|n| n >= 600));
    let italic = attr(tp, "font-style").map(|s| matches!(s, "italic" | "oblique"));
    let underline = attr(tp, "text-underline-style").map(|s| s != "none");
    [bold, italic, underline, None]
}

/// A tracked change region.
#[derive(Clone, Debug)]
struct Change {
    kind: ChangeKind,
    author: String,
    date: String,
    /// The removed text, for deletions.
    text: String,
}

/// The document's change regions by id; format changes are left out.
fn tracked_changes(body: Node<'_, '_>) -> HashMap<String, Change> {
    let mut out = HashMap::new();
    let Some(tc) = child(body, "tracked-changes") else {
        return out;
    };
    for region in tc
        .children()
        .filter(|n| n.tag_name().name() == "changed-region")
    {
        let Some(id) = attr(region, "id") else {
            continue;
        };
        let Some(what) = region.children().find(Node::is_element) else {
            continue;
        };
        let kind = match what.tag_name().name() {
            "insertion" => ChangeKind::Inserted,
            "deletion" => ChangeKind::Deleted,
            _ => continue,
        };
        let info = child(what, "change-info");
        let field = |name: &str| {
            info.and_then(|i| i.children().find(|n| n.tag_name().name() == name))
                .map(plain_text)
                .unwrap_or_default()
        };
        let text = what
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() != "change-info")
            .map(plain_text)
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        out.insert(
            id.to_owned(),
            Change {
                kind,
                author: field("creator"),
                date: field("date"),
                text,
            },
        );
        if out.len() >= 100_000 {
            break;
        }
    }
    out
}

/// A comment as read, before replies are attached to their parents.
struct Comment {
    c: DocumentComment,
    parent: Option<String>,
}

/// Replies (LibreOffice's `loext:parent-name`) moved under their parent.
fn attach_replies(comments: Vec<Comment>) -> Vec<DocumentComment> {
    let mut top: Vec<DocumentComment> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut replies = Vec::new();
    for cm in comments {
        match cm.parent {
            Some(p) => replies.push((p, cm.c)),
            None => {
                index.insert(cm.c.id.clone(), top.len());
                top.push(cm.c);
            }
        }
    }
    for (p, r) in replies {
        match index.get(&p).and_then(|&i| top.get_mut(i)) {
            Some(parent) => parent.replies.push(CommentReply {
                author: r.author,
                date: r.date,
                text: r.text,
            }),
            None => top.push(r),
        }
    }
    top
}

struct Conv<'a> {
    b: Builder,
    options: &'a LoadOptions,
    styles: &'a Styles,
    changes: &'a HashMap<String, Change>,
    /// Insertions being said, by change id.
    open_changes: HashMap<String, revision::Open>,
    /// Open list markers, innermost last.
    lists: Vec<OpenId>,
    /// Item counters, one per list depth.
    counters: Vec<u64>,
    in_cell: usize,
    /// Open bold, italic, underline, and inline code markers.
    fmt: [Option<OpenId>; 4],
    /// Formatting wanted by the enclosing paragraph and spans.
    want: Vec<[Option<bool>; 4]>,
    code: Option<OpenId>,
    deferred: Vec<(String, String)>,
    note_count: usize,
    depth: usize,
    flattened: bool,
    comments: Vec<Comment>,
    revisions: usize,
}

const FMT_KINDS: [MarkerKind; 4] = [
    MarkerKind::Bold,
    MarkerKind::Italic,
    MarkerKind::Underline,
    MarkerKind::Code,
];

impl Conv<'_> {
    fn marker(kind: MarkerKind) -> Marker {
        Marker::new(kind, CharRange::empty(0))
    }

    fn enter(&mut self) -> bool {
        if self.depth >= crate::MAX_NESTING {
            return false;
        }
        self.depth += 1;
        true
    }

    /// Content nested past the limit: its text, without structure.
    fn flatten(&mut self, node: Node<'_, '_>) {
        self.flattened = true;
        self.set_fmt([false; 4]);
        self.b.space();
        self.b.text(&plain_text(node));
        self.b.space();
    }

    fn close_lists(&mut self) {
        while let Some(id) = self.lists.pop() {
            self.b.close(id);
        }
        self.counters.clear();
    }

    fn close_code(&mut self) {
        if let Some(id) = self.code.take() {
            self.b.close(id);
            self.b.paragraph_break();
        }
    }

    fn set_fmt(&mut self, want: [bool; 4]) {
        for (i, w) in want.into_iter().enumerate() {
            match (self.fmt[i], w) {
                (None, true) => self.fmt[i] = Some(self.b.open(Self::marker(FMT_KINDS[i]))),
                (Some(id), false) => {
                    self.b.close(id);
                    self.fmt[i] = None;
                }
                _ => {}
            }
        }
    }

    /// The formatting the current spans ask for.
    fn wanted(&self) -> [bool; 4] {
        let mut out = [false; 4];
        for w in &self.want {
            for (o, v) in out.iter_mut().zip(w) {
                if let Some(v) = v {
                    *o = *v;
                }
            }
        }
        out
    }

    fn text(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        let w = self.wanted();
        if s.chars().any(|c| !c.is_whitespace()) {
            self.set_fmt(w);
        }
        self.b.text(s);
    }

    fn blocks(&mut self, node: Node<'_, '_>) {
        if !self.enter() {
            self.flatten(node);
            return;
        }
        for c in node.children().filter(Node::is_element) {
            self.block(c);
        }
        self.depth -= 1;
    }

    fn block(&mut self, c: Node<'_, '_>) {
        match c.tag_name().name() {
            FLAT_ELEMENT => self.flatten(c),
            "p" => self.paragraph(c, None),
            "h" => {
                let level = attr(c, "outline-level")
                    .and_then(|v| v.parse::<u8>().ok())
                    .unwrap_or(1)
                    .clamp(1, 6);
                self.paragraph(c, Some(level));
            }
            "list" => {
                self.close_code();
                if self.in_cell > 0 {
                    self.blocks(c);
                } else {
                    let style = attr(c, "style-name").map(str::to_owned);
                    self.list(c, style.as_deref());
                }
            }
            "list-item" | "list-header" => self.blocks(c),
            "table" => self.table(c),
            "tracked-changes" | "sequence-decls" | "variable-decls" | "user-field-decls"
            | "forms" | "annotation" => {}
            "section" | "table-of-content" | "illustration-index" | "alphabetical-index"
            | "bibliography" | "user-index" | "object-index" | "table-index" | "index-body"
            | "index-title" | "text" | "deletion" | "insertion" | "change-start" | "change-end"
            | "change" | "soft-page-break" | "bookmark" => {
                if c.has_children() {
                    self.blocks(c);
                } else {
                    // Change points between paragraphs.
                    self.inline_element(c);
                }
            }
            _ => {}
        }
    }

    fn paragraph(&mut self, p: Node<'_, '_>, level: Option<u8>) {
        let style = attr(p, "style-name");
        if self.in_cell > 0 {
            self.close_code();
            self.b.space();
            self.inline_para(p, style);
            return;
        }
        if level.is_none() && self.styles.is_code(style) {
            self.code_paragraph(p);
            return;
        }
        self.close_code();
        let level = level.or_else(|| self.styles.is_title(style).then_some(1));
        let in_list = !self.lists.is_empty();
        if let Some(level) = level.filter(|_| !in_list) {
            self.b.paragraph_break();
            let id = self
                .b
                .open(Self::marker(MarkerKind::Heading).with_level(level));
            self.inline_para(p, style);
            self.b.close(id);
            self.b.paragraph_break();
        } else if in_list {
            // A further paragraph of a list item.
            self.b.line_break();
            self.inline_para(p, style);
        } else {
            self.b.paragraph_break();
            let id = self.b.open(Self::marker(MarkerKind::Paragraph));
            self.inline_para(p, style);
            self.b.close(id);
            self.b.paragraph_break();
        }
    }

    fn inline_para(&mut self, p: Node<'_, '_>, style: Option<&str>) {
        self.want.push(self.styles.fmt(style));
        self.inline(p);
        self.want.pop();
        self.set_fmt([false; 4]);
    }

    fn code_paragraph(&mut self, p: Node<'_, '_>) {
        if self.options.skip_code {
            return;
        }
        let mut text = String::new();
        for n in p.descendants() {
            if n.is_text() {
                if n.ancestors()
                    .any(|a| matches!(a.tag_name().name(), "note" | "annotation"))
                {
                    continue;
                }
                text.push_str(n.text().unwrap_or(""));
                continue;
            }
            match n.tag_name().name() {
                "s" => {
                    let count = attr(n, "c")
                        .and_then(|c| c.parse::<usize>().ok())
                        .unwrap_or(1);
                    text.push_str(&" ".repeat(count.clamp(1, 80)));
                }
                "tab" => text.push('\t'),
                "line-break" => text.push('\n'),
                _ => {}
            }
        }
        if self.code.is_some() {
            self.b.line_break();
        } else {
            self.b.paragraph_break();
            self.code = Some(self.b.open(Self::marker(MarkerKind::Code).with_level(1)));
        }
        self.b.verbatim(&text);
    }

    fn list(&mut self, node: Node<'_, '_>, style: Option<&str>) {
        if !self.enter() {
            self.flatten(node);
            return;
        }
        let depth = self.lists.len() + 1;
        let d = u8::try_from(depth.min(9)).unwrap_or(9);
        if self.lists.is_empty() {
            self.b.paragraph_break();
        }
        let id = self.b.open(Self::marker(MarkerKind::List).with_level(d));
        self.lists.push(id);
        let level = style
            .and_then(|s| self.styles.lists.get(s))
            .and_then(|l| l.get(&d))
            .cloned();
        self.counters.truncate(depth - 1);
        self.counters.push(0);
        for item in node.children().filter(Node::is_element) {
            match item.tag_name().name() {
                "list-item" => {
                    let start = attr(item, "start-value")
                        .and_then(|v| v.trim().parse::<i128>().ok())
                        .map(crate::counter::clamp);
                    let n = match (start, self.counters.last().copied().unwrap_or(0)) {
                        (Some(s), _) => s,
                        (None, 0) => level.as_ref().map_or(1, |l| l.start),
                        (None, c) => crate::counter::next(c),
                    };
                    if let Some(last) = self.counters.last_mut() {
                        *last = n;
                    }
                    let label = level.as_ref().and_then(|l| self.label(l));
                    self.item(item, d, label, style);
                }
                "list-header" => self.item(item, d, None, style),
                FLAT_ELEMENT => self.flatten(item),
                _ => {}
            }
        }
        if let Some(id) = self.lists.pop() {
            self.b.close(id);
        }
        self.counters.truncate(depth - 1);
        self.depth -= 1;
    }

    fn label(&self, level: &ListLevel) -> Option<String> {
        use crate::counter::{decimal, letters, roman};
        let format = level.format.as_deref()?;
        let fmt = |n: u64| match format {
            "a" => letters(n).to_lowercase(),
            "A" => letters(n),
            "i" => roman(n).to_lowercase(),
            "I" => roman(n),
            _ => decimal(n),
        };
        let from = self.counters.len().saturating_sub(level.display);
        let numbers: Vec<String> = self.counters[from..]
            .iter()
            .map(|&n| fmt(n.max(1)))
            .collect();
        Some(crate::counter::cap_label(format!(
            "{}{}{}",
            level.prefix,
            numbers.join("."),
            level.suffix
        )))
    }

    fn item(&mut self, item: Node<'_, '_>, depth: u8, label: Option<String>, style: Option<&str>) {
        let mut m = Self::marker(MarkerKind::ListItem).with_level(depth);
        if let Some(l) = label {
            m = m.with_label(l);
        }
        let mut open: Option<OpenId> = None;
        for c in item.children().filter(Node::is_element) {
            match c.tag_name().name() {
                "p" | "h" if open.is_none() => {
                    self.close_code();
                    self.b.line_break();
                    open = Some(self.b.open(m.clone()));
                    self.inline_para(c, attr(c, "style-name"));
                }
                "list" => {
                    if let Some(id) = open.take() {
                        self.b.close(id);
                    }
                    let inner = attr(c, "style-name").or(style).map(str::to_owned);
                    self.list(c, inner.as_deref());
                }
                _ => self.block(c),
            }
        }
        if let Some(id) = open {
            self.b.close(id);
        }
    }

    fn inline(&mut self, node: Node<'_, '_>) {
        if !self.enter() {
            self.flatten(node);
            return;
        }
        for c in node.children() {
            if c.is_text() {
                self.text(c.text().unwrap_or(""));
            } else if c.is_element() {
                self.inline_element(c);
            }
        }
        self.depth -= 1;
    }

    fn inline_element(&mut self, c: Node<'_, '_>) {
        match c.tag_name().name() {
            FLAT_ELEMENT => self.flatten(c),
            "span" => {
                self.want.push(self.styles.fmt(attr(c, "style-name")));
                self.inline(c);
                self.want.pop();
            }
            "a" => match attr(c, "href").filter(|h| !h.trim().is_empty()) {
                Some(href) => {
                    self.set_fmt([false; 4]);
                    let id = self
                        .b
                        .open(Self::marker(MarkerKind::Link).with_reference(href));
                    self.inline(c);
                    self.set_fmt([false; 4]);
                    self.b.close(id);
                }
                None => self.inline(c),
            },
            "s" | "tab" => self.b.space(),
            "line-break" => {
                if self.in_cell > 0 {
                    self.b.space();
                } else {
                    self.b.line_break();
                }
            }
            "note" => self.note(c),
            "frame" => self.frame(c),
            "annotation" => self.annotation(c),
            "annotation-end" => {
                if let Some(name) = attr(c, "name")
                    && let Some(id) = self.b.open_anchor_id(name)
                {
                    self.b.close(id);
                }
            }
            "change-start" => self.change_start(c),
            "change-end" => {
                if let Some(id) = attr(c, "change-id")
                    && let Some(open) = self.open_changes.remove(id)
                {
                    self.set_fmt([false; 4]);
                    revision::close(&mut self.b, open);
                }
            }
            "change" => self.change_point(c),
            "tracked-changes"
            | "note-citation"
            | "bookmark"
            | "bookmark-start"
            | "bookmark-end"
            | "reference-mark"
            | "reference-mark-start"
            | "reference-mark-end"
            | "soft-page-break"
            | "toc-mark"
            | "alphabetical-index-mark" => {}
            "p" | "h" => {
                // A paragraph inside inline content (a text box).
                self.b.space();
                self.inline_para(c, attr(c, "style-name"));
                self.b.space();
            }
            _ => self.inline(c),
        }
    }

    fn change_start(&mut self, c: Node<'_, '_>) {
        let Some(id) = attr(c, "change-id") else {
            return;
        };
        let Some(ch) = self.changes.get(id) else {
            return;
        };
        if ch.kind != ChangeKind::Inserted {
            return;
        }
        self.revisions += 1;
        if self.options.revisions == RevisionMode::Marked && !self.open_changes.contains_key(id) {
            self.set_fmt([false; 4]);
            let author = Some(ch.author.as_str()).filter(|a| !a.is_empty());
            let date = Some(ch.date.as_str()).filter(|d| !d.is_empty());
            let open = revision::open(&mut self.b, ch.kind, author, date);
            self.open_changes.insert(id.to_owned(), open);
        }
    }

    fn change_point(&mut self, c: Node<'_, '_>) {
        let Some(ch) = attr(c, "change-id").and_then(|id| self.changes.get(id)) else {
            return;
        };
        if ch.kind != ChangeKind::Deleted {
            return;
        }
        self.revisions += 1;
        if self.options.revisions != RevisionMode::Marked || ch.text.is_empty() {
            return;
        }
        self.set_fmt([false; 4]);
        let author = Some(ch.author.as_str()).filter(|a| !a.is_empty());
        let date = Some(ch.date.as_str()).filter(|d| !d.is_empty());
        let open = revision::open(&mut self.b, ch.kind, author, date);
        self.b.text(&ch.text);
        revision::close(&mut self.b, open);
    }

    fn annotation(&mut self, a: Node<'_, '_>) {
        if self.comments.len() >= crate::annotations::MAX_COMMENTS {
            return;
        }
        let field = |name: &str| {
            a.children()
                .find(|n| n.tag_name().name() == name)
                .map(plain_text)
                .unwrap_or_default()
        };
        let text = a
            .children()
            .filter(|n| matches!(n.tag_name().name(), "p" | "h" | "list"))
            .map(plain_text)
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let key = match attr(a, "name") {
            Some(n) => n.to_owned(),
            None => format!("odt-comment-{}", self.comments.len() + 1),
        };
        let resolved = attr(a, "resolved").is_some_and(|v| v == "true");
        let parent = attr(a, "parent-name").map(str::to_owned);
        let id = self.b.open_anchor(key.clone());
        if attr(a, "name").is_none() || parent.is_some() {
            // A comment at a point (or a reply, anchored with its parent).
            self.b.close(id);
        }
        self.comments.push(Comment {
            c: DocumentComment {
                id: key,
                range: CharRange::default(),
                author: field("creator"),
                date: field("date"),
                text: clean_text(&text),
                replies: Vec::new(),
                resolved,
            },
            parent,
        });
    }

    fn note(&mut self, n: Node<'_, '_>) {
        let text = child(n, "note-body").map(plain_text).unwrap_or_default();
        if text.is_empty() {
            return;
        }
        self.set_fmt([false; 4]);
        match self.options.footnotes {
            FootnoteMode::Skip => {}
            FootnoteMode::Inline => {
                self.note_count += 1;
                let label = self.note_count.to_string();
                self.b.inline_footnote(&label, &text);
            }
            FootnoteMode::Deferred => {
                self.note_count += 1;
                let label = self.note_count.to_string();
                self.b.footnote_reference(&label);
                self.deferred.push((label, text));
            }
        }
    }

    fn frame(&mut self, f: Node<'_, '_>) {
        let alt = ["title", "desc"]
            .into_iter()
            .filter_map(|name| f.children().find(|n| n.tag_name().name() == name))
            .map(plain_text)
            .find(|t| !t.is_empty());
        if let Some(tb) = child(f, "text-box") {
            self.b.space();
            self.inline(tb);
            self.b.space();
            return;
        }
        let Some(alt) = alt else {
            return;
        };
        self.set_fmt([false; 4]);
        self.b.space();
        let id = self.b.open(Self::marker(MarkerKind::Image));
        self.b.text(&alt);
        self.b.close(id);
        self.b.soft_space();
    }

    fn table(&mut self, tbl: Node<'_, '_>) {
        if !self.enter() {
            self.flatten(tbl);
            return;
        }
        let mut rows: Vec<(Node<'_, '_>, bool)> = Vec::new();
        collect_rows(tbl, false, &mut rows, 0);
        if self.in_cell > 0 {
            self.in_cell += 1;
            for (tr, _) in &rows {
                for tc in tr
                    .children()
                    .filter(|n| n.tag_name().name() == "table-cell")
                {
                    self.b.space();
                    self.blocks(tc);
                }
            }
            self.in_cell -= 1;
            self.depth -= 1;
            return;
        }
        self.close_code();
        self.close_lists();
        self.b.paragraph_break();
        let table = self.b.open(Self::marker(MarkerKind::Table));
        for (tr, header) in rows {
            for _ in 0..repeats(tr, "number-rows-repeated") {
                self.row(tr, header);
            }
        }
        self.b.close(table);
        self.b.paragraph_break();
        self.depth -= 1;
    }

    fn row(&mut self, tr: Node<'_, '_>, header: bool) {
        let mut cells: Vec<Node<'_, '_>> = Vec::new();
        for tc in tr
            .children()
            .filter(|n| n.tag_name().name() == "table-cell")
        {
            for _ in 0..repeats(tc, "number-columns-repeated") {
                if cells.len() < 1_000 {
                    cells.push(tc);
                }
            }
        }
        if cells.is_empty() {
            return;
        }
        self.b.line_break();
        let mut rm = Self::marker(MarkerKind::TableRow);
        if header {
            rm = rm.with_label(HEADER_ROW_LABEL);
        }
        let row = self.b.open(rm);
        for (j, tc) in cells.into_iter().enumerate() {
            if j > 0 {
                self.b.separator(crate::CELL_SEPARATOR);
            }
            let cell = self.b.open_here(Self::marker(MarkerKind::TableCell));
            self.in_cell += 1;
            self.blocks(tc);
            self.in_cell -= 1;
            self.set_fmt([false; 4]);
            self.b.close(cell);
        }
        self.b.close(row);
    }
}

/// How many times a row or cell is read: its repeat count capped at
/// [`MAX_REPEAT`] when it has text; an empty one read once, or not at all
/// when it is repeated (the padding a word processor adds to fill a page).
fn repeats(n: Node<'_, '_>, attribute: &str) -> usize {
    let count = attr(n, attribute).and_then(|v| v.trim().parse::<usize>().ok());
    if !plain_text(n).is_empty() {
        return count.unwrap_or(1).clamp(1, MAX_REPEAT);
    }
    match count {
        Some(c) if c > 1 => 0,
        _ => 1,
    }
}

/// The rows of a table, in order, each with whether it is a header row
/// (row groups followed at most a few levels deep).
fn collect_rows<'a, 'i>(
    node: Node<'a, 'i>,
    header: bool,
    out: &mut Vec<(Node<'a, 'i>, bool)>,
    level: usize,
) {
    for c in node.children().filter(Node::is_element) {
        match c.tag_name().name() {
            "table-row" => out.push((c, header)),
            "table-header-rows" if level < 8 => collect_rows(c, true, out, level + 1),
            "table-rows" | "table-row-group" if level < 8 => {
                collect_rows(c, header, out, level + 1);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    const NS: &str = r#"xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:meta="urn:oasis:names:tc:opendocument:xmlns:meta:1.0" xmlns:loext="urn:org:documentfoundation:names:experimental:office:xmlns:loext:1.0""#;

    fn package(content: &str, meta: Option<&str>) -> Vec<u8> {
        let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default();
        z.start_file("mimetype", opts).expect("zip entry");
        z.write_all(b"application/vnd.oasis.opendocument.text")
            .expect("zip write");
        z.start_file("content.xml", opts).expect("zip entry");
        z.write_all(content.as_bytes()).expect("zip write");
        if let Some(m) = meta {
            z.start_file("meta.xml", opts).expect("zip entry");
            z.write_all(m.as_bytes()).expect("zip write");
        }
        z.finish().expect("zip finish").into_inner()
    }

    fn content(styles: &str, body: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><office:document-content {NS}><office:automatic-styles>{styles}</office:automatic-styles><office:body><office:text>{body}</office:text></office:body></office:document-content>"#
        )
    }

    fn load_bytes(data: Vec<u8>, options: &LoadOptions) -> Document {
        OdtLoader
            .load(
                &Source::Bytes {
                    data,
                    hint: "odt".into(),
                },
                options,
            )
            .expect("the ODT loads")
    }

    fn load(styles: &str, body: &str) -> Document {
        load_bytes(
            package(&content(styles, body), None),
            &LoadOptions::default(),
        )
    }

    fn texts(doc: &Document, kind: MarkerKind) -> Vec<String> {
        doc.markers()
            .iter()
            .filter(|m| m.kind == kind)
            .map(|m| doc.slice(m.range).to_string())
            .collect()
    }

    #[test]
    fn headings_paragraphs_formatting_and_links() {
        let styles = r#"<style:style style:name="T1" style:family="text"><style:text-properties fo:font-weight="bold"/></style:style><style:style style:name="T2" style:family="text"><style:text-properties fo:font-style="italic"/></style:style>"#;
        let body = r#"<text:h text:outline-level="1">Week one</text:h><text:p>Some <text:span text:style-name="T1">bold</text:span> and <text:span text:style-name="T2">italic</text:span>,<text:s/>a <text:a xlink:href="https://example.org/">link</text:a>.</text:p><text:h text:outline-level="8">Deep</text:h>"#;
        let meta = format!(
            r#"<office:document-meta {NS}><office:meta><dc:title>Notes</dc:title><meta:initial-creator>Ada Example</meta:initial-creator><dc:language>en-US</dc:language></office:meta></office:document-meta>"#
        );
        let doc = load_bytes(
            package(&content(styles, body), Some(&meta)),
            &LoadOptions::default(),
        );
        assert_eq!(
            doc.text().to_string(),
            "Week one\n\nSome bold and italic, a link.\n\nDeep"
        );
        let levels: Vec<u8> = doc
            .markers()
            .iter()
            .filter(|m| m.kind == MarkerKind::Heading)
            .map(|m| m.level)
            .collect();
        assert_eq!(levels, vec![1, 6]);
        assert_eq!(texts(&doc, MarkerKind::Bold), vec!["bold"]);
        assert_eq!(texts(&doc, MarkerKind::Italic), vec!["italic"]);
        assert_eq!(texts(&doc, MarkerKind::Link), vec!["link"]);
        assert_eq!(doc.meta.title.as_deref(), Some("Notes"));
        assert_eq!(doc.meta.author.as_deref(), Some("Ada Example"));
        assert_eq!(doc.meta.language.as_deref(), Some("en-US"));
    }

    #[test]
    fn nested_numbered_and_bulleted_lists() {
        let styles = r#"<text:list-style style:name="L1"><text:list-level-style-number text:level="1" style:num-format="1" style:num-suffix="."/><text:list-level-style-number text:level="2" style:num-format="a" style:num-suffix=")" text:display-levels="1"/></text:list-style><text:list-style style:name="L2"><text:list-level-style-bullet text:level="1" text:bullet-char="•"/></text:list-style>"#;
        let body = r#"<text:list text:style-name="L1"><text:list-item><text:p>First</text:p><text:list><text:list-item><text:p>Inner</text:p></text:list-item><text:list-item><text:p>Second inner</text:p></text:list-item></text:list></text:list-item><text:list-item><text:p>Second</text:p></text:list-item></text:list><text:list text:style-name="L2"><text:list-item><text:p>Dot</text:p></text:list-item></text:list><text:p>After</text:p>"#;
        let doc = load(styles, body);
        assert_eq!(
            doc.text().to_string(),
            "First\nInner\nSecond inner\nSecond\n\nDot\n\nAfter"
        );
        let items: Vec<(String, u8, Option<String>)> = doc
            .markers()
            .iter()
            .filter(|m| m.kind == MarkerKind::ListItem)
            .map(|m| (doc.slice(m.range).to_string(), m.level, m.label.clone()))
            .collect();
        assert_eq!(
            items,
            vec![
                ("First".into(), 1, Some("1.".into())),
                ("Inner".into(), 2, Some("a)".into())),
                ("Second inner".into(), 2, Some("b)".into())),
                ("Second".into(), 1, Some("2.".into())),
                ("Dot".into(), 1, None),
            ]
        );
    }

    #[test]
    fn tables_with_header_rows_and_hostile_repeats() {
        let body = r#"<table:table><table:table-column table:number-columns-repeated="2"/><table:table-header-rows><table:table-row><table:table-cell><text:p>Name</text:p></table:table-cell><table:table-cell><text:p>Mark</text:p></table:table-cell></table:table-row></table:table-header-rows><table:table-row><table:table-cell><text:p>Ada</text:p></table:table-cell><table:table-cell><text:p>A</text:p><text:p>plus</text:p></table:table-cell><table:table-cell table:number-columns-repeated="1000000"/></table:table-row><table:table-row table:number-rows-repeated="1000000"><table:table-cell/></table:table-row></table:table>"#;
        let doc = load("", body);
        // Repeated empty rows and cells are padding, and read as nothing.
        assert_eq!(doc.text().to_string(), "Name | Mark\nAda | A plus");
        let rows: Vec<Option<String>> = doc
            .markers()
            .iter()
            .filter(|m| m.kind == MarkerKind::TableRow)
            .map(|m| m.label.clone())
            .collect();
        assert_eq!(rows, vec![Some(HEADER_ROW_LABEL.to_owned()), None]);
    }

    #[test]
    fn footnotes_images_and_code() {
        let styles = r#"<style:style style:name="Preformatted_20_Text" style:display-name="Preformatted Text" style:family="paragraph"/><style:style style:name="P1" style:family="paragraph" style:parent-style-name="Preformatted_20_Text"/>"#;
        let body = r#"<text:p>See<text:note text:note-class="footnote"><text:note-citation>1</text:note-citation><text:note-body><text:p>The source.</text:p></text:note-body></text:note> this <draw:frame><draw:image/><svg:title>A crow on a wire</svg:title></draw:frame><draw:frame><draw:image/></draw:frame>.</text:p><text:p text:style-name="P1">let x<text:s text:c="2"/>= 1;</text:p><text:p text:style-name="P1">x</text:p>"#;
        let doc = load(styles, body);
        assert_eq!(
            doc.text().to_string(),
            "See[1] this A crow on a wire.\n\nlet x  = 1;\nx\n\nFootnotes\n\n[1] The source."
        );
        assert_eq!(texts(&doc, MarkerKind::Image), vec!["A crow on a wire"]);
        assert_eq!(texts(&doc, MarkerKind::Code), vec!["let x  = 1;\nx"]);
    }

    #[test]
    fn comments_with_ranges_replies_and_resolved_state() {
        let body = r#"<text:p>Read <office:annotation office:name="c1" loext:resolved="true"><dc:creator>Ada Example</dc:creator><dc:date>2026-09-01T10:00:00</dc:date><text:p>Check this date.</text:p></office:annotation>chapter two<office:annotation-end office:name="c1"/> first.<office:annotation><dc:creator>Bo Example</dc:creator><text:p>A point.</text:p></office:annotation></text:p><text:p><office:annotation office:name="c2" loext:parent-name="c1"><dc:creator>Bo Example</dc:creator><text:p>Done.</text:p></office:annotation>Next.</text:p>"#;
        let doc = load("", body);
        assert_eq!(doc.text().to_string(), "Read chapter two first.\n\nNext.");
        let cs = crate::comments(&doc.meta);
        assert_eq!(cs.len(), 2);
        assert_eq!(doc.slice(cs[0].range).to_string(), "chapter two");
        assert_eq!(cs[0].author, "Ada Example");
        assert!(cs[0].resolved);
        assert_eq!(cs[0].replies.len(), 1);
        assert_eq!(cs[0].replies[0].text, "Done.");
        assert_eq!(
            cs[0].spoken(),
            "Comment by Ada Example: Check this date. Reply by Bo Example: Done. Resolved."
        );
        assert!(cs[1].range.is_empty());
        assert_eq!(cs[1].text, "A point.");
    }

    #[test]
    fn tracked_changes_follow_the_option() {
        let body = r#"<text:tracked-changes><text:changed-region text:id="ct1"><text:insertion><office:change-info><dc:creator>Ada Example</dc:creator><dc:date>2026-09-01T10:00:00</dc:date></office:change-info></text:insertion></text:changed-region><text:changed-region text:id="ct2"><text:deletion><office:change-info><dc:creator>Bo Example</dc:creator><dc:date>2026-09-02T10:00:00</dc:date></office:change-info><text:p>old</text:p></text:deletion></text:changed-region></text:tracked-changes><text:p>The <text:change text:change-id="ct2"/><text:change-start text:change-id="ct1"/>new<text:change-end text:change-id="ct1"/> plan.</text:p>"#;
        let doc = load("", body);
        assert_eq!(doc.text().to_string(), "The new plan.");
        assert_eq!(crate::revision_count(&doc.meta), 2);
        let marked = load_bytes(
            package(&content("", body), None),
            &LoadOptions {
                revisions: RevisionMode::Marked,
                ..LoadOptions::default()
            },
        );
        assert_eq!(
            marked.text().to_string(),
            "The (deleted by Bo Example: old) (inserted by Ada Example: new) plan."
        );
        assert_eq!(texts(&marked, MarkerKind::Strikethrough), vec!["old"]);
        assert_eq!(texts(&marked, MarkerKind::Underline), vec!["new"]);
    }

    #[test]
    fn flat_xml_and_hostile_nesting() {
        let fodt = format!(
            r#"<?xml version="1.0"?><office:document {NS}><office:meta><dc:title>Flat</dc:title></office:meta><office:body><office:text><text:h text:outline-level="2">Flat heading</text:h><text:p>Body.</text:p></office:text></office:body></office:document>"#
        );
        let doc = OdtLoader
            .load(
                &Source::Bytes {
                    data: fodt.into_bytes(),
                    hint: "fodt".into(),
                },
                &LoadOptions::default(),
            )
            .expect("flat XML loads");
        assert_eq!(doc.text().to_string(), "Flat heading\n\nBody.");
        assert_eq!(doc.meta.title.as_deref(), Some("Flat"));
        let deep = format!(
            "<text:p>{}deep{}</text:p>",
            "<text:span>".repeat(20_000),
            "</text:span>".repeat(20_000)
        );
        let doc = load("", &deep);
        assert_eq!(doc.text().to_string(), "deep");
        assert!(crate::warnings(&doc.meta).contains(&crate::NESTING_WARNING.to_owned()));
        let not = OdtLoader.load(
            &Source::Bytes {
                data: package("<x/>", None),
                hint: "odt".into(),
            },
            &LoadOptions::default(),
        );
        assert!(matches!(not, Err(LoadError::Parse(_))));
    }
}
