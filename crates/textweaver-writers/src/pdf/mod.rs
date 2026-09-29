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
//! Links work: web and mail links open, and links to a heading in the
//! document (`#section-title`, as Markdown writes them) and footnote
//! references jump to their target. Options add a title page, a table of
//! contents whose entries are links (`TOC`/`TOCI`, with page numbers),
//! large print, and the page size, margins, and line spacing
//! ([`PdfOptions`](crate::PdfOptions)). Images without a description are
//! marked decorative and reported.
//!
//! Fonts: by default the bundled Atkinson Hyperlegible Next for text and
//! Atkinson Hyperlegible Mono for code, so a PDF looks the same everywhere
//! and never fails for lack of an installed font. Another bundled or
//! installed family, or a font file, can be chosen by name
//! ([`PdfOptions::font_family`](crate::PdfOptions),
//! [`PdfOptions::code_font_family`](crate::PdfOptions),
//! [`PdfOptions::font`](crate::PdfOptions), or `TEXTWEAVER_PDF_FONT`).
//! Installed fallback fonts cover characters the chosen fonts lack;
//! characters no font can show become `?` and are reported.
//!
//! With [`PdfOptions::pdf_ua`](crate::PdfOptions) on (the default), krilla
//! checks the PDF/UA-1 rules it can check while writing (title, language,
//! tagging, alt text, heading titles, outline, character mappings, font
//! embedding permissions) and the write fails if any is broken.
//!
//! Page labels (ADR-0041): when the document has print page breaks (from
//! a DAISY book, an EPUB page list, or a scanned PDF), each PDF page is
//! labelled with the print page its first line belongs to, as a printed
//! book's running page number would be, so a viewer's "go to page 42" goes
//! to the page where print page 42 is under way at the top. Pages
//! before the first print page (a title page, the contents) are labelled
//! i, ii, and so on. A publishing template's title page carries its fields
//! (see [`Template`](crate::Template)).

mod font;

use crate::math::Formula;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::num::{NonZeroU16, NonZeroU32};
use std::rc::Rc;

use krilla::action::{Action, LinkAction};
use krilla::annotation::{Annotation, LinkAnnotation, Target};
use krilla::configure::{Accessibility, ConfigurationBuilder, ValidationError};
use krilla::destination::{Destination, XyzDestination};
use krilla::geom::{PathBuilder, Point, Rect, Size, Transform};
use krilla::image::Image as KrillaImage;
use krilla::metadata::{DateTime, Metadata};
use krilla::outline::{Outline, OutlineNode};
use krilla::page::{NumberingStyle, PageLabel, PageSettings};
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
use crate::template::{self, TitlePage};
use crate::{
    Format, LARGE_PRINT_MIN_SIZE, WriteError, WriteOptions, WriteReport, Writer, civil, timestamp,
};
use font::Fonts;

/// Checks, without writing anything, that the fonts PDF output needs can be
/// found and read with these options: [`WriteError::NoFont`] when no font
/// is bundled, installed, or named, [`WriteError::Font`] when the named one
/// cannot be found or used. A batch converter calls this once before
/// converting many files, so a missing font is one clear message rather
/// than one failure per file.
pub fn check_fonts(options: &WriteOptions) -> Result<(), WriteError> {
    Fonts::discover(&options.pdf).map(|_| ())
}

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
        let fonts = Fonts::discover(&options.pdf)?;
        let anchors = Anchors::of(&blocks);
        let mut resources = Resources::new(doc, options);
        let mut layout = Layout::new(&fonts, options, &anchors, &mut report, &mut resources);
        if options.pdf.title_page {
            // A template's title page (APA, AMA, manuscript) has its own
            // fields; otherwise the title, the author, and the date.
            let page = options
                .template
                .and_then(|t| template::title_page(t, doc, options, &facts, &blocks))
                .unwrap_or_else(|| {
                    let mut lines: Vec<String> = facts.author.iter().cloned().collect();
                    lines.extend(template::title_date(doc, options));
                    TitlePage {
                        title: facts.title.clone(),
                        lines,
                    }
                });
            layout.title_page(&page);
        }
        if options.pdf.toc && !anchors.headings.is_empty() {
            layout.contents(options.pdf.toc_depth.clamp(1, 6));
        }
        layout.blocks(&blocks, ROOT, 0.0, false);
        let laid = layout.finish();
        let bytes = render(laid, &fonts, &facts, options)?;
        out.write_all(&bytes)?;
        Ok(report)
    }
}

/// The root of the tag tree under construction.
const ROOT: usize = 0;

/// Height of the strikethrough line above the baseline, as a fraction of
/// the font size (about the middle of the lowercase letters).
const STRIKE_RISE: f32 = 0.3;

/// Thickness of the strikethrough line, as a fraction of the font size.
const STRIKE_THICKNESS: f32 = 0.06;

/// Font style of a run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Look {
    bold: bool,
    italic: bool,
    mono: bool,
    /// Struck through: a line is drawn through the text (as an artifact;
    /// the text itself is tagged as usual).
    strike: bool,
}

/// Where a link goes.
#[derive(Clone, Debug, PartialEq, Eq)]
enum LinkTarget {
    /// A web, mail, or other external address.
    Uri(String),
    /// The `n`th heading of the document.
    Heading(usize),
    /// The body of footnote `id`.
    Note(String),
    /// Not a link: a formula, tagged `Formula` with this spoken alt text.
    Formula(String),
}

/// Headings and footnotes, found before layout so links (and the table of
/// contents) can point at them before they are laid out.
#[derive(Default)]
struct Anchors {
    /// (level, title) of every heading, in layout order.
    headings: Vec<(u8, String)>,
    /// Heading slugs (`#introduction`) to heading index.
    slugs: HashMap<String, usize>,
    /// Footnote ids with a body.
    notes: HashSet<String>,
}

impl Anchors {
    fn of(blocks: &[Block]) -> Anchors {
        let mut a = Anchors::default();
        a.walk(blocks);
        a
    }

    fn walk(&mut self, blocks: &[Block]) {
        for b in blocks {
            match b {
                Block::Heading { level, content } => {
                    let title = model::collapse_ws(&Inline::plain(content));
                    let n = self.headings.len();
                    // GitHub's rule: a repeated slug gets -1, -2, ...
                    let base = slug(&title);
                    let mut s = base.clone();
                    let mut k = 1;
                    while self.slugs.contains_key(&s) {
                        s = format!("{base}-{k}");
                        k += 1;
                    }
                    self.slugs.insert(s, n);
                    self.headings.push((*level, title));
                }
                Block::Footnote { id, .. } => {
                    self.notes.insert(id.clone());
                }
                Block::Quote(inner) => self.walk(inner),
                Block::List(list) => {
                    for item in &list.items {
                        self.walk(&item.blocks);
                    }
                }
                _ => {}
            }
        }
    }

    /// The heading a `#fragment` link points at: its slug, or its title.
    fn fragment(&self, fragment: &str) -> Option<usize> {
        let f = fragment.trim();
        self.slugs
            .get(f)
            .or_else(|| self.slugs.get(&slug(f)))
            .copied()
    }
}

/// A heading's anchor as Markdown renderers make it: lowercase, letters,
/// digits, hyphens, and underscores kept, spaces turned into hyphens, other
/// punctuation dropped.
fn slug(title: &str) -> String {
    title
        .trim()
        .chars()
        .flat_map(char::to_lowercase)
        .filter_map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                Some(c)
            } else if c.is_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .collect()
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
    /// The page number of heading `heading`, right-aligned at `right` (the
    /// table of contents; known only once the body is laid out).
    PageRef {
        right: f32,
        y: f32,
        face: usize,
        size: f32,
        heading: usize,
        slot: usize,
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
        target: LinkTarget,
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

impl Line {
    fn width(&self, layout: &Layout<'_>, size: f32) -> f32 {
        self.parts.last().map_or(0.0, |(x, p)| {
            x + layout.measure(&p.text, p.look, size * layout.look_scale(p.look))
        })
    }
}

/// A table cell laid out: its lines and its link targets.
type CellLines = (Vec<Line>, Vec<LinkTarget>);

/// Inline content flattened for layout.
struct Flow<'a> {
    anchors: &'a Anchors,
    words: Vec<Word>,
    /// Link targets.
    links: Vec<LinkTarget>,
    /// `#fragment` links with no heading to go to.
    dangling: usize,
    /// The footnote whose body this is (its own label is not a link).
    in_note: Option<String>,
    pending_space: bool,
    pending_break: bool,
}

impl<'a> Flow<'a> {
    fn new(anchors: &'a Anchors, in_note: Option<String>) -> Self {
        Flow {
            anchors,
            words: Vec::new(),
            links: Vec::new(),
            dangling: 0,
            in_note,
            pending_space: false,
            pending_break: false,
        }
    }

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

    fn linked(&mut self, target: LinkTarget, children: &[Inline], look: Look) {
        self.links.push(target);
        let l = self.links.len() - 1;
        self.inlines(children, look, Some(l));
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
                    Style::Strikethrough => self.inlines(
                        children,
                        Look {
                            strike: true,
                            ..look
                        },
                        link,
                    ),
                    // A formula prints in linear form ("πr²"), tagged
                    // `Formula` with its spoken form as alt text; display
                    // math stands on its own line.
                    Style::Math { display } => {
                        let f = Formula::from_marked(&Inline::plain(children), *display);
                        let tag = match link {
                            Some(l) => Some(l),
                            None => {
                                self.links.push(LinkTarget::Formula(f.spoken()));
                                Some(self.links.len() - 1)
                            }
                        };
                        if *display {
                            self.pending_break = true;
                        }
                        self.push_text(&f.linear(), look, tag);
                        if *display {
                            self.pending_break = true;
                        }
                    }
                    Style::Link(uri) if link.is_none() && is_external(uri) => {
                        self.linked(LinkTarget::Uri(uri.trim().to_owned()), children, look);
                    }
                    Style::Link(uri) if link.is_none() && uri.trim().starts_with('#') => {
                        match self.anchors.fragment(&uri.trim()[1..]) {
                            Some(h) => self.linked(LinkTarget::Heading(h), children, look),
                            None => {
                                self.dangling += 1;
                                self.inlines(children, look, link);
                            }
                        }
                    }
                    Style::FootnoteRef(id)
                        if link.is_none()
                            && self.anchors.notes.contains(id)
                            && self.in_note.as_deref() != Some(id.as_str()) =>
                    {
                        self.linked(LinkTarget::Note(id.clone()), children, look);
                    }
                    // Underline, links to other files, and images (as
                    // their alt text) are plain text here.
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
    /// Footnote id → (page, y) of its body.
    notes: HashMap<String, (usize, f32)>,
    /// (page, y) of the table of contents, when there is one.
    contents: Option<(usize, f32)>,
    /// Pages without a footer (the title page).
    no_footer: usize,
    /// (PDF page, print page label, starts at the page's top) where each
    /// print page starts.
    print_pages: Vec<(usize, String, bool)>,
    page_w: f32,
    page_h: f32,
    margin: f32,
    base: f32,
}

struct Layout<'a> {
    fonts: &'a Fonts,
    anchors: &'a Anchors,
    report: &'a mut WriteReport,
    resources: &'a mut Resources,
    base: f32,
    spacing: f32,
    large: bool,
    page_w: f32,
    page_h: f32,
    margin: f32,
    /// Lowest y content may reach (above the page number).
    bottom: f32,
    pages: Vec<Vec<Op>>,
    y: f32,
    tree: Tree,
    headings: Vec<(u8, String, usize, f32)>,
    notes: HashMap<String, (usize, f32)>,
    contents: Option<(usize, f32)>,
    no_footer: usize,
    /// (PDF page, print page label, starts at the page's top) where each
    /// print page starts.
    print_pages: Vec<(usize, String, bool)>,
    /// A print page break waiting for the next line of text, which fixes
    /// the PDF page it starts on.
    pending_print: Option<String>,
    missing: Vec<char>,
    /// Images with no description, written as decorative.
    undescribed: usize,
    /// `#fragment` links that point nowhere.
    dangling: usize,
}

impl<'a> Layout<'a> {
    fn new(
        fonts: &'a Fonts,
        options: &WriteOptions,
        anchors: &'a Anchors,
        report: &'a mut WriteReport,
        resources: &'a mut Resources,
    ) -> Self {
        let pdf = &options.pdf;
        let (page_w, page_h) = pdf.page_size.points();
        let large = pdf.large_print;
        let mut base = if pdf.font_size.is_finite() {
            pdf.font_size.clamp(6.0, 72.0)
        } else {
            12.0
        };
        let mut spacing = if pdf.line_spacing.is_finite() {
            pdf.line_spacing.clamp(1.0, 3.0)
        } else {
            1.5
        };
        if large {
            base = base.max(LARGE_PRINT_MIN_SIZE);
            spacing = spacing.max(1.5);
        }
        let margin = if pdf.margin.is_finite() {
            pdf.margin.clamp(18.0, page_w / 4.0)
        } else {
            72.0
        };
        Layout {
            fonts,
            anchors,
            report,
            resources,
            base,
            spacing,
            large,
            page_w,
            page_h,
            margin,
            bottom: page_h - margin - base * 2.0,
            pages: vec![Vec::new()],
            y: margin,
            tree: Tree::new(),
            headings: Vec::new(),
            notes: HashMap::new(),
            contents: None,
            no_footer: 0,
            print_pages: Vec::new(),
            pending_print: None,
            missing: Vec::new(),
            undescribed: 0,
            dangling: 0,
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

    /// Space after a paragraph.
    fn paragraph_gap(&self, size: f32) -> f32 {
        if self.large { size * 0.9 } else { size * 0.6 }
    }

    /// Monospaced text is set slightly smaller, except in large print.
    fn look_scale(&self, look: Look) -> f32 {
        if look.mono && !self.large { 0.9 } else { 1.0 }
    }

    fn face_for(&self, look: Look) -> usize {
        let f = &self.fonts.family;
        match (look.mono, look.bold, look.italic) {
            (true, true, _) => f.mono_bold,
            (true, false, _) => f.mono,
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
            .map(|p| self.measure(&p.text, p.look, size * self.look_scale(p.look)))
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
                    let psize = size * self.look_scale(p.look);
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
                let pw = self.measure(&p.text, p.look, size * self.look_scale(p.look));
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
        links: &[LinkTarget],
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
            if let Some(label) = self.pending_print.take() {
                let top = self.at_top();
                self.print_pages.push((self.pages.len() - 1, label, top));
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
                let psize = size * self.look_scale(p.look);
                let parent = match p.link {
                    Some(l) if slots => *link_groups.entry(l).or_insert_with(|| match &links[l] {
                        LinkTarget::Formula(alt) => {
                            self.tree.group(group, Tag::Formula(Some(alt.clone())))
                        }
                        _ => self.tree.group(group, Tag::Link),
                    }),
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
                if p.look.strike {
                    self.strike(&p, start_x, rx, baseline, psize);
                }
                if let (Some(l), true) = (p.link, slots)
                    && !matches!(links[l], LinkTarget::Formula(_))
                {
                    let slot = self.tree.slot(parent);
                    let top = self.y;
                    self.op(Op::Link {
                        rect: (start_x, top, rx.max(start_x + 1.0), top + lh),
                        target: links[l].clone(),
                        alt: format!("Link: {}", p.text.trim()),
                        slot,
                    });
                }
            }
            self.y += lh;
        }
    }

    /// A line through struck text drawn from `x0` to `x1` at `baseline`,
    /// leaving out the spaces that start or end the part. It sits about the
    /// middle of the lowercase letters (0.3 of the size above the
    /// baseline), in the text colour, with a thickness that scales with the
    /// size. It is a layout artifact: the text keeps its tags.
    fn strike(&mut self, p: &Part, x0: f32, x1: f32, baseline: f32, size: f32) {
        let space = self.measure(" ", p.look, size);
        let lead = p.text.chars().take_while(|c| *c == ' ').count();
        if lead == p.text.chars().count() {
            return;
        }
        let trail = p.text.chars().rev().take_while(|c| *c == ' ').count();
        let x0 = x0 + space * lead as f32;
        let x1 = x1 - space * trail as f32;
        if x1 <= x0 {
            return;
        }
        self.op(Op::Rule {
            x0,
            x1,
            y: baseline - size * STRIKE_RISE,
            width: (size * STRIKE_THICKNESS).max(0.4),
        });
    }

    fn baseline(&self, size: f32) -> f32 {
        let lh = self.line_height(size);
        let ascent = self.fonts.faces[self.fonts.family.regular].ascent;
        self.y + (lh - size) / 2.0 + ascent * size
    }

    fn flow(&mut self, inlines: &[Inline], look: Look) -> Flow<'a> {
        self.flow_in(inlines, look, None)
    }

    /// A flow inside footnote `in_note`, whose own label is not a link.
    fn flow_in(&mut self, inlines: &[Inline], look: Look, in_note: Option<String>) -> Flow<'a> {
        let mut f = Flow::new(self.anchors, in_note);
        f.inlines(inlines, look, None);
        self.dangling += f.dangling;
        f
    }

    fn paragraph(&mut self, inlines: &[Inline], parent: usize, indent: f32, tight: bool) {
        let flow = self.flow(inlines, Look::default());
        let size = self.base;
        let lines = self.lines(&flow.words, self.width() - indent, size);
        let group = self.tree.group(parent, Tag::P);
        self.emit_lines(&lines, &flow.links, group, indent, size, true);
        self.y += if tight {
            size * 0.25
        } else {
            self.paragraph_gap(size)
        };
    }

    /// Heading sizes as multiples of the body size; large print keeps them
    /// closer to the (already large) body text.
    fn heading_scale(&self, level: u8) -> f32 {
        let i = usize::from(level.clamp(1, 6) - 1);
        if self.large {
            [1.5, 1.33, 1.2, 1.1, 1.0, 1.0][i]
        } else {
            [2.0, 1.667, 1.417, 1.25, 1.083, 1.0][i]
        }
    }

    fn heading(&mut self, level: u8, content: &[Inline], parent: usize, indent: f32) {
        let size = self.base * self.heading_scale(level);
        let flow = self.flow(
            content,
            Look {
                bold: true,
                italic: level == 6,
                mono: false,
                strike: false,
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

    /// Lines of `text` centred on the page, tagged as one paragraph.
    fn centred(&mut self, text: &str, look: Look, size: f32) {
        let flow = self.flow(&[Inline::Text(text.to_owned())], look);
        let lines = self.lines(&flow.words, self.width(), size);
        let group = self.tree.group(ROOT, Tag::P);
        for line in &lines {
            let indent = ((self.width() - line.width(self, size)) / 2.0).max(0.0);
            self.emit_lines(std::slice::from_ref(line), &[], group, indent, size, true);
        }
    }

    /// A title page: the title a third of the way down, then its lines
    /// (the author and the date, or a template's fields; an empty line is
    /// space). It has no page number in its footer.
    fn title_page(&mut self, page: &TitlePage) {
        self.y = self.top() + (self.bottom - self.top()) * 0.3;
        let bold = Look {
            bold: true,
            ..Look::default()
        };
        let title_size = self.base * if self.large { 1.8 } else { 2.4 };
        self.centred(&page.title, bold, title_size);
        self.y += self.base * 1.5;
        let mut first = true;
        for line in &page.lines {
            if line.is_empty() {
                self.y += self.base;
                continue;
            }
            // The first line (the author, usually) a little larger.
            let size = if first { self.base * 1.25 } else { self.base };
            first = false;
            self.centred(line, Look::default(), size);
            self.y += self.base * 0.5;
        }
        self.no_footer = 1;
        self.new_page();
    }

    /// A print page break: labelled as the source labels it, else one
    /// more than the last numbered print page. The PDF page it starts on
    /// is known when the next line of text is placed.
    fn print_page(&mut self, label: Option<&str>) {
        let label = label
            .map(model::collapse_ws)
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| {
                let last = self
                    .pending_print
                    .iter()
                    .chain(self.print_pages.iter().map(|(_, l, _)| l).rev())
                    .find_map(|l| l.parse::<u32>().ok());
                last.map_or(self.print_pages.len() + 1, |n| n as usize + 1)
                    .to_string()
            });
        if let Some(prev) = self.pending_print.replace(label) {
            // Two breaks with no text between: the first print page is
            // empty, and still starts here.
            let top = self.at_top();
            self.print_pages.push((self.pages.len() - 1, prev, top));
        }
    }

    /// The table of contents: a "Contents" heading, then one entry per
    /// heading down to `depth`, indented by level, each a link to its
    /// heading with the page number at the right (`TOC`, `TOCI`, `Link`).
    fn contents(&mut self, depth: u8) {
        let size = self.base * self.heading_scale(1);
        self.contents = Some((self.pages.len() - 1, self.y));
        let h = self
            .tree
            .group(ROOT, Tag::Hn(NonZeroU16::MIN, Some("Contents".to_owned())));
        let bold = Look {
            bold: true,
            ..Look::default()
        };
        let flow = self.flow(&[Inline::Text("Contents".to_owned())], bold);
        let lines = self.lines(&flow.words, self.width(), size);
        self.emit_lines(&lines, &[], h, 0.0, size, true);
        self.y += size * 0.5;
        let toc = self.tree.group(ROOT, Tag::TOC);
        let size = self.base;
        let lh = self.line_height(size);
        let number_w = self.measure("0000", Look::default(), size);
        let entries: Vec<(usize, u8, String)> = self
            .anchors
            .headings
            .iter()
            .enumerate()
            .filter(|(_, (level, _))| *level <= depth)
            .map(|(n, (level, title))| (n, *level, title.clone()))
            .collect();
        let min_level = entries.iter().map(|e| e.1).min().unwrap_or(1);
        for (n, level, title) in entries {
            let indent = f32::from(level - min_level) * size * 1.5;
            let title = if title.is_empty() {
                "Untitled heading".to_owned()
            } else {
                title
            };
            let flow = self.flow(&[Inline::Text(title.clone())], Look::default());
            let avail = self.width() - indent - number_w - size;
            let lines = self.lines(&flow.words, avail, size);
            self.ensure(lines.len() as f32 * lh);
            let item = self.tree.group(toc, Tag::TOCI);
            let link = self.tree.group(item, Tag::Link);
            let top = self.y;
            let start_page = self.pages.len() - 1;
            self.emit_lines(&lines, &[], link, indent, size, true);
            // The page number sits on the entry's last line.
            let y = self.y - lh;
            let baseline = {
                let saved = self.y;
                self.y = y;
                let b = self.baseline(size);
                self.y = saved;
                b
            };
            let slot = self.tree.slot(link);
            let face = self.fonts.family.regular;
            self.op(Op::PageRef {
                right: self.margin + self.width(),
                y: baseline,
                face,
                size,
                heading: n,
                slot,
            });
            // The link area: the entry's lines on its (last) page.
            let area_top = if self.pages.len() - 1 == start_page {
                top
            } else {
                self.top()
            };
            let slot = self.tree.slot(link);
            self.op(Op::Link {
                rect: (
                    self.margin + indent,
                    area_top,
                    self.margin + self.width(),
                    self.y,
                ),
                target: LinkTarget::Heading(n),
                alt: format!("Go to {title}"),
                slot,
            });
            self.y += size * 0.2;
        }
        self.new_page();
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
            Block::Footnote { id, content } => {
                let g = self.tree.group(parent, Tag::Note);
                let flow = self.flow_in(content, Look::default(), Some(id.clone()));
                let size = self.base;
                let lines = self.lines(&flow.words, self.width() - indent, size);
                if self.y + self.line_height(size) > self.bottom && !self.at_top() {
                    self.new_page();
                }
                self.notes
                    .entry(id.clone())
                    .or_insert((self.pages.len() - 1, self.y));
                self.emit_lines(&lines, &flow.links, g, indent, size, true);
                self.y += size * 0.4;
            }
            Block::SectionBreak { .. } => {
                if !self.at_top() {
                    self.new_page();
                }
            }
            Block::PageBreak { label } => self.print_page(label.as_deref()),
            // A horizontal rule: a line across the text, as layout.
            Block::Rule => {
                let size = self.base;
                self.ensure(self.line_height(size));
                let y = self.y + self.line_height(size) / 2.0;
                self.op(Op::Rule {
                    x0: self.margin + indent,
                    x1: self.margin + self.width(),
                    y,
                    width: 0.75,
                });
                self.y += self.line_height(size);
            }
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
        let look = Look {
            mono: true,
            ..Look::default()
        };
        let size = self.base * self.look_scale(look);
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
        // Parts carry the base size; emit_lines applies the mono scale.
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
        self.emit_lines(&lines, &[], g, indent + 12.0, self.base, true);
        self.y += self.paragraph_gap(self.base);
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
            // An image with no description is an artifact: screen readers
            // skip it. Say so, since it may carry meaning.
            self.undescribed += 1;
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
        self.y += h + self.paragraph_gap(self.base);
    }

    fn finish(mut self) -> Laid {
        if let Some(label) = self.pending_print.take() {
            let top = self.at_top();
            self.print_pages.push((self.pages.len() - 1, label, top));
        }
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
        match self.undescribed {
            0 => {}
            1 => self.report.warn(
                "An image has no description, so it was marked as decorative and screen readers will skip it. Give it alt text if it carries meaning.",
            ),
            n => self.report.warn(format!(
                "{n} images have no description, so they were marked as decorative and screen readers will skip them. Give them alt text if they carry meaning."
            )),
        }
        match self.dangling {
            0 => {}
            1 => self.report.warn(
                "A link points to a heading that is not in this document, so it was written as plain text.",
            ),
            n => self.report.warn(format!(
                "{n} links point to headings that are not in this document, so they were written as plain text."
            )),
        }
        Laid {
            pages: self.pages,
            tree: self.tree,
            headings: self.headings,
            notes: self.notes,
            contents: self.contents,
            no_footer: self.no_footer,
            print_pages: self.print_pages,
            page_w: self.page_w,
            page_h: self.page_h,
            margin: self.margin,
            base: self.base,
        }
    }
}

/// A PDF page's label.
#[derive(Clone, Debug, PartialEq, Eq)]
enum PrintLabel {
    /// Before the first print page: i, ii, iii.
    Front(u32),
    /// A print page's label ("42", "xii", "A-3").
    Print(String),
}

impl PrintLabel {
    fn krilla(&self) -> PageLabel {
        match self {
            PrintLabel::Front(n) => {
                PageLabel::new(Some(NumberingStyle::LowerRoman), None, NonZeroU32::new(*n))
            }
            PrintLabel::Print(p) => match p.parse::<u32>().ok().and_then(NonZeroU32::new) {
                // A plain number keeps its numeric form.
                Some(n) if !p.starts_with('0') => {
                    PageLabel::new(Some(NumberingStyle::Arabic), None, Some(n))
                }
                _ => PageLabel::new(None, Some(p.clone()), None),
            },
        }
    }
}

/// Labels for `total` PDF pages from where print pages start (PDF page,
/// label, whether it starts at the page's top): each page takes the print
/// page its first line belongs to, the last one started before that line.
/// Print pages that start and end in the middle of one PDF page give no
/// label (one page has one label). Pages before the first print page are
/// numbered i, ii, iii. Empty when the document has no print pages.
fn page_labels(print: &[(usize, String, bool)], total: usize) -> Vec<PrintLabel> {
    if print.is_empty() {
        return Vec::new();
    }
    let mut labels = Vec::with_capacity(total);
    let mut current: Option<&str> = None;
    let mut next = 0;
    for page in 0..total {
        // Breaks before this page's first line: earlier pages, or its top.
        while let Some((p, l, top)) = print.get(next)
            && (*p < page || (*p == page && *top))
        {
            current = Some(l.as_str());
            next += 1;
        }
        labels.push(match current {
            Some(l) => PrintLabel::Print(l.to_owned()),
            None => PrintLabel::Front(u32::try_from(page + 1).unwrap_or(u32::MAX)),
        });
    }
    labels
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

fn outline(laid: &Laid, title: &str) -> Outline {
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
    let margin = laid.margin;
    let mut outline = Outline::new();
    if let Some((page, y)) = laid.contents {
        outline.push_child(OutlineNode::new(
            "Contents".to_owned(),
            XyzDestination::new(page, Point::from_xy(margin, y)),
        ));
    }
    if laid.headings.is_empty() {
        outline.push_child(OutlineNode::new(
            title.to_owned(),
            XyzDestination::new(0, Point::from_xy(margin, margin)),
        ));
    } else {
        let mut i = 0;
        for node in build(&laid.headings, &mut i, 0, margin) {
            outline.push_child(node);
        }
    }
    outline
}

/// Where a link lands: a place in this document, or an action.
fn link_target(laid: &Laid, target: &LinkTarget) -> Option<Target> {
    let place = |page: usize, y: f32| {
        Target::Destination(Destination::Xyz(XyzDestination::new(
            page,
            Point::from_xy(laid.margin, y),
        )))
    };
    match target {
        LinkTarget::Uri(uri) => Some(Target::Action(Action::Link(LinkAction::new(uri.clone())))),
        LinkTarget::Heading(n) => laid.headings.get(*n).map(|h| place(h.2, h.3)),
        LinkTarget::Note(id) => laid.notes.get(id).map(|&(p, y)| place(p, y)),
        LinkTarget::Formula(_) => None,
    }
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
    let labels = page_labels(&laid.print_pages, total);
    let mut images: HashMap<*const Resource, KrillaImage> = HashMap::new();
    for (n, ops) in laid.pages.iter().enumerate() {
        let mut settings = PageSettings::new(size);
        if let Some(label) = labels.get(n) {
            settings = settings.with_page_label(label.krilla());
        }
        let mut page = document.start_page_with(settings);
        {
            let mut surface = page.surface();
            surface.set_fill(Some(Fill::default()));
            for op in ops {
                draw(&mut surface, op, fonts, &laid, &mut ids, &mut images);
            }
            // Page numbers are artifacts.
            if options.pdf.page_numbers && n >= laid.no_footer {
                let label = match labels.get(n) {
                    Some(PrintLabel::Print(p)) => {
                        format!("Page {} of {total}, print page {p}", n + 1)
                    }
                    _ => format!("Page {} of {total}", n + 1),
                };
                let fsize = (laid.base * 0.8).max(6.0);
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
            }
            surface.finish();
        }
        for op in ops {
            if let Op::Link {
                rect: (l, t, r, b),
                target,
                alt,
                slot,
            } = op
                && let Some(rect) = Rect::from_ltrb(*l, *t, *r, *b)
                && let Some(target) = link_target(&laid, target)
            {
                let link = LinkAnnotation::new(rect, target);
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
    document.set_outline(outline(&laid, &facts.title));
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
    laid: &Laid,
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
        Op::PageRef {
            right,
            y,
            face,
            size,
            heading,
            slot,
        } => {
            let Some(page) = laid.headings.get(*heading).map(|h| h.2) else {
                return;
            };
            // A leading space keeps the number a separate word when the
            // entry is read as text.
            let text = format!(" {}", page + 1);
            let w = fonts.faces[*face].width(&text, *size);
            let id = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
            surface.draw_text(
                Point::from_xy(right - w, *y),
                fonts.faces[*face].krilla.clone(),
                *size,
                &text,
                false,
                TextDirection::Auto,
            );
            surface.end_tagged();
            ids[*slot] = Some(id);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_labels_follow_print_pages() {
        let p = |s: &str| PrintLabel::Print(s.to_owned());
        assert!(page_labels(&[], 3).is_empty());
        // A title page, then print page 7 at the top of PDF page 1, going
        // on over page 2; 8 starts at the foot of page 2, so page 3 opens
        // in 8; 9 starts at the top of page 4 and 10 in its middle, so
        // page 4 is 9 and page 5 opens in 10.
        let print = [
            (1, "7".to_owned(), true),
            (2, "8".to_owned(), false),
            (4, "9".to_owned(), true),
            (4, "10".to_owned(), false),
        ];
        assert_eq!(
            page_labels(&print, 6),
            [
                PrintLabel::Front(1),
                p("7"),
                p("7"),
                p("8"),
                p("9"),
                p("10")
            ]
        );
        // Labels that are not numbers are kept as written.
        let print = [(0, "xii".to_owned(), true), (1, "A-3".to_owned(), true)];
        assert_eq!(page_labels(&print, 2), [p("xii"), p("A-3")]);
        assert_eq!(
            p("42").krilla(),
            PageLabel::new(Some(NumberingStyle::Arabic), None, NonZeroU32::new(42))
        );
        assert_eq!(
            p("042").krilla(),
            PageLabel::new(None, Some("042".to_owned()), None)
        );
    }

    #[test]
    fn slugs_follow_markdown_renderers() {
        assert_eq!(slug("Getting Started"), "getting-started");
        assert_eq!(slug("What's new in 2.0?"), "whats-new-in-20");
        assert_eq!(slug("  Über  Café "), "über--café");
        let blocks = vec![
            Block::Heading {
                level: 1,
                content: vec![Inline::Text("Intro".into())],
            },
            Block::Heading {
                level: 2,
                content: vec![Inline::Text("Intro".into())],
            },
            Block::Footnote {
                id: "1".into(),
                content: vec![Inline::Text("Note.".into())],
            },
        ];
        let a = Anchors::of(&blocks);
        assert_eq!(a.fragment("intro"), Some(0));
        assert_eq!(a.fragment("intro-1"), Some(1));
        assert_eq!(a.fragment("Intro"), Some(0));
        assert_eq!(a.fragment("missing"), None);
        assert!(a.notes.contains("1"));
    }

    use textweaver_formats::{LoadOptions, Loader, MarkdownLoader, PdfLoader, Source};

    /// A Markdown document.
    fn markdown(text: &str) -> Document {
        MarkdownLoader
            .load(
                &Source::Bytes {
                    data: text.as_bytes().to_vec(),
                    hint: "md".into(),
                },
                &LoadOptions::default(),
            )
            .expect("markdown loads")
    }

    /// Uncompressed PDF of `doc`, or `None` when no font is available.
    fn pdf(doc: &Document) -> Option<Vec<u8>> {
        let options = WriteOptions {
            timestamp: Some(1_790_339_696),
            pdf: crate::PdfOptions {
                compress: false,
                ..crate::PdfOptions::default()
            },
            ..WriteOptions::default()
        };
        let mut out = Vec::new();
        match PdfWriter.write(doc, &options, &mut out) {
            Ok(_) => Some(out),
            Err(WriteError::NoFont) => None,
            Err(e) => panic!("{e}"),
        }
    }

    /// Stroked paths (the `S` operator on a line of its own) in the
    /// uncompressed content streams.
    fn strokes(bytes: &[u8]) -> usize {
        String::from_utf8_lossy(bytes)
            .lines()
            .filter(|l| l.trim() == "S")
            .count()
    }

    /// The text of a PDF, read back with the PDF loader.
    fn read_back(bytes: Vec<u8>) -> String {
        PdfLoader
            .load(
                &Source::Bytes {
                    data: bytes,
                    hint: "pdf".into(),
                },
                &LoadOptions::default(),
            )
            .expect("the PDF loads")
            .text()
            .to_string()
    }

    #[test]
    fn strikethrough_draws_a_line_and_keeps_the_text() {
        let (Some(plain), Some(struck)) = (
            pdf(&markdown("Keep the old plan here.\n")),
            pdf(&markdown("Keep the ~~old plan~~ here.\n")),
        ) else {
            return;
        };
        // One more stroked line: the strikethrough, as a layout artifact.
        assert_eq!(strokes(&struck), strokes(&plain) + 1);
        let text = read_back(struck);
        assert!(text.contains("Keep the old plan here."), "{text:?}");
    }

    #[test]
    fn strikethrough_follows_line_wraps() {
        let words = "struck words wrap across lines ".repeat(30);
        let words = words.trim_end();
        let (Some(plain), Some(struck)) = (
            pdf(&markdown(&format!("Start {words} end.\n"))),
            pdf(&markdown(&format!("Start ~~{words}~~ end.\n"))),
        ) else {
            return;
        };
        // A line per wrapped line of struck text.
        let extra = strokes(&struck) - strokes(&plain);
        assert!(extra >= 3, "{extra} strike lines");
        let text = read_back(struck);
        assert!(text.contains("wrap across lines"), "{text:?}");
        assert!(text.contains("end."), "{text:?}");
    }

    #[test]
    fn strike_line_sits_above_the_baseline_and_skips_spaces() {
        let fonts = match Fonts::discover(&crate::PdfOptions::default()) {
            Ok(f) => f,
            Err(WriteError::NoFont) => return,
            Err(e) => panic!("{e}"),
        };
        let options = WriteOptions::default();
        let anchors = Anchors::default();
        let mut report = WriteReport::default();
        let doc = Document::from_plain_text("");
        let mut resources = Resources::new(&doc, &options);
        let mut layout = Layout::new(&fonts, &options, &anchors, &mut report, &mut resources);
        let look = Look {
            strike: true,
            ..Look::default()
        };
        let part = Part {
            text: "gone ".into(),
            look,
            link: None,
        };
        let space = layout.measure(" ", look, 12.0);
        layout.strike(&part, 100.0, 140.0, 500.0, 12.0);
        let blank = Part {
            text: "  ".into(),
            look,
            link: None,
        };
        layout.strike(&blank, 100.0, 110.0, 500.0, 12.0);
        let rules: Vec<_> = layout.pages[0]
            .iter()
            .filter_map(|op| match op {
                Op::Rule { x0, x1, y, width } => Some((*x0, *x1, *y, *width)),
                _ => None,
            })
            .collect();
        assert_eq!(rules.len(), 1);
        let (x0, x1, y, width) = rules[0];
        assert!((x0 - 100.0).abs() < 0.001);
        assert!((x1 - (140.0 - space)).abs() < 0.001);
        assert!((y - (500.0 - 12.0 * STRIKE_RISE)).abs() < 0.001);
        assert!((width - 12.0 * STRIKE_THICKNESS).abs() < 0.001);
    }
}
