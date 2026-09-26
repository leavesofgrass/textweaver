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
//! - Links are hyperlinks with the `Hyperlink` style; footnote references
//!   link to a bookmark on the footnote's text (textweaver keeps footnotes
//!   where the document has them rather than moving them into Word
//!   footnotes).
//! - The document language is the default run language, and the title,
//!   author, and language are in the core properties.

use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::rc::Rc;

use textweaver_text::Document;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

use crate::model::{self, Block, Facts, Image, Inline, List, Style, Table};
use crate::resource::{ImageKind, Resource, Resources};
use crate::{Format, WriteError, WriteOptions, WriteReport, Writer, iso8601, timestamp, xml};

/// Writes DOCX.
#[derive(Clone, Copy, Debug, Default)]
pub struct DocxWriter;

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const REL_HYPERLINK: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink";
const REL_IMAGE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";

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
            notes: collect_notes(&blocks),
            notes_marked: Vec::new(),
            drawings: 0,
            text_width,
        };
        body.blocks(&blocks, &Ctx::default());
        body.out.push_str(&format!(
            "<w:sectPr><w:pgSz w:w=\"{page_w}\" w:h=\"{page_h}\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\" w:header=\"720\" w:footer=\"720\" w:gutter=\"0\"/></w:sectPr>"
        ));
        let document = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:document xmlns:w=\"{W}\" xmlns:r=\"{R}\" xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><w:body>{}</w:body></w:document>\n",
            body.out
        );
        let numbering = numbering_part(&body.nums);
        let styles = styles_part(&facts.language);
        let doc_rels = document_rels(&body.rels);
        let content_types = content_types(&body.images);
        let modified = iso8601(timestamp(options));
        let core = core_part(&facts, &modified);
        let images = std::mem::take(&mut body.images);
        drop(body);

        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let zerr = |e: zip::result::ZipError| WriteError::Zip(e.to_string());
        let parts: [(&str, &str); 9] = [
            ("[Content_Types].xml", &content_types),
            ("_rels/.rels", PACKAGE_RELS),
            ("docProps/core.xml", &core),
            ("docProps/app.xml", APP),
            ("word/document.xml", &document),
            ("word/styles.xml", &styles),
            ("word/numbering.xml", &numbering),
            ("word/settings.xml", SETTINGS),
            ("word/_rels/document.xml.rels", &doc_rels),
        ];
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

/// Footnote ids that have a body in the document.
fn collect_notes(blocks: &[Block]) -> Vec<String> {
    let mut v = Vec::new();
    fn walk(blocks: &[Block], v: &mut Vec<String>) {
        for b in blocks {
            match b {
                Block::Footnote { id, .. } => {
                    if !v.contains(id) {
                        v.push(id.clone());
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
    code: bool,
    link: bool,
}

struct Body<'a> {
    out: String,
    /// Relationships after the fixed ones: (id, type, target, external).
    rels: Vec<(String, &'static str, String, bool)>,
    links: HashMap<String, String>,
    nums: Vec<Num>,
    resources: Resources,
    /// (path under word/, image).
    images: Vec<(String, Rc<Resource>)>,
    image_rels: HashMap<String, (String, Rc<Resource>)>,
    report: &'a mut WriteReport,
    bookmarks: usize,
    notes: Vec<String>,
    notes_marked: Vec<String>,
    drawings: usize,
    text_width: u32,
}

impl Body<'_> {
    fn rel(&mut self, kind: &'static str, target: String, external: bool) -> String {
        // rId1 to rId3 are styles, numbering, and settings.
        let id = format!("rId{}", self.rels.len() + 4);
        self.rels.push((id.clone(), kind, target, external));
        id
    }

    fn blocks(&mut self, blocks: &[Block], ctx: &Ctx) {
        for b in blocks {
            self.block(b, ctx);
        }
    }

    fn para_open(&mut self, style: Option<&str>, extra: &str) {
        self.out.push_str("<w:p>");
        if style.is_some() || !extra.is_empty() {
            self.out.push_str("<w:pPr>");
            if let Some(s) = style {
                self.out.push_str(&format!("<w:pStyle w:val=\"{s}\"/>"));
            }
            self.out.push_str(extra);
            self.out.push_str("</w:pPr>");
        }
    }

    /// Style and indentation of a body paragraph in `ctx`.
    fn body_style(ctx: &Ctx) -> (Option<&'static str>, String) {
        if ctx.quote {
            (Some("Quote"), String::new())
        } else if ctx.depth > 0 {
            (
                Some("ListParagraph"),
                format!("<w:ind w:left=\"{}\"/>", 720 * ctx.depth),
            )
        } else {
            (None, String::new())
        }
    }

    fn block(&mut self, b: &Block, ctx: &Ctx) {
        match b {
            Block::Heading { level, content } => {
                let style = format!("Heading{}", (*level).clamp(1, 6));
                self.para_open(Some(&style), "");
                self.inlines(content, Props::default(), ctx);
                self.out.push_str("</w:p>");
            }
            Block::Paragraph(content) => {
                let (style, extra) = Self::body_style(ctx);
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
                let (style, extra) = Self::body_style(ctx);
                self.para_open(style, &extra);
                if !self.drawing(img) && !img.alt.is_empty() {
                    self.run(&img.alt, Props::default());
                }
                self.out.push_str("</w:p>");
            }
            Block::Footnote { id, content } => {
                let (style, extra) = Self::body_style(ctx);
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
                self.out
                    .push_str("<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>");
            }
            // Print page numbers have no place in a reflowing document.
            Block::PageBreak { .. } => {}
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
        if p.bold {
            rpr.push_str("<w:b/><w:bCs/>");
        }
        if p.italic {
            rpr.push_str("<w:i/><w:iCs/>");
        }
        if p.underline && !p.link {
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
            Style::Link(target) if !p.link && is_external(target) => {
                let target = target.trim().to_owned();
                let id = match self.links.get(&target) {
                    Some(id) => id.clone(),
                    None => {
                        let id = self.rel(REL_HYPERLINK, target.clone(), true);
                        self.links.insert(target, id.clone());
                        id
                    }
                };
                self.out
                    .push_str(&format!("<w:hyperlink r:id=\"{id}\" w:history=\"1\">"));
                self.inlines(children, Props { link: true, ..p }, ctx);
                self.out.push_str("</w:hyperlink>");
            }
            Style::FootnoteRef(id)
                if !p.link
                    && self.notes.contains(id)
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
        let (rid, res) = match self.image_rels.get(src) {
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
                let name = format!(
                    "media/image{}.{}",
                    self.images.len() + 1,
                    res.kind.extension()
                );
                let rid = self.rel(REL_IMAGE, name.clone(), false);
                self.images.push((name, res.clone()));
                self.image_rels
                    .insert(src.to_owned(), (rid.clone(), res.clone()));
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

const SETTINGS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:defaultTabStop w:val="720"/><w:characterSpacingControl w:val="doNotCompress"/><w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>
"#;

fn content_types(images: &[(String, Rc<Resource>)]) -> String {
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
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/>{defaults}{}{}{}{}{}{}</Types>\n",
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

fn document_rels(rels: &[(String, &'static str, String, bool)]) -> String {
    let mut s = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering\" Target=\"numbering.xml\"/><Relationship Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings\" Target=\"settings.xml\"/>",
    );
    for (id, kind, target, external) in rels {
        s.push_str(&format!(
            "<Relationship Id=\"{id}\" Type=\"{kind}\" Target=\"{}\"{}/>",
            xml::attr(target),
            if *external {
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

fn styles_part(language: &str) -> String {
    let lang = xml::attr(language);
    let mut s = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:styles xmlns:w=\"{W}\"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii=\"Calibri\" w:hAnsi=\"Calibri\" w:eastAsia=\"Calibri\" w:cs=\"Calibri\"/><w:sz w:val=\"24\"/><w:szCs w:val=\"24\"/><w:lang w:val=\"{lang}\" w:eastAsia=\"{lang}\" w:bidi=\"{lang}\"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after=\"160\" w:line=\"360\" w:lineRule=\"auto\"/></w:pPr></w:pPrDefault></w:docDefaults>"
    );
    s.push_str("<w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/><w:qFormat/></w:style>");
    s.push_str("<w:style w:type=\"character\" w:default=\"1\" w:styleId=\"DefaultParagraphFont\"><w:name w:val=\"Default Paragraph Font\"/><w:uiPriority w:val=\"1\"/><w:semiHidden/><w:unhideWhenUsed/></w:style>");
    // Heading sizes in half-points.
    let sizes = [36, 32, 28, 26, 24, 24];
    for (i, sz) in sizes.iter().enumerate() {
        let n = i + 1;
        let italic = if n == 6 { "<w:i/><w:iCs/>" } else { "" };
        s.push_str(&format!(
            "<w:style w:type=\"paragraph\" w:styleId=\"Heading{n}\"><w:name w:val=\"heading {n}\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:uiPriority w:val=\"9\"/><w:qFormat/><w:pPr><w:keepNext/><w:keepLines/><w:spacing w:before=\"{}\" w:after=\"120\"/><w:outlineLvl w:val=\"{i}\"/></w:pPr><w:rPr><w:b/><w:bCs/>{italic}<w:sz w:val=\"{sz}\"/><w:szCs w:val=\"{sz}\"/></w:rPr></w:style>",
            if n <= 2 { 360 } else { 240 }
        ));
    }
    s.push_str("<w:style w:type=\"paragraph\" w:styleId=\"ListParagraph\"><w:name w:val=\"List Paragraph\"/><w:basedOn w:val=\"Normal\"/><w:uiPriority w:val=\"34\"/><w:qFormat/><w:pPr><w:spacing w:after=\"60\"/><w:ind w:left=\"720\"/></w:pPr></w:style>");
    s.push_str("<w:style w:type=\"paragraph\" w:styleId=\"Quote\"><w:name w:val=\"Quote\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:uiPriority w:val=\"29\"/><w:qFormat/><w:pPr><w:ind w:left=\"720\" w:right=\"720\"/></w:pPr><w:rPr><w:i/><w:iCs/></w:rPr></w:style>");
    s.push_str("<w:style w:type=\"paragraph\" w:styleId=\"Caption\"><w:name w:val=\"caption\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:uiPriority w:val=\"35\"/><w:qFormat/><w:pPr><w:spacing w:after=\"120\"/></w:pPr><w:rPr><w:b/><w:bCs/></w:rPr></w:style>");
    s.push_str("<w:style w:type=\"paragraph\" w:customStyle=\"1\" w:styleId=\"SourceCode\"><w:name w:val=\"Source Code\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:spacing w:after=\"160\" w:line=\"240\" w:lineRule=\"auto\"/></w:pPr><w:rPr><w:rFonts w:ascii=\"Consolas\" w:hAnsi=\"Consolas\" w:cs=\"Consolas\"/><w:sz w:val=\"22\"/><w:szCs w:val=\"22\"/></w:rPr></w:style>");
    s.push_str("<w:style w:type=\"character\" w:customStyle=\"1\" w:styleId=\"SourceCodeChar\"><w:name w:val=\"Source Code Char\"/><w:basedOn w:val=\"DefaultParagraphFont\"/><w:rPr><w:rFonts w:ascii=\"Consolas\" w:hAnsi=\"Consolas\" w:cs=\"Consolas\"/></w:rPr></w:style>");
    s.push_str("<w:style w:type=\"character\" w:styleId=\"Hyperlink\"><w:name w:val=\"Hyperlink\"/><w:basedOn w:val=\"DefaultParagraphFont\"/><w:uiPriority w:val=\"99\"/><w:unhideWhenUsed/><w:rPr><w:color w:val=\"0563C1\"/><w:u w:val=\"single\"/></w:rPr></w:style>");
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
