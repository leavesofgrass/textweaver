//! EPUB 3 with accessibility metadata.
//!
//! The package (`OEBPS/`) holds one XHTML content document per chapter,
//! a navigation document (`nav.xhtml`: table of contents from the headings,
//! landmarks, and a page list when the source had print page breaks), an
//! NCX for EPUB 2 reading systems, a small stylesheet, and the images found
//! on disk. Content documents use semantic HTML: `h1` to `h6`, `ul`/`ol`,
//! tables with `caption`, `thead`, and `th scope="col"`, `figure` with `img
//! alt`, `pre`/`code`, `blockquote`, footnotes as `aside
//! epub:type="footnote"` with `noteref` links, print page breaks as
//! `epub:type="pagebreak"`, and the document language on every root
//! element.
//!
//! With [`EpubOptions::font`](crate::EpubOptions) or
//! [`EpubOptions::code_font`](crate::EpubOptions), a bundled font is
//! embedded (`OEBPS/fonts/<family>/`, with its `OFL.txt`) and named in the
//! stylesheet; reading systems may still let the reader choose another.
//!
//! The package metadata declares the schema.org accessibility properties
//! (EPUB Accessibility 1.1): access modes, sufficient access mode, features
//! (structural navigation, table of contents, reading order, alternative
//! text, page navigation), hazards (none), and a spoken summary. It makes no
//! WCAG conformance claim, which needs a human evaluation.

use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::rc::Rc;

use textweaver_fonts::{BundledFamily, bundled};
use textweaver_text::Document;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

use crate::model::{self, Block, Facts, Image, Inline, List, Style, Table};
use crate::resource::{Resource, Resources};
use crate::{
    EpubOptions, Format, WriteError, WriteOptions, WriteReport, Writer, iso8601, timestamp, xml,
};

/// Writes EPUB 3.
#[derive(Clone, Copy, Debug, Default)]
pub struct EpubWriter;

impl Writer for EpubWriter {
    fn format(&self) -> Format {
        Format::Epub
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
        let mut chapters = split(blocks, options.epub.split_chapters);
        let index = Index::build(&chapters);
        label_chapters(&mut chapters, &index);
        let mut resources = Resources::new(doc, options);
        let mut images = Images::default();

        let mut files: Vec<(String, String)> = Vec::new();
        let mut pages: Vec<(String, String)> = Vec::new();
        let mut headings = 0usize;
        let mut notes: HashMap<String, usize> = HashMap::new();
        for (n, chapter) in chapters.iter().enumerate() {
            let name = chapter_file(n);
            let mut html = Html {
                out: String::new(),
                file: &name,
                index: &index,
                resources: &mut resources,
                images: &mut images,
                report: &mut report,
                headings: &mut headings,
                notes: &mut notes,
                pages: &mut pages,
                in_note: None,
                split: options.epub.split_chapters,
            };
            html.blocks(&chapter.blocks);
            let body = html.out;
            let title = chapter.title.clone().unwrap_or_else(|| facts.title.clone());
            files.push((
                name,
                content_document(&facts, &title, &body, chapter.labelled_by.as_deref()),
            ));
        }

        let toc = toc_tree(&index, &chapters, &facts);
        let nav = nav_document(&facts, &toc, &pages);
        let ncx = ncx_document(&facts, &toc, &identifier(doc, &facts));
        let fonts = EmbeddedFonts::choose(&options.epub, &mut report);
        let opf = package_document(
            doc,
            options,
            &facts,
            &files,
            &images,
            &fonts,
            !pages.is_empty(),
        );

        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let zerr = |e: zip::result::ZipError| WriteError::Zip(e.to_string());
        zip.start_file("mimetype", stored).map_err(zerr)?;
        zip.write_all(b"application/epub+zip")?;
        zip.start_file("META-INF/container.xml", deflated)
            .map_err(zerr)?;
        zip.write_all(CONTAINER.as_bytes())?;
        zip.start_file("OEBPS/content.opf", deflated)
            .map_err(zerr)?;
        zip.write_all(opf.as_bytes())?;
        zip.start_file("OEBPS/nav.xhtml", deflated).map_err(zerr)?;
        zip.write_all(nav.as_bytes())?;
        zip.start_file("OEBPS/toc.ncx", deflated).map_err(zerr)?;
        zip.write_all(ncx.as_bytes())?;
        zip.start_file("OEBPS/style.css", deflated).map_err(zerr)?;
        zip.write_all(STYLE.as_bytes())?;
        zip.write_all(fonts.css().as_bytes())?;
        for (href, data) in fonts.files() {
            zip.start_file(format!("OEBPS/{href}"), deflated)
                .map_err(zerr)?;
            zip.write_all(data)?;
        }
        for (name, body) in &files {
            zip.start_file(format!("OEBPS/{name}"), deflated)
                .map_err(zerr)?;
            zip.write_all(body.as_bytes())?;
        }
        for (href, res) in &images.files {
            zip.start_file(format!("OEBPS/{href}"), stored)
                .map_err(zerr)?;
            zip.write_all(&res.bytes)?;
        }
        let bytes = zip.finish().map_err(zerr)?.into_inner();
        out.write_all(&bytes)?;
        Ok(report)
    }
}

const CONTAINER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"#;

/// Bundled fonts the book embeds (SIL OFL 1.1, whose licence travels in
/// the book beside the font files).
struct EmbeddedFonts {
    /// The text family, when one was asked for.
    text: Option<&'static BundledFamily>,
    /// The code family, when one was asked for.
    code: Option<&'static BundledFamily>,
}

impl EmbeddedFonts {
    fn choose(options: &EpubOptions, report: &mut WriteReport) -> EmbeddedFonts {
        let mut pick = |name: &Option<String>| -> Option<&'static BundledFamily> {
            let name = name.as_deref().map(str::trim).filter(|n| !n.is_empty())?;
            let found = bundled::family(name);
            if found.is_none() {
                let names: Vec<&str> = bundled::BUNDLED.iter().map(|f| f.name).collect();
                report.warn(if names.is_empty() {
                    format!("The font {name} was not embedded: this build has no bundled fonts.")
                } else {
                    format!(
                        "The font {name} was not embedded: only bundled fonts can go into an EPUB ({}).",
                        names.join(", ")
                    )
                });
            }
            found
        };
        let text = pick(&options.font);
        let code = pick(&options.code_font);
        EmbeddedFonts { text, code }
    }

    fn families(&self) -> Vec<&'static BundledFamily> {
        let mut v: Vec<&'static BundledFamily> = self.text.into_iter().collect();
        if let Some(c) = self.code
            && !v.iter().any(|f| f.key == c.key)
        {
            v.push(c);
        }
        v
    }

    /// (href inside `OEBPS/`, bytes) of every font file and licence.
    fn files(&self) -> Vec<(String, &'static [u8])> {
        let mut out = Vec::new();
        for f in self.families() {
            for face in &f.faces {
                out.push((format!("fonts/{}/{}", f.key, face.file_name), face.data));
            }
            out.push((
                format!("fonts/{}/OFL.txt", f.key),
                f.license_text.as_bytes(),
            ));
        }
        out
    }

    /// Manifest items: (id, href, media type).
    fn manifest(&self) -> Vec<(String, String, &'static str)> {
        let mut out = Vec::new();
        for f in self.families() {
            for (n, face) in f.faces.iter().enumerate() {
                out.push((
                    format!("font-{}-{}", f.key, n + 1),
                    format!("fonts/{}/{}", f.key, face.file_name),
                    face.media_type(),
                ));
            }
            out.push((
                format!("font-{}-licence", f.key),
                format!("fonts/{}/OFL.txt", f.key),
                "text/plain",
            ));
        }
        out
    }

    /// `@font-face` rules and the families for text and code.
    fn css(&self) -> String {
        let mut css = String::new();
        for f in self.families() {
            css.push_str(&format!(
                "/* {} {}: {}. SIL Open Font License 1.1; see fonts/{}/OFL.txt. */\n",
                f.name, f.version, f.copyright, f.key
            ));
            for face in &f.faces {
                css.push_str(&format!(
                    "@font-face {{ font-family: \"{}\"; font-weight: {}; font-style: {}; src: url(\"fonts/{}/{}\"); }}\n",
                    f.name,
                    if face.style.is_bold() { "bold" } else { "normal" },
                    if face.style.is_italic() { "italic" } else { "normal" },
                    f.key,
                    face.file_name
                ));
            }
        }
        if let Some(f) = self.text {
            let generic = if f.monospace {
                "monospace"
            } else {
                "sans-serif"
            };
            css.push_str(&format!(
                "body {{ font-family: \"{}\", {generic}; }}\n",
                f.name
            ));
        }
        if let Some(f) = self.code {
            css.push_str(&format!(
                "pre, code {{ font-family: \"{}\", monospace; }}\n",
                f.name
            ));
        }
        css
    }
}

/// A readable default: reading systems and user settings override it.
const STYLE: &str = "body { line-height: 1.5; margin: 0 4%; }
h1, h2, h3, h4, h5, h6 { line-height: 1.25; margin: 1.2em 0 0.5em; }
p { margin: 0 0 0.8em; }
table { border-collapse: collapse; margin: 1em 0; }
caption { font-weight: bold; text-align: left; margin-bottom: 0.3em; }
th, td { border: 1px solid; padding: 0.25em 0.5em; text-align: left; vertical-align: top; }
pre { white-space: pre-wrap; font-family: monospace; margin: 1em 0; }
code { font-family: monospace; }
blockquote { margin: 1em 2em; }
figure { margin: 1em 0; }
img { max-width: 100%; height: auto; }
aside { margin: 0.5em 0; font-size: 0.95em; }
hr.section-break { margin: 2em 0; }
";

fn chapter_file(n: usize) -> String {
    format!("chapter-{}.xhtml", n + 1)
}

/// A chapter: its blocks and its title.
struct Chapter {
    title: Option<String>,
    /// Id of the heading that labels the chapter, filled by the index.
    labelled_by: Option<String>,
    blocks: Vec<Block>,
}

fn has_content(blocks: &[Block]) -> bool {
    blocks
        .iter()
        .any(|b| !matches!(b, Block::SectionBreak { .. } | Block::PageBreak { .. }))
}

/// Splits the top-level blocks into chapters: at section breaks when there
/// are any, otherwise before level 1 headings.
fn split(blocks: Vec<Block>, enabled: bool) -> Vec<Chapter> {
    let at_breaks = blocks
        .iter()
        .any(|b| matches!(b, Block::SectionBreak { .. }));
    let mut chapters: Vec<Chapter> = Vec::new();
    let mut current: Vec<Block> = Vec::new();
    let mut break_title: Option<String> = None;
    let mut push = |current: &mut Vec<Block>, title: Option<String>| {
        if has_content(current) || (chapters.is_empty() && !current.is_empty()) {
            chapters.push(Chapter {
                title,
                labelled_by: None,
                blocks: std::mem::take(current),
            });
        } else if let Some(last) = chapters.last_mut() {
            // Only breaks: keep them (page breaks) in the previous chapter.
            last.blocks.append(current);
        } else {
            current.clear();
        }
    };
    for b in blocks {
        let boundary = enabled
            && match &b {
                Block::SectionBreak { .. } => at_breaks,
                Block::Heading { level: 1, .. } => !at_breaks,
                _ => false,
            };
        if boundary && has_content(&current) {
            let title = break_title.take();
            push(&mut current, title);
        }
        if let Block::SectionBreak { title } = &b
            && enabled
        {
            break_title = title.clone();
            if at_breaks {
                continue;
            }
        }
        current.push(b);
    }
    let title = break_title.take();
    push(&mut current, title);
    if chapters.is_empty() {
        chapters.push(Chapter {
            title: None,
            labelled_by: None,
            blocks: Vec::new(),
        });
    }
    // A chapter's title is its first heading when it has one.
    for c in &mut chapters {
        if let Some(Block::Heading { content, .. }) =
            c.blocks.iter().find(|b| matches!(b, Block::Heading { .. }))
        {
            let t = model::collapse_ws(&Inline::plain(content));
            if !t.is_empty() {
                c.title = Some(t);
            }
        }
    }
    chapters
}

/// Where every heading and footnote body lives, before rendering.
#[derive(Default)]
struct Index {
    /// (level, text, file, id) in document order.
    headings: Vec<(u8, String, String, String)>,
    /// Footnote id → (file, element id), first body wins.
    notes: HashMap<String, (String, String)>,
}

impl Index {
    fn build(chapters: &[Chapter]) -> Index {
        let mut index = Index::default();
        let mut used: Vec<String> = Vec::new();
        for (n, c) in chapters.iter().enumerate() {
            let file = chapter_file(n);
            index.walk(&c.blocks, &file, &mut used);
        }
        index
    }

    fn walk(&mut self, blocks: &[Block], file: &str, used: &mut Vec<String>) {
        for b in blocks {
            match b {
                Block::Heading { level, content } => {
                    let id = format!("h-{}", self.headings.len() + 1);
                    let text = model::collapse_ws(&Inline::plain(content));
                    self.headings.push((*level, text, file.to_owned(), id));
                }
                Block::Footnote { id, .. } => {
                    let mut xid = xml::id("fn-", id);
                    let base = xid.clone();
                    let mut k = 2;
                    while used.contains(&xid) {
                        xid = format!("{base}-{k}");
                        k += 1;
                    }
                    used.push(xid.clone());
                    self.notes
                        .entry(id.clone())
                        .or_insert_with(|| (file.to_owned(), xid));
                }
                Block::Quote(inner) => self.walk(inner, file, used),
                Block::List(list) => {
                    for item in &list.items {
                        self.walk(&item.blocks, file, used);
                    }
                }
                _ => {}
            }
        }
    }
}

/// Images copied into the package.
#[derive(Default)]
struct Images {
    /// (href inside OEBPS, image).
    files: Vec<(String, Rc<Resource>)>,
    by_src: HashMap<String, String>,
}

struct Html<'a> {
    out: String,
    file: &'a str,
    index: &'a Index,
    resources: &'a mut Resources,
    images: &'a mut Images,
    report: &'a mut WriteReport,
    headings: &'a mut usize,
    notes: &'a mut HashMap<String, usize>,
    pages: &'a mut Vec<(String, String)>,
    /// Inside the body of this footnote.
    in_note: Option<String>,
    split: bool,
}

impl Html<'_> {
    fn blocks(&mut self, blocks: &[Block]) {
        for b in blocks {
            self.block(b);
        }
    }

    fn block(&mut self, b: &Block) {
        match b {
            Block::Heading { level, content } => {
                *self.headings += 1;
                let l = (*level).clamp(1, 6);
                self.out
                    .push_str(&format!("<h{l} id=\"h-{}\">", *self.headings));
                self.inlines(content);
                self.out.push_str(&format!("</h{l}>\n"));
            }
            Block::Paragraph(content) => {
                self.out.push_str("<p>");
                self.inlines(content);
                self.out.push_str("</p>\n");
            }
            Block::List(list) => self.list(list),
            Block::Table(table) => self.table(table),
            Block::Code { language, text } => {
                self.out.push_str("<pre><code");
                if let Some(lang) = language {
                    let class: String = lang
                        .chars()
                        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '+'))
                        .collect();
                    if !class.is_empty() {
                        self.out
                            .push_str(&format!(" class=\"language-{}\"", xml::attr(&class)));
                    }
                }
                self.out.push('>');
                self.out.push_str(&xml::text(text));
                self.out.push_str("</code></pre>\n");
            }
            Block::Quote(inner) => {
                self.out.push_str("<blockquote>\n");
                self.blocks(inner);
                self.out.push_str("</blockquote>\n");
            }
            Block::Figure(img) => {
                if let Some(href) = self.image_href(img) {
                    self.out.push_str(&format!(
                        "<figure><img src=\"{}\" alt=\"{}\"/></figure>\n",
                        xml::attr(&href),
                        xml::attr(&img.alt)
                    ));
                } else if !img.alt.is_empty() {
                    self.out.push_str("<p>");
                    self.missing_image(&img.alt);
                    self.out.push_str("</p>\n");
                }
            }
            Block::Footnote { id, content } => {
                let xid = self
                    .index
                    .notes
                    .get(id)
                    .filter(|(f, _)| f == self.file)
                    .map(|(_, x)| x.clone());
                // Later bodies with the same id get their own ids.
                let n = self.notes.entry(id.clone()).or_insert(0);
                *n += 1;
                let xid = match (xid, *n) {
                    (Some(x), 1) => x,
                    (_, k) => format!("{}-{k}", xml::id("fn-", id)),
                };
                self.out.push_str(&format!(
                    "<aside epub:type=\"footnote\" role=\"doc-footnote\" id=\"{xid}\"><p>"
                ));
                self.in_note = Some(id.clone());
                self.inlines(content);
                self.in_note = None;
                self.out.push_str("</p></aside>\n");
            }
            Block::SectionBreak { .. } => {
                if !self.split {
                    self.out.push_str("<hr class=\"section-break\"/>\n");
                }
            }
            Block::PageBreak { label } => {
                let label = label
                    .clone()
                    .filter(|l| !l.trim().is_empty())
                    .unwrap_or_else(|| (self.pages.len() + 1).to_string());
                let id = format!("page-{}", self.pages.len() + 1);
                self.out.push_str(&format!(
                    "<span epub:type=\"pagebreak\" role=\"doc-pagebreak\" id=\"{id}\" aria-label=\"{}\"></span>\n",
                    xml::attr(&label)
                ));
                self.pages.push((label, format!("{}#{id}", self.file)));
            }
        }
    }

    fn list(&mut self, list: &List) {
        if list.ordered {
            if list.start == 1 {
                self.out.push_str("<ol>\n");
            } else {
                self.out
                    .push_str(&format!("<ol start=\"{}\">\n", list.start));
            }
        } else {
            self.out.push_str("<ul>\n");
        }
        for item in &list.items {
            self.out.push_str("<li>");
            let tight = item
                .blocks
                .iter()
                .filter(|b| matches!(b, Block::Paragraph(_)))
                .count()
                <= 1;
            let mut rest: &[Block] = &item.blocks;
            if tight && let Some(Block::Paragraph(content)) = rest.first() {
                self.inlines(content);
                rest = &rest[1..];
                if !rest.is_empty() {
                    self.out.push('\n');
                }
            }
            self.blocks(rest);
            self.out.push_str("</li>\n");
        }
        self.out
            .push_str(if list.ordered { "</ol>\n" } else { "</ul>\n" });
    }

    fn table(&mut self, table: &Table) {
        self.out.push_str("<table>\n");
        if let Some(c) = &table.caption {
            self.out
                .push_str(&format!("<caption>{}</caption>\n", xml::text(c)));
        }
        let head = table.rows.iter().take_while(|r| r.header).count();
        let (header, body) = table.rows.split_at(head);
        if !header.is_empty() {
            self.out.push_str("<thead>\n");
            for row in header {
                self.out.push_str("<tr>");
                for cell in &row.cells {
                    self.out.push_str("<th scope=\"col\">");
                    self.inlines(cell);
                    self.out.push_str("</th>");
                }
                self.out.push_str("</tr>\n");
            }
            self.out.push_str("</thead>\n");
        }
        if !body.is_empty() {
            self.out.push_str("<tbody>\n");
            for row in body {
                self.out.push_str("<tr>");
                let tag = if row.header { "th" } else { "td" };
                for cell in &row.cells {
                    self.out.push_str(&format!("<{tag}>"));
                    self.inlines(cell);
                    self.out.push_str(&format!("</{tag}>"));
                }
                self.out.push_str("</tr>\n");
            }
            self.out.push_str("</tbody>\n");
        }
        self.out.push_str("</table>\n");
    }

    /// The href of an image copied into the package, if it can be.
    fn image_href(&mut self, img: &Image) -> Option<String> {
        let src = img.src.as_deref()?;
        if let Some(h) = self.images.by_src.get(src) {
            return Some(h.clone());
        }
        let res = self.resources.get(src, self.report)?;
        let href = format!(
            "images/image-{}.{}",
            self.images.files.len() + 1,
            res.kind.extension()
        );
        self.images.files.push((href.clone(), res));
        self.images.by_src.insert(src.to_owned(), href.clone());
        Some(href)
    }

    /// An image that is not in the package: its description, as an image.
    fn missing_image(&mut self, alt: &str) {
        self.out.push_str(&format!(
            "<span role=\"img\" aria-label=\"{}\">{}</span>",
            xml::attr(alt),
            xml::text(alt)
        ));
    }

    fn inlines(&mut self, inlines: &[Inline]) {
        for i in inlines {
            match i {
                Inline::Text(t) => self.out.push_str(&xml::text(t)),
                Inline::LineBreak => self.out.push_str("<br/>"),
                Inline::Span(style, children) => self.span(style, children),
            }
        }
    }

    fn span(&mut self, style: &Style, children: &[Inline]) {
        let wrap = |me: &mut Self, open: &str, close: &str| {
            me.out.push_str(open);
            me.inlines(children);
            me.out.push_str(close);
        };
        match style {
            Style::Bold => wrap(self, "<strong>", "</strong>"),
            Style::Italic => wrap(self, "<em>", "</em>"),
            Style::Underline => wrap(self, "<u>", "</u>"),
            Style::Code => wrap(self, "<code>", "</code>"),
            Style::Link(target) => {
                if is_external(target) {
                    let open = format!("<a href=\"{}\">", xml::attr(target.trim()));
                    wrap(self, &open, "</a>");
                } else {
                    // Links to files outside the book cannot resolve.
                    self.inlines(children);
                }
            }
            Style::FootnoteRef(id) => {
                let target = self.index.notes.get(id).cloned();
                match target {
                    Some((file, xid)) if self.in_note.as_deref() != Some(id.as_str()) => {
                        let href = if file == self.file {
                            format!("#{xid}")
                        } else {
                            format!("{file}#{xid}")
                        };
                        let open = format!(
                            "<a epub:type=\"noteref\" role=\"doc-noteref\" href=\"{}\">",
                            xml::attr(&href)
                        );
                        wrap(self, &open, "</a>");
                    }
                    _ => self.inlines(children),
                }
            }
            Style::Image(src) => {
                let img = Image {
                    alt: model::collapse_ws(&Inline::plain(children)),
                    src: src.clone(),
                };
                if let Some(href) = self.image_href(&img) {
                    self.out.push_str(&format!(
                        "<img src=\"{}\" alt=\"{}\"/>",
                        xml::attr(&href),
                        xml::attr(&img.alt)
                    ));
                } else if !img.alt.is_empty() {
                    self.missing_image(&img.alt);
                }
            }
        }
    }
}

fn is_external(target: &str) -> bool {
    let t = target.trim().to_ascii_lowercase();
    ["http://", "https://", "mailto:", "ftp://", "tel:"]
        .iter()
        .any(|p| t.starts_with(p))
        && !t.contains(char::is_whitespace)
}

fn html_root(lang: &str) -> String {
    let lang = xml::attr(lang);
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" lang=\"{lang}\" xml:lang=\"{lang}\">\n"
    )
}

fn content_document(facts: &Facts, title: &str, body: &str, labelled_by: Option<&str>) -> String {
    let mut s = html_root(&facts.language);
    s.push_str(&format!(
        "<head>\n<meta charset=\"UTF-8\"/>\n<title>{}</title>\n<link rel=\"stylesheet\" type=\"text/css\" href=\"style.css\"/>\n</head>\n<body epub:type=\"bodymatter\">\n",
        xml::text(title)
    ));
    match labelled_by {
        Some(id) => s.push_str(&format!(
            "<section epub:type=\"chapter\" role=\"doc-chapter\" aria-labelledby=\"{id}\">\n{body}</section>\n"
        )),
        None => s.push_str(&format!("<section>\n{body}</section>\n")),
    }
    s.push_str("</body>\n</html>\n");
    s
}

/// A table-of-contents entry.
struct TocNode {
    text: String,
    href: String,
    children: Vec<TocNode>,
}

fn toc_tree(index: &Index, chapters: &[Chapter], facts: &Facts) -> Vec<TocNode> {
    let entries: Vec<(u8, String, String)> = index
        .headings
        .iter()
        .filter(|(_, t, _, _)| !t.is_empty())
        .map(|(l, t, f, id)| (*l, t.clone(), format!("{f}#{id}")))
        .collect();
    if entries.is_empty() {
        // No headings: one entry per chapter.
        return chapters
            .iter()
            .enumerate()
            .map(|(n, c)| TocNode {
                text: c.title.clone().unwrap_or_else(|| {
                    if chapters.len() == 1 {
                        facts.title.clone()
                    } else {
                        format!("{} {}", facts.title, n + 1)
                    }
                }),
                href: chapter_file(n),
                children: Vec::new(),
            })
            .collect();
    }
    let mut i = 0;
    build_toc(&entries, &mut i, 0)
}

fn build_toc(entries: &[(u8, String, String)], i: &mut usize, min_level: u8) -> Vec<TocNode> {
    let mut nodes = Vec::new();
    while *i < entries.len() {
        let (level, text, href) = &entries[*i];
        if *level < min_level {
            break;
        }
        *i += 1;
        let children = if *i < entries.len() && entries[*i].0 > *level {
            build_toc(entries, i, level + 1)
        } else {
            Vec::new()
        };
        nodes.push(TocNode {
            text: text.clone(),
            href: href.clone(),
            children,
        });
    }
    nodes
}

fn nav_list(nodes: &[TocNode], out: &mut String) {
    out.push_str("<ol>\n");
    for n in nodes {
        out.push_str(&format!(
            "<li><a href=\"{}\">{}</a>",
            xml::attr(&n.href),
            xml::text(&n.text)
        ));
        if !n.children.is_empty() {
            out.push('\n');
            nav_list(&n.children, out);
        }
        out.push_str("</li>\n");
    }
    out.push_str("</ol>\n");
}

fn nav_document(facts: &Facts, toc: &[TocNode], pages: &[(String, String)]) -> String {
    let mut s = html_root(&facts.language);
    s.push_str(&format!(
        "<head>\n<meta charset=\"UTF-8\"/>\n<title>{}</title>\n<link rel=\"stylesheet\" type=\"text/css\" href=\"style.css\"/>\n</head>\n<body>\n",
        xml::text(&facts.title)
    ));
    s.push_str(
        "<nav epub:type=\"toc\" role=\"doc-toc\" id=\"toc\" aria-labelledby=\"toc-title\">\n<h1 id=\"toc-title\">Contents</h1>\n",
    );
    nav_list(toc, &mut s);
    s.push_str("</nav>\n");
    s.push_str("<nav epub:type=\"landmarks\" id=\"landmarks\" hidden=\"hidden\">\n<h2>Landmarks</h2>\n<ol>\n<li><a epub:type=\"toc\" href=\"nav.xhtml#toc\">Contents</a></li>\n");
    s.push_str(&format!(
        "<li><a epub:type=\"bodymatter\" href=\"{}\">Start of content</a></li>\n</ol>\n</nav>\n",
        chapter_file(0)
    ));
    if !pages.is_empty() {
        s.push_str("<nav epub:type=\"page-list\" id=\"page-list\" hidden=\"hidden\">\n<h2>Pages</h2>\n<ol>\n");
        for (label, href) in pages {
            s.push_str(&format!(
                "<li><a href=\"{}\">{}</a></li>\n",
                xml::attr(href),
                xml::text(label)
            ));
        }
        s.push_str("</ol>\n</nav>\n");
    }
    s.push_str("</body>\n</html>\n");
    s
}

fn ncx_points(nodes: &[TocNode], order: &mut usize, depth: usize, out: &mut String) {
    for n in nodes {
        *order += 1;
        let pad = "  ".repeat(depth + 2);
        out.push_str(&format!(
            "{pad}<navPoint id=\"nav-{o}\" playOrder=\"{o}\">\n{pad}  <navLabel><text>{}</text></navLabel>\n{pad}  <content src=\"{}\"/>\n",
            xml::text(&n.text),
            xml::attr(&n.href),
            o = *order
        ));
        ncx_points(&n.children, order, depth + 1, out);
        out.push_str(&format!("{pad}</navPoint>\n"));
    }
}

fn toc_depth(nodes: &[TocNode]) -> usize {
    nodes
        .iter()
        .map(|n| 1 + toc_depth(&n.children))
        .max()
        .unwrap_or(0)
}

fn ncx_document(facts: &Facts, toc: &[TocNode], uid: &str) -> String {
    let mut s = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<ncx xmlns=\"http://www.daisy.org/z3986/2005/ncx/\" version=\"2005-1\" xml:lang=\"{}\">\n  <head>\n    <meta name=\"dtb:uid\" content=\"{}\"/>\n    <meta name=\"dtb:depth\" content=\"{}\"/>\n    <meta name=\"dtb:totalPageCount\" content=\"0\"/>\n    <meta name=\"dtb:maxPageNumber\" content=\"0\"/>\n  </head>\n  <docTitle><text>{}</text></docTitle>\n  <navMap>\n",
        xml::attr(&facts.language),
        xml::attr(uid),
        toc_depth(toc).max(1),
        xml::text(&facts.title)
    );
    let mut order = 0;
    ncx_points(toc, &mut order, 0, &mut s);
    s.push_str("  </navMap>\n</ncx>\n");
    s
}

/// A stable `urn:uuid:` from the document's title and text (RFC 9562
/// version 8), so rewriting the same document keeps its identifier.
fn identifier(doc: &Document, facts: &Facts) -> String {
    let mut a: u64 = 0xcbf2_9ce4_8422_2325;
    let mut b: u64 = 0x8422_2325_cbf2_9ce4;
    let mut feed = |bytes: &[u8]| {
        for &x in bytes {
            a = (a ^ u64::from(x)).wrapping_mul(0x0000_0100_0000_01b3);
            b = (b ^ u64::from(x))
                .wrapping_mul(0x0000_0100_0000_01b3)
                .rotate_left(5);
        }
    };
    feed(facts.title.as_bytes());
    for chunk in doc.text().chunks() {
        feed(chunk.as_bytes());
    }
    let hex = format!("{a:016x}{b:016x}");
    format!(
        "urn:uuid:{}-{}-8{}-{}{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[13..16],
        // Variant bits 10xx.
        ["8", "9", "a", "b"][usize::from(hex.as_bytes()[16] % 4)],
        &hex[17..20],
        &hex[20..32]
    )
}

fn package_document(
    doc: &Document,
    options: &WriteOptions,
    facts: &Facts,
    files: &[(String, String)],
    images: &Images,
    fonts: &EmbeddedFonts,
    has_pages: bool,
) -> String {
    let uid = identifier(doc, facts);
    let modified = iso8601(timestamp(options));
    let mut m = String::new();
    m.push_str(&format!(
        "    <dc:identifier id=\"book-id\">{}</dc:identifier>\n    <dc:title>{}</dc:title>\n    <dc:language>{}</dc:language>\n",
        xml::text(&uid),
        xml::text(&facts.title),
        xml::text(&facts.language)
    ));
    if let Some(a) = &facts.author {
        m.push_str(&format!("    <dc:creator>{}</dc:creator>\n", xml::text(a)));
    }
    m.push_str(&format!(
        "    <meta property=\"dcterms:modified\">{modified}</meta>\n"
    ));
    let has_images = !images.files.is_empty();
    let mut meta = |p: &str, v: &str| {
        m.push_str(&format!(
            "    <meta property=\"{p}\">{}</meta>\n",
            xml::text(v)
        ));
    };
    meta("schema:accessMode", "textual");
    if has_images {
        meta("schema:accessMode", "visual");
        meta("schema:accessModeSufficient", "textual,visual");
    }
    meta("schema:accessModeSufficient", "textual");
    for f in ["structuralNavigation", "tableOfContents", "readingOrder"] {
        meta("schema:accessibilityFeature", f);
    }
    if has_images {
        meta("schema:accessibilityFeature", "alternativeText");
    }
    if has_pages {
        meta("schema:accessibilityFeature", "pageNavigation");
        meta("schema:accessibilityFeature", "printPageNumbers");
    }
    meta("schema:accessibilityHazard", "none");
    let mut summary = String::from(
        "This publication has a table of contents, headings for structural navigation, and text in reading order",
    );
    if has_images {
        summary.push_str("; images have text descriptions");
    }
    if has_pages {
        summary.push_str("; print page numbers are marked");
    }
    summary.push_str(". It was converted by textweaver and has not been evaluated against WCAG.");
    meta("schema:accessibilitySummary", &summary);

    let mut manifest = String::from(
        "    <item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n    <item id=\"ncx\" href=\"toc.ncx\" media-type=\"application/x-dtbncx+xml\"/>\n    <item id=\"css\" href=\"style.css\" media-type=\"text/css\"/>\n",
    );
    let mut spine = String::new();
    for (n, (name, _)) in files.iter().enumerate() {
        manifest.push_str(&format!(
            "    <item id=\"chapter-{}\" href=\"{name}\" media-type=\"application/xhtml+xml\"/>\n",
            n + 1
        ));
        spine.push_str(&format!("    <itemref idref=\"chapter-{}\"/>\n", n + 1));
    }
    for (n, (href, res)) in images.files.iter().enumerate() {
        manifest.push_str(&format!(
            "    <item id=\"image-{}\" href=\"{}\" media-type=\"{}\"/>\n",
            n + 1,
            xml::attr(href),
            res.kind.media_type()
        ));
    }
    for (id, href, media_type) in fonts.manifest() {
        manifest.push_str(&format!(
            "    <item id=\"{}\" href=\"{}\" media-type=\"{media_type}\"/>\n",
            xml::attr(&id),
            xml::attr(&href),
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"book-id\" xml:lang=\"{}\">\n  <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n{m}  </metadata>\n  <manifest>\n{manifest}  </manifest>\n  <spine toc=\"ncx\">\n{spine}  </spine>\n</package>\n",
        xml::attr(&facts.language)
    )
}

impl Chapter {
    fn label_with(&mut self, id: String) {
        self.labelled_by = Some(id);
    }
}

/// Fills each chapter's `labelled_by` with the id of its first heading when
/// the chapter starts with it.
fn label_chapters(chapters: &mut [Chapter], index: &Index) {
    let mut h = 0usize;
    for c in chapters.iter_mut() {
        let first_is_heading = c
            .blocks
            .iter()
            .find(|b| !matches!(b, Block::PageBreak { .. } | Block::SectionBreak { .. }))
            .is_some_and(|b| matches!(b, Block::Heading { .. }));
        let count = count_headings(&c.blocks);
        if first_is_heading && count > 0 {
            c.label_with(index.headings[h].3.clone());
        }
        h += count;
    }
}

fn count_headings(blocks: &[Block]) -> usize {
    blocks
        .iter()
        .map(|b| match b {
            Block::Heading { .. } => 1,
            Block::Quote(inner) => count_headings(inner),
            Block::List(l) => l.items.iter().map(|i| count_headings(&i.blocks)).sum(),
            _ => 0,
        })
        .sum()
}
