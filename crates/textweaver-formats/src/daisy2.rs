//! DAISY 2.02 books: a folder (or a zip) holding `ncc.html`, SMIL 1.0
//! files, and XHTML text.
//!
//! - **`ncc.html`**, the navigation control center, lists the book's
//!   headings (`h1`–`h6`) and print pages (a `span` whose class is
//!   `page-front`, `page-normal` or `page-special`), each a link into a SMIL
//!   file. Its `meta` tags give the title (`dc:title`), the authors
//!   (`dc:creator`), and the language (`dc:language`).
//! - **The SMIL files** give the reading order: the XHTML files are read
//!   through the HTML loader in the order the SMIL `text` elements point
//!   into them. A link to a `par`, to a `text`, or to the SMIL file itself
//!   lands on the text that `par` shows.
//! - **Sections and pages** use the same markers as a DAISY 3 book: each
//!   heading of the NCC starts a `SectionBreak` (label = the heading, level
//!   = its rank) where its text begins, and each print page number starts a
//!   `PageBreak` labeled with the printed number, which is not read aloud.
//!   Page numbers marked in the text by those classes are used first; else
//!   the NCC's page links say where each page starts.
//! - **A book with no text** (the audio books whose only text is the NCC)
//!   reads the NCC itself, its headings and pages, and carries
//!   [`NO_TEXT_WARNING`].
//!
//! Open `ncc.html` (the HTML loader hands it here), or the zip the book
//! came in (see [`ArchiveLoader`](crate::ArchiveLoader)). The recorded
//! audio is found by [`crate::book_audio()`].

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use ropey::Rope;
use scraper::{ElementRef, Html};
use textweaver_text::{Document, DocumentMeta};

use crate::builder::Builder;
use crate::daisy::{Collect, MAX_FILES, ReadFile, decode_xml, read_beside, read_smil, smil_target};
use crate::epub::{Sections, TocEntry, label_sections_by_heading};
use crate::html::{Pages, is_daisy_page};
use crate::package::{dir_of, parse_xml, resolve};
use crate::{LoadError, LoadOptions, Source, title_from_path};

/// The warning a DAISY 2.02 book with no text files carries.
pub const NO_TEXT_WARNING: &str = "This DAISY book has no text, only headings and its recording.";

/// The page number classes of DAISY 2.02.
const PAGE_CLASSES: [&str; 3] = ["page-front", "page-normal", "page-special"];

/// True for the name of a DAISY 2.02 navigation file: `ncc.html` in any
/// case (the specification allows `NCC.HTML` and `ncc.html`), or
/// `ncc.htm`.
pub fn is_ncc_name(name: &str) -> bool {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    base.eq_ignore_ascii_case("ncc.html") || base.eq_ignore_ascii_case("ncc.htm")
}

/// The DAISY 2.02 navigation file directly inside the folder `dir`, if it
/// holds one, so opening a book's folder opens the book.
pub fn ncc_in(dir: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| is_ncc_name(&n.to_string_lossy()))
        })
        .collect();
    // `ncc.html` before `ncc.htm`, whatever order the folder lists them.
    found.sort();
    found.pop()
}

/// True when `bytes`, opened from `source`, are a DAISY 2.02 NCC: by its
/// file name, or by an `ncc:` meta tag near its start.
pub(crate) fn is_ncc(source: &Source, bytes: &[u8]) -> bool {
    if let Source::Path(p) = source
        && is_ncc_name(&p.to_string_lossy())
    {
        return true;
    }
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(16 * 1024)]).to_ascii_lowercase();
    head.contains("name=\"ncc:") || head.contains("name='ncc:")
}

/// Loads the DAISY 2.02 book whose NCC is `source` (holding `bytes`); its
/// other files are read from the NCC's folder, which may be inside an
/// archive.
pub(crate) fn load_source(
    source: &Source,
    bytes: &[u8],
    options: &LoadOptions,
) -> Result<Document, LoadError> {
    let (name, folder) = match source {
        Source::Path(p) => (
            p.to_string_lossy()
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or("")
                .to_owned(),
            p.parent().map(Path::to_owned),
        ),
        _ => (String::from("ncc.html"), None),
    };
    let mut read = |rel: &str| read_beside(folder.as_deref(), rel);
    let mut doc = load_ncc(&name, bytes, &mut read, options, None)?;
    if let Source::Path(p) = source {
        doc.meta.path = Some(p.clone());
    }
    if doc.meta.title.is_none() {
        doc.meta.title = title_from_path(source);
    }
    Ok(doc)
}

/// A heading of the NCC.
struct Heading {
    level: u8,
    title: String,
    id: Option<String>,
    href: Option<String>,
}

/// What the NCC says.
#[derive(Default)]
struct Ncc {
    title: Option<String>,
    authors: Vec<String>,
    language: Option<String>,
    headings: Vec<Heading>,
    /// Print pages: the printed number and the link.
    pages: Vec<(String, String)>,
    /// Every heading and page link, in the NCC's order.
    links: Vec<String>,
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// HTML bytes as text, in their declared character set.
fn decode_html(bytes: &[u8]) -> String {
    let declared = crate::encoding::sniff_html_charset(bytes);
    crate::decode_bytes(bytes, declared.as_deref()).text
}

/// Reads the NCC with the HTML parser, so a page that is not quite XHTML
/// still reads.
fn parse_ncc(text: &str) -> Ncc {
    let html = Html::parse_document(text);
    let root = html.root_element();
    let mut ncc = Ncc {
        language: root
            .attr("lang")
            .or_else(|| root.attr("xml:lang"))
            .map(collapse)
            .filter(|l| !l.is_empty()),
        ..Ncc::default()
    };
    let mut page_title = None;
    for node in root.descendants() {
        let Some(el) = ElementRef::wrap(node) else {
            continue;
        };
        let text = || collapse(&el.text().collect::<String>());
        let href = || {
            el.descendants()
                .filter_map(ElementRef::wrap)
                .find(|a| a.value().name() == "a")
                .and_then(|a| a.attr("href"))
                .map(str::to_owned)
        };
        let name = el.value().name();
        match name {
            "meta" => {
                let (Some(key), Some(content)) = (el.attr("name"), el.attr("content")) else {
                    continue;
                };
                let content = collapse(content);
                if content.is_empty() {
                    continue;
                }
                match key.to_ascii_lowercase().as_str() {
                    "dc:title" if ncc.title.is_none() => ncc.title = Some(content),
                    "dc:creator" => ncc.authors.push(content),
                    "dc:language" => ncc.language = Some(content),
                    _ => {}
                }
            }
            "title" if page_title.is_none() => {
                page_title = Some(text()).filter(|t| !t.is_empty());
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let href = href();
                if let Some(h) = &href {
                    ncc.links.push(h.clone());
                }
                ncc.headings.push(Heading {
                    level: name[1..].parse().unwrap_or(1),
                    title: text(),
                    id: el.attr("id").map(str::to_owned),
                    href,
                });
            }
            "span" if is_daisy_page(&el) => {
                if let Some(h) = href() {
                    ncc.links.push(h.clone());
                    ncc.pages.push((text(), h));
                }
            }
            _ => {}
        }
    }
    if ncc.title.is_none() {
        ncc.title = page_title;
    }
    ncc
}

/// True for a path the HTML loader reads (a DAISY 2.02 text file).
fn is_content(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [".html", ".htm", ".xhtml", ".xht"]
        .iter()
        .any(|e| lower.ends_with(e))
}

/// Loads a DAISY 2.02 book from its NCC. `ncc_path` is the NCC's path
/// among the book's files, and `read` reads another of them.
pub(crate) fn load_ncc(
    ncc_path: &str,
    ncc_bytes: &[u8],
    read: &mut ReadFile<'_>,
    options: &LoadOptions,
    mut collect: Option<&mut Collect>,
) -> Result<Document, LoadError> {
    let ncc_text = decode_html(ncc_bytes);
    let ncc = parse_ncc(&ncc_text);
    let mut meta = DocumentMeta {
        format: "daisy".into(),
        title: ncc.title.clone(),
        author: (!ncc.authors.is_empty()).then(|| ncc.authors.join(", ")),
        language: ncc.language.clone(),
        ..DocumentMeta::default()
    };
    let base = dir_of(ncc_path).to_owned();

    // The SMIL files, in the order the NCC first names them.
    let mut smil_files = Vec::new();
    let mut named = HashSet::new();
    for href in &ncc.links {
        let file = resolve(&base, href).0;
        if !file.is_empty()
            && !is_content(&file)
            && smil_files.len() < MAX_FILES
            && named.insert(file.clone())
        {
            smil_files.push(file);
        }
    }
    let mut order = Vec::new();
    let mut seen = HashSet::new();
    let mut smil = HashMap::new();
    for file in &smil_files {
        let Some(bytes) = read(file)? else { continue };
        let text = decode_xml(&bytes, &mut meta);
        let Ok(xml) = parse_xml(&text) else { continue };
        let targets = read_smil(&xml, dir_of(file), &mut order, &mut seen);
        smil.insert(file.clone(), targets);
        if let Some(c) = collect.as_deref_mut() {
            c.smils.push((file.clone(), text.clone()));
        }
    }
    // Only text files, and never the NCC again (a SMIL pointing back at it).
    order.retain(|f| is_content(f) && !f.eq_ignore_ascii_case(ncc_path));

    let target = |href: &str| {
        let (file, frag) = resolve(&base, href);
        smil_target(&smil, &file, frag.as_deref(), is_content(&file))
    };
    let toc: Vec<TocEntry> = ncc
        .headings
        .iter()
        .filter_map(|h| {
            let (file, fragment) = target(h.href.as_deref()?)?;
            Some(TocEntry {
                title: h.title.clone(),
                file,
                fragment,
                depth: h.level,
            })
        })
        .collect();
    let mut pages_by_file: HashMap<String, HashMap<String, String>> = HashMap::new();
    for (label, href) in &ncc.pages {
        if let Some((file, Some(frag))) = target(href) {
            pages_by_file
                .entry(file)
                .or_default()
                .entry(frag)
                .or_insert_with(|| label.clone());
        }
    }

    let mut b = Builder::new();
    let mut sections = Sections { open: Vec::new() };
    let mut pages = Pages::default();
    pages.by_class = true;
    let mut read_any = false;
    for file in &order {
        let Some(bytes) = read(file)? else { continue };
        read_any = true;
        let text = decode_html(&bytes);
        // Page numbers marked in the text win over the NCC's page links,
        // so a page never starts twice.
        let by_id = if PAGE_CLASSES.iter().any(|c| text.contains(c)) {
            HashMap::new()
        } else {
            pages_by_file.remove(file).unwrap_or_default()
        };
        let entries: Vec<&TocEntry> = toc.iter().filter(|e| &e.file == file).collect();
        let chapter = Chapter {
            text: &text,
            file,
            toc: &entries,
            by_id,
        };
        pages = chapter.read(
            &mut b,
            &mut sections,
            pages,
            options,
            &mut meta,
            collect.as_deref_mut(),
        );
    }
    if !read_any {
        // No text: the NCC's own headings and pages.
        crate::add_warning(&mut meta, NO_TEXT_WARNING);
        let own: Vec<TocEntry> = ncc
            .headings
            .iter()
            .map(|h| TocEntry {
                title: h.title.clone(),
                file: ncc_path.to_owned(),
                fragment: h.id.clone(),
                depth: h.level,
            })
            .collect();
        let entries: Vec<&TocEntry> = own.iter().collect();
        if let Some(c) = collect.as_deref_mut() {
            // Each heading leads to its place in the audio.
            for h in &ncc.headings {
                if let (Some(id), Some(href)) = (&h.id, &h.href) {
                    c.nav
                        .push(((ncc_path.to_owned(), id.clone()), resolve(&base, href)));
                }
            }
        }
        let chapter = Chapter {
            text: &ncc_text,
            file: ncc_path,
            toc: &entries,
            by_id: HashMap::new(),
        };
        pages = chapter.read(&mut b, &mut sections, pages, options, &mut meta, collect);
    }
    pages.close(&mut b);
    let (text, mut markers) = b.finish();
    label_sections_by_heading(&text, &mut markers);
    Ok(Document::new(meta, Rope::from_str(&text), markers))
}

/// One text file of the book.
/// Reads `html`, the content file `file`, as one chapter with no page
/// list: `toc` entries open sections where their ids are, and `collect`
/// notes where each id starts.
#[allow(clippy::too_many_arguments)]
pub(crate) fn read_chapter(
    b: &mut Builder,
    sections: &mut Sections,
    html: &str,
    file: &str,
    toc: &[&TocEntry],
    options: &LoadOptions,
    meta: &mut DocumentMeta,
    collect: Option<&mut Collect>,
) {
    let chapter = Chapter {
        text: html,
        file,
        toc,
        by_id: HashMap::new(),
    };
    let mut pages = chapter.read(b, sections, Pages::default(), options, meta, collect);
    pages.close(b);
}

struct Chapter<'a> {
    text: &'a str,
    /// Its path among the book's files.
    file: &'a str,
    /// The sections that start in it.
    toc: &'a [&'a TocEntry],
    /// Page labels by element id, from the NCC.
    by_id: HashMap<String, String>,
}

impl Chapter<'_> {
    /// Reads the file into `b` through the HTML loader's rules: its
    /// sections open at the elements they name (or at its start), and
    /// `pages` carries the open print page on into the next file;
    /// `collect` notes where each id starts.
    fn read(
        self,
        b: &mut Builder,
        sections: &mut Sections,
        mut pages: Pages,
        options: &LoadOptions,
        meta: &mut DocumentMeta,
        mut collect: Option<&mut Collect>,
    ) -> Pages {
        let ids = crate::html::anchor_ids(self.text);
        let mut anchored: HashMap<&str, Vec<&TocEntry>> = HashMap::new();
        b.paragraph_break();
        for &e in self.toc {
            match e.fragment.as_deref() {
                Some(f) if ids.contains(f) => anchored.entry(f).or_default().push(e),
                _ => sections.open(b, e),
            }
        }
        pages.by_id = self.by_id;
        let mut scratch = DocumentMeta::default();
        let file = self.file;
        let mut hook = |id: &str, b: &mut Builder| {
            if let Some(c) = collect.as_deref_mut() {
                c.note(file, id, b.next_start());
            }
            if let Some(es) = anchored.remove(id) {
                for e in es {
                    sections.open(b, e);
                }
            }
        };
        let pages = crate::html::walk_epub_into(
            b,
            self.text,
            options,
            &mut scratch,
            Some(&mut hook),
            pages,
        );
        for w in crate::warnings(&scratch) {
            crate::add_warning(meta, &w);
        }
        pages
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::PathBuf;

    use textweaver_core::MarkerKind;

    use super::*;
    use crate::{Loader, Registry};

    const BOOK: &str = "../../fixtures/r2/daisy202";
    const FILES: [&str; 5] = [
        "ncc.html",
        "ch1.smil",
        "ch2.smil",
        "content1.html",
        "content2.html",
    ];

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(BOOK)
            .join(name)
    }

    fn texts(d: &Document, kind: MarkerKind) -> Vec<(String, u8, Option<String>)> {
        d.marker_index()
            .iter(kind, None)
            .map(|m| (d.slice(m.range).to_string(), m.level, m.label.clone()))
            .collect()
    }

    fn check_book(d: &Document) {
        assert_eq!(d.meta.format, "daisy");
        assert_eq!(d.meta.title.as_deref(), Some("A Small Talking Book"));
        assert_eq!(d.meta.author.as_deref(), Some("Ada Example"));
        assert_eq!(d.meta.language.as_deref(), Some("en"));
        assert!(crate::warnings(&d.meta).is_empty());
        let text = d.text().to_string();
        assert_eq!(
            text,
            "A Small Talking Book\n\nA note before the book begins.\n\nChapter One\n\n\
             The first page of the first chapter.\n\nA Smaller Part\n\n\
             Text before the second page.\n\nThe second page starts here.\n\n\
             Chapter Two\n\nThe last words."
        );
        let sections: Vec<_> = texts(d, MarkerKind::SectionBreak)
            .into_iter()
            .map(|(_, level, label)| (label.unwrap_or_default(), level))
            .collect();
        assert_eq!(
            sections,
            [
                ("A Small Talking Book".to_owned(), 1),
                ("Chapter One".to_owned(), 1),
                ("A Smaller Part".to_owned(), 2),
                ("Chapter Two".to_owned(), 1),
            ]
        );
        let pages = texts(d, MarkerKind::PageBreak);
        let labels: Vec<_> = pages
            .iter()
            .map(|p| p.2.clone().unwrap_or_default())
            .collect();
        assert_eq!(labels, ["i", "1", "2"]);
        assert!(pages[0].0.starts_with("A note before"), "{pages:?}");
        assert!(pages[1].0.starts_with("The first page"), "{pages:?}");
        // Page 2 comes from the NCC's link, through the SMIL file.
        assert!(pages[2].0.starts_with("The second page"), "{pages:?}");
        assert!(pages[2].0.ends_with("The last words."), "{pages:?}");
    }

    #[test]
    fn ncc_html_in_a_folder_opens_the_book() {
        let d = Registry::with_builtins()
            .load(&Source::Path(fixture("ncc.html")), &LoadOptions::default())
            .unwrap();
        check_book(&d);
        assert_eq!(d.meta.path, Some(fixture("ncc.html")));
    }

    #[test]
    fn the_books_folder_opens_its_ncc() {
        let folder = fixture("ncc.html").parent().unwrap().to_path_buf();
        let d = Registry::with_builtins()
            .load(&Source::Path(folder), &LoadOptions::default())
            .unwrap();
        check_book(&d);
        assert_eq!(d.meta.path, Some(fixture("ncc.html")));
        // A folder without one is still not a document.
        let empty = tempfile::tempdir().unwrap();
        assert!(ncc_in(empty.path()).is_none());
    }

    #[test]
    fn a_zip_opens_the_book_under_any_case_of_ncc() {
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for name in FILES {
            let inside = if name == "ncc.html" { "NCC.HTML" } else { name };
            z.start_file(
                format!("Book/{inside}"),
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            z.write_all(&std::fs::read(fixture(name)).unwrap()).unwrap();
        }
        let data = z.finish().unwrap().into_inner();
        let d = crate::ArchiveLoader
            .load(
                &Source::Bytes {
                    data,
                    hint: "zip".into(),
                },
                &LoadOptions::default(),
            )
            .unwrap();
        check_book(&d);
    }

    #[test]
    fn a_book_with_only_its_ncc_reads_the_headings() {
        // As bytes, with no folder: found by its `ncc:` meta tag.
        let data = std::fs::read(fixture("ncc.html")).unwrap();
        let d = crate::HtmlLoader
            .load(
                &Source::Bytes {
                    data,
                    hint: "html".into(),
                },
                &LoadOptions::default(),
            )
            .unwrap();
        assert_eq!(d.meta.format, "daisy");
        assert_eq!(crate::warnings(&d.meta), [NO_TEXT_WARNING]);
        let text = d.text().to_string();
        assert!(text.contains("Chapter One"), "{text}");
        assert!(!text.contains("\ni\n"), "page numbers are not read: {text}");
        let sections = texts(&d, MarkerKind::SectionBreak);
        assert_eq!(sections.len(), 4);
        assert_eq!(sections[2].1, 2);
        assert_eq!(texts(&d, MarkerKind::PageBreak).len(), 3);
    }

    #[test]
    fn other_html_is_not_an_ncc() {
        let page = Source::Bytes {
            data: b"<html><body><p>Hello.</p></body></html>".to_vec(),
            hint: "html".into(),
        };
        let d = crate::HtmlLoader
            .load(&page, &LoadOptions::default())
            .unwrap();
        assert_eq!(d.meta.format, "html");
        assert!(is_ncc_name("Book/NCC.HTML"));
        assert!(is_ncc_name("ncc.htm"));
        assert!(!is_ncc_name("syncc.html"));
    }
}
