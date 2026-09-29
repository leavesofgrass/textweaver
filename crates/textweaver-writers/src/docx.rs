//! DOCX (Office Open XML WordprocessingML) with real structure.
//!
//! - Headings use the built-in `heading 1` to `heading 6` styles with outline
//!   levels, so Word's navigation pane, screen readers' heading navigation,
//!   and PDF export all see them.
//! - Lists use real numbering (`numbering.xml`): bullets or decimal numbers,
//!   one numbering instance per list so each numbered list restarts, nesting
//!   as list levels.
//! - Tables use the `Table Grid` style; the header row repeats on every page
//!   and is marked as a header row (`w:tblHeader`), and a caption is both a
//!   `Caption` paragraph and the table's alternative text title.
//! - Images found on disk are embedded inline with their alt text as the
//!   picture description (`wp:docPr/@descr`); images that cannot be found
//!   are written as their alt text.
//! - Links are hyperlinks with the `Hyperlink` style.
//! - Footnotes are real Word footnotes (`footnotes.xml`, ADR-0041): Word
//!   numbers them, shows them at the foot of the page, and screen readers
//!   announce the reference as a footnote. A footnote referenced twice is
//!   one footnote with a `NOTEREF` cross-reference at the second place; a
//!   footnote nothing references stays where the document has it. With
//!   [`DocxOptions::word_footnotes`](crate::DocxOptions) off, every
//!   footnote stays in place and its reference links to it.
//! - A publishing template ([`Template`]) sets the styles (font, size,
//!   spacing, heading looks, colors), a title page, page numbers in the
//!   header, and hanging indents for references, always through the same
//!   named styles, so the structure screen readers use never changes.
//! - The document language is the default run language, and the title,
//!   author, and language are in the core properties.

use crate::math::Formula;
use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::rc::Rc;

use textweaver_text::Document;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

use crate::model::{self, Block, Facts, Image, Inline, List, Style, Table};
use crate::resource::{ImageKind, Resource, Resources};
use crate::template::{self, DocxLook, TitlePage};
use crate::{
    Format, Template, WriteError, WriteOptions, WriteReport, Writer, iso8601, timestamp, xml,
};

/// Writes DOCX.
#[derive(Clone, Copy, Debug, Default)]
pub struct DocxWriter;

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
/// Namespaces for parts that hold paragraphs (document, footnotes).
const PART_NS: &str = "xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\" xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"";
const REL_HYPERLINK: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink";
const REL_IMAGE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
const REL_FOOTNOTES: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/footnotes";
const REL_HEADER: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/header";

/// Twentieths of a point per inch.
const TWIPS_PER_INCH: u32 = 1440;
/// English Metric Units per pixel at 96 dpi.
const EMU_PER_PX: u64 = 9525;

impl Writer for DocxWriter {
    fn format(&self) -> Format {
        Format::Docx
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
        let (page_w, page_h) = options.pdf.page_size.points();
        let page_w = (page_w * 20.0).round() as u32;
        let page_h = (page_h * 20.0).round() as u32;
        let text_width = page_w
            .saturating_sub(2 * TWIPS_PER_INCH)
            .max(TWIPS_PER_INCH);
        let look = match options.template {
            Some(t) => t.docx(),
            None => DocxLook::of(None),
        };
        let bodies = collect_notes(&blocks);
        let referenced: Vec<String> = if options.docx.word_footnotes {
            model::footnote_refs(&blocks)
                .into_iter()
                .filter(|id| bodies.iter().any(|(b, _)| b == id))
                .collect()
        } else {
            Vec::new()
        };
        let paper = matches!(
            options.template,
            Some(Template::ApaStudentPaper | Template::AmaManuscript | Template::Manuscript)
        );
        let shift = if paper && title_is_only_top_heading(&blocks, &facts.title) {
            1
        } else {
            0
        };

        let mut body = Body {
            out: String::new(),
            rels: Vec::new(),
            links: HashMap::new(),
            nums: Vec::new(),
            resources: Resources::new(doc, options),
            images: Vec::new(),
            image_rels: HashMap::new(),
            report: &mut report,
            bookmarks: 0,
            notes: bodies.iter().map(|(id, _)| id.clone()).collect(),
            notes_marked: Vec::new(),
            drawings: 0,
            text_width,
            look: &look,
            word_notes: referenced,
            bodies,
            note_numbers: Vec::new(),
            notes_xml: String::new(),
            in_notes: false,
            break_before: false,
            references: None,
        };
        if let Some(t) = options.template
            && let Some(page) = template::title_page(t, doc, options, &facts, &blocks)
        {
            body.title_page(&page);
        }
        body.blocks(&blocks, &Ctx::default());
        let has_notes = !body.note_numbers.is_empty();
        let header = look.page_numbers.then(|| header_part(&look, &facts));
        if has_notes {
            body.rels.push(Rel {
                id: "rIdFootnotes".to_owned(),
                kind: REL_FOOTNOTES,
                target: "footnotes.xml".to_owned(),
                external: false,
                notes: false,
            });
        }
        let header_ref = if header.is_some() {
            body.rels.push(Rel {
                id: "rIdHeader1".to_owned(),
                kind: REL_HEADER,
                target: "header1.xml".to_owned(),
                external: false,
                notes: false,
            });
            "<w:headerReference w:type=\"default\" r:id=\"rIdHeader1\"/>"
        } else {
            ""
        };
        body.out.push_str(&format!(
            "<w:sectPr>{header_ref}<w:pgSz w:w=\"{page_w}\" w:h=\"{page_h}\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\" w:header=\"720\" w:footer=\"720\" w:gutter=\"0\"/></w:sectPr>"
        ));
        let background = look
            .background
            .map(|c| format!("<w:background w:color=\"{c}\"/>"))
            .unwrap_or_default();
        let document = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:document xmlns:w=\"{W}\" xmlns:r=\"{R}\" {PART_NS}>{background}<w:body>{}</w:body></w:document>\n",
            body.out
        );
        let footnotes = has_notes.then(|| {
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:footnotes xmlns:w=\"{W}\" xmlns:r=\"{R}\" {PART_NS}><w:footnote w:type=\"separator\" w:id=\"-1\"><w:p><w:pPr><w:spacing w:after=\"0\" w:line=\"240\" w:lineRule=\"auto\"/></w:pPr><w:r><w:separator/></w:r></w:p></w:footnote><w:footnote w:type=\"continuationSeparator\" w:id=\"0\"><w:p><w:pPr><w:spacing w:after=\"0\" w:line=\"240\" w:lineRule=\"auto\"/></w:pPr><w:r><w:continuationSeparator/></w:r></w:p></w:footnote>{}</w:footnotes>\n",
                body.notes_xml
            )
        });
        let numbering = numbering_part(&body.nums);
        let styles = styles_part(&facts.language, &look, shift);
        let doc_rels = rels_part(&body.rels, false);
        let notes_rels = body
            .rels
            .iter()
            .any(|r| r.notes)
            .then(|| rels_part(&body.rels, true));
        let content_types = content_types(&body.images, has_notes, header.is_some());
        let settings = settings_part(has_notes, look.background.is_some());
        let modified = iso8601(timestamp(options));
        let core = core_part(&facts, &modified);
        let images = std::mem::take(&mut body.images);
        drop(body);

        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let zerr = |e: zip::result::ZipError| WriteError::Zip(e.to_string());
        let mut parts: Vec<(&str, &str)> = vec![
            ("[Content_Types].xml", &content_types),
            ("_rels/.rels", PACKAGE_RELS),
            ("docProps/core.xml", &core),
            ("docProps/app.xml", APP),
            ("word/document.xml", &document),
            ("word/styles.xml", &styles),
            ("word/numbering.xml", &numbering),
            ("word/settings.xml", &settings),
            ("word/_rels/document.xml.rels", &doc_rels),
        ];
        if let Some(f) = &footnotes {
            parts.push(("word/footnotes.xml", f));
        }
        if let Some(r) = &notes_rels {
            parts.push(("word/_rels/footnotes.xml.rels", r));
        }
        if let Some(h) = &header {
            parts.push(("word/header1.xml", h));
        }
        for (name, data) in parts {
            zip.start_file(name, deflated).map_err(zerr)?;
            zip.write_all(data.as_bytes())?;
        }
        for (name, res) in &images {
            zip.start_file(format!("word/{name}"), stored)
                .map_err(zerr)?;
            zip.write_all(&res.bytes)?;
        }
        let bytes = zip.finish().map_err(zerr)?.into_inner();
        out.write_all(&bytes)?;
        Ok(report)
    }
}

/// True when the document's only level 1 heading is its first block and
/// says its title: a paper whose sections start at level 2, so the
/// template's first heading look goes to level 2.
fn title_is_only_top_heading(blocks: &[Block], title: &str) -> bool {
    let first = blocks
        .iter()
        .find(|b| !matches!(b, Block::PageBreak { .. } | Block::SectionBreak { .. }));
    let Some(Block::Heading { level: 1, content }) = first else {
        return false;
    };
    let tops = blocks
        .iter()
        .filter(|b| matches!(b, Block::Heading { level: 1, .. }))
        .count();
    tops == 1 && model::collapse_ws(&Inline::plain(content)) == title
}

/// Footnote bodies in the document, by id, in order.
fn collect_notes(blocks: &[Block]) -> Vec<(String, Vec<Inline>)> {
    let mut v = Vec::new();
    fn walk(blocks: &[Block], v: &mut Vec<(String, Vec<Inline>)>) {
        for b in blocks {
            match b {
                Block::Footnote { id, content } => {
                    if !v.iter().any(|(i, _)| i == id) {
                        v.push((id.clone(), content.clone()));
                    }
                }
                Block::Quote(inner) => walk(inner, v),
                Block::List(l) => {
                    for i in &l.items {
                        walk(&i.blocks, v);
                    }
                }
                _ => {}
            }
        }
    }
    walk(blocks, &mut v);
    v
}

/// A footnote body without the label loaders put at its start (a
/// reference to itself, such as "[1]"), since Word shows its own number.
fn without_own_label(content: &[Inline], id: &str) -> Vec<Inline> {
    let mut rest = content;
    if let Some(Inline::Span(Style::FootnoteRef(r), _)) = rest.first()
        && r == id
    {
        rest = &rest[1..];
    } else if let Some(Inline::Text(t)) = rest.first()
        && t.trim_start().starts_with(&format!("[{id}]"))
    {
        let t = t.trim_start()[id.len() + 2..].trim_start().to_owned();
        let mut v = vec![Inline::Text(t)];
        v.extend_from_slice(&rest[1..]);
        return v;
    }
    let mut v = rest.to_vec();
    if let Some(Inline::Text(t)) = v.first_mut() {
        *t = t.trim_start().to_owned();
    }
    v
}

/// Bookmark name for a footnote body (at most 40 chars, starting with a
/// letter).
fn note_bookmark(id: &str) -> String {
    xml::id("fn_", id).chars().take(40).collect()
}

/// A numbering instance: bullets or numbers starting at `start`, first used
/// at list level `level`.
struct Num {
    ordered: bool,
    start: u64,
    level: usize,
}

/// A relationship of the document part or of the footnotes part.
struct Rel {
    id: String,
    kind: &'static str,
    target: String,
    external: bool,
    /// Declared by `footnotes.xml` rather than `document.xml`.
    notes: bool,
}

/// Paragraph context while rendering.
#[derive(Clone, Debug, Default)]
struct Ctx {
    /// Inside a block quote.
    quote: bool,
    /// List nesting depth (0 outside lists).
    depth: usize,
    /// Inside the body of this footnote.
    note: Option<String>,
}

/// Run properties.
#[derive(Clone, Copy, Debug, Default)]
struct Props {
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
    code: bool,
    link: bool,
}

struct Body<'a> {
    out: String,
    rels: Vec<Rel>,
    /// (in footnotes, target) → relationship id.
    links: HashMap<(bool, String), String>,
    nums: Vec<Num>,
    resources: Resources,
    /// (path under word/, image).
    images: Vec<(String, Rc<Resource>)>,
    /// (in footnotes, source) → (relationship id, image).
    image_rels: HashMap<(bool, String), (String, Rc<Resource>)>,
    report: &'a mut WriteReport,
    bookmarks: usize,
    /// Every footnote id with a body.
    notes: Vec<String>,
    notes_marked: Vec<String>,
    drawings: usize,
    text_width: u32,
    look: &'a DocxLook,
    /// Footnotes written as Word footnotes: referenced ones with a body.
    word_notes: Vec<String>,
    /// Footnote bodies by id.
    bodies: Vec<(String, Vec<Inline>)>,
    /// Word footnotes written so far, by id; the footnote's number is its
    /// position plus one.
    note_numbers: Vec<String>,
    /// `w:footnote` elements.
    notes_xml: String,
    /// Rendering a footnote body (relationships go to the footnotes part).
    in_notes: bool,
    /// The next paragraph starts a new page (after a title page).
    break_before: bool,
    /// Inside a reference list under a heading of this level.
    references: Option<u8>,
}

impl Body<'_> {
    fn rel(&mut self, kind: &'static str, target: String, external: bool) -> String {
        // rId1 to rId3 are styles, numbering, and settings.
        let id = format!("rId{}", self.rels.len() + 4);
        self.rels.push(Rel {
            id: id.clone(),
            kind,
            target,
            external,
            notes: self.in_notes,
        });
        id
    }

    fn blocks(&mut self, blocks: &[Block], ctx: &Ctx) {
        for (i, b) in blocks.iter().enumerate() {
            if self.heads_only_word_notes(blocks, i) {
                continue;
            }
            self.block(b, ctx);
        }
    }

    /// True for a heading (a loader's "Footnotes") whose section holds
    /// only footnotes that became Word footnotes: it would head an empty
    /// section, so it is left out.
    fn heads_only_word_notes(&self, blocks: &[Block], i: usize) -> bool {
        let Some(Block::Heading { level, .. }) = blocks.get(i) else {
            return false;
        };
        let section: Vec<&Block> = blocks[i + 1..]
            .iter()
            .take_while(|b| !matches!(b, Block::Heading { level: l, .. } if l <= level))
            .filter(|b| !matches!(b, Block::PageBreak { .. }))
            .collect();
        !section.is_empty()
            && section
                .iter()
                .all(|b| matches!(b, Block::Footnote { id, .. } if self.word_notes.contains(id)))
    }

    /// Opens a paragraph; `extra` holds the paragraph properties after the
    /// style, in schema order (`keepNext` first when present).
    fn para_open(&mut self, style: Option<&str>, extra: &str) {
        self.out.push_str("<w:p>");
        let brk = std::mem::take(&mut self.break_before);
        if style.is_some() || !extra.is_empty() || brk {
            self.out.push_str("<w:pPr>");
            if let Some(s) = style {
                self.out.push_str(&format!("<w:pStyle w:val=\"{s}\"/>"));
            }
            if brk {
                // keepNext comes before pageBreakBefore in the schema.
                let rest = match extra.strip_prefix("<w:keepNext/>") {
                    Some(rest) => {
                        self.out.push_str("<w:keepNext/>");
                        rest
                    }
                    None => extra,
                };
                self.out.push_str("<w:pageBreakBefore/>");
                self.out.push_str(rest);
            } else {
                self.out.push_str(extra);
            }
            self.out.push_str("</w:pPr>");
        }
    }

    /// Style and indentation of a body paragraph in `ctx`.
    fn body_style(&self, ctx: &Ctx) -> (Option<&'static str>, String) {
        if ctx.quote {
            (Some("Quote"), String::new())
        } else if ctx.depth > 0 {
            (
                Some("ListParagraph"),
                format!("<w:ind w:left=\"{}\"/>", 720 * ctx.depth),
            )
        } else if self.references.is_some() && self.look.hanging_references {
            (Some("Bibliography"), String::new())
        } else if self.look.first_line > 0 && ctx.note.is_none() {
            (Some("BodyText"), String::new())
        } else {
            (None, String::new())
        }
    }

    /// The template's title page, then a page break before the text.
    fn title_page(&mut self, page: &TitlePage) {
        self.para_open(Some("Title"), "");
        self.run(&page.title, Props::default());
        self.out.push_str("</w:p>");
        for line in &page.lines {
            self.para_open(None, "<w:jc w:val=\"center\"/>");
            self.run(line, Props::default());
            self.out.push_str("</w:p>");
        }
        self.break_before = true;
    }

    fn block(&mut self, b: &Block, ctx: &Ctx) {
        match b {
            Block::Heading { level, content } => {
                if self.references.is_some_and(|l| *level <= l) {
                    self.references = None;
                }
                if model::is_references_heading(&Inline::plain(content)) {
                    self.references = Some(*level);
                }
                let style = format!("Heading{}", (*level).clamp(1, 6));
                self.para_open(Some(&style), "");
                self.inlines(content, Props::default(), ctx);
                self.out.push_str("</w:p>");
            }
            Block::Paragraph(content) => {
                let (style, extra) = self.body_style(ctx);
                self.para_open(style, &extra);
                self.inlines(content, Props::default(), ctx);
                self.out.push_str("</w:p>");
            }
            Block::List(list) => self.list(list, ctx),
            Block::Table(table) => self.table(table, ctx),
            Block::Code { text, .. } => {
                self.para_open(Some("SourceCode"), "");
                for (n, line) in text.split('\n').enumerate() {
                    if n > 0 {
                        self.out.push_str("<w:r><w:br/></w:r>");
                    }
                    self.run(line, Props::default());
                }
                self.out.push_str("</w:p>");
            }
            Block::Quote(inner) => {
                let inner_ctx = Ctx {
                    quote: true,
                    ..ctx.clone()
                };
                self.blocks(inner, &inner_ctx);
            }
            Block::Figure(img) => {
                let (style, extra) = self.body_style(ctx);
                self.para_open(style, &extra);
                if !self.drawing(img) && !img.alt.is_empty() {
                    self.run(&img.alt, Props::default());
                }
                self.out.push_str("</w:p>");
            }
            // Written as a Word footnote at its first reference.
            Block::Footnote { id, .. } if self.word_notes.contains(id) => {}
            Block::Footnote { id, content } => {
                let (style, extra) = self.body_style(&Ctx {
                    note: Some(id.clone()),
                    ..ctx.clone()
                });
                self.para_open(style, &extra);
                let mark = !self.notes_marked.contains(id);
                let bm = self.bookmarks;
                if mark {
                    self.notes_marked.push(id.clone());
                    self.bookmarks += 1;
                    self.out.push_str(&format!(
                        "<w:bookmarkStart w:id=\"{bm}\" w:name=\"{}\"/>",
                        note_bookmark(id)
                    ));
                }
                let note_ctx = Ctx {
                    note: Some(id.clone()),
                    ..ctx.clone()
                };
                self.inlines(content, Props::default(), &note_ctx);
                if mark {
                    self.out
                        .push_str(&format!("<w:bookmarkEnd w:id=\"{bm}\"/>"));
                }
                self.out.push_str("</w:p>");
            }
            Block::SectionBreak { .. } => {
                // A chapter starts on a new page.
                self.break_before = false;
                self.out
                    .push_str("<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>");
            }
            // Print page numbers have no place in a reflowing document.
            Block::PageBreak { .. } => {}
            // A manuscript's scene break: a centered number sign.
            Block::Rule if self.look.scene_breaks => {
                self.para_open(None, "<w:jc w:val=\"center\"/>");
                self.run("#", Props::default());
                self.out.push_str("</w:p>");
            }
            // A horizontal rule: an empty paragraph with a bottom border.
            Block::Rule => {
                self.para_open(
                    None,
                    "<w:pBdr><w:bottom w:val=\"single\" w:sz=\"6\" w:space=\"1\" w:color=\"auto\"/></w:pBdr>",
                );
                self.out.push_str("</w:p>");
            }
        }
    }

    fn list(&mut self, list: &List, ctx: &Ctx) {
        let level = ctx.depth.min(8);
        let num_id = self.nums.len() + 1;
        self.nums.push(Num {
            ordered: list.ordered,
            start: list.start,
            level,
        });
        let inner = Ctx {
            depth: ctx.depth + 1,
            ..ctx.clone()
        };
        for item in &list.items {
            let mut rest: &[Block] = &item.blocks;
            let numpr = format!(
                "<w:numPr><w:ilvl w:val=\"{level}\"/><w:numId w:val=\"{num_id}\"/></w:numPr>"
            );
            self.para_open(Some("ListParagraph"), &numpr);
            if let Some(Block::Paragraph(content)) = rest.first() {
                self.inlines(content, Props::default(), &inner);
                rest = &rest[1..];
            }
            self.out.push_str("</w:p>");
            self.blocks(rest, &inner);
        }
    }

    fn table(&mut self, table: &Table, ctx: &Ctx) {
        let cols = table.columns().max(1);
        if let Some(c) = &table.caption {
            self.para_open(Some("Caption"), "<w:keepNext/>");
            self.run(c, Props::default());
            self.out.push_str("</w:p>");
        } else if self.break_before {
            // A table cannot carry the page break; an empty paragraph does.
            self.para_open(None, "<w:spacing w:after=\"0\"/>");
            self.out.push_str("</w:p>");
        }
        let indent = if ctx.depth > 0 {
            format!("<w:tblInd w:w=\"{}\" w:type=\"dxa\"/>", 720 * ctx.depth)
        } else {
            String::new()
        };
        let width = self
            .text_width
            .saturating_sub(720 * ctx.depth as u32)
            .max(TWIPS_PER_INCH);
        let col_w = width / cols as u32;
        let caption = table
            .caption
            .as_deref()
            .map(|c| format!("<w:tblCaption w:val=\"{}\"/>", xml::attr(c)))
            .unwrap_or_default();
        self.out.push_str(&format!(
            "<w:tbl><w:tblPr><w:tblStyle w:val=\"TableGrid\"/><w:tblW w:w=\"{width}\" w:type=\"dxa\"/>{indent}<w:tblLook w:val=\"04A0\" w:firstRow=\"1\" w:lastRow=\"0\" w:firstColumn=\"0\" w:lastColumn=\"0\" w:noHBand=\"0\" w:noVBand=\"1\"/>{caption}</w:tblPr><w:tblGrid>"
        ));
        for _ in 0..cols {
            self.out.push_str(&format!("<w:gridCol w:w=\"{col_w}\"/>"));
        }
        self.out.push_str("</w:tblGrid>");
        let cell_ctx = Ctx {
            depth: 0,
            quote: false,
            ..ctx.clone()
        };
        let empty: Vec<Inline> = Vec::new();
        for (r, row) in table.rows.iter().enumerate() {
            self.out.push_str("<w:tr>");
            // Leading header rows repeat on each page.
            let leading_header = row.header && table.rows[..r].iter().all(|x| x.header);
            if leading_header {
                self.out.push_str("<w:trPr><w:tblHeader/></w:trPr>");
            }
            for c in 0..cols {
                let cell = row.cells.get(c).unwrap_or(&empty);
                self.out.push_str(&format!(
                    "<w:tc><w:tcPr><w:tcW w:w=\"{col_w}\" w:type=\"dxa\"/></w:tcPr><w:p><w:pPr><w:spacing w:after=\"0\"/></w:pPr>"
                ));
                let props = Props {
                    bold: row.header,
                    ..Props::default()
                };
                self.inlines(cell, props, &cell_ctx);
                self.out.push_str("</w:p></w:tc>");
            }
            self.out.push_str("</w:tr>");
        }
        self.out.push_str("</w:tbl>");
        // Word needs a paragraph between adjacent tables.
        self.out
            .push_str("<w:p><w:pPr><w:spacing w:after=\"0\"/></w:pPr></w:p>");
    }

    /// A run of text; tabs become tab characters.
    fn run(&mut self, text: &str, p: Props) {
        if text.is_empty() {
            return;
        }
        let mut rpr = String::new();
        if p.link {
            rpr.push_str("<w:rStyle w:val=\"Hyperlink\"/>");
        } else if p.code {
            rpr.push_str("<w:rStyle w:val=\"SourceCodeChar\"/>");
        }
        // Templates that avoid italic and plain underline (large print,
        // dyslexia-friendly) show that emphasis in bold.
        let plain_underline = p.underline && !p.link;
        let as_bold = self.look.no_italic && (p.italic || plain_underline);
        if p.bold || as_bold {
            rpr.push_str("<w:b/><w:bCs/>");
        }
        if p.italic && !self.look.no_italic {
            rpr.push_str("<w:i/><w:iCs/>");
        }
        if p.strike {
            rpr.push_str("<w:strike/>");
        }
        if plain_underline && !self.look.no_italic {
            rpr.push_str("<w:u w:val=\"single\"/>");
        }
        self.out.push_str("<w:r>");
        if !rpr.is_empty() {
            self.out.push_str(&format!("<w:rPr>{rpr}</w:rPr>"));
        }
        for (n, part) in text.split('\t').enumerate() {
            if n > 0 {
                self.out.push_str("<w:tab/>");
            }
            if !part.is_empty() {
                self.out.push_str(&format!(
                    "<w:t xml:space=\"preserve\">{}</w:t>",
                    xml::text(part)
                ));
            }
        }
        self.out.push_str("</w:r>");
    }

    fn inlines(&mut self, inlines: &[Inline], p: Props, ctx: &Ctx) {
        for i in inlines {
            match i {
                Inline::Text(t) => self.run(t, p),
                Inline::LineBreak => self.out.push_str("<w:r><w:br/></w:r>"),
                Inline::Span(style, children) => self.span(style, children, p, ctx),
            }
        }
    }

    /// A reference to a Word footnote: the footnote itself at the first
    /// reference, a `NOTEREF` cross-reference to it after that.
    fn word_footnote(&mut self, id: &str) {
        if let Some(n) = self.note_numbers.iter().position(|x| x == id) {
            let number = n + 1;
            self.out.push_str(&format!(
                "<w:r><w:fldChar w:fldCharType=\"begin\"/></w:r><w:r><w:instrText xml:space=\"preserve\"> NOTEREF _RefNote{number} \\f \\h </w:instrText></w:r><w:r><w:fldChar w:fldCharType=\"separate\"/></w:r><w:r><w:rPr><w:rStyle w:val=\"FootnoteReference\"/></w:rPr><w:t>{number}</w:t></w:r><w:r><w:fldChar w:fldCharType=\"end\"/></w:r>"
            ));
            return;
        }
        self.note_numbers.push(id.to_owned());
        let number = self.note_numbers.len();
        let bm = self.bookmarks;
        self.bookmarks += 1;
        self.out.push_str(&format!(
            "<w:bookmarkStart w:id=\"{bm}\" w:name=\"_RefNote{number}\"/><w:r><w:rPr><w:rStyle w:val=\"FootnoteReference\"/></w:rPr><w:footnoteReference w:id=\"{number}\"/></w:r><w:bookmarkEnd w:id=\"{bm}\"/>"
        ));
        // The body, rendered into footnotes.xml.
        let content = self
            .bodies
            .iter()
            .find(|(i, _)| i == id)
            .map(|(_, c)| without_own_label(c, id))
            .unwrap_or_default();
        let saved = std::mem::take(&mut self.out);
        let was_in_notes = std::mem::replace(&mut self.in_notes, true);
        self.out.push_str(&format!(
            "<w:footnote w:id=\"{number}\"><w:p><w:pPr><w:pStyle w:val=\"FootnoteText\"/></w:pPr><w:r><w:rPr><w:rStyle w:val=\"FootnoteReference\"/></w:rPr><w:footnoteRef/></w:r><w:r><w:t xml:space=\"preserve\"> </w:t></w:r>"
        ));
        let note_ctx = Ctx {
            note: Some(id.to_owned()),
            ..Ctx::default()
        };
        self.inlines(&content, Props::default(), &note_ctx);
        self.out.push_str("</w:p></w:footnote>");
        let note = std::mem::replace(&mut self.out, saved);
        self.in_notes = was_in_notes;
        self.notes_xml.push_str(&note);
    }

    fn span(&mut self, style: &Style, children: &[Inline], p: Props, ctx: &Ctx) {
        match style {
            Style::Bold => self.inlines(children, Props { bold: true, ..p }, ctx),
            Style::Italic => self.inlines(children, Props { italic: true, ..p }, ctx),
            Style::Underline => self.inlines(
                children,
                Props {
                    underline: true,
                    ..p
                },
                ctx,
            ),
            Style::Code => self.inlines(children, Props { code: true, ..p }, ctx),
            Style::Strikethrough => self.inlines(children, Props { strike: true, ..p }, ctx),
            // Office Math, which Word draws and reads aloud.
            Style::Math { display } => {
                let f = Formula::from_marked(&Inline::plain(children), *display);
                self.out.push_str(&f.omml());
            }
            Style::Link(target) if !p.link && is_external(target) => {
                let target = target.trim().to_owned();
                let key = (self.in_notes, target.clone());
                let id = match self.links.get(&key) {
                    Some(id) => id.clone(),
                    None => {
                        let id = self.rel(REL_HYPERLINK, target, true);
                        self.links.insert(key, id.clone());
                        id
                    }
                };
                self.out
                    .push_str(&format!("<w:hyperlink r:id=\"{id}\" w:history=\"1\">"));
                self.inlines(children, Props { link: true, ..p }, ctx);
                self.out.push_str("</w:hyperlink>");
            }
            // A Word footnote; never inside a footnote or a link.
            Style::FootnoteRef(id)
                if !p.link && ctx.note.is_none() && self.word_notes.contains(id) =>
            {
                self.word_footnote(id);
            }
            Style::FootnoteRef(id)
                if !p.link
                    && self.notes.contains(id)
                    && !self.word_notes.contains(id)
                    && ctx.note.as_deref() != Some(id.as_str()) =>
            {
                self.out.push_str(&format!(
                    "<w:hyperlink w:anchor=\"{}\" w:history=\"1\">",
                    note_bookmark(id)
                ));
                self.inlines(children, Props { link: true, ..p }, ctx);
                self.out.push_str("</w:hyperlink>");
            }
            Style::Link(_) | Style::FootnoteRef(_) => self.inlines(children, p, ctx),
            Style::Image(src) => {
                let img = Image {
                    alt: model::collapse_ws(&Inline::plain(children)),
                    src: src.clone(),
                };
                if !self.drawing(&img) && !img.alt.is_empty() {
                    self.run(&img.alt, p);
                }
            }
        }
    }

    /// An inline picture; false when the image cannot be embedded.
    fn drawing(&mut self, img: &Image) -> bool {
        let Some(src) = img.src.as_deref() else {
            return false;
        };
        let key = (self.in_notes, src.to_owned());
        let (rid, res) = match self.image_rels.get(&key) {
            Some((rid, res)) => (rid.clone(), res.clone()),
            None => {
                let Some(res) = self.resources.get(src, self.report) else {
                    return false;
                };
                if !matches!(res.kind, ImageKind::Png | ImageKind::Jpeg | ImageKind::Gif)
                    || res.size.is_none()
                {
                    self.report.warn(format!(
                        "An image in {} format cannot go into a Word document, so its description was written instead.",
                        res.kind.extension().to_uppercase()
                    ));
                    return false;
                }
                // One media file per image, shared by both parts.
                let name = match self.images.iter().find(|(_, r)| Rc::ptr_eq(r, &res)) {
                    Some((name, _)) => name.clone(),
                    None => {
                        let name = format!(
                            "media/image{}.{}",
                            self.images.len() + 1,
                            res.kind.extension()
                        );
                        self.images.push((name.clone(), res.clone()));
                        name
                    }
                };
                let rid = self.rel(REL_IMAGE, name, false);
                self.image_rels.insert(key, (rid.clone(), res.clone()));
                (rid, res)
            }
        };
        let Some((w, h)) = res.size else {
            return false;
        };
        // Pixels at 96 dpi, scaled down to the text width.
        let max_cx = u64::from(self.text_width) * 635;
        let mut cx = u64::from(w) * EMU_PER_PX;
        let mut cy = u64::from(h) * EMU_PER_PX;
        if cx > max_cx {
            cy = cy * max_cx / cx;
            cx = max_cx;
        }
        self.drawings += 1;
        let n = self.drawings;
        let alt = xml::attr(&img.alt);
        self.out.push_str(&format!(
            "<w:r><w:drawing><wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\"><wp:extent cx=\"{cx}\" cy=\"{cy}\"/><wp:effectExtent l=\"0\" t=\"0\" r=\"0\" b=\"0\"/><wp:docPr id=\"{n}\" name=\"Picture {n}\" descr=\"{alt}\"/><wp:cNvGraphicFramePr><a:graphicFrameLocks noChangeAspect=\"1\"/></wp:cNvGraphicFramePr><a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><pic:pic><pic:nvPicPr><pic:cNvPr id=\"{n}\" name=\"Picture {n}\" descr=\"{alt}\"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed=\"{rid}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r>"
        ));
        true
    }
}

fn is_external(target: &str) -> bool {
    let t = target.trim().to_ascii_lowercase();
    ["http://", "https://", "mailto:", "ftp://", "tel:"]
        .iter()
        .any(|p| t.starts_with(p))
}

const PACKAGE_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/></Relationships>
"#;

const APP: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>textweaver</Application></Properties>
"#;

/// `settings.xml`: the page color shown on screen when there is one, the
/// footnote separators when there are footnotes, and Word 2013 layout.
fn settings_part(footnotes: bool, background: bool) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:settings xmlns:w=\"{W}\">{}<w:defaultTabStop w:val=\"720\"/><w:characterSpacingControl w:val=\"doNotCompress\"/>{}<w:compat><w:compatSetting w:name=\"compatibilityMode\" w:uri=\"http://schemas.microsoft.com/office/word\" w:val=\"15\"/></w:compat></w:settings>\n",
        if background {
            "<w:displayBackgroundShape/>"
        } else {
            ""
        },
        if footnotes {
            "<w:footnotePr><w:footnote w:id=\"-1\"/><w:footnote w:id=\"0\"/></w:footnotePr>"
        } else {
            ""
        }
    )
}

/// The page header: the page number at the right, after a running head
/// ("Surname / Short title / ") for manuscripts.
fn header_part(look: &DocxLook, facts: &Facts) -> String {
    let running = if look.running_head {
        let surname = facts
            .author
            .as_deref()
            .and_then(|a| a.split_whitespace().last())
            .map(str::to_owned);
        let short: Vec<&str> = facts.title.split_whitespace().take(4).collect();
        let mut parts: Vec<String> = surname.into_iter().collect();
        parts.push(short.join(" "));
        format!(
            "<w:r><w:t xml:space=\"preserve\">{} / </w:t></w:r>",
            xml::text(&parts.join(" / "))
        )
    } else {
        String::new()
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:hdr xmlns:w=\"{W}\" xmlns:r=\"{R}\"><w:p><w:pPr><w:pStyle w:val=\"Header\"/><w:jc w:val=\"right\"/></w:pPr>{running}<w:r><w:fldChar w:fldCharType=\"begin\"/></w:r><w:r><w:instrText xml:space=\"preserve\"> PAGE </w:instrText></w:r><w:r><w:fldChar w:fldCharType=\"separate\"/></w:r><w:r><w:t>1</w:t></w:r><w:r><w:fldChar w:fldCharType=\"end\"/></w:r></w:p></w:hdr>\n"
    )
}

fn content_types(images: &[(String, Rc<Resource>)], footnotes: bool, header: bool) -> String {
    let mut defaults = String::new();
    let mut seen: Vec<&str> = Vec::new();
    for (name, res) in images {
        let ext = name.rsplit('.').next().unwrap_or("");
        if !seen.contains(&ext) {
            seen.push(ext);
            defaults.push_str(&format!(
                "<Default Extension=\"{ext}\" ContentType=\"{}\"/>",
                res.kind.media_type()
            ));
        }
    }
    let ov = |part: &str, ct: &str| format!("<Override PartName=\"{part}\" ContentType=\"{ct}\"/>");
    let mut extra = String::new();
    if footnotes {
        extra.push_str(&ov(
            "/word/footnotes.xml",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.footnotes+xml",
        ));
    }
    if header {
        extra.push_str(&ov(
            "/word/header1.xml",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml",
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/>{defaults}{}{}{}{}{}{}{extra}</Types>\n",
        ov(
            "/word/document.xml",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"
        ),
        ov(
            "/word/styles.xml",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"
        ),
        ov(
            "/word/numbering.xml",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"
        ),
        ov(
            "/word/settings.xml",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"
        ),
        ov(
            "/docProps/core.xml",
            "application/vnd.openxmlformats-package.core-properties+xml"
        ),
        ov(
            "/docProps/app.xml",
            "application/vnd.openxmlformats-officedocument.extended-properties+xml"
        ),
    )
}

/// The relationships of the document part (`notes` false, with the fixed
/// styles, numbering, and settings) or of the footnotes part.
fn rels_part(rels: &[Rel], notes: bool) -> String {
    let mut s = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    );
    if !notes {
        s.push_str("<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering\" Target=\"numbering.xml\"/><Relationship Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings\" Target=\"settings.xml\"/>");
    }
    for r in rels.iter().filter(|r| r.notes == notes) {
        s.push_str(&format!(
            "<Relationship Id=\"{}\" Type=\"{}\" Target=\"{}\"{}/>",
            r.id,
            r.kind,
            xml::attr(&r.target),
            if r.external {
                " TargetMode=\"External\""
            } else {
                ""
            }
        ));
    }
    s.push_str("</Relationships>\n");
    s
}

fn core_part(facts: &Facts, modified: &str) -> String {
    let creator = facts
        .author
        .as_deref()
        .map(|a| format!("<dc:creator>{}</dc:creator>", xml::text(a)))
        .unwrap_or_default();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:dcterms=\"http://purl.org/dc/terms/\" xmlns:dcmitype=\"http://purl.org/dc/dcmitype/\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"><dc:title>{}</dc:title>{creator}<dc:language>{}</dc:language><dcterms:created xsi:type=\"dcterms:W3CDTF\">{modified}</dcterms:created><dcterms:modified xsi:type=\"dcterms:W3CDTF\">{modified}</dcterms:modified></cp:coreProperties>\n",
        xml::text(&facts.title),
        xml::text(&facts.language)
    )
}

fn styles_part(language: &str, look: &DocxLook, shift: usize) -> String {
    let lang = xml::attr(language);
    let font = look.font;
    let size = look.size;
    let color = look
        .color
        .map(|c| format!("<w:color w:val=\"{c}\"/>"))
        .unwrap_or_default();
    let spacing = if look.letter_spacing > 0 {
        format!("<w:spacing w:val=\"{}\"/>", look.letter_spacing)
    } else {
        String::new()
    };
    let mut s = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:styles xmlns:w=\"{W}\"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii=\"{font}\" w:hAnsi=\"{font}\" w:eastAsia=\"{font}\" w:cs=\"{font}\"/>{color}{spacing}<w:sz w:val=\"{size}\"/><w:szCs w:val=\"{size}\"/><w:lang w:val=\"{lang}\" w:eastAsia=\"{lang}\" w:bidi=\"{lang}\"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after=\"{}\" w:line=\"{}\" w:lineRule=\"auto\"/></w:pPr></w:pPrDefault></w:docDefaults>",
        look.after, look.line
    );
    s.push_str("<w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/><w:qFormat/></w:style>");
    s.push_str("<w:style w:type=\"character\" w:default=\"1\" w:styleId=\"DefaultParagraphFont\"><w:name w:val=\"Default Paragraph Font\"/><w:uiPriority w:val=\"1\"/><w:semiHidden/><w:unhideWhenUsed/></w:style>");
    // A paper whose only level 1 heading is its title gives the template's
    // first heading look to level 2, and so on down; level 1 keeps the
    // first look (the title, centered in APA).
    for n in 1..=6usize {
        let i = n - 1;
        let h = look.headings[i.saturating_sub(shift)];
        let bold_italic = if h.italic { "<w:i/><w:iCs/>" } else { "" };
        let jc = if h.center {
            "<w:jc w:val=\"center\"/>"
        } else {
            ""
        };
        let ind = if h.indent > 0 {
            format!("<w:ind w:firstLine=\"{}\"/>", h.indent)
        } else {
            String::new()
        };
        s.push_str(&format!(
            "<w:style w:type=\"paragraph\" w:styleId=\"Heading{n}\"><w:name w:val=\"heading {n}\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:uiPriority w:val=\"9\"/><w:qFormat/><w:pPr><w:keepNext/><w:keepLines/><w:spacing w:before=\"{}\" w:after=\"{}\"/>{ind}{jc}<w:outlineLvl w:val=\"{i}\"/></w:pPr><w:rPr><w:b/><w:bCs/>{bold_italic}<w:sz w:val=\"{}\"/><w:szCs w:val=\"{}\"/></w:rPr></w:style>",
            h.before, h.after, h.size, h.size
        ));
    }
    s.push_str("<w:style w:type=\"paragraph\" w:styleId=\"ListParagraph\"><w:name w:val=\"List Paragraph\"/><w:basedOn w:val=\"Normal\"/><w:uiPriority w:val=\"34\"/><w:qFormat/><w:pPr><w:spacing w:after=\"60\"/><w:ind w:left=\"720\"/></w:pPr></w:style>");
    let quote_italic = if look.no_italic {
        ""
    } else {
        "<w:rPr><w:i/><w:iCs/></w:rPr>"
    };
    s.push_str(&format!("<w:style w:type=\"paragraph\" w:styleId=\"Quote\"><w:name w:val=\"Quote\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:uiPriority w:val=\"29\"/><w:qFormat/><w:pPr><w:ind w:left=\"720\" w:right=\"720\"/></w:pPr>{quote_italic}</w:style>"));
    s.push_str("<w:style w:type=\"paragraph\" w:styleId=\"Caption\"><w:name w:val=\"caption\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:uiPriority w:val=\"35\"/><w:qFormat/><w:pPr><w:spacing w:after=\"120\"/></w:pPr><w:rPr><w:b/><w:bCs/></w:rPr></w:style>");
    s.push_str("<w:style w:type=\"paragraph\" w:customStyle=\"1\" w:styleId=\"SourceCode\"><w:name w:val=\"Source Code\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:spacing w:after=\"160\" w:line=\"240\" w:lineRule=\"auto\"/></w:pPr><w:rPr><w:rFonts w:ascii=\"Consolas\" w:hAnsi=\"Consolas\" w:cs=\"Consolas\"/><w:sz w:val=\"22\"/><w:szCs w:val=\"22\"/></w:rPr></w:style>");
    s.push_str("<w:style w:type=\"character\" w:customStyle=\"1\" w:styleId=\"SourceCodeChar\"><w:name w:val=\"Source Code Char\"/><w:basedOn w:val=\"DefaultParagraphFont\"/><w:rPr><w:rFonts w:ascii=\"Consolas\" w:hAnsi=\"Consolas\" w:cs=\"Consolas\"/></w:rPr></w:style>");
    s.push_str(&format!("<w:style w:type=\"character\" w:styleId=\"Hyperlink\"><w:name w:val=\"Hyperlink\"/><w:basedOn w:val=\"DefaultParagraphFont\"/><w:uiPriority w:val=\"99\"/><w:unhideWhenUsed/><w:rPr><w:color w:val=\"{}\"/><w:u w:val=\"single\"/></w:rPr></w:style>", look.link_color));
    // Word's own footnote styles, so its footnote pane and screen readers
    // treat the notes as footnotes.
    s.push_str("<w:style w:type=\"paragraph\" w:styleId=\"FootnoteText\"><w:name w:val=\"footnote text\"/><w:basedOn w:val=\"Normal\"/><w:uiPriority w:val=\"99\"/><w:unhideWhenUsed/><w:pPr><w:spacing w:after=\"0\" w:line=\"240\" w:lineRule=\"auto\"/></w:pPr><w:rPr><w:sz w:val=\"20\"/><w:szCs w:val=\"20\"/></w:rPr></w:style>");
    s.push_str("<w:style w:type=\"character\" w:styleId=\"FootnoteReference\"><w:name w:val=\"footnote reference\"/><w:basedOn w:val=\"DefaultParagraphFont\"/><w:uiPriority w:val=\"99\"/><w:unhideWhenUsed/><w:rPr><w:vertAlign w:val=\"superscript\"/></w:rPr></w:style>");
    // Template styles: the title page's title, indented body text, the
    // reference list, and the page header.
    s.push_str(&format!("<w:style w:type=\"paragraph\" w:styleId=\"Title\"><w:name w:val=\"Title\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:uiPriority w:val=\"10\"/><w:qFormat/><w:pPr><w:spacing w:before=\"{}\" w:after=\"0\"/><w:jc w:val=\"center\"/></w:pPr><w:rPr><w:b/><w:bCs/></w:rPr></w:style>", look.line * 3));
    s.push_str(&format!("<w:style w:type=\"paragraph\" w:styleId=\"BodyText\"><w:name w:val=\"Body Text\"/><w:basedOn w:val=\"Normal\"/><w:uiPriority w:val=\"1\"/><w:qFormat/><w:pPr><w:ind w:firstLine=\"{}\"/></w:pPr></w:style>", look.first_line));
    s.push_str("<w:style w:type=\"paragraph\" w:styleId=\"Bibliography\"><w:name w:val=\"Bibliography\"/><w:basedOn w:val=\"Normal\"/><w:uiPriority w:val=\"37\"/><w:unhideWhenUsed/><w:pPr><w:ind w:left=\"720\" w:hanging=\"720\"/></w:pPr></w:style>");
    s.push_str("<w:style w:type=\"paragraph\" w:styleId=\"Header\"><w:name w:val=\"header\"/><w:basedOn w:val=\"Normal\"/><w:uiPriority w:val=\"99\"/><w:unhideWhenUsed/><w:pPr><w:spacing w:after=\"0\" w:line=\"240\" w:lineRule=\"auto\"/></w:pPr></w:style>");
    s.push_str("<w:style w:type=\"table\" w:default=\"1\" w:styleId=\"TableNormal\"><w:name w:val=\"Normal Table\"/><w:uiPriority w:val=\"99\"/><w:semiHidden/><w:unhideWhenUsed/><w:tblPr><w:tblInd w:w=\"0\" w:type=\"dxa\"/><w:tblCellMar><w:top w:w=\"0\" w:type=\"dxa\"/><w:left w:w=\"108\" w:type=\"dxa\"/><w:bottom w:w=\"0\" w:type=\"dxa\"/><w:right w:w=\"108\" w:type=\"dxa\"/></w:tblCellMar></w:tblPr></w:style>");
    s.push_str("<w:style w:type=\"table\" w:styleId=\"TableGrid\"><w:name w:val=\"Table Grid\"/><w:basedOn w:val=\"TableNormal\"/><w:uiPriority w:val=\"39\"/><w:tblPr><w:tblBorders><w:top w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"auto\"/><w:left w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"auto\"/><w:bottom w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"auto\"/><w:right w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"auto\"/><w:insideH w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"auto\"/><w:insideV w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"auto\"/></w:tblBorders></w:tblPr></w:style>");
    s.push_str("</w:styles>\n");
    s
}

/// Bullets by level, alternating so nesting is visible.
const BULLETS: [&str; 3] = ["\u{2022}", "\u{25E6}", "\u{25AA}"];

fn numbering_part(nums: &[Num]) -> String {
    let mut s = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:numbering xmlns:w=\"{W}\">"
    );
    // Abstract 0: bullets; abstract 1: decimal numbers.
    for (abs, ordered) in [(0, false), (1, true)] {
        s.push_str(&format!(
            "<w:abstractNum w:abstractNumId=\"{abs}\"><w:multiLevelType w:val=\"hybridMultilevel\"/>"
        ));
        for lvl in 0..9 {
            let (fmt, text) = if ordered {
                ("decimal", format!("%{}.", lvl + 1))
            } else {
                ("bullet", BULLETS[lvl % 3].to_owned())
            };
            s.push_str(&format!(
                "<w:lvl w:ilvl=\"{lvl}\"><w:start w:val=\"1\"/><w:numFmt w:val=\"{fmt}\"/><w:lvlText w:val=\"{text}\"/><w:lvlJc w:val=\"left\"/><w:pPr><w:ind w:left=\"{}\" w:hanging=\"360\"/></w:pPr></w:lvl>",
                720 * (lvl + 1)
            ));
        }
        s.push_str("</w:abstractNum>");
    }
    for (i, n) in nums.iter().enumerate() {
        let id = i + 1;
        if n.ordered {
            s.push_str(&format!(
                "<w:num w:numId=\"{id}\"><w:abstractNumId w:val=\"1\"/><w:lvlOverride w:ilvl=\"{}\"><w:startOverride w:val=\"{}\"/></w:lvlOverride></w:num>",
                n.level,
                n.start.min(u64::from(u32::MAX))
            ));
        } else {
            s.push_str(&format!(
                "<w:num w:numId=\"{id}\"><w:abstractNumId w:val=\"0\"/></w:num>"
            ));
        }
    }
    s.push_str("</w:numbering>\n");
    s
}
