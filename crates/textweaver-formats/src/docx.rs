//! DOCX loader: `word/document.xml` (Office Open XML WordprocessingML) into
//! canonical text and markers, without Pandoc.
//!
//! star sent DOCX through Pandoc to Markdown when Pandoc was installed, and
//! on Windows decoded Pandoc's UTF-8 output with the ANSI code page, which
//! corrupted every non-ASCII letter (the star parity reference, Part 1 §1.5).
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
//!   non-breaking hyphens are hyphens, field codes are not read.
//! - **Tracked changes** (`w:ins`, `w:del` with its `w:delText`,
//!   `w:moveFrom`, `w:moveTo`) follow [`LoadOptions::revisions`]: read as
//!   the final text by default, or said in place with their author and
//!   date. They are counted under
//!   [`REVISIONS_PROPERTY`](crate::REVISIONS_PROPERTY).
//! - **Comments** (`w:comment` in `word/comments.xml`, anchored by
//!   `w:commentRangeStart` and `w:commentRangeEnd`, or at their
//!   `w:commentReference`; replies and resolved state from `w15:commentEx`
//!   in `word/commentsExtended.xml`) become [`DocumentComment`]s.
//! - **Hyperlinks** become `Link` markers with the relationship's target (or
//!   `#bookmark`).
//! - **Images** (`w:drawing`) read as their alt text (`wp:docPr` `descr`,
//!   else `title`) under an `Image` marker; images without alt text are
//!   decorative and read as nothing, as in the HTML loader.
//! - **Equations** (Office Math, `m:oMath`) become LaTeX with its
//!   delimiters under a `Math` marker, as in the Markdown loader: `$…$` at
//!   level 0 inline, and `$$…$$` at level 1 on a line of its own for
//!   display math (`m:oMathPara`). See the `omml` module for the elements
//!   covered.
//! - **Tables** stay in place, one row per line, cells separated by
//!   [`CELL_SEPARATOR`](crate::CELL_SEPARATOR); the first row is the header
//!   when the table's look says so (`w:tblLook` first row, Word's default)
//!   or the row repeats as a header (`w:tblHeader`). A table in a cell reads
//!   as running text.
//! - **Footnotes and endnotes** are numbered in reading order and follow
//!   [`LoadOptions::footnotes`] like Markdown footnotes.
//! - **Metadata** from `docProps/core.xml` (title, creator, language); the
//!   title falls back to the first level-1 heading, then the file name.

use std::collections::{HashMap, HashSet};

use ropey::Rope;
use roxmltree::Node;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, HEADER_ROW_LABEL, Marker};

use crate::annotations::{CommentReply, DocumentComment, clean_text};
use crate::builder::{Builder, OpenId};
use crate::package::{Package, child, parse_xml};
use crate::revision::{self, ChangeKind};
use crate::{
    FootnoteMode, LoadError, LoadOptions, Loader, RevisionMode, Source, meta_for, title_from_path,
};

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
        let comments_xml = part(&mut pkg, "word/comments.xml")?;
        let extended_xml = part(&mut pkg, "word/commentsExtended.xml")?;
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
            in_del: 0,
            changes: Vec::new(),
            comment_starts: HashSet::new(),
        };
        c.blocks(body);
        c.close_code();
        c.close_lists();
        let deferred = std::mem::take(&mut c.deferred);
        c.b.footnotes_section(&deferred);
        let flattened = c.flattened || pkg.flattened();
        let mut changes = std::mem::take(&mut c.changes);
        let moves = move_keys(body);
        for ch in &mut changes {
            if matches!(ch.kind, ChangeKind::MovedAway | ChangeKind::MovedHere)
                && let Some(key) = moves.get(&ch.id)
            {
                ch.pair = key.clone();
            }
        }
        let (text, markers, anchors) = c.b.finish_with_anchors();
        let comments = match comments_xml {
            Some(xml) => read_comments(&xml, extended_xml.as_deref(), &anchors)?,
            None => Vec::new(),
        };
        if flattened {
            crate::add_warning(&mut meta, crate::NESTING_WARNING);
        }
        crate::annotations::record(&mut meta, comments, changes);
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

/// The anchor key of comment `id`.
fn comment_key(id: &str) -> String {
    format!("comment:{id}")
}

/// The comments in `word/comments.xml` that have an anchor in the text,
/// with replies and resolved state from `commentsExtended.xml`.
fn read_comments(
    text: &str,
    extended: Option<&str>,
    anchors: &[(String, CharRange)],
) -> Result<Vec<DocumentComment>, LoadError> {
    let ranges: HashMap<&str, CharRange> = anchors.iter().map(|(k, r)| (k.as_str(), *r)).collect();
    // The paraId of a comment's last paragraph, to its parent's paraId and
    // whether it is done.
    let mut ext: HashMap<String, (Option<String>, bool)> = HashMap::new();
    if let Some(e) = extended {
        let xml = parse_xml(e)?;
        for n in xml
            .descendants()
            .filter(|n| n.tag_name().name() == "commentEx")
        {
            if let Some(p) = attr(n, "paraId") {
                let parent = attr(n, "paraIdParent").map(str::to_owned);
                let done = matches!(attr(n, "done"), Some("1" | "true" | "on"));
                ext.insert(p.to_owned(), (parent, done));
            }
        }
    }
    let xml = parse_xml(text)?;
    let mut all: Vec<(Option<String>, DocumentComment)> = Vec::new();
    for c in xml
        .descendants()
        .filter(|n| n.tag_name().name() == "comment")
    {
        if all.len() >= crate::annotations::MAX_COMMENTS {
            break;
        }
        let Some(id) = attr(c, "id") else {
            continue;
        };
        let paras: Vec<Node<'_, '_>> = c
            .descendants()
            .filter(|n| n.tag_name().name() == "p")
            .collect();
        let body = paras
            .iter()
            .map(|p| {
                collapse(
                    &p.descendants()
                        .filter(|n| n.tag_name().name() == "t" && !in_deletion(*n))
                        .filter_map(|n| n.text())
                        .collect::<String>(),
                )
            })
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let para_id = paras.last().and_then(|p| attr(*p, "paraId"));
        all.push((
            para_id.map(str::to_owned),
            DocumentComment {
                id: id.to_owned(),
                range: ranges
                    .get(comment_key(id).as_str())
                    .copied()
                    .unwrap_or_default(),
                author: attr(c, "author").unwrap_or("").trim().to_owned(),
                date: attr(c, "date").unwrap_or("").trim().to_owned(),
                text: clean_text(&body),
                replies: Vec::new(),
                resolved: false,
            },
        ));
    }
    // Replies go under the comment they answer, in order.
    let owner: HashMap<String, usize> = all
        .iter()
        .enumerate()
        .filter_map(|(i, (p, _))| Some((p.clone()?, i)))
        .collect();
    let mut parent_of: Vec<Option<usize>> = vec![None; all.len()];
    for (i, (p, _)) in all.iter().enumerate() {
        if let Some((Some(parent), _)) = p.as_ref().and_then(|p| ext.get(p))
            && let Some(&j) = owner.get(parent)
            && j != i
        {
            parent_of[i] = Some(j);
        }
    }
    let mut top: Vec<Option<DocumentComment>> = Vec::with_capacity(all.len());
    let mut replies: Vec<(usize, CommentReply)> = Vec::new();
    for (i, (p, mut c)) in all.into_iter().enumerate() {
        c.resolved = p.as_ref().and_then(|p| ext.get(p)).is_some_and(|e| e.1);
        match parent_of[i] {
            // Replies to replies go to the thread's first comment.
            Some(mut j) => {
                let mut hops = 0;
                while let Some(k) = parent_of[j].filter(|_| hops < 16) {
                    j = k;
                    hops += 1;
                }
                replies.push((
                    j,
                    CommentReply {
                        author: c.author,
                        date: c.date,
                        text: c.text,
                    },
                ));
                top.push(None);
            }
            None => top.push(Some(c)),
        }
    }
    for (j, r) in replies {
        if let Some(Some(parent)) = top.get_mut(j) {
            parent.replies.push(r);
        }
    }
    Ok(top
        .into_iter()
        .flatten()
        .filter(|c| ranges.contains_key(comment_key(&c.id).as_str()))
        .collect())
}

/// The move each `w:moveFrom` and `w:moveTo` belongs to, by its id: the
/// name of the move range around it, or `#n` for the n-th half of its kind
/// outside any range, in reading order. textweaver-writers' `docx_update`
/// pairs the halves by the same rule when it writes a decision back.
fn move_keys(body: Node<'_, '_>) -> HashMap<String, String> {
    let mut halves = HashMap::new();
    let mut active: [Vec<(String, String)>; 2] = [Vec::new(), Vec::new()];
    let mut unnamed = [0usize; 2];
    for n in body.descendants().filter(|n| n.is_element()) {
        let name = n.tag_name().name();
        let id = attr(n, "id").unwrap_or_default().to_owned();
        let side = usize::from(name.starts_with("moveTo"));
        let in_properties = n
            .parent_element()
            .is_some_and(|p| p.tag_name().name().ends_with("Pr"));
        match name {
            "moveFromRangeStart" | "moveToRangeStart" => {
                let key = attr(n, "name").unwrap_or(&id).to_owned();
                active[side].push((id, key));
            }
            "moveFromRangeEnd" | "moveToRangeEnd" => active[side].retain(|(r, _)| *r != id),
            "moveFrom" | "moveTo" if !in_properties => {
                let key = match active[side].last() {
                    Some((_, k)) => k.clone(),
                    None => {
                        unnamed[side] += 1;
                        format!("#{}", unnamed[side])
                    }
                };
                halves.insert(id, key);
            }
            _ => {}
        }
    }
    halves
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
    /// Inside a deletion being said ([`RevisionMode::Marked`]): its
    /// `w:delText` is read.
    in_del: usize,
    /// Tracked changes seen, in reading order.
    changes: Vec<crate::DocumentChange>,
    /// Comments whose range has started (their `w:commentReference` then
    /// adds nothing).
    comment_starts: HashSet<String>,
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
                "commentRangeStart" => self.comment_start(c),
                "commentRangeEnd" => self.comment_end(c),
                _ => {}
            }
        }
    }

    /// The start of a comment's range.
    fn comment_start(&mut self, c: Node<'_, '_>) {
        let Some(id) = attr(c, "id") else {
            return;
        };
        if self.comment_starts.len() < crate::annotations::MAX_COMMENTS
            && self.comment_starts.insert(id.to_owned())
        {
            self.b.open_anchor(comment_key(id));
        }
    }

    /// The end of a comment's range.
    fn comment_end(&mut self, c: Node<'_, '_>) {
        if let Some(id) = attr(c, "id").and_then(|id| self.b.open_anchor_id(&comment_key(id))) {
            self.b.close(id);
        }
    }

    /// A comment's reference: the comment's point, when it has no range.
    fn comment_reference(&mut self, c: Node<'_, '_>) {
        let Some(id) = attr(c, "id") else {
            return;
        };
        if self.comment_starts.len() < crate::annotations::MAX_COMMENTS
            && self.comment_starts.insert(id.to_owned())
        {
            let open = self.b.open_anchor(comment_key(id));
            self.b.close(open);
        }
    }

    /// A tracked change (`w:ins`, `w:del`, `w:moveFrom`, `w:moveTo`):
    /// counted, and read as the final text or said in place.
    fn change(&mut self, c: Node<'_, '_>, kind: ChangeKind) {
        let has_text = c.descendants().any(|n| {
            matches!(n.tag_name().name(), "t" | "delText")
                && n.text().is_some_and(|t| !t.trim().is_empty())
        });
        let deleted = !kind.adds_text();
        if !has_text {
            if !deleted {
                self.inline(c);
            }
            return;
        }
        let say = self.options.revisions == RevisionMode::Marked;
        if say {
            self.set_fmt([false; 4]);
        }
        let mut open = revision::open(
            &mut self.b,
            kind,
            attr(c, "author"),
            attr(c, "date"),
            attr(c, "id"),
            say,
        );
        if let Some(first) = c
            .descendants()
            .filter(|n| matches!(n.tag_name().name(), "t" | "delText"))
            .filter_map(|n| n.text())
            .find(|t| !t.is_empty())
        {
            open.saw_text(first);
        }
        if deleted && say {
            self.in_del += 1;
            self.inline(c);
            self.in_del -= 1;
        } else if deleted {
            let text: String = c
                .descendants()
                .filter(|n| matches!(n.tag_name().name(), "t" | "delText"))
                .filter_map(|n| n.text())
                .collect();
            open.push_deleted(&text);
        } else {
            self.inline(c);
        }
        if say {
            self.set_fmt([false; 4]);
        }
        revision::close(&mut self.b, open, &mut self.changes);
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
                "fldSimple" | "smartTag" | "customXml" | "sdtContent" | "dir" | "bdo" => {
                    self.inline(c);
                }
                "ins" => self.change(c, ChangeKind::Inserted),
                "moveTo" => self.change(c, ChangeKind::MovedHere),
                "del" => self.change(c, ChangeKind::Deleted),
                "moveFrom" => self.change(c, ChangeKind::MovedAway),
                "commentRangeStart" => self.comment_start(c),
                "commentRangeEnd" => self.comment_end(c),
                "oMath" => self.math(c, false),
                // Display math: each equation of the paragraph on its own.
                "oMathPara" => {
                    let mut any = false;
                    for m in c.children().filter(|n| n.tag_name().name() == "oMath") {
                        self.math(m, true);
                        any = true;
                    }
                    if !any {
                        self.math(c, true);
                    }
                }
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
                "delText" if self.in_del > 0 => {
                    self.set_fmt(fmt);
                    self.b.text(c.text().unwrap_or(""));
                }
                "commentReference" => self.comment_reference(c),
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

    /// Office Math as LaTeX with its delimiters under a `Math` marker, as
    /// the Markdown loader writes it: `$…$` at level 0, or display math
    /// `$$…$$` at level 1 on a line of its own (in a table cell, where a
    /// line break would split the row, between spaces).
    fn math(&mut self, node: Node<'_, '_>, display: bool) {
        let latex = crate::omml::to_latex(node);
        if latex.is_empty() {
            return;
        }
        self.set_fmt([false; 4]);
        self.display_break(display);
        let text = if display {
            format!("$${latex}$$")
        } else {
            format!("${latex}$")
        };
        let id = self
            .b
            .open(Self::marker(MarkerKind::Math).with_level(u8::from(display)));
        self.b.text(&text);
        self.b.close(id);
        self.display_break(display);
    }

    /// The break around display math: a line break, or a space in a cell.
    fn display_break(&mut self, display: bool) {
        if !display {
            return;
        }
        if self.in_cell > 0 {
            self.b.space();
        } else {
            self.b.line_break();
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

    use std::io::{Cursor, Write};

    const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    const M: &str = "http://schemas.openxmlformats.org/officeDocument/2006/math";

    /// A Word document with this body, loaded.
    fn load(body: &str) -> Document {
        let xml = format!(
            r#"<?xml version="1.0"?><w:document xmlns:w="{W}" xmlns:m="{M}"><w:body>{body}</w:body></w:document>"#
        );
        let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
        z.start_file(
            "word/document.xml",
            zip::write::SimpleFileOptions::default(),
        )
        .expect("zip entry");
        z.write_all(xml.as_bytes()).expect("zip write");
        let data = z.finish().expect("zip finish").into_inner();
        DocxLoader
            .load(
                &Source::Bytes {
                    data,
                    hint: "docx".into(),
                },
                &LoadOptions::default(),
            )
            .expect("the document loads")
    }

    /// (covered text, level) of every `Math` marker.
    fn math(doc: &Document) -> Vec<(String, u8)> {
        doc.markers()
            .iter()
            .filter(|m| m.kind == MarkerKind::Math)
            .map(|m| (doc.slice(m.range).to_string(), m.level))
            .collect()
    }

    fn r(text: &str) -> String {
        format!("<m:r><m:t>{text}</m:t></m:r>")
    }

    fn wr(text: &str) -> String {
        format!(r#"<w:r><w:t xml:space="preserve">{text}</w:t></w:r>"#)
    }

    #[test]
    fn inline_equations_read_as_latex_math() {
        let frac = format!(
            "<m:oMath><m:f><m:num>{}</m:num><m:den>{}</m:den></m:f></m:oMath>",
            r("a"),
            r("b")
        );
        let doc = load(&format!(
            "<w:p>{}{frac}{}</w:p>",
            wr("Half is "),
            wr(" of it.")
        ));
        assert_eq!(doc.text().to_string(), "Half is $\\frac{a}{b}$ of it.");
        assert_eq!(math(&doc), vec![("$\\frac{a}{b}$".to_owned(), 0)]);
        let m = doc
            .markers()
            .iter()
            .find(|m| m.kind == MarkerKind::Math)
            .map(|m| m.range);
        assert_eq!(m, Some(CharRange::new(8, 21)));
    }

    #[test]
    fn display_equations_stand_on_their_own() {
        let sum = format!(
            r#"<m:oMathPara><m:oMath><m:nary><m:naryPr><m:chr m:val="∑"/></m:naryPr><m:sub>{}</m:sub><m:sup>{}</m:sup><m:e>{}</m:e></m:nary><m:r><m:t>=</m:t></m:r><m:f><m:num>{}</m:num><m:den>{}</m:den></m:f></m:oMath></m:oMathPara>"#,
            r("i=1"),
            r("n"),
            r("i"),
            r("n(n+1)"),
            r("2")
        );
        let doc = load(&format!(
            "<w:p>{}</w:p><w:p>{sum}</w:p><w:p>{}</w:p>",
            wr("Gauss:"),
            wr("So it goes.")
        ));
        let latex = "$$\\sum_{i=1}^{n} i=\\frac{n(n+1)}{2}$$";
        assert_eq!(
            doc.text().to_string(),
            format!("Gauss:\n\n{latex}\n\nSo it goes.")
        );
        assert_eq!(math(&doc), vec![(latex.to_owned(), 1)]);
        // The equation is a paragraph of its own.
        let para = doc
            .markers()
            .iter()
            .filter(|m| m.kind == MarkerKind::Paragraph)
            .map(|m| doc.slice(m.range).to_string())
            .collect::<Vec<_>>();
        assert_eq!(para, vec!["Gauss:", latex, "So it goes."]);
    }

    #[test]
    fn display_math_with_text_around_gets_its_own_line() {
        let eq = format!("<m:oMathPara><m:oMath>{}</m:oMath></m:oMathPara>", r("x=1"));
        let doc = load(&format!(
            "<w:p>{}{eq}{}</w:p>",
            wr("Take"),
            wr("then stop.")
        ));
        assert_eq!(doc.text().to_string(), "Take\n$$x=1$$\nthen stop.");
        assert_eq!(math(&doc), vec![("$$x=1$$".to_owned(), 1)]);
    }

    #[test]
    fn several_equations_in_one_math_paragraph_are_separate() {
        let eq = format!(
            "<m:oMathPara><m:oMath>{}</m:oMath><m:oMath>{}</m:oMath></m:oMathPara>",
            r("a=1"),
            r("b=2")
        );
        let doc = load(&format!("<w:p>{eq}</w:p>"));
        assert_eq!(doc.text().to_string(), "$$a=1$$\n$$b=2$$");
        assert_eq!(
            math(&doc),
            vec![("$$a=1$$".to_owned(), 1), ("$$b=2$$".to_owned(), 1)]
        );
    }

    #[test]
    fn every_element_reads_back_from_a_document() {
        let e = |b: &str| format!("<m:e>{b}</m:e>");
        let cases: Vec<(String, &str)> = vec![
            (r("α≤π"), "\\alpha\\leq\\pi"),
            (
                format!("<m:sSup>{}<m:sup>{}</m:sup></m:sSup>", e(&r("x")), r("2")),
                "x^{2}",
            ),
            (
                format!("<m:sSub>{}<m:sub>{}</m:sub></m:sSub>", e(&r("x")), r("i")),
                "x_{i}",
            ),
            (
                format!(
                    "<m:sSubSup>{}<m:sub>{}</m:sub><m:sup>{}</m:sup></m:sSubSup>",
                    e(&r("x")),
                    r("i"),
                    r("2")
                ),
                "x_{i}^{2}",
            ),
            (
                format!(
                    "<m:sPre><m:sub>{}</m:sub><m:sup>{}</m:sup>{}</m:sPre>",
                    r("a"),
                    r("b"),
                    e(&r("C"))
                ),
                "{}_{a}^{b}C",
            ),
            (
                format!(
                    r#"<m:rad><m:radPr><m:degHide m:val="1"/></m:radPr><m:deg/>{}</m:rad>"#,
                    e(&r("x"))
                ),
                "\\sqrt{x}",
            ),
            (
                format!("<m:rad><m:deg>{}</m:deg>{}</m:rad>", r("n"), e(&r("x"))),
                "\\sqrt[n]{x}",
            ),
            (
                format!(
                    "<m:nary><m:sub>{}</m:sub><m:sup>{}</m:sup>{}</m:nary>",
                    r("0"),
                    r("1"),
                    e(&r("x dx"))
                ),
                "\\int_{0}^{1} x dx",
            ),
            (format!("<m:d>{}</m:d>", e(&r("a"))), "\\left( a \\right)"),
            (
                format!(
                    "<m:d><m:dPr><m:begChr m:val=\"[\"/><m:endChr m:val=\"]\"/></m:dPr><m:e><m:m><m:mr>{}{}</m:mr><m:mr>{}{}</m:mr></m:m></m:e></m:d>",
                    e(&r("1")),
                    e(&r("0")),
                    e(&r("0")),
                    e(&r("1"))
                ),
                "\\left[ \\begin{matrix} 1 & 0 \\\\ 0 & 1 \\end{matrix} \\right]",
            ),
            (
                format!(
                    r#"<m:func><m:fName><m:r><m:rPr><m:sty m:val="p"/></m:rPr><m:t>cos</m:t></m:r></m:fName>{}</m:func>"#,
                    e(&r("θ"))
                ),
                "\\cos \\theta",
            ),
            (
                format!(
                    "<m:acc><m:accPr><m:chr m:val=\"\u{20D7}\"/></m:accPr>{}</m:acc>",
                    e(&r("v"))
                ),
                "\\vec{v}",
            ),
            (
                format!(
                    r#"<m:bar><m:barPr><m:pos m:val="top"/></m:barPr>{}</m:bar>"#,
                    e(&r("z"))
                ),
                "\\overline{z}",
            ),
            (
                format!(
                    "<m:func><m:fName><m:limLow>{}<m:lim>{}</m:lim></m:limLow></m:fName>{}</m:func>",
                    e(&r("lim")),
                    r("n→∞"),
                    e(&r("a"))
                ),
                "\\lim_{n\\to\\infty} a",
            ),
            (
                format!("<m:borderBox>{}</m:borderBox>", e(&r("y"))),
                "\\boxed{y}",
            ),
            (
                format!("<m:groupChr>{}</m:groupChr>", e(&r("abc"))),
                "\\underbrace{abc}",
            ),
            (
                format!(
                    "<m:eqArr>{}{}</m:eqArr>",
                    e(&r("a&amp;=b")),
                    e(&r("c&amp;=d"))
                ),
                "\\begin{aligned} a & =b \\\\ c & =d \\end{aligned}",
            ),
            (
                "<m:r><m:rPr><m:nor/></m:rPr><m:t>for all x</m:t></m:r>".to_owned(),
                "\\text{for all x}",
            ),
        ];
        for (omml, want) in cases {
            let doc = load(&format!("<w:p><m:oMath>{omml}</m:oMath></w:p>"));
            let want = format!("${want}$");
            assert_eq!(doc.text().to_string(), want, "for {omml}");
            assert_eq!(math(&doc), vec![(want.clone(), 0)], "for {omml}");
        }
    }

    /// A Word document with this body and these other parts, loaded.
    fn load_parts(body: &str, parts: &[(&str, &str)], options: &LoadOptions) -> Document {
        let xml = format!(
            r#"<?xml version="1.0"?><w:document xmlns:w="{W}" xmlns:m="{M}"><w:body>{body}</w:body></w:document>"#
        );
        let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default();
        z.start_file("word/document.xml", opts).expect("zip entry");
        z.write_all(xml.as_bytes()).expect("zip write");
        for (name, text) in parts {
            z.start_file(*name, opts).expect("zip entry");
            z.write_all(text.as_bytes()).expect("zip write");
        }
        let data = z.finish().expect("zip finish").into_inner();
        DocxLoader
            .load(
                &Source::Bytes {
                    data,
                    hint: "docx".into(),
                },
                options,
            )
            .expect("the document loads")
    }

    #[test]
    fn comments_become_document_comments_with_replies() {
        const W14: &str = "http://schemas.microsoft.com/office/word/2010/wordml";
        const W15: &str = "http://schemas.microsoft.com/office/word/2012/wordml";
        let comments = format!(
            r#"<w:comments xmlns:w="{W}" xmlns:w14="{W14}"><w:comment w:id="0" w:author="Ada Example" w:date="2026-09-01T10:00:00Z"><w:p w14:paraId="00000001"><w:r><w:annotationRef/></w:r><w:r><w:t>Check this date.</w:t></w:r></w:p></w:comment><w:comment w:id="1" w:author="Bo Example"><w:p w14:paraId="00000002"><w:r><w:t>Fixed.</w:t></w:r></w:p></w:comment><w:comment w:id="2" w:author="Bo Example"><w:p w14:paraId="00000003"><w:r><w:t>A point.</w:t></w:r></w:p></w:comment><w:comment w:id="3"><w:p><w:r><w:t>Never anchored.</w:t></w:r></w:p></w:comment></w:comments>"#
        );
        let extended = format!(
            r#"<w15:commentsEx xmlns:w15="{W15}"><w15:commentEx w15:paraId="00000001" w15:done="1"/><w15:commentEx w15:paraId="00000002" w15:paraIdParent="00000001" w15:done="0"/></w15:commentsEx>"#
        );
        let body = format!(
            r#"<w:p>{}<w:commentRangeStart w:id="0"/><w:commentRangeStart w:id="1"/>{}<w:commentRangeEnd w:id="0"/><w:commentRangeEnd w:id="1"/><w:r><w:commentReference w:id="0"/></w:r>{}<w:r><w:commentReference w:id="2"/></w:r></w:p>"#,
            wr("Read "),
            wr("chapter two"),
            wr(" first.")
        );
        let doc = load_parts(
            &body,
            &[
                ("word/comments.xml", &comments),
                ("word/commentsExtended.xml", &extended),
            ],
            &LoadOptions::default(),
        );
        assert_eq!(doc.text().to_string(), "Read chapter two first.");
        let cs = crate::comments(&doc.meta);
        assert_eq!(cs.len(), 2, "{cs:?}");
        assert_eq!(doc.slice(cs[0].range).to_string(), "chapter two");
        assert_eq!(cs[0].author, "Ada Example");
        assert_eq!(cs[0].date, "2026-09-01T10:00:00Z");
        assert!(cs[0].resolved);
        assert_eq!(
            cs[0].spoken(),
            "Comment by Ada Example: Check this date. Reply by Bo Example: Fixed. Resolved."
        );
        assert!(cs[1].range.is_empty());
        assert_eq!(
            cs[1].range.start.0,
            "Read chapter two first.".chars().count()
        );
        assert_eq!(cs[1].text, "A point.");
    }

    #[test]
    fn tracked_changes_are_final_text_or_said_in_place() {
        let body = format!(
            r#"<w:p>{}<w:del w:id="5" w:author="Bo Example" w:date="2026-09-02T00:00:00Z"><w:r><w:delText xml:space="preserve">old </w:delText></w:r></w:del><w:ins w:id="6" w:author="Ada Example" w:date="2026-09-01T00:00:00Z">{}</w:ins>{}</w:p><w:p><w:moveFrom w:author="Ada Example">{}</w:moveFrom>{}<w:moveTo w:author="Ada Example">{}</w:moveTo></w:p>"#,
            wr("The "),
            wr("new "),
            wr("plan."),
            wr("Moved"),
            wr(" text "),
            wr("moved")
        );
        let doc = load_parts(&body, &[], &LoadOptions::default());
        assert_eq!(doc.text().to_string(), "The new plan.\n\ntext moved");
        assert_eq!(crate::revision_count(&doc.meta), 4);
        let marked = load_parts(
            &body,
            &[],
            &LoadOptions {
                revisions: RevisionMode::Marked,
                ..LoadOptions::default()
            },
        );
        assert_eq!(
            marked.text().to_string(),
            "The (deleted by Bo Example: old) (inserted by Ada Example: new) plan.\n\n(moved away by Ada Example: Moved) text (moved here by Ada Example: moved)"
        );
        let struck: Vec<(String, Option<String>)> = marked
            .markers()
            .iter()
            .filter(|m| m.kind == MarkerKind::Strikethrough)
            .map(|m| (marked.slice(m.range).to_string(), m.reference.clone()))
            .collect();
        assert_eq!(
            struck,
            vec![
                ("old".to_owned(), Some("2026-09-02T00:00:00Z".to_owned())),
                ("Moved".to_owned(), None)
            ]
        );
    }

    #[test]
    fn math_in_a_table_cell_keeps_the_row_whole() {
        let eq = format!(
            "<m:oMathPara><m:oMath>{}</m:oMath></m:oMathPara>",
            r("E=mc")
        );
        let doc = load(&format!(
            "<w:tbl><w:tr><w:tc><w:p>{}</w:p></w:tc><w:tc><w:p>{eq}</w:p></w:tc></w:tr></w:tbl>",
            wr("Energy")
        ));
        assert_eq!(doc.text().to_string(), "Energy | $$E=mc$$");
        assert_eq!(math(&doc), vec![("$$E=mc$$".to_owned(), 1)]);
    }
}
