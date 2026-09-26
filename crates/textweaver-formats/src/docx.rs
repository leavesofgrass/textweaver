//! DOCX loader: `word/document.xml` (Office Open XML WordprocessingML) into
//! canonical text and markers, without Pandoc.
//!
//! Star sent DOCX through Pandoc to Markdown when Pandoc was installed, and
//! on Windows decoded Pandoc's UTF-8 output with the ANSI code page, which
//! corrupted every non-ASCII letter (docs/star-parity.md, Part 1 §1.5).
//! This loader reads the package directly:
//!
//! - **Headings**: paragraph styles named `heading 1`–`heading 9` (levels
//!   above 6 read as 6), `Title` as level 1, or an outline level on the
//!   paragraph or its style chain (`w:basedOn`). A style based on `Title`
//!   (Pandoc's `Author`, `Subtitle`) is not a heading.
//! - **Code**: paragraphs in a code style (`Source Code`, `HTML
//!   Preformatted`, `Macro Text`, `Code`) keep their spacing and line breaks
//!   under a `Code` block marker, consecutive ones forming one block
//!   (dropped with [`LoadOptions::skip_code`]); runs in a character style
//!   named like `Verbatim Char` or `HTML Code` are inline code.
//! - **Lists**: `w:numPr` on the paragraph or its style; the level is
//!   `ilvl + 1`; numbered levels get their label from `numbering.xml`
//!   (`lvlText` with decimal, letter, and roman counters, honoring `start`
//!   and restarting deeper levels), bullets have none. Consecutive items
//!   form `List` markers, nested by level.
//! - **Runs**: bold, italic, and underline (direct formatting or a character
//!   style) become `Bold`, `Italic`, and `Underline` markers spanning runs
//!   with the same setting; tabs read as spaces, line breaks break the line,
//!   non-breaking hyphens are hyphens, deleted revisions and field codes are
//!   not read, inserted revisions are.
//! - **Hyperlinks** become `Link` markers with the relationship's target (or
//!   `#bookmark`).
//! - **Images** (`w:drawing`) read as their alt text (`wp:docPr` `descr`,
//!   else `title`) under an `Image` marker; images without alt text are
//!   decorative and read as nothing, as in the HTML loader.
//! - **Tables** stay in place, one row per line, cells separated by
//!   [`CELL_SEPARATOR`](crate::CELL_SEPARATOR); the first row is the header
//!   when the table's look says so (`w:tblLook` first row, Word's default)
//!   or the row repeats as a header (`w:tblHeader`). A table in a cell reads
//!   as running text.
//! - **Footnotes and endnotes** are numbered in reading order and follow
//!   [`LoadOptions::footnotes`] like Markdown footnotes.
//! - **Metadata** from `docProps/core.xml` (title, creator, language); the
//!   title falls back to the first level-1 heading, then the file name.

use std::collections::HashMap;

use ropey::Rope;
use roxmltree::Node;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, HEADER_ROW_LABEL, Marker};

use crate::builder::{Builder, OpenId};
use crate::package::{Package, child, parse_xml};
use crate::{FootnoteMode, LoadError, LoadOptions, Loader, Source, meta_for, title_from_path};

/// Loads Word documents (`.docx`, and macro-enabled `.docm`).
#[derive(Clone, Copy, Debug, Default)]
pub struct DocxLoader;

impl Loader for DocxLoader {
    fn id(&self) -> &'static str {
        "docx"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["docx", "docm"]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        let mut pkg = Package::open(source.read()?, "DOCX")?;
        let body_xml = pkg
            .read_text("word/document.xml")?
            .ok_or_else(|| LoadError::Parse("not a Word document: no word/document.xml".into()))?;
        let part = |pkg: &mut Package, name: &str| pkg.read_text(name);
        let styles = part(&mut pkg, "word/styles.xml")?
            .map(|t| Styles::parse(&t))
            .transpose()?
            .unwrap_or_default();
        let numbering = part(&mut pkg, "word/numbering.xml")?
            .map(|t| Numbering::parse(&t))
            .transpose()?
            .unwrap_or_default();
        let rels = part(&mut pkg, "word/_rels/document.xml.rels")?
            .map(|t| relationships(&t))
            .transpose()?
            .unwrap_or_default();
        let mut notes = HashMap::new();
        for (name, kind) in [
            ("word/footnotes.xml", "footnote"),
            ("word/endnotes.xml", "endnote"),
        ] {
            if let Some(t) = part(&mut pkg, name)? {
                read_notes(&t, kind, &mut notes)?;
            }
        }
        let mut meta = meta_for(source, self.id());
        if let Some(core) = part(&mut pkg, "docProps/core.xml")? {
            let xml = parse_xml(&core)?;
            for n in xml.descendants().filter(|n| n.is_element()) {
                let t = collapse(&n.descendants().filter_map(|d| d.text()).collect::<String>());
                if t.is_empty() {
                    continue;
                }
                match n.tag_name().name() {
                    "title" => meta.title = Some(t),
                    "creator" => meta.author = Some(t),
                    "language" => meta.language = Some(t),
                    _ => {}
                }
            }
        }

        let xml = parse_xml(&body_xml)?;
        let body = xml
            .descendants()
            .find(|n| n.tag_name().name() == "body")
            .ok_or_else(|| LoadError::Parse("word/document.xml has no body".into()))?;
        let mut c = Conv {
            b: Builder::new(),
            options,
            styles: &styles,
            numbering: &numbering,
            rels: &rels,
            notes: &notes,
            lists: Vec::new(),
            counters: HashMap::new(),
            in_cell: 0,
            fmt: [None; 4],
            code: None,
            deferred: Vec::new(),
            note_count: 0,
            depth: 0,
            flattened: false,
        };
        c.blocks(body);
        c.close_code();
        c.close_lists();
        let deferred = std::mem::take(&mut c.deferred);
        c.b.footnotes_section(&deferred);
        let flattened = c.flattened || pkg.flattened();
        let (text, markers) = c.b.finish();
        if flattened {
            crate::add_warning(&mut meta, crate::NESTING_WARNING);
        }
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

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The value of attribute `local` (any namespace).
fn attr<'a>(n: Node<'a, '_>, local: &str) -> Option<&'a str> {
    n.attributes()
        .find(|a| a.name() == local)
        .map(|a| a.value())
}

/// `w:val` of child `name`.
fn child_val<'a>(n: Node<'a, '_>, name: &str) -> Option<&'a str> {
    child(n, name).and_then(|c| attr(c, "val"))
}

/// A toggle property (`<w:b/>`, `<w:b w:val="0"/>`) if present.
fn toggle(rpr: Node<'_, '_>, name: &str) -> Option<bool> {
    let c = child(rpr, name)?;
    Some(!matches!(
        attr(c, "val"),
        Some("0" | "false" | "off" | "none")
    ))
}

#[derive(Clone, Debug, Default)]
struct StyleInfo {
    name: String,
    based_on: Option<String>,
    outline: Option<u8>,
    num: Option<(String, u8)>,
    /// Bold, italic, underline, and inline code set by the style.
    fmt: [Option<bool>; 4],
}

#[derive(Debug, Default)]
struct Styles {
    map: HashMap<String, StyleInfo>,
}

impl Styles {
    fn parse(text: &str) -> Result<Self, LoadError> {
        let xml = parse_xml(text)?;
        let mut map = HashMap::new();
        for s in xml.descendants().filter(|n| n.tag_name().name() == "style") {
            let Some(id) = attr(s, "styleId") else {
                continue;
            };
            let mut info = StyleInfo {
                name: child_val(s, "name").unwrap_or("").to_lowercase(),
                based_on: child_val(s, "basedOn").map(str::to_owned),
                ..StyleInfo::default()
            };
            if let Some(ppr) = child(s, "pPr") {
                info.outline = child_val(ppr, "outlineLvl").and_then(|v| v.parse().ok());
                info.num = num_pr(ppr);
            }
            if let Some(rpr) = child(s, "rPr") {
                info.fmt = run_fmt(rpr);
            }
            if attr(s, "type") == Some("character")
                && (info.name.contains("verbatim") || info.name.contains("code"))
            {
                info.fmt[3] = Some(true);
            }
            map.insert(id.to_owned(), info);
        }
        Ok(Styles { map })
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
            cur = s.based_on.as_deref();
        }
        out
    }

    fn heading_level(&self, id: Option<&str>) -> Option<u8> {
        let chain = self.chain(id);
        // Names count for the paragraph's own style only: `Author` is based
        // on `Title` but is not a heading. Outline levels are inherited.
        if let Some(own) = chain.first() {
            if let Some(n) = own.name.strip_prefix("heading ")
                && let Ok(n) = n.trim().parse::<u8>()
            {
                return Some(n.clamp(1, 6));
            }
            if own.name == "title" {
                return Some(1);
            }
        }
        chain
            .iter()
            .find_map(|s| s.outline)
            .filter(|&o| o < 9)
            .map(|o| (o + 1).min(6))
    }

    fn is_code(&self, id: Option<&str>) -> bool {
        self.chain(id).iter().any(|s| {
            matches!(
                s.name.as_str(),
                "source code" | "html preformatted" | "macro text" | "code" | "code block"
            )
        })
    }

    fn num(&self, id: Option<&str>) -> Option<(String, u8)> {
        self.chain(id).into_iter().find_map(|s| s.num.clone())
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
}

fn run_fmt(rpr: Node<'_, '_>) -> [Option<bool>; 4] {
    [toggle(rpr, "b"), toggle(rpr, "i"), toggle(rpr, "u"), None]
}

fn num_pr(ppr: Node<'_, '_>) -> Option<(String, u8)> {
    let np = child(ppr, "numPr")?;
    let id = child_val(np, "numId")?.to_owned();
    let lvl = child_val(np, "ilvl")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    Some((id, lvl))
}

#[derive(Clone, Debug, Default)]
struct Level {
    fmt: String,
    text: String,
    /// Clamped to `1..=MAX_COUNTER` when read.
    start: u64,
}

#[derive(Debug, Default)]
struct Numbering {
    /// numId → (abstractNumId, start overrides by level).
    nums: HashMap<String, (String, HashMap<u8, u64>)>,
    abstracts: HashMap<String, HashMap<u8, Level>>,
}

impl Numbering {
    fn parse(text: &str) -> Result<Self, LoadError> {
        let xml = parse_xml(text)?;
        let mut n = Numbering::default();
        for a in xml
            .descendants()
            .filter(|n| n.tag_name().name() == "abstractNum")
        {
            let Some(id) = attr(a, "abstractNumId") else {
                continue;
            };
            let mut levels = HashMap::new();
            for l in a.children().filter(|n| n.tag_name().name() == "lvl") {
                let Some(ilvl) = attr(l, "ilvl").and_then(|v| v.parse().ok()) else {
                    continue;
                };
                levels.insert(
                    ilvl,
                    Level {
                        fmt: child_val(l, "numFmt").unwrap_or("decimal").to_owned(),
                        text: child_val(l, "lvlText").unwrap_or("").to_owned(),
                        start: child_val(l, "start").map_or(1, parse_counter),
                    },
                );
            }
            n.abstracts.insert(id.to_owned(), levels);
        }
        for num in xml.descendants().filter(|n| n.tag_name().name() == "num") {
            let (Some(id), Some(abs)) = (attr(num, "numId"), child_val(num, "abstractNumId"))
            else {
                continue;
            };
            let mut overrides = HashMap::new();
            for o in num
                .children()
                .filter(|n| n.tag_name().name() == "lvlOverride")
            {
                if let (Some(l), Some(s)) = (
                    attr(o, "ilvl").and_then(|v| v.parse().ok()),
                    child_val(o, "startOverride").map(parse_counter),
                ) {
                    overrides.insert(l, s);
                }
            }
            n.nums.insert(id.to_owned(), (abs.to_owned(), overrides));
        }
        Ok(n)
    }

    fn level(&self, num: &str, ilvl: u8) -> Option<Level> {
        let (abs, overrides) = self.nums.get(num)?;
        let mut l = self.abstracts.get(abs)?.get(&ilvl)?.clone();
        if let Some(&s) = overrides.get(&ilvl) {
            l.start = s;
        }
        Some(l)
    }
}

/// A start value from `numbering.xml`, clamped: Word caps starts at
/// 32,767, but a hostile file can claim any integer.
fn parse_counter(v: &str) -> u64 {
    v.trim().parse::<i128>().map_or(1, crate::counter::clamp)
}

/// `n` formatted as a list counter of format `fmt`.
fn counter(n: u64, fmt: &str) -> String {
    use crate::counter::{decimal, letters, roman};
    match fmt {
        "lowerLetter" => letters(n).to_lowercase(),
        "upperLetter" => letters(n),
        "lowerRoman" => roman(n).to_lowercase(),
        "upperRoman" => roman(n),
        _ => decimal(n),
    }
}

fn relationships(text: &str) -> Result<HashMap<String, String>, LoadError> {
    let xml = parse_xml(text)?;
    Ok(xml
        .descendants()
        .filter(|n| n.tag_name().name() == "Relationship")
        .filter_map(|n| Some((attr(n, "Id")?.to_owned(), attr(n, "Target")?.to_owned())))
        .collect())
}

/// Reads footnote or endnote bodies into `out`, keyed `"footnote:ID"`.
fn read_notes(text: &str, kind: &str, out: &mut HashMap<String, String>) -> Result<(), LoadError> {
    let xml = parse_xml(text)?;
    for note in xml.descendants().filter(|n| n.tag_name().name() == kind) {
        if attr(note, "type").is_some_and(|t| t != "normal") {
            continue;
        }
        let Some(id) = attr(note, "id") else {
            continue;
        };
        let mut parts = Vec::new();
        for p in note.descendants().filter(|n| n.tag_name().name() == "p") {
            let t: String = p
                .descendants()
                .filter(|n| n.tag_name().name() == "t" && !in_deletion(*n))
                .filter_map(|n| n.text())
                .collect();
            let t = collapse(&t);
            if !t.is_empty() {
                parts.push(t);
            }
        }
        out.insert(format!("{kind}:{id}"), parts.join(" "));
    }
    Ok(())
}

fn in_deletion(n: Node<'_, '_>) -> bool {
    n.ancestors()
        .any(|a| matches!(a.tag_name().name(), "del" | "moveFrom"))
}

struct Conv<'a> {
    b: Builder,
    options: &'a LoadOptions,
    styles: &'a Styles,
    numbering: &'a Numbering,
    rels: &'a HashMap<String, String>,
    notes: &'a HashMap<String, String>,
    /// Open list markers with their depth and numbering instance, innermost
    /// last.
    lists: Vec<(u8, String, OpenId)>,
    /// Counters per numbering instance and level.
    counters: HashMap<String, [u64; 9]>,
    in_cell: usize,
    /// Open bold, italic, underline, and inline code markers.
    fmt: [Option<OpenId>; 4],
    /// The open code block (consecutive code paragraphs).
    code: Option<OpenId>,
    deferred: Vec<(String, String)>,
    note_count: usize,
    /// Nesting of `blocks` and `inline` calls, bounded by
    /// [`MAX_NESTING`](crate::MAX_NESTING).
    depth: usize,
    /// Set once content past the nesting limit was flattened.
    flattened: bool,
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

    /// Enters one level of nesting, or returns false at the limit.
    fn enter(&mut self) -> bool {
        if self.depth >= crate::MAX_NESTING {
            return false;
        }
        self.depth += 1;
        true
    }

    /// Content nested past the limit: its text, runs separated by spaces,
    /// without structure. Iterative, so no depth can overflow the stack.
    fn flatten(&mut self, node: Node<'_, '_>) {
        self.flattened = true;
        self.set_fmt([false; 4]);
        self.b.space();
        for n in node.descendants() {
            if n.is_text() {
                // Run text (`w:t`), and the spaces left between flattened
                // elements; other elements' text (field codes, deleted
                // revisions) is not read.
                let parent = n.parent().map(|p| p.tag_name().name());
                if matches!(parent, Some("t" | crate::xmldepth::FLAT_ELEMENT)) {
                    self.b.text(n.text().unwrap_or(""));
                }
                continue;
            }
            if matches!(n.tag_name().name(), "p" | "tab" | "br" | "cr" | "tc") {
                self.b.space();
            }
        }
        self.b.space();
    }

    fn blocks(&mut self, node: Node<'_, '_>) {
        if !self.enter() {
            self.flatten(node);
            return;
        }
        self.blocks_inner(node);
        self.depth -= 1;
    }

    fn blocks_inner(&mut self, node: Node<'_, '_>) {
        for c in node.children().filter(Node::is_element) {
            match c.tag_name().name() {
                crate::xmldepth::FLAT_ELEMENT => self.flatten(c),
                "p" => self.paragraph(c),
                "tbl" => self.table(c),
                "sdt" => {
                    if let Some(content) = child(c, "sdtContent") {
                        self.blocks(content);
                    }
                }
                "customXml" | "ins" | "moveTo" | "tc" | "txbxContent" => self.blocks(c),
                "AlternateContent" => {
                    if let Some(choice) = child(c, "Choice") {
                        self.blocks(choice);
                    }
                }
                _ => {}
            }
        }
    }

    fn close_lists(&mut self) {
        self.close_lists_from(1);
    }

    /// Closes the open lists at `depth` and deeper.
    fn close_lists_from(&mut self, depth: u8) {
        while self.lists.last().is_some_and(|(d, _, _)| *d >= depth) {
            if let Some((_, _, id)) = self.lists.pop() {
                self.b.close(id);
            }
        }
    }

    fn close_code(&mut self) {
        if let Some(id) = self.code.take() {
            self.b.close(id);
            self.b.paragraph_break();
        }
    }

    /// A paragraph in a code style: its text verbatim, joined to the open
    /// code block.
    fn code_paragraph(&mut self, p: Node<'_, '_>) {
        self.close_lists();
        if self.options.skip_code {
            return;
        }
        let mut text = String::new();
        for n in p.descendants().filter(|n| !in_deletion(*n)) {
            match n.tag_name().name() {
                "t" => text.push_str(n.text().unwrap_or("")),
                "tab" => text.push('\t'),
                "br" | "cr" => text.push('\n'),
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

    fn paragraph(&mut self, p: Node<'_, '_>) {
        let ppr = child(p, "pPr");
        let style = ppr.and_then(|n| child_val(n, "pStyle"));
        let outline = ppr
            .and_then(|n| child_val(n, "outlineLvl"))
            .and_then(|v| v.parse::<u8>().ok())
            .filter(|&o| o < 9)
            .map(|o| (o + 1).min(6));
        let heading = outline.or_else(|| self.styles.heading_level(style));
        let num = ppr
            .and_then(num_pr)
            .or_else(|| self.styles.num(style))
            .filter(|(id, _)| id != "0");

        if self.in_cell == 0 && self.styles.is_code(style) {
            self.code_paragraph(p);
            return;
        }
        self.close_code();
        if self.in_cell > 0 {
            // Paragraphs in a cell run together with spaces.
            self.b.space();
            self.inline(p);
            self.set_fmt([false; 4]);
            return;
        }
        if let Some(level) = heading {
            self.close_lists();
            self.b.paragraph_break();
            let id = self
                .b
                .open(Self::marker(MarkerKind::Heading).with_level(level));
            self.inline(p);
            self.set_fmt([false; 4]);
            self.b.close(id);
            self.b.paragraph_break();
        } else if let Some((num_id, ilvl)) = num {
            let depth = ilvl.min(8) + 1;
            self.close_lists_from(depth + 1);
            // Another numbering instance at this depth is another list.
            if self
                .lists
                .last()
                .is_some_and(|(d, n, _)| *d == depth && *n != num_id)
            {
                self.close_lists_from(depth);
            }
            if self.lists.is_empty() {
                self.b.paragraph_break();
            }
            while self.lists.last().is_none_or(|(d, _, _)| *d < depth) {
                let d = self.lists.last().map_or(1, |(d, _, _)| d + 1);
                let id = self.b.open(Self::marker(MarkerKind::List).with_level(d));
                self.lists.push((d, num_id.clone(), id));
            }
            self.b.line_break();
            let mut m = Self::marker(MarkerKind::ListItem).with_level(depth);
            if let Some(label) = self.list_label(&num_id, ilvl) {
                m = m.with_label(label);
            }
            let id = self.b.open(m);
            self.inline(p);
            self.set_fmt([false; 4]);
            self.b.close(id);
        } else {
            self.close_lists();
            self.b.paragraph_break();
            let id = self.b.open(Self::marker(MarkerKind::Paragraph));
            self.inline(p);
            self.set_fmt([false; 4]);
            self.b.close(id);
            self.b.paragraph_break();
        }
    }

    /// Advances the counter for an item and returns its label (`None` for
    /// bullets).
    fn list_label(&mut self, num_id: &str, ilvl: u8) -> Option<String> {
        let ilvl = ilvl.min(8);
        let level = self.numbering.level(num_id, ilvl);
        let counters = self.counters.entry(num_id.to_owned()).or_insert([0; 9]);
        let i = usize::from(ilvl);
        let start = level.as_ref().map_or(1, |l| l.start);
        counters[i] = if counters[i] == 0 {
            start
        } else {
            crate::counter::next(counters[i])
        };
        for c in counters.iter_mut().skip(i + 1) {
            *c = 0;
        }
        let level = level?;
        if matches!(level.fmt.as_str(), "bullet" | "none") {
            return None;
        }
        if level.text.is_empty() {
            return Some(format!("{}.", counter(counters[i], &level.fmt)));
        }
        let mut label = crate::counter::cap_label(level.text.clone());
        for l in (0..=ilvl).rev() {
            let placeholder = format!("%{}", l + 1);
            if label.contains(&placeholder) {
                let fmt = self
                    .numbering
                    .level(num_id, l)
                    .map_or_else(|| "decimal".to_owned(), |x| x.fmt);
                let n = counters[usize::from(l)].max(1);
                label = label.replace(&placeholder, &counter(n, &fmt));
            }
        }
        Some(crate::counter::cap_label(label))
    }

    fn inline(&mut self, node: Node<'_, '_>) {
        if !self.enter() {
            self.flatten(node);
            return;
        }
        self.inline_inner(node);
        self.depth -= 1;
    }

    fn inline_inner(&mut self, node: Node<'_, '_>) {
        for c in node.children().filter(Node::is_element) {
            match c.tag_name().name() {
                crate::xmldepth::FLAT_ELEMENT => self.flatten(c),
                "r" => self.run(c),
                "hyperlink" => {
                    let target = attr(c, "id")
                        .and_then(|id| self.rels.get(id).cloned())
                        .or_else(|| attr(c, "anchor").map(|a| format!("#{a}")));
                    match target {
                        Some(t) => {
                            let id = self
                                .b
                                .open(Self::marker(MarkerKind::Link).with_reference(t));
                            self.inline(c);
                            self.b.close(id);
                        }
                        None => self.inline(c),
                    }
                }
                "fldSimple" | "smartTag" | "ins" | "moveTo" | "customXml" | "sdtContent"
                | "dir" | "bdo" => self.inline(c),
                "sdt" => {
                    if let Some(content) = child(c, "sdtContent") {
                        self.inline(content);
                    }
                }
                "AlternateContent" => {
                    if let Some(choice) = child(c, "Choice") {
                        self.inline(choice);
                    }
                }
                _ => {}
            }
        }
    }

    fn run(&mut self, r: Node<'_, '_>) {
        let rpr = child(r, "rPr");
        let mut fmt = self.styles.fmt(rpr.and_then(|n| child_val(n, "rStyle")));
        if let Some(rpr) = rpr {
            for (f, v) in fmt.iter_mut().zip(run_fmt(rpr)) {
                if v.is_some() {
                    *f = v;
                }
            }
        }
        let fmt = fmt.map(|f| f.unwrap_or(false));

        for c in r.children().filter(Node::is_element) {
            match c.tag_name().name() {
                "t" => {
                    self.set_fmt(fmt);
                    self.b.text(c.text().unwrap_or(""));
                }
                "tab" | "ptab" => self.b.space(),
                "br" | "cr" => {
                    // Page and column breaks are layout; in a cell a line
                    // break would split the table row.
                    if matches!(attr(c, "type"), Some("page" | "column")) || self.in_cell > 0 {
                        self.b.space();
                    } else {
                        self.b.line_break();
                    }
                }
                "noBreakHyphen" => {
                    self.set_fmt(fmt);
                    self.b.text("-");
                }
                "footnoteReference" => self.note("footnote", attr(c, "id")),
                "endnoteReference" => self.note("endnote", attr(c, "id")),
                "drawing" | "pict" | "object" => self.image(c),
                "AlternateContent" => {
                    if let Some(img) = child(c, "Choice") {
                        self.image(img);
                    }
                }
                _ => {}
            }
        }
    }

    fn image(&mut self, node: Node<'_, '_>) {
        let doc_pr = node
            .descendants()
            .find(|n| matches!(n.tag_name().name(), "docPr" | "cNvPr"));
        let alt = doc_pr
            .and_then(|d| {
                [attr(d, "descr"), attr(d, "title")]
                    .into_iter()
                    .flatten()
                    .map(collapse)
                    .find(|s| !s.is_empty())
            })
            .or_else(|| {
                node.descendants()
                    .find_map(|n| attr(n, "alt").or_else(|| attr(n, "title")))
                    .map(collapse)
                    .filter(|s| !s.is_empty())
            });
        let Some(alt) = alt else {
            return;
        };
        self.set_fmt([false; 4]);
        self.b.space();
        let id = self.b.open(Self::marker(MarkerKind::Image));
        self.b.text(&alt);
        self.b.close(id);
        self.b.space();
    }

    fn note(&mut self, kind: &str, id: Option<&str>) {
        let Some(text) = id.and_then(|id| self.notes.get(&format!("{kind}:{id}"))) else {
            return;
        };
        self.set_fmt([false; 4]);
        match self.options.footnotes {
            FootnoteMode::Skip => {}
            FootnoteMode::Inline => {
                self.note_count += 1;
                let label = self.note_count.to_string();
                self.b.inline_footnote(&label, text);
            }
            FootnoteMode::Deferred => {
                self.note_count += 1;
                let label = self.note_count.to_string();
                self.b.footnote_reference(&label);
                self.deferred.push((label, text.clone()));
            }
        }
    }

    /// A table cell's content (or flattened content standing in for it).
    fn cell(&mut self, tc: Node<'_, '_>) {
        if tc.tag_name().name() == crate::xmldepth::FLAT_ELEMENT {
            self.flatten(tc);
        } else {
            self.blocks(tc);
        }
    }

    fn table(&mut self, tbl: Node<'_, '_>) {
        use crate::xmldepth::FLAT_ELEMENT;
        // Rows and cells; content flattened for depth (see `xmldepth`) is
        // a row of one cell, or a cell.
        let rows: Vec<Node<'_, '_>> = tbl
            .children()
            .filter(|n| matches!(n.tag_name().name(), "tr" | FLAT_ELEMENT))
            .collect();
        fn cells_of<'a, 'i>(tr: Node<'a, 'i>) -> Vec<Node<'a, 'i>> {
            if tr.tag_name().name() == FLAT_ELEMENT {
                return vec![tr];
            }
            tr.children()
                .filter(|n| matches!(n.tag_name().name(), "tc" | FLAT_ELEMENT))
                .collect()
        }
        if self.in_cell > 0 {
            self.in_cell += 1;
            for tr in &rows {
                for tc in cells_of(*tr) {
                    self.cell(tc);
                    self.b.space();
                }
            }
            self.in_cell -= 1;
            return;
        }
        self.close_code();
        self.close_lists();
        let first_row_header = child(tbl, "tblPr")
            .and_then(|p| child(p, "tblLook"))
            .is_some_and(|look| {
                attr(look, "firstRow").map_or_else(
                    || {
                        attr(look, "val")
                            .and_then(|v| u32::from_str_radix(v, 16).ok())
                            .is_some_and(|v| v & 0x20 != 0)
                    },
                    |v| matches!(v, "1" | "true" | "on"),
                )
            });
        self.b.paragraph_break();
        let table = self.b.open(Self::marker(MarkerKind::Table));
        for (i, tr) in rows.iter().enumerate() {
            let cells = cells_of(*tr);
            if cells.is_empty() {
                continue;
            }
            let repeats = child(*tr, "trPr").is_some_and(|p| toggle(p, "tblHeader") == Some(true));
            self.b.line_break();
            let mut rm = Self::marker(MarkerKind::TableRow);
            if repeats || (i == 0 && first_row_header) {
                rm = rm.with_label(HEADER_ROW_LABEL);
            }
            let row = self.b.open(rm);
            for (j, tc) in cells.into_iter().enumerate() {
                if j > 0 {
                    self.b.separator(crate::CELL_SEPARATOR);
                }
                let cell = self.b.open_here(Self::marker(MarkerKind::TableCell));
                self.in_cell += 1;
                self.cell(tc);
                self.in_cell -= 1;
                self.set_fmt([false; 4]);
                self.b.close(cell);
            }
            self.b.close(row);
        }
        self.b.close(table);
        self.b.paragraph_break();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_format_like_word() {
        assert_eq!(counter(4, "decimal"), "4");
        assert_eq!(counter(3, "lowerLetter"), "c");
        assert_eq!(counter(28, "upperLetter"), "BB");
        assert_eq!(counter(14, "lowerRoman"), "xiv");
        assert_eq!(counter(1999, "upperRoman"), "MCMXCIX");
    }
}
