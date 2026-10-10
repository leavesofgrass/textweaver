//! Comments written back into the package: replies and new comments
//! added, resolved marks set, deleted threads removed with their marks in
//! the document.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use roxmltree::Node;

use super::{
    COMMENTS, DOC_RELS, DOCUMENT, DocxUpdate, EXTENDED, EXTENSIBLE, IDS, Parts, TYPES, ThreadReply,
    UpdateReport, W, W14, W15, attr, inner, is_w, is_w_named, parse, q, self_closing,
    start_tag_end,
};
use crate::{WriteError, xml};

const EMPTY_COMMENTS: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    "\r\n",
    r#"<w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"></w:comments>"#
);
const EMPTY_EXTENDED: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    "\r\n",
    r#"<w15:commentsEx xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w15="http://schemas.microsoft.com/office/word/2012/wordml" mc:Ignorable="w15"></w15:commentsEx>"#
);
const COMMENTS_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml";
const EXTENDED_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.commentsExtended+xml";
const COMMENTS_REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments";
const EXTENDED_REL: &str =
    "http://schemas.microsoft.com/office/2011/relationships/commentsExtended";

/// Byte edits to one part, as (start, end, text): applied in order of
/// start, then of addition, so insertions at one place keep their order.
#[derive(Default)]
struct Edits(Vec<(usize, usize, String)>);

impl Edits {
    fn insert(&mut self, at: usize, text: impl Into<String>) {
        self.0.push((at, at, text.into()));
    }

    fn replace(&mut self, r: Range<usize>, text: impl Into<String>) {
        self.0.push((r.start, r.end, text.into()));
    }

    fn remove(&mut self, r: Range<usize>) {
        self.replace(r, "");
    }

    /// `src` edited, or `None` for no edits. An edit inside a range
    /// already removed adds only its text.
    fn apply(mut self, src: &str) -> Option<String> {
        if self.0.is_empty() {
            return None;
        }
        self.0.sort_by_key(|e| e.0);
        let mut out = String::with_capacity(src.len());
        let mut at = 0;
        for (start, end, text) in self.0 {
            if start >= at {
                out.push_str(&src[at..start]);
            }
            out.push_str(&text);
            at = at.max(end);
        }
        out.push_str(&src[at..]);
        Some(out)
    }
}

/// The prefix written for namespace `ns` at `n`: `Some("")` for a default
/// namespace, `None` when it is not declared.
fn prefix(n: Node<'_, '_>, ns: &str) -> Option<String> {
    if let Some(p) = n.lookup_prefix(ns) {
        return Some(p.to_owned());
    }
    n.namespaces()
        .any(|d| d.name().is_none() && d.uri() == ns)
        .then(String::new)
}

/// Where an attribute can be added to `n`'s start tag.
fn attr_at(src: &str, n: Node<'_, '_>) -> usize {
    start_tag_end(src, n) - if self_closing(src, n) { 2 } else { 1 }
}

/// Adds `content` at the end of the root element `root`, opening it when
/// it is written `<a/>`.
fn append_to_root(src: &str, root: Node<'_, '_>, content: &str, edits: &mut Edits) {
    if content.is_empty() {
        return;
    }
    if self_closing(src, root) {
        let st = start_tag_end(src, root);
        let name = super::qname(src, root);
        edits.replace(st - 2..st, format!(">{content}</{name}>"));
    } else {
        edits.insert(inner(src, root).end, content);
    }
}

/// A comment in `word/comments.xml`.
struct FileComment {
    id: String,
    range: Range<usize>,
    /// Its last paragraph's `w14:paraId`.
    para: Option<String>,
    /// Where a `w14:paraId` can be added to its last paragraph.
    para_at: Option<usize>,
    /// The comment it answers, by index.
    parent: Option<usize>,
}

/// A `w15:commentEx` in `word/commentsExtended.xml`.
struct Ext {
    range: Range<usize>,
    parent: Option<String>,
    done: bool,
    /// The `w15:done` value's place, when it has one.
    done_value: Option<Range<usize>>,
    attr_at: usize,
}

fn read_comments(src: &str, root: Node<'_, '_>) -> Vec<FileComment> {
    root.children()
        .filter(|c| is_w_named(*c, "comment"))
        .filter_map(|c| {
            let last = c.descendants().rfind(|p| is_w_named(*p, "p"));
            Some(FileComment {
                id: attr(c, "id")?.to_owned(),
                range: c.range(),
                para: last.and_then(|p| attr(p, "paraId")).map(str::to_owned),
                para_at: last.map(|p| attr_at(src, p)),
                parent: None,
            })
        })
        .collect()
}

fn read_extended(src: &str, root: Node<'_, '_>) -> HashMap<String, Ext> {
    root.children()
        .filter(|c| c.is_element() && c.tag_name().name() == "commentEx")
        .filter_map(|c| {
            let para = attr(c, "paraId")?.to_owned();
            let done = c.attributes().find(|a| a.name() == "done");
            Some((
                para,
                Ext {
                    range: c.range(),
                    parent: attr(c, "paraIdParent").map(str::to_owned),
                    done: matches!(done.map(|a| a.value()), Some("1" | "true" | "on")),
                    done_value: done.map(|a| a.range_value()),
                    attr_at: attr_at(src, c),
                },
            ))
        })
        .collect()
}

/// A comment's marks in the document.
#[derive(Default)]
struct Marks {
    /// `w:commentRangeStart`.
    start: Option<Range<usize>>,
    /// `w:commentRangeEnd`.
    end: Option<Range<usize>>,
    /// What to remove for its `w:commentReference` (its run, when the run
    /// holds nothing else), and where a reply's reference run goes.
    reference: Option<(Range<usize>, usize)>,
}

fn read_marks(root: Node<'_, '_>) -> HashMap<String, Marks> {
    let mut marks: HashMap<String, Marks> = HashMap::new();
    for n in root.descendants().filter(|n| is_w(*n)) {
        let Some(id) = attr(n, "id") else { continue };
        match n.tag_name().name() {
            "commentRangeStart" => marks.entry(id.to_owned()).or_default().start = Some(n.range()),
            "commentRangeEnd" => marks.entry(id.to_owned()).or_default().end = Some(n.range()),
            "commentReference" => {
                let run = n.parent_element().filter(|r| is_w_named(*r, "r"));
                let reference = match run {
                    Some(r) => {
                        let alone = r
                            .children()
                            .filter(|c| c.is_element())
                            .all(|c| is_w_named(c, "rPr") || is_w_named(c, "commentReference"));
                        let gone = if alone { r.range() } else { n.range() };
                        (gone, r.range().end)
                    }
                    None => (n.range(), n.range().end),
                };
                marks.entry(id.to_owned()).or_default().reference = Some(reference);
            }
            _ => {}
        }
    }
    marks
}

/// Every `paraId="..."` value in `src`.
fn para_ids(src: &str, into: &mut HashSet<String>) {
    for (i, _) in src.match_indices("paraId=\"") {
        let rest = &src[i + 8..];
        if let Some(end) = rest.find('"') {
            into.insert(rest[..end].to_ascii_uppercase());
        }
    }
}

/// Initials for `author`: "Ada Example" gives "AE".
fn initials(author: &str) -> String {
    author
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .flat_map(char::to_uppercase)
        .collect()
}

/// Whitespace runs as one space, trimmed.
fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The document's text, to find where a new comment goes: whitespace runs
/// as one space, a line break between paragraphs, and for each character
/// the run it is in.
#[derive(Default)]
struct TextIndex {
    text: String,
    /// Each character's byte in `text` and its run's place in the part.
    runs: Vec<(usize, Range<usize>)>,
}

impl TextIndex {
    fn of(root: Node<'_, '_>) -> TextIndex {
        let mut t = TextIndex::default();
        for p in root.descendants().filter(|n| is_w_named(*n, "p")) {
            if !t.text.is_empty() {
                t.text.push('\n');
            }
            let mut space = true;
            for n in p.descendants().filter(|n| is_w(*n)) {
                if n.ancestors().skip(1).find(|a| is_w_named(*a, "p")) != Some(p)
                    || n.ancestors()
                        .any(|a| is_w(a) && matches!(a.tag_name().name(), "del" | "moveFrom"))
                {
                    continue;
                }
                let Some(run) = n.ancestors().find(|a| is_w_named(*a, "r")) else {
                    continue;
                };
                let text = match n.tag_name().name() {
                    "t" => n.text().unwrap_or_default(),
                    "tab" | "br" | "cr" => " ",
                    _ => continue,
                };
                for c in text.chars() {
                    let c = if c.is_whitespace() { ' ' } else { c };
                    if c == ' ' && space {
                        continue;
                    }
                    space = c == ' ';
                    t.runs.push((t.text.len(), run.range()));
                    t.text.push(c);
                }
            }
        }
        t
    }

    /// The run holding the character at byte `at` of the text.
    fn run_at(&self, at: usize) -> Option<Range<usize>> {
        let i = self
            .runs
            .partition_point(|(b, _)| *b <= at)
            .checked_sub(1)?;
        self.runs.get(i).map(|(_, r)| r.clone())
    }

    /// Where a comment on `anchor` (its `occurrence`, else its first)
    /// starts and ends in the part: before its first run, after its last.
    fn find(&self, anchor: &str, occurrence: usize) -> Option<(usize, usize)> {
        let needle = collapse(anchor);
        if needle.is_empty() {
            return None;
        }
        let hits: Vec<usize> = self.text.match_indices(&needle).map(|(i, _)| i).collect();
        let at = *hits.get(occurrence).or(hits.first())?;
        let last = at + needle.len() - needle.chars().last().map_or(1, char::len_utf8);
        Some((self.run_at(at)?.start, self.run_at(last)?.end))
    }

    /// Where a comment on `anchor` goes: the text itself, else its first
    /// line, else (`false`) the document's first run.
    fn place(&self, anchor: &str, occurrence: usize) -> Option<((usize, usize), bool)> {
        if let Some(found) = self.find(anchor, occurrence) {
            return Some((found, true));
        }
        if let Some(found) = anchor
            .lines()
            .find(|l| !l.trim().is_empty())
            .and_then(|l| self.find(l, 0))
        {
            return Some((found, true));
        }
        let (_, first) = self.runs.first()?;
        Some(((first.start, first.end), false))
    }
}

/// The names written in each part, and what writes new elements.
struct Writer {
    /// `w` in the document.
    w_doc: String,
    /// `w` and `w14` in `comments.xml`.
    w: String,
    w14: String,
    /// `w15` in `commentsExtended.xml`.
    w15: String,
    used_paras: HashSet<String>,
    next_para: u32,
    next_id: u64,
}

impl Writer {
    /// A paragraph id no part uses yet (below 0x80000000, as Word wants).
    fn new_para(&mut self) -> String {
        loop {
            self.next_para += 1;
            let p = format!("{:08X}", self.next_para);
            if self.used_paras.insert(p.clone()) {
                return p;
            }
        }
    }

    fn new_id(&mut self) -> String {
        let id = self.next_id;
        self.next_id += 1;
        id.to_string()
    }

    /// A `w:comment` whose last paragraph has id `para`; an empty author
    /// is written as "textweaver".
    fn comment(&self, id: &str, author: &str, date: &str, text: &str, para: &str) -> String {
        let w = |l: &str| q(&self.w, l);
        let author = match author.trim() {
            "" => "textweaver",
            a => a,
        };
        let mut out = format!(
            "<{} {}=\"{}\" {}=\"{}\"",
            w("comment"),
            w("id"),
            xml::attr(id),
            w("author"),
            xml::attr(author)
        );
        if !date.trim().is_empty() {
            out.push_str(&format!(" {}=\"{}\"", w("date"), xml::attr(date.trim())));
        }
        out.push_str(&format!(
            " {}=\"{}\">",
            w("initials"),
            xml::attr(&initials(author))
        ));
        let lines: Vec<&str> = text.lines().collect();
        let lines = if lines.is_empty() { vec![""] } else { lines };
        for (i, line) in lines.iter().enumerate() {
            if i + 1 == lines.len() {
                out.push_str(&format!(
                    "<{} {}=\"{para}\">",
                    w("p"),
                    q(&self.w14, "paraId")
                ));
            } else {
                out.push_str(&format!("<{}>", w("p")));
            }
            if i == 0 {
                out.push_str(&format!("<{r}><{}/></{r}>", w("annotationRef"), r = w("r")));
            }
            out.push_str(&format!(
                "<{r}><{t} xml:space=\"preserve\">{}</{t}></{r}></{p}>",
                xml::text(line),
                r = w("r"),
                t = w("t"),
                p = w("p")
            ));
        }
        out.push_str(&format!("</{}>", w("comment")));
        out
    }

    /// A `w15:commentEx` for paragraph `para`.
    fn ext(&self, para: &str, parent: Option<&str>, done: bool) -> String {
        let w15 = |l: &str| q(&self.w15, l);
        let parent = parent
            .map(|p| format!(" {}=\"{p}\"", w15("paraIdParent")))
            .unwrap_or_default();
        format!(
            "<{} {}=\"{para}\"{parent} {}=\"{}\"/>",
            w15("commentEx"),
            w15("paraId"),
            w15("done"),
            u8::from(done)
        )
    }

    /// A mark in the document: `commentRangeStart`, `commentRangeEnd`, or
    /// a run with the `commentReference`.
    fn mark(&self, kind: &str, id: &str) -> String {
        let w = |l: &str| q(&self.w_doc, l);
        let mark = format!("<{} {}=\"{id}\"/>", w(kind), w("id"));
        if kind == "commentReference" {
            format!("<{r}>{mark}</{r}>", r = w("r"))
        } else {
            mark
        }
    }
}

/// Writes `update`'s comment changes into `parts`.
pub(super) fn write(
    parts: &mut Parts,
    update: &DocxUpdate,
    report: &mut UpdateReport,
) -> Result<(), WriteError> {
    let doc_src = parts.get(DOCUMENT).unwrap_or_default();
    let had_comments = parts.get(COMMENTS).is_some();
    let com_src = parts
        .get(COMMENTS)
        .unwrap_or_else(|| EMPTY_COMMENTS.to_owned());
    let had_ext = parts.get(EXTENDED).is_some();
    let ext_src = parts
        .get(EXTENDED)
        .unwrap_or_else(|| EMPTY_EXTENDED.to_owned());
    let doc_xml = parse(DOCUMENT, &doc_src)?;
    let com_xml = parse(COMMENTS, &com_src)?;
    let ext_xml = parse(EXTENDED, &ext_src)?;
    let (droot, croot, xroot) = (
        doc_xml.root_element(),
        com_xml.root_element(),
        ext_xml.root_element(),
    );
    let (mut de, mut ce, mut xe) = (Edits::default(), Edits::default(), Edits::default());

    let mut file = read_comments(&com_src, croot);
    let ext = read_extended(&ext_src, xroot);
    let by_para: HashMap<String, usize> = file
        .iter()
        .enumerate()
        .filter_map(|(i, c)| Some((c.para.clone()?, i)))
        .collect();
    for (i, c) in file.iter_mut().enumerate() {
        c.parent = c
            .para
            .as_ref()
            .and_then(|p| ext.get(p))
            .and_then(|e| e.parent.as_ref())
            .and_then(|p| by_para.get(p))
            .copied()
            .filter(|&j| j != i);
    }
    let root_of = |mut i: usize| {
        for _ in 0..16 {
            match file[i].parent {
                Some(j) => i = j,
                None => break,
            }
        }
        i
    };
    let top: HashMap<&str, usize> = file
        .iter()
        .enumerate()
        .filter(|(_, c)| c.parent.is_none())
        .map(|(i, c)| (c.id.as_str(), i))
        .collect();
    let marks = read_marks(droot);

    let mut used_paras = HashSet::new();
    para_ids(&doc_src, &mut used_paras);
    para_ids(&com_src, &mut used_paras);
    let w14 = prefix(croot, W14).unwrap_or_else(|| {
        ce.insert(attr_at(&com_src, croot), format!(" xmlns:w14=\"{W14}\""));
        "w14".to_owned()
    });
    let w15 = prefix(xroot, W15).unwrap_or_else(|| {
        xe.insert(attr_at(&ext_src, xroot), format!(" xmlns:w15=\"{W15}\""));
        "w15".to_owned()
    });
    let mut wr = Writer {
        w_doc: prefix(droot, W).unwrap_or_else(|| "w".into()),
        w: prefix(croot, W).unwrap_or_else(|| "w".into()),
        w14,
        w15,
        used_paras,
        next_para: 0x1000_0000,
        next_id: file
            .iter()
            .filter_map(|c| c.id.parse::<u64>().ok())
            .max()
            .map_or(0, |m| m + 1),
    };
    let (mut c_add, mut x_add) = (String::new(), String::new());

    // Deleted threads, with their replies and marks.
    let deleted: HashSet<&str> = update.deleted_comments.iter().map(String::as_str).collect();
    let mut paras_gone = HashSet::new();
    for (i, c) in file.iter().enumerate() {
        if !deleted.contains(file[root_of(i)].id.as_str()) {
            continue;
        }
        ce.remove(c.range.clone());
        if let Some(p) = &c.para {
            if let Some(e) = ext.get(p) {
                xe.remove(e.range.clone());
            }
            paras_gone.insert(p.to_ascii_uppercase());
        }
        if let Some(m) = marks.get(&c.id) {
            for r in [
                m.start.clone(),
                m.end.clone(),
                m.reference.clone().map(|r| r.0),
            ]
            .into_iter()
            .flatten()
            {
                de.remove(r);
            }
        }
    }

    // Threads: resolved marks and replies on those in the file, the rest
    // new.
    let mut index: Option<TextIndex> = None;
    for t in &update.threads {
        if deleted.contains(t.id.as_str()) {
            continue;
        }
        let Some(&i) = top.get(t.id.as_str()) else {
            let id = wr.new_id();
            let para = wr.new_para();
            c_add.push_str(&wr.comment(&id, &t.author, &t.date, &t.text, &para));
            x_add.push_str(&wr.ext(&para, None, t.resolved));
            let replies = add_replies(&mut wr, &t.replies, &para, &mut c_add, &mut x_add);
            let index = index.get_or_insert_with(|| TextIndex::of(droot));
            match index.place(&t.anchor, t.occurrence) {
                Some(((start, end), found)) => {
                    if !found {
                        report.unplaced += 1;
                    }
                    let ids = std::iter::once(&id).chain(&replies);
                    let starts: String = ids
                        .clone()
                        .map(|r| wr.mark("commentRangeStart", r))
                        .collect();
                    let ends: String = ids
                        .clone()
                        .map(|r| wr.mark("commentRangeEnd", r))
                        .chain(ids.map(|r| wr.mark("commentReference", r)))
                        .collect();
                    de.insert(start, starts);
                    de.insert(end, ends);
                }
                None => report.unplaced += 1,
            }
            report.new_comment_ids.push((t.id.clone(), id));
            continue;
        };
        let have = (0..file.len())
            .filter(|&j| j != i && root_of(j) == i)
            .count();
        let new_replies = t.replies.get(have..).unwrap_or_default();
        let ext_now = file[i].para.as_ref().and_then(|p| ext.get(p));
        let done_now = ext_now.is_some_and(|e| e.done);
        if new_replies.is_empty() && done_now == t.resolved {
            continue;
        }
        let para = match &file[i].para {
            Some(p) => p.clone(),
            None => {
                let p = wr.new_para();
                if let Some(at) = file[i].para_at {
                    ce.insert(at, format!(" {}=\"{p}\"", q(&wr.w14, "paraId")));
                }
                p
            }
        };
        if done_now != t.resolved {
            let v = if t.resolved { "1" } else { "0" };
            match ext_now {
                Some(e) => match &e.done_value {
                    Some(r) => xe.replace(r.clone(), v),
                    None => xe.insert(e.attr_at, format!(" {}=\"{v}\"", q(&wr.w15, "done"))),
                },
                None => x_add.push_str(&wr.ext(&para, None, t.resolved)),
            }
        }
        let replies = add_replies(&mut wr, new_replies, &para, &mut c_add, &mut x_add);
        // Each reply is marked beside the comment it answers.
        if let Some(m) = marks.get(&file[i].id) {
            for r in &replies {
                if let Some(s) = &m.start {
                    de.insert(s.end, wr.mark("commentRangeStart", r));
                }
                if let Some(e) = &m.end {
                    de.insert(e.end, wr.mark("commentRangeEnd", r));
                }
                let at = m
                    .reference
                    .as_ref()
                    .map(|x| x.1)
                    .or(m.end.as_ref().map(|e| e.end));
                if let Some(at) = at {
                    de.insert(at, wr.mark("commentReference", r));
                }
            }
        }
    }

    // The deleted comments' entries in the newer comment parts.
    if !paras_gone.is_empty() {
        let mut durable = HashSet::new();
        if let Some(src) = parts.get(IDS) {
            let xml = parse(IDS, &src)?;
            let mut e = Edits::default();
            for n in xml.root_element().children().filter(|n| n.is_element()) {
                if attr(n, "paraId").is_some_and(|p| paras_gone.contains(&p.to_ascii_uppercase())) {
                    durable.extend(attr(n, "durableId").map(str::to_owned));
                    e.remove(n.range());
                }
            }
            if let Some(new) = e.apply(&src) {
                parts.set(IDS, new);
            }
        }
        if let Some(src) = parts.get(EXTENSIBLE).filter(|_| !durable.is_empty()) {
            let xml = parse(EXTENSIBLE, &src)?;
            let mut e = Edits::default();
            for n in xml.root_element().children().filter(|n| n.is_element()) {
                if attr(n, "durableId").is_some_and(|d| durable.contains(d)) {
                    e.remove(n.range());
                }
            }
            if let Some(new) = e.apply(&src) {
                parts.set(EXTENSIBLE, new);
            }
        }
    }

    append_to_root(&com_src, croot, &c_add, &mut ce);
    append_to_root(&ext_src, xroot, &x_add, &mut xe);
    if let Some(new) = de.apply(&doc_src) {
        parts.set(DOCUMENT, new);
    }
    if let Some(new) = ce.apply(&com_src) {
        parts.set(COMMENTS, new);
        if !had_comments {
            register(parts, "word/comments.xml", COMMENTS_TYPE, COMMENTS_REL)?;
        }
    }
    if let Some(new) = xe.apply(&ext_src) {
        parts.set(EXTENDED, new);
        if !had_ext {
            register(
                parts,
                "word/commentsExtended.xml",
                EXTENDED_TYPE,
                EXTENDED_REL,
            )?;
        }
    }
    Ok(())
}

/// Adds `replies` to `c_add` and `x_add` as answers to the comment whose
/// paragraph is `parent`; returns their ids.
fn add_replies(
    wr: &mut Writer,
    replies: &[ThreadReply],
    parent: &str,
    c_add: &mut String,
    x_add: &mut String,
) -> Vec<String> {
    replies
        .iter()
        .map(|r| {
            let id = wr.new_id();
            let para = wr.new_para();
            c_add.push_str(&wr.comment(&id, &r.author, &r.date, &r.text, &para));
            x_add.push_str(&wr.ext(&para, Some(parent), false));
            id
        })
        .collect()
}

/// Lists a new part in `[Content_Types].xml` and in the document's
/// relationships.
fn register(
    parts: &mut Parts,
    part: &str,
    content_type: &str,
    rel: &str,
) -> Result<(), WriteError> {
    let types = parts
        .get(TYPES)
        .ok_or_else(|| WriteError::Zip("the package has no [Content_Types].xml".into()))?;
    if !types.contains(&format!("PartName=\"/{part}\"")) {
        let at = types
            .rfind("</Types>")
            .ok_or_else(|| WriteError::Zip("[Content_Types].xml has no Types element".into()))?;
        let mut new = types.clone();
        new.insert_str(
            at,
            &format!("<Override PartName=\"/{part}\" ContentType=\"{content_type}\"/>"),
        );
        parts.set(TYPES, new);
    }
    let target = part.trim_start_matches("word/");
    let rels = parts.get(DOC_RELS).unwrap_or_else(|| {
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            "\r\n",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"></Relationships>"#
        )
        .to_owned()
    });
    if rels.contains(&format!("Target=\"{target}\"")) {
        return Ok(());
    }
    let mut n = 1;
    while rels.contains(&format!("Id=\"rId{n}\"")) {
        n += 1;
    }
    let at = rels.rfind("</Relationships>").ok_or_else(|| {
        WriteError::Zip("the document's relationships have no Relationships element".into())
    })?;
    let mut new = rels.clone();
    new.insert_str(
        at,
        &format!("<Relationship Id=\"rId{n}\" Type=\"{rel}\" Target=\"{target}\"/>"),
    );
    parts.set(DOC_RELS, new);
    Ok(())
}
