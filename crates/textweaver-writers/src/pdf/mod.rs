//! Tagged PDF through `krilla`, validated against PDF/UA-1.
//!
//! The document is laid out in one flowing column (headings, paragraphs,
//! lists with labels, tables on a grid with the header row repeated on each
//! page, code in a monospaced font, quotes indented, images scaled to the
//! column), and every piece of text is tagged in reading order: `H1` to
//! `H6` with titles, `P`, `L`/`LI`/`Lbl`/`LBody` with list numbering,
//! `Table`/`TR`/`TH` (column scope)/`TD` with a `Caption`, `Figure` with
//! alt text, `BlockQuote`, `Code`, `Note`, and `Link` with its link
//! annotation. Rules, repeated table headers, and page numbers are
//! artifacts. The document has a title (shown in the window title), a
//! language, an outline (bookmarks) built from the headings, and embedded,
//! subset fonts.
//!
//! Fonts: [`PdfOptions::font`](crate::PdfOptions), then the
//! `TEXTWEAVER_PDF_FONT` environment variable, then the first installed
//! family of Atkinson Hyperlegible, Verdana, Segoe UI, Arial, DejaVu Sans,
//! Liberation Sans, and Noto Sans (bold and italic from the same family), a
//! monospaced font for code, and fallback fonts for characters the body
//! font lacks. Characters no font can show become `?` and are reported.
//!
//! With [`PdfOptions::pdf_ua`](crate::PdfOptions) on (the default), krilla
//! checks the PDF/UA-1 rules it can check while writing (title, language,
//! tagging, alt text, heading titles, outline, character mappings, font
//! embedding permissions) and the write fails if any is broken.

mod font;

use std::collections::HashMap;
use std::io::Write;
use std::num::NonZeroU16;
use std::rc::Rc;

use krilla::action::{Action, LinkAction};
use krilla::annotation::{Annotation, LinkAnnotation, Target};
use krilla::configure::{Accessibility, ConfigurationBuilder, ValidationError};
use krilla::destination::XyzDestination;
use krilla::geom::{PathBuilder, Point, Rect, Size, Transform};
use krilla::image::Image as KrillaImage;
use krilla::metadata::{DateTime, Metadata};
use krilla::outline::{Outline, OutlineNode};
use krilla::page::PageSettings;
use krilla::paint::{Fill, Stroke};
use krilla::surface::Surface;
use krilla::tagging::{
    Artifact, ArtifactType, ContentTag, Identifier, ListNumbering, Node, SpanTag, TableHeaderScope,
    Tag, TagGroup, TagKind, TagTree,
};
use krilla::text::TextDirection;
use krilla::{Document as KrillaDocument, SerializeSettings};
use textweaver_text::Document;

use crate::model::{self, Block, Facts, Image, Inline, List, Style, Table};
use crate::resource::{ImageKind, Resource, Resources};
use crate::{Format, WriteError, WriteOptions, WriteReport, Writer, civil, timestamp};
use font::Fonts;

/// Writes tagged PDF.
#[derive(Clone, Copy, Debug, Default)]
pub struct PdfWriter;

impl Writer for PdfWriter {
    fn format(&self) -> Format {
        Format::Pdf
    }

    fn write(
        &self,
        doc: &Document,
        options: &WriteOptions,
        out: &mut dyn Write,
    ) -> Result<WriteReport, WriteError> {
        let mut report = WriteReport::default();
        let blocks = model::blocks(doc);
        let facts = Facts::of(doc, options, &blocks);
        let fonts = Fonts::discover(options.pdf.font.as_deref())?;
        let mut resources = Resources::new(doc, options);
        let mut layout = Layout::new(&fonts, options, &mut report, &mut resources);
        layout.blocks(&blocks, ROOT, 0.0, false);
        let laid = layout.finish();
        let bytes = render(laid, &fonts, &facts, options)?;
        out.write_all(&bytes)?;
        Ok(report)
    }
}

/// The root of the tag tree under construction.
const ROOT: usize = 0;

/// Font style of a run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Look {
    bold: bool,
    italic: bool,
    mono: bool,
}

/// A drawing operation on a page. `slot` places the content in the tag
/// tree; `None` makes it an artifact.
enum Op {
    Text {
        x: f32,
        y: f32,
        face: usize,
        size: f32,
        text: String,
        slot: Option<usize>,
        artifact: ArtifactType,
    },
    Image {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        image: Rc<Resource>,
        slot: Option<usize>,
    },
    Rule {
        x0: f32,
        x1: f32,
        y: f32,
        width: f32,
    },
    Link {
        rect: (f32, f32, f32, f32),
        uri: String,
        alt: String,
        slot: usize,
    },
}

/// A node of the tag tree under construction.
struct TNode {
    tag: Option<TagKind>,
    children: Vec<Child>,
}

enum Child {
    Node(usize),
    Slot(usize),
}

/// The tag tree: groups created in reading order, leaves as slots filled
/// with identifiers while rendering.
struct Tree {
    nodes: Vec<TNode>,
    slots: usize,
}

impl Tree {
    fn new() -> Tree {
        Tree {
            nodes: vec![TNode {
                tag: None,
                children: Vec::new(),
            }],
            slots: 0,
        }
    }

    fn group(&mut self, parent: usize, tag: impl Into<TagKind>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(TNode {
            tag: Some(tag.into()),
            children: Vec::new(),
        });
        self.nodes[parent].children.push(Child::Node(id));
        id
    }

    fn slot(&mut self, parent: usize) -> usize {
        let s = self.slots;
        self.slots += 1;
        self.nodes[parent].children.push(Child::Slot(s));
        s
    }

    fn build(&self, ids: &[Option<Identifier>], node: usize) -> Vec<Node> {
        self.nodes[node]
            .children
            .iter()
            .filter_map(|c| match c {
                Child::Slot(s) => ids.get(*s).copied().flatten().map(Node::Leaf),
                Child::Node(n) => {
                    let children = self.build(ids, *n);
                    let tag = self.nodes[*n].tag.clone()?;
                    (!children.is_empty())
                        .then(|| Node::Group(TagGroup::with_children(tag, children)))
                }
            })
            .collect()
    }
}

/// A piece of a word: text in one look, maybe linked.
#[derive(Clone, Debug)]
struct Part {
    text: String,
    look: Look,
    /// Index into the paragraph's links.
    link: Option<usize>,
}

/// A word: parts with no space between them.
#[derive(Clone, Debug)]
struct Word {
    parts: Vec<Part>,
    space_before: bool,
    /// A forced line break before this word.
    break_before: bool,
}

/// A laid-out line: parts with x offsets from the line start.
#[derive(Clone, Debug, Default)]
struct Line {
    parts: Vec<(f32, Part)>,
}

/// A table cell laid out: its lines and its link targets.
type CellLines = (Vec<Line>, Vec<String>);

/// Inline content flattened for layout.
#[derive(Default)]
struct Flow {
    words: Vec<Word>,
    /// Link targets.
    links: Vec<String>,
    pending_space: bool,
    pending_break: bool,
}

impl Flow {
    fn push_text(&mut self, text: &str, look: Look, link: Option<usize>) {
        for (n, chunk) in text.split(' ').enumerate() {
            if n > 0 {
                self.pending_space = true;
            }
            if chunk.is_empty() {
                continue;
            }
            let glue = !self.pending_space && !self.pending_break && !self.words.is_empty();
            let part = Part {
                text: chunk.to_owned(),
                look,
                link,
            };
            if glue && let Some(w) = self.words.last_mut() {
                w.parts.push(part);
            } else {
                self.words.push(Word {
                    parts: vec![part],
                    space_before: self.pending_space,
                    break_before: self.pending_break,
                });
            }
            self.pending_space = false;
            self.pending_break = false;
        }
    }

    fn inlines(&mut self, inlines: &[Inline], look: Look, link: Option<usize>) {
        for i in inlines {
            match i {
                Inline::Text(t) => {
                    let t = t.replace('\t', " ");
                    self.push_text(&t, look, link);
                }
                Inline::LineBreak => {
                    self.pending_break = true;
                    self.pending_space = false;
                }
                Inline::Span(style, children) => match style {
                    Style::Bold => self.inlines(children, Look { bold: true, ..look }, link),
                    Style::Italic => self.inlines(
                        children,
                        Look {
                            italic: true,
                            ..look
                        },
                        link,
                    ),
                    Style::Code => self.inlines(children, Look { mono: true, ..look }, link),
                    Style::Link(uri) if link.is_none() && is_external(uri) => {
                        self.links.push(uri.trim().to_owned());
                        let l = self.links.len() - 1;
                        self.inlines(children, look, Some(l));
                    }
                    // Underline, footnote references, local links, and
                    // images (as their alt text) are plain text here.
                    _ => self.inlines(children, look, link),
                },
            }
        }
    }
}

fn is_external(target: &str) -> bool {
    let t = target.trim().to_ascii_lowercase();
    ["http://", "https://", "mailto:", "ftp://", "tel:"]
        .iter()
        .any(|p| t.starts_with(p))
}

/// Everything laid out, ready to render.
struct Laid {
    pages: Vec<Vec<Op>>,
    tree: Tree,
    /// (level, title, page, y) of every heading.
    headings: Vec<(u8, String, usize, f32)>,
    page_w: f32,
    page_h: f32,
    margin: f32,
}

struct Layout<'a> {
    fonts: &'a Fonts,
    report: &'a mut WriteReport,
    resources: &'a mut Resources,
    base: f32,
    spacing: f32,
    page_w: f32,
    page_h: f32,
    margin: f32,
    /// Lowest y content may reach (above the page number).
    bottom: f32,
    pages: Vec<Vec<Op>>,
    y: f32,
    tree: Tree,
    headings: Vec<(u8, String, usize, f32)>,
    missing: Vec<char>,
}

impl<'a> Layout<'a> {
    fn new(
        fonts: &'a Fonts,
        options: &WriteOptions,
        report: &'a mut WriteReport,
        resources: &'a mut Resources,
    ) -> Self {
        let (page_w, page_h) = options.pdf.page_size.points();
        let base = options.pdf.font_size.clamp(6.0, 72.0);
        let margin = options.pdf.margin.clamp(18.0, page_w / 4.0);
        Layout {
            fonts,
            report,
            resources,
            base,
            spacing: options.pdf.line_spacing.clamp(1.0, 3.0),
            page_w,
            page_h,
            margin,
            bottom: page_h - margin - base * 2.0,
            pages: vec![Vec::new()],
            y: margin,
            tree: Tree::new(),
            headings: Vec::new(),
            missing: Vec::new(),
        }
    }

    fn top(&self) -> f32 {
        self.margin
    }

    fn at_top(&self) -> bool {
        self.y <= self.top() + 0.01
    }

    fn width(&self) -> f32 {
        self.page_w - 2.0 * self.margin
    }

    fn new_page(&mut self) {
        self.pages.push(Vec::new());
        self.y = self.top();
    }

    fn ensure(&mut self, h: f32) {
        if self.y + h > self.bottom && !self.at_top() {
            self.new_page();
        }
    }

    fn op(&mut self, op: Op) {
        if let Some(p) = self.pages.last_mut() {
            p.push(op);
        }
    }

    fn face_for(&self, look: Look) -> usize {
        let f = &self.fonts.family;
        match (look.mono, look.bold, look.italic) {
            (true, _, _) => f.mono,
            (false, true, true) => f.bold_italic,
            (false, true, false) => f.bold,
            (false, false, true) => f.italic,
            (false, false, false) => f.regular,
        }
    }

    /// Splits text into runs by the face that can draw each char; chars no
    /// face has become `?`.
    fn runs(&mut self, text: &str, look: Look) -> Vec<(usize, String)> {
        let preferred = self.face_for(look);
        let mut out: Vec<(usize, String)> = Vec::new();
        for c in text.chars() {
            let c = if c.is_control() { ' ' } else { c };
            let bad = matches!(c, '\u{FEFF}' | '\u{FFFE}' | '\u{E000}'..='\u{F8FF}');
            let (face, c) = match (!bad).then(|| self.fonts.resolve(preferred, c)).flatten() {
                Some(f) => (f, c),
                None => {
                    if !self.missing.contains(&c) {
                        self.missing.push(c);
                    }
                    (preferred, '?')
                }
            };
            match out.last_mut() {
                Some((f, s)) if *f == face => s.push(c),
                _ => out.push((face, c.to_string())),
            }
        }
        out
    }

    fn measure(&self, text: &str, look: Look, size: f32) -> f32 {
        let preferred = self.face_for(look);
        text.chars()
            .map(|c| {
                let face = self.fonts.resolve(preferred, c).unwrap_or(preferred);
                self.fonts.faces[face].advance(
                    if face == preferred && !self.fonts.faces[face].has(c) {
                        '?'
                    } else {
                        c
                    },
                    size,
                )
            })
            .sum()
    }

    fn word_width(&self, w: &Word, size: f32) -> f32 {
        w.parts
            .iter()
            .map(|p| self.measure(&p.text, p.look, size * look_scale(p.look)))
            .sum()
    }

    /// Breaks words into lines of at most `avail` points.
    fn lines(&self, words: &[Word], avail: f32, size: f32) -> Vec<Line> {
        let mut lines: Vec<Line> = Vec::new();
        let mut line = Line::default();
        let mut x = 0.0f32;
        let space = self.measure(" ", Look::default(), size);
        for w in words {
            let ww = self.word_width(w, size);
            let lead = if w.space_before && !line.parts.is_empty() {
                space
            } else {
                0.0
            };
            if w.break_before || (!line.parts.is_empty() && x + lead + ww > avail) {
                lines.push(std::mem::take(&mut line));
                x = 0.0;
            }
            let lead = if w.space_before && !line.parts.is_empty() {
                space
            } else {
                0.0
            };
            if line.parts.is_empty() && ww > avail {
                // A word longer than the line: split it by characters.
                for p in &w.parts {
                    let mut chunk = String::new();
                    let mut cw = 0.0;
                    let psize = size * look_scale(p.look);
                    for c in p.text.chars() {
                        let adv = self.measure(&c.to_string(), p.look, psize);
                        if x + cw + adv > avail && (x + cw) > 0.0 {
                            if !chunk.is_empty() {
                                line.parts.push((
                                    x,
                                    Part {
                                        text: std::mem::take(&mut chunk),
                                        ..p.clone()
                                    },
                                ));
                            }
                            lines.push(std::mem::take(&mut line));
                            x = 0.0;
                            cw = 0.0;
                        }
                        chunk.push(c);
                        cw += adv;
                    }
                    if !chunk.is_empty() {
                        line.parts.push((
                            x,
                            Part {
                                text: chunk,
                                ..p.clone()
                            },
                        ));
                    }
                    x += cw;
                }
                continue;
            }
            x += lead;
            let mut first = true;
            for p in &w.parts {
                let mut part = p.clone();
                // Keep the space in the text (tagged PDF best practice),
                // at the end of the previous part on the line.
                if first && lead > 0.0 {
                    if let Some((_, prev)) = line.parts.last_mut() {
                        prev.text.push(' ');
                    } else {
                        part.text.insert(0, ' ');
                    }
                }
                first = false;
                let pw = self.measure(&p.text, p.look, size * look_scale(p.look));
                line.parts.push((x, part));
                x += pw;
            }
        }
        if !line.parts.is_empty() || lines.is_empty() {
            lines.push(line);
        }
        lines
    }

    fn line_height(&self, size: f32) -> f32 {
        size * self.spacing
    }

    /// Emits lines at `indent`, tagging text under `group` (links under a
    /// `Link` group per link, created on first use).
    fn emit_lines(
        &mut self,
        lines: &[Line],
        links: &[String],
        group: usize,
        indent: f32,
        size: f32,
        slots: bool,
    ) {
        let mut link_groups: HashMap<usize, usize> = HashMap::new();
        let lh = self.line_height(size);
        for line in lines {
            if self.y + lh > self.bottom && !self.at_top() {
                self.new_page();
            }
            let baseline = self.baseline(size);
            // Merge neighbouring parts with the same look and link.
            let mut merged: Vec<(f32, Part)> = Vec::new();
            for (x, p) in &line.parts {
                match merged.last_mut() {
                    Some((_, m)) if m.look == p.look && m.link == p.link => {
                        m.text.push_str(&p.text)
                    }
                    _ => merged.push((*x, p.clone())),
                }
            }
            for (x, p) in merged {
                let psize = size * look_scale(p.look);
                let parent = match p.link {
                    Some(l) if slots => *link_groups
                        .entry(l)
                        .or_insert_with(|| self.tree.group(group, Tag::Link)),
                    _ => group,
                };
                let mut rx = self.margin + indent + x;
                let start_x = rx;
                for (face, text) in self.runs(&p.text, p.look) {
                    let w = self.fonts.faces[face].width(&text, psize);
                    let slot = slots.then(|| self.tree.slot(parent));
                    self.op(Op::Text {
                        x: rx,
                        y: baseline,
                        face,
                        size: psize,
                        text,
                        slot,
                        artifact: ArtifactType::PaginationOther,
                    });
                    rx += w;
                }
                if let (Some(l), true) = (p.link, slots) {
                    let slot = self.tree.slot(parent);
                    let top = self.y;
                    self.op(Op::Link {
                        rect: (start_x, top, rx.max(start_x + 1.0), top + lh),
                        uri: links[l].clone(),
                        alt: format!("Link: {}", p.text.trim()),
                        slot,
                    });
                }
            }
            self.y += lh;
        }
    }

    fn baseline(&self, size: f32) -> f32 {
        let lh = self.line_height(size);
        let ascent = self.fonts.faces[self.fonts.family.regular].ascent;
        self.y + (lh - size) / 2.0 + ascent * size
    }

    fn flow(&self, inlines: &[Inline], look: Look) -> Flow {
        let mut f = Flow::default();
        f.inlines(inlines, look, None);
        f
    }

    fn paragraph(&mut self, inlines: &[Inline], parent: usize, indent: f32, tight: bool) {
        let flow = self.flow(inlines, Look::default());
        let size = self.base;
        let lines = self.lines(&flow.words, self.width() - indent, size);
        let group = self.tree.group(parent, Tag::P);
        self.emit_lines(&lines, &flow.links, group, indent, size, true);
        self.y += if tight { size * 0.25 } else { size * 0.6 };
    }

    fn heading(&mut self, level: u8, content: &[Inline], parent: usize, indent: f32) {
        let scale = [2.0, 1.667, 1.417, 1.25, 1.083, 1.0][usize::from(level.clamp(1, 6) - 1)];
        let size = self.base * scale;
        let flow = self.flow(
            content,
            Look {
                bold: true,
                italic: level == 6,
                mono: false,
            },
        );
        let lines = self.lines(&flow.words, self.width() - indent, size);
        let before = if self.at_top() { 0.0 } else { size * 0.8 };
        // Keep the heading with the first line after it.
        let need =
            before + lines.len() as f32 * self.line_height(size) + self.line_height(self.base);
        self.ensure(need);
        if !self.at_top() {
            self.y += before;
        }
        let title = model::collapse_ws(&Inline::plain(content));
        let title = if title.is_empty() {
            "Untitled heading".to_owned()
        } else {
            title
        };
        let level16 = NonZeroU16::new(u16::from(level.clamp(1, 6))).unwrap_or(NonZeroU16::MIN);
        let group = self
            .tree
            .group(parent, Tag::Hn(level16, Some(title.clone())));
        self.headings
            .push((level, title, self.pages.len() - 1, self.y));
        self.emit_lines(&lines, &flow.links, group, indent, size, true);
        self.y += size * 0.3;
    }

    fn blocks(&mut self, blocks: &[Block], parent: usize, indent: f32, tight: bool) {
        for b in blocks {
            self.block(b, parent, indent, tight);
        }
    }

    fn block(&mut self, b: &Block, parent: usize, indent: f32, tight: bool) {
        match b {
            Block::Heading { level, content } => self.heading(*level, content, parent, indent),
            Block::Paragraph(content) => self.paragraph(content, parent, indent, tight),
            Block::List(list) => self.list(list, parent, indent),
            Block::Table(table) => self.table(table, parent, indent),
            Block::Code { text, .. } => self.code(text, parent, indent),
            Block::Quote(inner) => {
                let g = self.tree.group(parent, Tag::BlockQuote);
                self.blocks(inner, g, indent + 24.0, tight);
            }
            Block::Figure(img) => self.figure(img, parent, indent, tight),
            Block::Footnote { content, .. } => {
                let g = self.tree.group(parent, Tag::Note);
                let flow = self.flow(content, Look::default());
                let size = self.base;
                let lines = self.lines(&flow.words, self.width() - indent, size);
                self.emit_lines(&lines, &flow.links, g, indent, size, true);
                self.y += size * 0.4;
            }
            Block::SectionBreak { .. } => {
                if !self.at_top() {
                    self.new_page();
                }
            }
            Block::PageBreak { .. } => {}
        }
    }

    fn list(&mut self, list: &List, parent: usize, indent: f32) {
        let numbering = if list.ordered {
            ListNumbering::Decimal
        } else {
            ListNumbering::Disc
        };
        let l = self.tree.group(parent, Tag::L(numbering));
        let bullet = if self.fonts.faces[self.fonts.family.regular].has('\u{2022}') {
            "\u{2022}"
        } else {
            "-"
        };
        let labels: Vec<String> = list
            .items
            .iter()
            .enumerate()
            .map(|(n, item)| {
                if list.ordered {
                    item.label
                        .clone()
                        .unwrap_or_else(|| format!("{}.", list.start + n as u64))
                } else {
                    bullet.to_owned()
                }
            })
            .collect();
        let label_w = labels
            .iter()
            .map(|s| self.measure(s, Look::default(), self.base))
            .fold(0.0f32, f32::max);
        let body_indent = indent + (label_w + self.base * 0.6).max(self.base * 1.5);
        for (item, label) in list.items.iter().zip(labels) {
            let li = self.tree.group(l, Tag::LI);
            self.ensure(self.line_height(self.base));
            let lbl = self.tree.group(li, Tag::Lbl);
            let line = Line {
                parts: vec![(
                    0.0,
                    Part {
                        text: label,
                        look: Look::default(),
                        link: None,
                    },
                )],
            };
            let y = self.y;
            self.emit_lines(&[line], &[], lbl, indent, self.base, true);
            // The body starts on the label's line.
            self.y = y;
            let body = self.tree.group(li, Tag::LBody);
            if item.blocks.is_empty() {
                self.y += self.line_height(self.base);
            }
            self.blocks(&item.blocks, body, body_indent, true);
        }
        self.y += self.base * 0.4;
    }

    fn table(&mut self, table: &Table, parent: usize, indent: f32) {
        let cols = table.columns().max(1);
        let t = self.tree.group(parent, Tag::Table);
        let size = self.base;
        let lh = self.line_height(size);
        let pad = size * 0.3;
        if let Some(c) = &table.caption {
            let flow = self.flow(
                &[Inline::Text(c.clone())],
                Look {
                    bold: true,
                    ..Look::default()
                },
            );
            let lines = self.lines(&flow.words, self.width() - indent, size);
            self.ensure(lines.len() as f32 * lh + 2.0 * lh);
            let g = self.tree.group(t, Tag::Caption);
            self.emit_lines(&lines, &flow.links, g, indent, size, true);
        }
        let avail = self.width() - indent;
        let col_w = avail / cols as f32;
        let x0 = self.margin + indent;
        let x1 = x0 + avail;
        let empty: Vec<Inline> = Vec::new();
        // Lines of every cell, computed up front.
        let rows: Vec<(bool, Vec<CellLines>)> = table
            .rows
            .iter()
            .map(|row| {
                let look = Look {
                    bold: row.header,
                    ..Look::default()
                };
                let cells = (0..cols)
                    .map(|c| {
                        let flow = self.flow(row.cells.get(c).unwrap_or(&empty), look);
                        (self.lines(&flow.words, col_w - 2.0 * pad, size), flow.links)
                    })
                    .collect();
                (row.header, cells)
            })
            .collect();
        let header_count = table.rows.iter().take_while(|r| r.header).count();
        self.ensure(lh * 2.0 + 2.0 * pad);
        self.op(Op::Rule {
            x0,
            x1,
            y: self.y,
            width: 0.75,
        });
        for (r, (header, cells)) in rows.iter().enumerate() {
            let tr = self.tree.group(t, Tag::TR);
            let height = cells.iter().map(|(l, _)| l.len()).max().unwrap_or(1).max(1);
            let row_h = height as f32 * lh + 2.0 * pad;
            if self.y + row_h > self.bottom && !self.at_top() {
                self.new_page();
                if r >= header_count && header_count > 0 {
                    // Repeat the header rows as artifacts.
                    self.op(Op::Rule {
                        x0,
                        x1,
                        y: self.y,
                        width: 0.75,
                    });
                    for (_, hcells) in rows.iter().take(header_count) {
                        let hh = hcells
                            .iter()
                            .map(|(l, _)| l.len())
                            .max()
                            .unwrap_or(1)
                            .max(1);
                        let top = self.y + pad;
                        for i in 0..hh {
                            for (c, (lines, links)) in hcells.iter().enumerate() {
                                if let Some(line) = lines.get(i) {
                                    self.y = top + i as f32 * lh;
                                    self.emit_lines(
                                        std::slice::from_ref(line),
                                        links,
                                        ROOT,
                                        indent + c as f32 * col_w + pad,
                                        size,
                                        false,
                                    );
                                }
                            }
                        }
                        self.y = top + hh as f32 * lh + pad;
                        self.op(Op::Rule {
                            x0,
                            x1,
                            y: self.y,
                            width: 1.5,
                        });
                    }
                }
            }
            let cell_groups: Vec<usize> = (0..cols)
                .map(|_| {
                    if *header {
                        self.tree.group(tr, Tag::TH(TableHeaderScope::Column))
                    } else {
                        self.tree.group(tr, Tag::TD)
                    }
                })
                .collect();
            let mut top = self.y + pad;
            for i in 0..height {
                if top + (i as f32 + 1.0) * lh > self.bottom && !(self.at_top() && i == 0) {
                    // A row taller than the page continues on the next.
                    self.new_page();
                    top = self.y - i as f32 * lh;
                }
                for (c, (lines, links)) in cells.iter().enumerate() {
                    if let Some(line) = lines.get(i) {
                        self.y = top + i as f32 * lh;
                        self.emit_lines(
                            std::slice::from_ref(line),
                            links,
                            cell_groups[c],
                            indent + c as f32 * col_w + pad,
                            size,
                            true,
                        );
                    }
                }
            }
            self.y = top + height as f32 * lh + pad;
            let width = if *header && r + 1 == header_count {
                1.5
            } else {
                0.5
            };
            self.op(Op::Rule {
                x0,
                x1,
                y: self.y,
                width,
            });
        }
        self.y += size * 0.8;
    }

    fn code(&mut self, text: &str, parent: usize, indent: f32) {
        let size = self.base * 0.9;
        let look = Look {
            mono: true,
            ..Look::default()
        };
        let p = self.tree.group(parent, Tag::P);
        let g = self.tree.group(p, Tag::Code);
        let avail = self.width() - indent - 12.0;
        let mut lines = Vec::new();
        for src in text.split('\n') {
            let src = src.replace('\t', "    ");
            // Hard-wrap at the column width, keeping every space.
            let mut chunk = String::new();
            let mut w = 0.0;
            for c in src.chars() {
                let adv = self.measure(&c.to_string(), look, size);
                if w + adv > avail && !chunk.is_empty() {
                    lines.push(std::mem::take(&mut chunk));
                    w = 0.0;
                }
                chunk.push(c);
                w += adv;
            }
            lines.push(chunk);
        }
        let lines: Vec<Line> = lines
            .into_iter()
            .map(|text| Line {
                parts: if text.is_empty() {
                    Vec::new()
                } else {
                    vec![(
                        0.0,
                        Part {
                            text,
                            look,
                            link: None,
                        },
                    )]
                },
            })
            .collect();
        self.emit_lines(&lines, &[], g, indent + 12.0, size, true);
        self.y += self.base * 0.6;
    }

    fn figure(&mut self, img: &Image, parent: usize, indent: f32, tight: bool) {
        let res = img
            .src
            .as_deref()
            .and_then(|src| self.resources.get(src, self.report))
            .filter(|r| {
                let ok = !matches!(r.kind, ImageKind::Svg);
                if !ok {
                    self.report.warn(
                        "An SVG image cannot go into the PDF yet, so its description was written instead.",
                    );
                }
                ok
            });
        let Some(res) = res else {
            if !img.alt.is_empty() {
                self.paragraph(&[Inline::Text(img.alt.clone())], parent, indent, tight);
            }
            return;
        };
        let (pw, ph) = match res.size {
            Some((w, h)) => (w as f32 * 0.75, h as f32 * 0.75),
            None => match decode(&res) {
                Some(i) => {
                    let (w, h) = i.size();
                    (w as f32 * 0.75, h as f32 * 0.75)
                }
                None => {
                    self.paragraph(&[Inline::Text(img.alt.clone())], parent, indent, tight);
                    return;
                }
            },
        };
        let max_w = self.width() - indent;
        let max_h = self.bottom - self.top();
        let scale = (max_w / pw).min(max_h / ph).min(1.0);
        let (w, h) = (pw * scale, ph * scale);
        self.ensure(h);
        let slot = if img.alt.is_empty() {
            // A decorative image is an artifact.
            None
        } else {
            let g = self.tree.group(parent, Tag::Figure(Some(img.alt.clone())));
            Some(self.tree.slot(g))
        };
        self.op(Op::Image {
            x: self.margin + indent,
            y: self.y,
            w,
            h,
            image: res,
            slot,
        });
        self.y += h + self.base * 0.6;
    }

    fn finish(self) -> Laid {
        if !self.missing.is_empty() {
            let list: Vec<String> = self
                .missing
                .iter()
                .take(10)
                .map(|c| format!("U+{:04X}", *c as u32))
                .collect();
            self.report.warn(format!(
                "{} characters have no font on this system and were written as question marks: {}.",
                self.missing.len(),
                list.join(", ")
            ));
        }
        Laid {
            pages: self.pages,
            tree: self.tree,
            headings: self.headings,
            page_w: self.page_w,
            page_h: self.page_h,
            margin: self.margin,
        }
    }
}

/// Monospaced text is set slightly smaller.
fn look_scale(look: Look) -> f32 {
    if look.mono { 0.9 } else { 1.0 }
}

fn decode(res: &Resource) -> Option<KrillaImage> {
    let data: krilla::Data = res.bytes.clone().into();
    match res.kind {
        ImageKind::Png => KrillaImage::from_png(data, false).ok(),
        ImageKind::Jpeg => KrillaImage::from_jpeg(data, false).ok(),
        ImageKind::Gif => KrillaImage::from_gif(data, false).ok(),
        ImageKind::Webp => KrillaImage::from_webp(data, false).ok(),
        ImageKind::Svg => None,
    }
}

fn outline(headings: &[(u8, String, usize, f32)], margin: f32, title: &str) -> Outline {
    fn build(
        h: &[(u8, String, usize, f32)],
        i: &mut usize,
        min_level: u8,
        margin: f32,
    ) -> Vec<OutlineNode> {
        let mut nodes = Vec::new();
        while *i < h.len() {
            let (level, title, page, y) = &h[*i];
            if *level < min_level {
                break;
            }
            *i += 1;
            let mut node = OutlineNode::new(
                title.clone(),
                XyzDestination::new(*page, Point::from_xy(margin, *y)),
            );
            if *i < h.len() && h[*i].0 > *level {
                for child in build(h, i, level + 1, margin) {
                    node.push_child(child);
                }
            }
            nodes.push(node);
        }
        nodes
    }
    let mut outline = Outline::new();
    if headings.is_empty() {
        outline.push_child(OutlineNode::new(
            title.to_owned(),
            XyzDestination::new(0, Point::from_xy(margin, margin)),
        ));
    } else {
        let mut i = 0;
        for node in build(headings, &mut i, 0, margin) {
            outline.push_child(node);
        }
    }
    outline
}

fn render(
    laid: Laid,
    fonts: &Fonts,
    facts: &Facts,
    options: &WriteOptions,
) -> Result<Vec<u8>, WriteError> {
    let mut builder = ConfigurationBuilder::new();
    if options.pdf.pdf_ua {
        builder = builder.with_accessibility_validator(Accessibility::UA1);
    }
    let configuration = builder
        .finish()
        .map_err(|e| WriteError::Pdf(format!("{e:?}")))?;
    let settings = SerializeSettings {
        configuration,
        enable_tagging: true,
        compress_content_streams: options.pdf.compress,
        xmp_metadata: true,
        ..SerializeSettings::default()
    };
    let mut document = KrillaDocument::new_with(settings);
    let mut ids: Vec<Option<Identifier>> = vec![None; laid.tree.slots];
    let total = laid.pages.len();
    let size = Size::from_wh(laid.page_w, laid.page_h)
        .ok_or_else(|| WriteError::Pdf("invalid page size".to_owned()))?;
    let footer_face = fonts.family.regular;
    let base = options.pdf.font_size.clamp(6.0, 72.0);
    let mut images: HashMap<*const Resource, KrillaImage> = HashMap::new();
    for (n, ops) in laid.pages.iter().enumerate() {
        let mut page = document.start_page_with(PageSettings::new(size));
        {
            let mut surface = page.surface();
            surface.set_fill(Some(Fill::default()));
            for op in ops {
                draw(&mut surface, op, fonts, &mut ids, &mut images);
            }
            // Page numbers are artifacts.
            let label = format!("Page {} of {total}", n + 1);
            let fsize = (base * 0.8).max(6.0);
            let face = &fonts.faces[footer_face];
            let w = face.width(&label, fsize);
            surface.start_tagged(ContentTag::Artifact(Artifact::with_kind(
                ArtifactType::Footer,
            )));
            surface.draw_text(
                Point::from_xy((laid.page_w - w) / 2.0, laid.page_h - laid.margin / 2.0),
                face.krilla.clone(),
                fsize,
                &label,
                false,
                TextDirection::Auto,
            );
            surface.end_tagged();
            surface.finish();
        }
        for op in ops {
            if let Op::Link {
                rect: (l, t, r, b),
                uri,
                alt,
                slot,
            } = op
                && let Some(rect) = Rect::from_ltrb(*l, *t, *r, *b)
            {
                let link = LinkAnnotation::new(
                    rect,
                    Target::Action(Action::Link(LinkAction::new(uri.clone()))),
                );
                let id = page.add_tagged_annotation(Annotation::new_link(link, Some(alt.clone())));
                ids[*slot] = Some(id);
            }
        }
        page.finish();
    }
    let children = laid.tree.build(&ids, ROOT);
    let mut tree = TagTree::new().with_lang(Some(facts.language.clone()));
    for c in children {
        tree.push(c);
    }
    document.set_tag_tree(tree);
    document.set_outline(outline(&laid.headings, laid.margin, &facts.title));
    let (y, mo, d, h, mi, s) = civil(timestamp(options));
    let date = DateTime::new(u16::try_from(y).unwrap_or(1970))
        .month(mo as u8)
        .day(d as u8)
        .hour(h as u8)
        .minute(mi as u8)
        .second(s as u8)
        .utc_offset_hour(0)
        .utc_offset_minute(0);
    let mut metadata = Metadata::new()
        .title(facts.title.clone())
        .language(facts.language.clone())
        .creator("textweaver".to_owned())
        .producer("textweaver with krilla".to_owned())
        .creation_date(date);
    if let Some(a) = &facts.author {
        metadata = metadata.authors(vec![a.clone()]);
    }
    document.set_metadata(metadata);
    document.finish().map_err(|e| WriteError::Pdf(describe(&e)))
}

fn draw(
    surface: &mut Surface<'_>,
    op: &Op,
    fonts: &Fonts,
    ids: &mut [Option<Identifier>],
    images: &mut HashMap<*const Resource, KrillaImage>,
) {
    match op {
        Op::Text {
            x,
            y,
            face,
            size,
            text,
            slot,
            artifact,
        } => {
            let id = match slot {
                Some(_) => surface.start_tagged(ContentTag::Span(SpanTag::empty())),
                None => surface.start_tagged(ContentTag::Artifact(Artifact::with_kind(*artifact))),
            };
            surface.draw_text(
                Point::from_xy(*x, *y),
                fonts.faces[*face].krilla.clone(),
                *size,
                text,
                false,
                TextDirection::Auto,
            );
            surface.end_tagged();
            if let Some(s) = slot {
                ids[*s] = Some(id);
            }
        }
        Op::Image {
            x,
            y,
            w,
            h,
            image,
            slot,
        } => {
            let key = Rc::as_ptr(image);
            let decoded = match images.get(&key) {
                Some(i) => Some(i.clone()),
                None => decode(image).inspect(|i| {
                    images.insert(key, i.clone());
                }),
            };
            let (Some(img), Some(size)) = (decoded, Size::from_wh(*w, *h)) else {
                return;
            };
            let id = match slot {
                Some(_) => surface.start_tagged(ContentTag::Other),
                None => surface.start_tagged(ContentTag::Artifact(Artifact::with_kind(
                    ArtifactType::Layout,
                ))),
            };
            surface.push_transform(&Transform::from_translate(*x, *y));
            surface.draw_image(img, size);
            surface.pop();
            surface.end_tagged();
            if let Some(s) = slot {
                ids[*s] = Some(id);
            }
        }
        Op::Rule { x0, x1, y, width } => {
            let mut pb = PathBuilder::new();
            pb.move_to(*x0, *y);
            pb.line_to(*x1, *y);
            if let Some(path) = pb.finish() {
                surface.start_tagged(ContentTag::Artifact(Artifact::with_kind(
                    ArtifactType::Layout,
                )));
                surface.set_stroke(Some(Stroke {
                    width: *width,
                    ..Stroke::default()
                }));
                surface.draw_path(&path);
                surface.set_stroke(None);
                surface.end_tagged();
            }
        }
        Op::Link { .. } => {}
    }
}

/// A readable account of a krilla error.
fn describe(e: &krilla::error::KrillaError) -> String {
    use krilla::error::KrillaError;
    match e {
        KrillaError::Validation(errors) => {
            let list: Vec<String> = errors
                .iter()
                .take(8)
                .map(|(v, _)| match v {
                    ValidationError::ContainsNotDefGlyph(_, _, text) => {
                        format!("the font has no glyph for {text:?}")
                    }
                    ValidationError::NoDocumentLanguage => {
                        "the document has no language".to_owned()
                    }
                    ValidationError::NoDocumentTitle => "the document has no title".to_owned(),
                    ValidationError::MissingAltText(_) => "an image has no description".to_owned(),
                    other => format!("{other:?}"),
                })
                .collect();
            format!("PDF/UA validation failed: {}", list.join("; "))
        }
        KrillaError::Font(_, why) => format!("a font could not be embedded: {why}"),
        other => format!("{other:?}"),
    }
}
