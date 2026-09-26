//! The block tree every writer renders: a [`Document`]'s canonical text and
//! markers turned into nested blocks (headings, paragraphs, lists, tables,
//! code, quotes, figures, footnotes, breaks) with inline spans (bold,
//! italic, underline, code, links, images, footnote references).
//!
//! Building the tree once keeps the writers small and consistent: they all
//! agree on what is a heading, where a list item ends, and which row of a
//! table is its header.
//!
//! Rules (ADR-0017):
//!
//! - Block markers (`Heading`, `Paragraph`, `List`, `ListItem`, `Table`,
//!   `Code` level 1, `Quote`, `Footnote` level 1) become blocks; text outside
//!   every block marker is split into paragraphs at blank lines, so a plain
//!   text document (no markers) is a list of paragraphs whose single line
//!   breaks are kept as [`Inline::LineBreak`].
//! - `SectionBreak`, `PageBreak`, and `Rule` are points at their start,
//!   emitted once between blocks: before a block that starts where they
//!   start, after a leaf block they fall inside; a rule at the very end is
//!   the last block.
//! - `Math` markers are [`Style::Math`] spans whose text is the source with
//!   its delimiters; each writer typesets it (see `math`).
//! - Inline markers that overlap without nesting are split so every writer
//!   can emit well-nested markup; a link inside a link keeps only the outer
//!   one.
//! - A paragraph whose only content is one image is a [`Block::Figure`].

use textweaver_core::{CharPos, CharRange, MarkerKind};
use textweaver_text::{Document, Marker};

/// Separator between table cells on a row of canonical text (ADR-0002).
pub const CELL_SEPARATOR: &str = " | ";

/// A block of the document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    /// A heading, level 1 to 6.
    Heading {
        /// 1 to 6.
        level: u8,
        /// The heading text.
        content: Vec<Inline>,
    },
    /// A paragraph.
    Paragraph(Vec<Inline>),
    /// A bulleted or numbered list.
    List(List),
    /// A table.
    Table(Table),
    /// A code block, verbatim.
    Code {
        /// The language, when declared.
        language: Option<String>,
        /// The code, lines separated by `\n`.
        text: String,
    },
    /// A block quote.
    Quote(Vec<Block>),
    /// An image standing alone as a paragraph.
    Figure(Image),
    /// A footnote body.
    Footnote {
        /// The footnote id (the reference's label).
        id: String,
        /// The footnote text.
        content: Vec<Inline>,
    },
    /// A chapter or section boundary.
    SectionBreak {
        /// The chapter title, when the source gave one.
        title: Option<String>,
    },
    /// A print page boundary.
    PageBreak {
        /// The print page number or label, when the source gave one.
        label: Option<String>,
    },
    /// A horizontal rule (thematic break) between blocks.
    Rule,
}

/// A list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct List {
    /// True for a numbered list.
    pub ordered: bool,
    /// The number of the first item (numbered lists).
    pub start: u64,
    /// The items.
    pub items: Vec<ListItem>,
}

/// One list item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListItem {
    /// The item's label as the source numbered it (`"3."`), if any.
    pub label: Option<String>,
    /// The item's content: usually a paragraph, then nested lists.
    pub blocks: Vec<Block>,
}

/// A table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    /// The caption, when the source gave one.
    pub caption: Option<String>,
    /// The rows, header rows first as in the source.
    pub rows: Vec<TableRow>,
}

impl Table {
    /// Number of columns: the widest row.
    pub fn columns(&self) -> usize {
        self.rows.iter().map(|r| r.cells.len()).max().unwrap_or(0)
    }

    /// True when the first row is a header row.
    pub fn has_header(&self) -> bool {
        self.rows.first().is_some_and(|r| r.header)
    }
}

/// One table row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableRow {
    /// True for a header row.
    pub header: bool,
    /// The cells.
    pub cells: Vec<Vec<Inline>>,
}

/// An image: alt text and source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    /// The alternative text (empty for a decorative image).
    pub alt: String,
    /// Where the image came from (a path or URL), when known.
    pub src: Option<String>,
}

/// Inline content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Inline {
    /// Text without line breaks.
    Text(String),
    /// A line break inside a block.
    LineBreak,
    /// Styled or linked content.
    Span(Style, Vec<Inline>),
}

/// What an [`Inline::Span`] means.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Style {
    /// Bold.
    Bold,
    /// Italic.
    Italic,
    /// Underlined.
    Underline,
    /// Inline code.
    Code,
    /// A link to a target.
    Link(String),
    /// A reference to a footnote by id.
    FootnoteRef(String),
    /// An inline image; the children are its alt text.
    Image(Option<String>),
    /// Struck-through text.
    Strikethrough,
    /// Math; the children are the LaTeX source with its delimiters.
    Math {
        /// Display (block) math rather than inline.
        display: bool,
    },
}

impl Inline {
    /// The plain text of inline content (line breaks as spaces).
    pub fn plain(inlines: &[Inline]) -> String {
        let mut s = String::new();
        plain_into(inlines, &mut s);
        s
    }
}

fn plain_into(inlines: &[Inline], out: &mut String) {
    for i in inlines {
        match i {
            Inline::Text(t) => out.push_str(t),
            Inline::LineBreak => out.push(' '),
            Inline::Span(_, children) => plain_into(children, out),
        }
    }
}

/// Document facts the writers need, with the fallbacks applied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Facts {
    /// Document title (never empty).
    pub title: String,
    /// BCP 47 language tag (never empty).
    pub language: String,
    /// Author, when known.
    pub author: Option<String>,
}

impl Facts {
    /// Title: the override, the document's title, its first heading, its
    /// file stem, or "Untitled document". Language: the override, the
    /// document's language, or `en`.
    pub fn of(doc: &Document, options: &crate::WriteOptions, blocks: &[Block]) -> Facts {
        let title = options
            .title
            .clone()
            .or_else(|| doc.meta.title.clone())
            .filter(|t| !t.trim().is_empty())
            .or_else(|| first_heading(blocks))
            .or_else(|| {
                doc.meta
                    .path
                    .as_ref()
                    .and_then(|p| p.file_stem())
                    .map(|s| s.to_string_lossy().into_owned())
            })
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| "Untitled document".to_owned());
        let language = options
            .language
            .clone()
            .or_else(|| doc.meta.language.clone())
            .map(|l| l.trim().to_owned())
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| "en".to_owned());
        let author = options
            .author
            .clone()
            .or_else(|| doc.meta.author.clone())
            .filter(|a| !a.trim().is_empty());
        Facts {
            title: collapse_ws(&title),
            language,
            author,
        }
    }
}

fn first_heading(blocks: &[Block]) -> Option<String> {
    blocks.iter().find_map(|b| match b {
        Block::Heading { content, .. } => {
            let t = collapse_ws(&Inline::plain(content));
            (!t.is_empty()).then_some(t)
        }
        _ => None,
    })
}

/// Whitespace runs collapsed to single spaces, trimmed.
pub fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Builds the block tree of a document.
pub fn blocks(doc: &Document) -> Vec<Block> {
    let mut b = TreeBuilder {
        doc,
        markers: doc.markers(),
        emitted: vec![false; doc.markers().len()],
    };
    let mut out = b.blocks_in(doc.full_range(), None);
    // Breaks at the very end (a closing rule) start at no char of the text.
    let end = CharPos(doc.len_chars());
    for i in 0..b.markers.len() {
        if is_break(&b.markers[i]) && b.markers[i].range.start >= end {
            b.emit_break(i, &mut out);
        }
    }
    out
}

struct TreeBuilder<'a> {
    doc: &'a Document,
    markers: &'a [Marker],
    /// Break markers already emitted, by index.
    emitted: Vec<bool>,
}

fn is_break(m: &Marker) -> bool {
    matches!(
        m.kind,
        MarkerKind::SectionBreak | MarkerKind::PageBreak | MarkerKind::Rule
    )
}

fn is_block_candidate(m: &Marker) -> bool {
    match m.kind {
        MarkerKind::Heading
        | MarkerKind::Paragraph
        | MarkerKind::List
        | MarkerKind::ListItem
        | MarkerKind::Table
        | MarkerKind::TableRow
        | MarkerKind::Quote => true,
        MarkerKind::Code | MarkerKind::Footnote => m.level == 1,
        _ => false,
    }
}

fn is_inline(m: &Marker) -> bool {
    match m.kind {
        MarkerKind::Bold
        | MarkerKind::Italic
        | MarkerKind::Underline
        | MarkerKind::Strikethrough
        | MarkerKind::Math
        | MarkerKind::Link
        | MarkerKind::Image => true,
        MarkerKind::Code | MarkerKind::Footnote => m.level == 0,
        _ => false,
    }
}

impl TreeBuilder<'_> {
    /// Indices of the markers starting in `range`.
    fn starting_in(&self, range: CharRange) -> std::ops::Range<usize> {
        let lo = self
            .markers
            .partition_point(|m| m.range.start < range.start);
        let hi = self.markers.partition_point(|m| m.range.start < range.end);
        lo..hi.max(lo)
    }

    fn emit_break(&mut self, i: usize, out: &mut Vec<Block>) {
        if self.emitted[i] {
            return;
        }
        self.emitted[i] = true;
        let m = &self.markers[i];
        out.push(match m.kind {
            MarkerKind::SectionBreak => Block::SectionBreak {
                title: m
                    .label
                    .clone()
                    .or_else(|| {
                        (!m.range.is_empty()).then(|| collapse_ws(&self.doc.slice(m.range)))
                    })
                    .filter(|t| !t.is_empty() && t.chars().count() <= 200),
            },
            MarkerKind::Rule => Block::Rule,
            _ => Block::PageBreak {
                label: m.label.clone(),
            },
        });
    }

    /// Emits the breaks starting exactly at `at` (before a block).
    fn breaks_at(&mut self, at: CharPos, out: &mut Vec<Block>) {
        for i in self.starting_in(CharRange::new(at, at.saturating_add(1))) {
            if is_break(&self.markers[i]) && self.markers[i].range.start == at {
                self.emit_break(i, out);
            }
        }
    }

    /// Emits the breaks starting inside `range` (after a leaf block).
    fn breaks_inside(&mut self, range: CharRange, out: &mut Vec<Block>) {
        for i in self.starting_in(range) {
            if is_break(&self.markers[i]) {
                self.emit_break(i, out);
            }
        }
    }

    /// The blocks in `range`, skipping the container marker `this` itself.
    fn blocks_in(&mut self, range: CharRange, this: Option<usize>) -> Vec<Block> {
        let mut out = Vec::new();
        let mut cursor = range.start;
        let idx = self.starting_in(range);
        let mut i = idx.start;
        while i < idx.end {
            let m = &self.markers[i];
            if Some(i) == this || (!range.contains_range(m.range) && !is_break(m)) {
                i += 1;
                continue;
            }
            if is_break(m) {
                if m.range.start >= cursor {
                    self.gap(CharRange::new(cursor, m.range.start), &mut out);
                    cursor = m.range.start;
                }
                self.emit_break(i, &mut out);
                i += 1;
                continue;
            }
            if !is_block_candidate(m) || m.range.start < cursor || m.range.is_empty() {
                i += 1;
                continue;
            }
            self.gap(CharRange::new(cursor, m.range.start), &mut out);
            self.breaks_at(m.range.start, &mut out);
            let end = m.range.end;
            match m.kind {
                MarkerKind::ListItem => {
                    // Items outside a list: gather consecutive ones.
                    let (list, next) = self.orphan_items(i, range);
                    out.push(Block::List(list));
                    cursor = self.markers[next - 1].range.end.max(end);
                    i = next;
                    continue;
                }
                MarkerKind::TableRow => {
                    let (table, next) = self.orphan_rows(i, range);
                    out.push(Block::Table(table));
                    cursor = self.markers[next - 1].range.end.max(end);
                    i = next;
                    continue;
                }
                _ => {
                    let block = self.block(i);
                    let leaf = !matches!(m.kind, MarkerKind::List | MarkerKind::Quote);
                    out.push(block);
                    if leaf {
                        self.breaks_inside(m.range, &mut out);
                    }
                }
            }
            cursor = end;
            i += 1;
        }
        self.gap(CharRange::new(cursor.min(range.end), range.end), &mut out);
        out
    }

    /// Text outside any block marker: paragraphs split at blank lines.
    fn gap(&mut self, range: CharRange, out: &mut Vec<Block>) {
        if range.is_empty() {
            return;
        }
        let text = self.doc.slice(range);
        if text.trim().is_empty() {
            return;
        }
        // Paragraphs are runs of non-blank lines; each spans from its first
        // to its last non-whitespace char.
        let mut paras: Vec<CharRange> = Vec::new();
        let mut current: Option<(usize, usize)> = None;
        let mut pos = range.start.0;
        for line in text.split('\n') {
            let len = line.chars().count();
            let lead = line.chars().take_while(|c| c.is_whitespace()).count();
            if lead == len {
                // A blank (or whitespace-only) line ends the paragraph.
                if let Some((s, e)) = current.take() {
                    paras.push(CharRange::new(s, e));
                }
            } else {
                let trail = line.chars().rev().take_while(|c| c.is_whitespace()).count();
                let (s, e) = (pos + lead, pos + len - trail);
                current = Some(match current {
                    Some((s0, _)) => (s0, e),
                    None => (s, e),
                });
            }
            pos += len + 1;
        }
        if let Some((s, e)) = current {
            paras.push(CharRange::new(s, e));
        }
        for r in paras {
            self.breaks_at(r.start, out);
            let p = self.paragraph(r);
            out.push(p);
            self.breaks_inside(r, out);
        }
    }

    fn paragraph(&mut self, range: CharRange) -> Block {
        let content = self.inlines(range);
        if let Some(img) = sole_image(&content) {
            return Block::Figure(img);
        }
        Block::Paragraph(content)
    }

    fn block(&mut self, i: usize) -> Block {
        let m = &self.markers[i];
        let range = m.range;
        match m.kind {
            MarkerKind::Heading => Block::Heading {
                level: m.level.clamp(1, 6),
                content: trim_inlines(self.inlines(range)),
            },
            MarkerKind::Paragraph => self.paragraph(range),
            MarkerKind::Code => Block::Code {
                language: m.label.clone().filter(|l| !l.is_empty()),
                text: self.doc.slice(range),
            },
            MarkerKind::Quote => Block::Quote(self.blocks_in(range, Some(i))),
            MarkerKind::Footnote => Block::Footnote {
                id: m.reference.clone().unwrap_or_default(),
                content: trim_inlines(self.inlines(range)),
            },
            MarkerKind::List => Block::List(self.list(i)),
            MarkerKind::Table => Block::Table(self.table(i)),
            // Handled by the callers.
            _ => Block::Paragraph(self.inlines(range)),
        }
    }

    fn list(&mut self, i: usize) -> List {
        let range = self.markers[i].range;
        let mut items = Vec::new();
        let mut cursor = range.start;
        let mut loose: Vec<Block> = Vec::new();
        for j in self.starting_in(range) {
            let m = &self.markers[j];
            if j == i
                || m.kind != MarkerKind::ListItem
                || m.range.start < cursor
                || !range.contains_range(m.range)
            {
                continue;
            }
            // Text between items that no item holds (rare): its own item.
            self.gap(CharRange::new(cursor, m.range.start), &mut loose);
            if !loose.is_empty() {
                items.push(ListItem {
                    label: None,
                    blocks: std::mem::take(&mut loose),
                });
            }
            let label = m.label.clone();
            let item_range = m.range;
            items.push(ListItem {
                label,
                blocks: self.blocks_in(item_range, Some(j)),
            });
            cursor = item_range.end;
        }
        self.gap(CharRange::new(cursor, range.end), &mut loose);
        if !loose.is_empty() {
            items.push(ListItem {
                label: None,
                blocks: loose,
            });
        }
        list_from_items(items)
    }

    fn orphan_items(&mut self, first: usize, within: CharRange) -> (List, usize) {
        let mut items = Vec::new();
        let mut j = first;
        let mut cursor = self.markers[first].range.start;
        while j < self.markers.len() {
            let m = &self.markers[j];
            if m.range.start >= within.end {
                break;
            }
            if m.range.start < cursor {
                j += 1;
                continue;
            }
            if m.kind != MarkerKind::ListItem || !within.contains_range(m.range) {
                break;
            }
            // Only whitespace may separate the items.
            if !self
                .doc
                .slice(CharRange::new(cursor, m.range.start))
                .trim()
                .is_empty()
            {
                break;
            }
            let r = m.range;
            let label = m.label.clone();
            items.push(ListItem {
                label,
                blocks: self.blocks_in(r, Some(j)),
            });
            cursor = r.end;
            j += 1;
        }
        (list_from_items(items), j.max(first + 1))
    }

    fn orphan_rows(&mut self, first: usize, within: CharRange) -> (Table, usize) {
        let mut rows = Vec::new();
        let mut j = first;
        let mut cursor = self.markers[first].range.start;
        while j < self.markers.len() {
            let m = &self.markers[j];
            if m.range.start >= within.end {
                break;
            }
            if m.range.start < cursor {
                j += 1;
                continue;
            }
            if m.kind != MarkerKind::TableRow || !within.contains_range(m.range) {
                break;
            }
            if !self
                .doc
                .slice(CharRange::new(cursor, m.range.start))
                .trim()
                .is_empty()
            {
                break;
            }
            rows.push(self.row(j));
            cursor = m.range.end;
            j += 1;
        }
        (
            Table {
                caption: None,
                rows,
            },
            j.max(first + 1),
        )
    }

    fn table(&mut self, i: usize) -> Table {
        let m = &self.markers[i];
        let range = m.range;
        let caption = m.label.clone().filter(|c| !c.trim().is_empty());
        let mut rows = Vec::new();
        let mut cursor = range.start;
        for j in self.starting_in(range) {
            let r = &self.markers[j];
            if r.kind == MarkerKind::TableRow
                && r.range.start >= cursor
                && range.contains_range(r.range)
            {
                cursor = r.range.end;
                rows.push(self.row(j));
            }
        }
        if rows.is_empty() {
            // No row markers: one row per line, cells split at the separator.
            let mut line_start = range.start.0;
            let text = self.doc.slice(range);
            for (pos, c) in (range.start.0..).zip(text.chars().chain(std::iter::once('\n'))) {
                if c == '\n' {
                    let lr = CharRange::new(line_start, pos.min(range.end.0));
                    if !self.doc.slice(lr).trim().is_empty() {
                        rows.push(TableRow {
                            header: false,
                            cells: self.split_cells(lr),
                        });
                    }
                    line_start = pos + 1;
                }
            }
        }
        Table { caption, rows }
    }

    fn row(&mut self, j: usize) -> TableRow {
        let row = &self.markers[j];
        let header = row.is_header_row();
        let range = row.range;
        let cells_idx: Vec<CharRange> = self
            .starting_in(CharRange::new(range.start, range.end.saturating_add(1)))
            .filter(|&k| {
                let c = &self.markers[k];
                c.kind == MarkerKind::TableCell
                    && c.range.start >= range.start
                    && c.range.end <= range.end
            })
            .map(|k| self.markers[k].range)
            .collect();
        let cells = if cells_idx.is_empty() {
            self.split_cells(range)
        } else {
            cells_idx
                .into_iter()
                .map(|r| trim_inlines(self.inlines(r)))
                .collect()
        };
        TableRow { header, cells }
    }

    fn split_cells(&mut self, range: CharRange) -> Vec<Vec<Inline>> {
        let text = self.doc.slice(range);
        let mut cells = Vec::new();
        let mut start = range.start.0;
        let mut byte = 0;
        while let Some(off) = text[byte..].find(CELL_SEPARATOR) {
            let chars = text[byte..byte + off].chars().count();
            cells.push(CharRange::new(start, start + chars));
            start += chars + CELL_SEPARATOR.chars().count();
            byte += off + CELL_SEPARATOR.len();
        }
        cells.push(CharRange::new(start, range.end.0));
        cells
            .into_iter()
            .map(|r| trim_inlines(self.inlines(r)))
            .collect()
    }

    /// Inline content of `range`: text with styled spans, `\n` as line
    /// breaks.
    fn inlines(&self, range: CharRange) -> Vec<Inline> {
        let mut spans: Vec<&Marker> = Vec::new();
        // Markers that start before the range but reach into it.
        let lo = self
            .markers
            .partition_point(|m| m.range.start < range.start);
        for m in &self.markers[..lo] {
            if is_inline(m) && m.range.end > range.start {
                spans.push(m);
            }
        }
        for j in self.starting_in(range) {
            let m = &self.markers[j];
            if is_inline(m) && (!m.range.is_empty() || m.kind == MarkerKind::Image) {
                spans.push(m);
            }
        }
        spans.sort_by_key(|m| m.sort_key());
        let clamp = |m: &Marker| {
            CharRange::new(
                m.range.start.max(range.start),
                m.range
                    .end
                    .min(range.end)
                    .max(m.range.start.max(range.start)),
            )
        };
        let mut cuts: Vec<usize> = vec![range.start.0, range.end.0];
        for m in &spans {
            let r = clamp(m);
            cuts.push(r.start.0);
            cuts.push(r.end.0);
        }
        cuts.sort_unstable();
        cuts.dedup();

        // Stack of open spans: (index into `spans`, children).
        let mut stack: Vec<(Option<usize>, Vec<Inline>)> = vec![(None, Vec::new())];
        let close_to = |stack: &mut Vec<(Option<usize>, Vec<Inline>)>, keep: usize| {
            while stack.len() > keep + 1 {
                if let Some((Some(k), children)) = stack.pop() {
                    let span = Inline::Span(style_of(spans[k]), children);
                    if let Some(parent) = stack.last_mut() {
                        parent.1.push(span);
                    }
                }
            }
        };
        let push_empty_images = |stack: &mut Vec<(Option<usize>, Vec<Inline>)>, at: usize| {
            for m in spans.iter() {
                if m.kind == MarkerKind::Image
                    && m.range.is_empty()
                    && m.range.start.0 == at
                    && let Some(top) = stack.last_mut()
                {
                    top.1.push(Inline::Span(style_of(m), Vec::new()));
                }
            }
        };
        for w in cuts.windows(2) {
            let (a, b) = (w[0], w[1]);
            push_empty_images(&mut stack, a);
            if a == b {
                continue;
            }
            // Active spans in canonical order, one link at most.
            let mut active: Vec<usize> = Vec::new();
            let mut has_link = false;
            for (k, m) in spans.iter().enumerate() {
                let r = clamp(m);
                if r.start.0 <= a && r.end.0 >= b && !r.is_empty() {
                    let linky = matches!(m.kind, MarkerKind::Link | MarkerKind::Footnote);
                    if linky && has_link {
                        continue;
                    }
                    has_link |= linky;
                    active.push(k);
                }
            }
            // Keep the common prefix of open spans, close the rest.
            let open: Vec<usize> = stack.iter().skip(1).filter_map(|(k, _)| *k).collect();
            let common = open.iter().zip(&active).take_while(|(x, y)| x == y).count();
            close_to(&mut stack, common);
            for &k in &active[common..] {
                stack.push((Some(k), Vec::new()));
            }
            let text = self.doc.slice(CharRange::new(a, b));
            if let Some(top) = stack.last_mut() {
                push_text(&mut top.1, &text);
            }
        }
        push_empty_images(&mut stack, range.end.0);
        close_to(&mut stack, 0);
        stack.pop().map(|(_, c)| c).unwrap_or_default()
    }
}

fn style_of(m: &Marker) -> Style {
    match m.kind {
        MarkerKind::Bold => Style::Bold,
        MarkerKind::Italic => Style::Italic,
        MarkerKind::Underline => Style::Underline,
        MarkerKind::Strikethrough => Style::Strikethrough,
        MarkerKind::Math => Style::Math {
            display: m.level == 1,
        },
        MarkerKind::Code => Style::Code,
        MarkerKind::Link => Style::Link(m.reference.clone().unwrap_or_default()),
        MarkerKind::Footnote => Style::FootnoteRef(m.reference.clone().unwrap_or_default()),
        _ => Style::Image(m.reference.clone()),
    }
}

/// Appends text, turning `\n` into line breaks and merging adjacent text.
fn push_text(out: &mut Vec<Inline>, text: &str) {
    for (n, part) in text.split('\n').enumerate() {
        if n > 0 {
            out.push(Inline::LineBreak);
        }
        if part.is_empty() {
            continue;
        }
        if let Some(Inline::Text(prev)) = out.last_mut() {
            prev.push_str(part);
        } else {
            out.push(Inline::Text(part.to_owned()));
        }
    }
}

/// Drops leading and trailing whitespace and line breaks.
fn trim_inlines(mut v: Vec<Inline>) -> Vec<Inline> {
    while matches!(v.last(), Some(Inline::LineBreak)) {
        v.pop();
    }
    while matches!(v.first(), Some(Inline::LineBreak)) {
        v.remove(0);
    }
    if let Some(Inline::Text(t)) = v.first_mut() {
        let trimmed = t.trim_start().to_owned();
        *t = trimmed;
    }
    if let Some(Inline::Text(t)) = v.last_mut() {
        let trimmed = t.trim_end().to_owned();
        *t = trimmed;
    }
    v.retain(|i| !matches!(i, Inline::Text(t) if t.is_empty()));
    v
}

fn sole_image(content: &[Inline]) -> Option<Image> {
    let mut found = None;
    for i in content {
        match i {
            Inline::Text(t) if t.trim().is_empty() => {}
            Inline::LineBreak => {}
            Inline::Span(Style::Image(src), children) if found.is_none() => {
                found = Some(Image {
                    alt: collapse_ws(&Inline::plain(children)),
                    src: src.clone(),
                });
            }
            _ => return None,
        }
    }
    found
}

fn list_from_items(items: Vec<ListItem>) -> List {
    let first = items.iter().find_map(|i| i.label.as_deref());
    let (ordered, start) = match first {
        Some(label) => {
            let digits = label.trim().trim_end_matches(['.', ')']);
            (true, digits.parse::<u64>().unwrap_or(1))
        }
        None => (false, 1),
    };
    List {
        ordered,
        start,
        items,
    }
}

#[cfg(test)]
mod tests {
    use ropey::Rope;
    use textweaver_text::DocumentMeta;

    use super::*;

    fn doc(text: &str, markers: Vec<Marker>) -> Document {
        Document::new(DocumentMeta::default(), Rope::from_str(text), markers)
    }

    fn m(kind: MarkerKind, a: usize, b: usize) -> Marker {
        Marker::new(kind, CharRange::new(a, b))
    }

    #[test]
    fn plain_text_is_paragraphs_with_line_breaks() {
        let d = Document::from_plain_text("One\ntwo\n\n  \nThree");
        assert_eq!(
            blocks(&d),
            vec![
                Block::Paragraph(vec![
                    Inline::Text("One".into()),
                    Inline::LineBreak,
                    Inline::Text("two".into())
                ]),
                Block::Paragraph(vec![Inline::Text("Three".into())]),
            ]
        );
    }

    #[test]
    fn overlapping_inline_markers_nest() {
        // "abcdef": bold 0..4, italic 2..6.
        let d = doc(
            "abcdef",
            vec![m(MarkerKind::Bold, 0, 4), m(MarkerKind::Italic, 2, 6)],
        );
        assert_eq!(
            blocks(&d),
            vec![Block::Paragraph(vec![
                Inline::Span(
                    Style::Bold,
                    vec![
                        Inline::Text("ab".into()),
                        Inline::Span(Style::Italic, vec![Inline::Text("cd".into())])
                    ]
                ),
                Inline::Span(Style::Italic, vec![Inline::Text("ef".into())]),
            ])]
        );
    }

    #[test]
    fn breaks_are_emitted_once_in_order() {
        // "Title\n\nBody" with a section break over everything and a page
        // break inside the body.
        let d = doc(
            "Title\n\nBody text",
            vec![
                m(MarkerKind::SectionBreak, 0, 16).with_label("Chapter"),
                m(MarkerKind::Heading, 0, 5).with_level(1),
                m(MarkerKind::Paragraph, 7, 16),
                m(MarkerKind::PageBreak, 9, 9).with_label("2"),
            ],
        );
        let b = blocks(&d);
        assert!(matches!(&b[0], Block::SectionBreak { title: Some(t) } if t == "Chapter"));
        assert!(matches!(&b[1], Block::Heading { level: 1, .. }));
        assert!(matches!(&b[2], Block::Paragraph(_)));
        assert!(matches!(&b[3], Block::PageBreak { label: Some(l) } if l == "2"));
        assert_eq!(b.len(), 4);
    }

    #[test]
    fn orphan_items_and_rows_are_grouped() {
        let d = doc(
            "a\nb\n\nx | y",
            vec![
                m(MarkerKind::ListItem, 0, 1).with_level(1),
                m(MarkerKind::ListItem, 2, 3).with_level(1),
                m(MarkerKind::TableRow, 5, 10),
            ],
        );
        let b = blocks(&d);
        assert_eq!(b.len(), 2);
        match &b[0] {
            Block::List(l) => assert_eq!(l.items.len(), 2),
            other => panic!("{other:?}"),
        }
        match &b[1] {
            Block::Table(t) => assert_eq!(t.rows[0].cells.len(), 2),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn sole_image_paragraph_is_a_figure() {
        let d = doc(
            "A cat",
            vec![
                m(MarkerKind::Paragraph, 0, 5),
                m(MarkerKind::Image, 0, 5).with_reference("cat.png"),
            ],
        );
        assert_eq!(
            blocks(&d),
            vec![Block::Figure(Image {
                alt: "A cat".into(),
                src: Some("cat.png".into())
            })]
        );
    }
}
