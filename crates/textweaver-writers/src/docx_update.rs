//! Writing a review back into a Word file in place (task B1-t2).
//!
//! [`update_docx`] takes the bytes of a `.docx` package and what was
//! decided while reviewing it ([`DocxUpdate`]), and returns the package with
//! those decisions written into its XML. Nothing is regenerated: every part
//! the review does not touch is copied byte for byte (its compressed data
//! too), and inside a touched part only the elements a decision names
//! change, so styles, numbering, headers, and everything textweaver does
//! not model survive as Word wrote them.
//!
//! Tracked changes, by Word id (`w:id`):
//!
//! - an accepted insertion (`w:ins`, `w:moveTo`) is unwrapped: its runs
//!   stay, the revision mark goes; a rejected one is removed with its text;
//! - an accepted deletion (`w:del`, `w:moveFrom`) is removed with its text;
//!   a rejected one is unwrapped, its `w:delText` becoming `w:t` again;
//! - the two halves of a move are decided together, paired by the name of
//!   their move ranges (`w:moveFromRangeStart`, `w:moveToRangeStart`), or,
//!   for moves without ranges, in reading order; the range marks go with
//!   them;
//! - a deleted paragraph mark that is accepted (or an inserted one that is
//!   rejected) joins its paragraph to the next, as Word does;
//! - formatting changes (`w:rPrChange`, `w:pPrChange`, and the table ones),
//!   inserted or deleted table rows, and paragraph marks are not in the
//!   changes list; they follow [`DocxUpdate::rest`], and a paragraph mark
//!   also follows a decided change in its paragraph by the same author at
//!   the same time (deleting a whole paragraph in Word marks both).
//!
//! Comments go to `word/comments.xml` and `word/commentsExtended.xml`:
//! replies and new comments are added (each anchored in the document beside
//! the comment it answers, or on the text it was made on), resolving sets
//! `w15:done`, and deleting removes a thread with its replies and its marks
//! in the document. The parts and their relationships are created when the
//! package has none.
//!
//! [`track_edits`] goes the other way (task B1-t3): edits made in
//! textweaver are written as tracked changes, `w:ins` and `w:del` with the
//! author and the time, for a reviewer in Word.

use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::ops::Range;
use std::path::{Path, PathBuf};

use roxmltree::Node;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::WriteError;

/// WordprocessingML's main namespace (transitional and strict).
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const W_STRICT: &str = "http://purl.oclc.org/ooxml/wordprocessingml/main";
/// Word 2010's namespace (`w14:paraId`).
const W14: &str = "http://schemas.microsoft.com/office/word/2010/wordml";
/// Word 2012's namespace (`w15:commentEx`).
const W15: &str = "http://schemas.microsoft.com/office/word/2012/wordml";

const DOCUMENT: &str = "word/document.xml";
const COMMENTS: &str = "word/comments.xml";
const EXTENDED: &str = "word/commentsExtended.xml";
const IDS: &str = "word/commentsIds.xml";
const EXTENSIBLE: &str = "word/commentsExtensible.xml";
const TYPES: &str = "[Content_Types].xml";
const DOC_RELS: &str = "word/_rels/document.xml.rels";

/// Largest part read, uncompressed, so a hostile package cannot fill
/// memory.
const MAX_PART: u64 = 512 * 1024 * 1024;

/// What to write back into a Word file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocxUpdate {
    /// Tracked changes decided, by their Word id (`w:id`): accepted (true)
    /// or rejected (false). One half of a move decides the other.
    pub decisions: HashMap<String, bool>,
    /// What happens to every tracked change not in
    /// [`decisions`](Self::decisions), formatting changes and paragraph
    /// marks included: accepted, rejected, or (`None`) left as it is.
    pub rest: Option<bool>,
    /// The document's comment threads as they are now, when any comment
    /// changed. A thread whose id is a comment in the file gains the
    /// replies after those it has there and takes its resolved state; any
    /// other thread is new. Empty leaves the comments alone.
    pub threads: Vec<CommentThread>,
    /// Ids of comment threads deleted, with their replies.
    pub deleted_comments: Vec<String>,
}

/// A comment thread as it is now.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommentThread {
    /// The comment's id: Word's `w:id` for a comment read from the file;
    /// for a new one, any id, reported back with the id written
    /// ([`UpdateReport::new_comment_ids`]).
    pub id: String,
    /// Who wrote it (a new comment).
    pub author: String,
    /// When, ISO 8601 (a new comment).
    pub date: String,
    /// The comment (a new one); lines become paragraphs.
    pub text: String,
    /// Resolved.
    pub resolved: bool,
    /// Every reply, in order: those after the ones in the file are added.
    pub replies: Vec<ThreadReply>,
    /// A new comment's text in the document (whitespace counts as one
    /// space), where it is anchored.
    pub anchor: String,
    /// Which occurrence of [`anchor`](Self::anchor) it is, from 0.
    pub occurrence: usize,
}

/// A reply in a comment thread.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadReply {
    /// Who wrote it.
    pub author: String,
    /// When, ISO 8601.
    pub date: String,
    /// The reply.
    pub text: String,
}

/// What [`update_docx`] did beyond the decisions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UpdateReport {
    /// New comments: the id given in [`CommentThread::id`], then the Word
    /// id written.
    pub new_comment_ids: Vec<(String, String)>,
    /// New comments whose text was not found in the document; they are
    /// anchored at the document's first words instead.
    pub unplaced: usize,
}

/// The package `package` (a `.docx` file's bytes) with `update` written
/// into it. Parts the update does not touch are copied unchanged.
pub fn update_docx(
    package: &[u8],
    update: &DocxUpdate,
) -> Result<(Vec<u8>, UpdateReport), WriteError> {
    let mut zip = ZipArchive::new(Cursor::new(package)).map_err(zip_err)?;
    let names: Vec<String> = zip.file_names().map(str::to_owned).collect();
    let mut parts = Parts::default();
    for name in &names {
        let wanted = revisable(name)
            || [COMMENTS, EXTENDED, IDS, EXTENSIBLE, TYPES, DOC_RELS].contains(&name.as_str());
        if wanted {
            let text = read_part(&mut zip, name)?;
            parts.original.insert(name.clone(), text);
        }
    }
    if !parts.original.contains_key(DOCUMENT) {
        return Err(WriteError::Zip(
            "this is not a Word document: it has no word/document.xml".into(),
        ));
    }
    if update.rest.is_some() || !update.decisions.is_empty() {
        for name in names.iter().filter(|n| revisable(n)) {
            let src = parts.get(name).unwrap_or_default();
            if let Some(new) = revise(&src, update)? {
                parts.set(name, new);
            }
        }
    }
    let mut report = UpdateReport::default();
    if !update.threads.is_empty() || !update.deleted_comments.is_empty() {
        comments::write(&mut parts, update, &mut report)?;
    }
    Ok((assemble(&mut zip, &names, &parts)?, report))
}

/// Copies `path` to a free name beside it that says what it is,
/// `report-original.docx` (then `report-original-2.docx`, and so on), and
/// returns that name. Run before the first write over a file.
pub fn backup(path: &Path) -> std::io::Result<PathBuf> {
    let target = backup_path(path);
    std::fs::copy(path, &target)?;
    Ok(target)
}

/// The free name [`backup`] copies `path` to.
pub fn backup_path(path: &Path) -> PathBuf {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document".into());
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_else(|| "docx".into());
    let dir = path.parent().unwrap_or(Path::new(""));
    let mut n = 1;
    loop {
        let name = if n == 1 {
            format!("{stem}-original.{ext}")
        } else {
            format!("{stem}-original-{n}.{ext}")
        };
        let p = dir.join(name);
        if !p.exists() {
            return p;
        }
        n += 1;
    }
}

fn zip_err(e: zip::result::ZipError) -> WriteError {
    WriteError::Zip(e.to_string())
}

/// The parts that hold tracked changes: the body, headers, footers,
/// footnotes, and endnotes.
fn revisable(name: &str) -> bool {
    let Some(file) = name.strip_prefix("word/") else {
        return false;
    };
    !file.contains('/')
        && file.ends_with(".xml")
        && (file == "document.xml"
            || file == "footnotes.xml"
            || file == "endnotes.xml"
            || file.starts_with("header")
            || file.starts_with("footer"))
}

fn read_part<R: Read + std::io::Seek>(
    zip: &mut ZipArchive<R>,
    name: &str,
) -> Result<String, WriteError> {
    let f = zip.by_name(name).map_err(zip_err)?;
    let mut bytes = Vec::new();
    f.take(MAX_PART + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_PART {
        return Err(WriteError::Zip(format!("{name} is too large to edit")));
    }
    String::from_utf8(bytes).map_err(|_| WriteError::Zip(format!("{name} is not UTF-8 text")))
}

/// The package's parts as read, and those changed or added.
#[derive(Default)]
struct Parts {
    original: HashMap<String, String>,
    changed: HashMap<String, String>,
    /// Parts added, in order.
    added: Vec<String>,
}

impl Parts {
    /// A part's text now.
    fn get(&self, name: &str) -> Option<String> {
        self.changed
            .get(name)
            .or_else(|| self.original.get(name))
            .cloned()
    }

    fn set(&mut self, name: &str, text: String) {
        if !self.original.contains_key(name) && !self.added.iter().any(|a| a == name) {
            self.added.push(name.to_owned());
        }
        self.changed.insert(name.to_owned(), text);
    }
}

/// The new package: every entry in its old order, those unchanged copied
/// raw, then the parts added.
fn assemble(
    zip: &mut ZipArchive<Cursor<&[u8]>>,
    names: &[String],
    parts: &Parts,
) -> Result<Vec<u8>, WriteError> {
    let mut out = ZipWriter::new(Cursor::new(Vec::new()));
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (i, name) in names.iter().enumerate() {
        match parts.changed.get(name) {
            Some(text) => {
                let options = match zip.by_index_raw(i).map_err(zip_err)?.last_modified() {
                    Some(t) => deflated.last_modified_time(t),
                    None => deflated,
                };
                out.start_file(name.as_str(), options).map_err(zip_err)?;
                std::io::Write::write_all(&mut out, text.as_bytes())?;
            }
            None => {
                let f = zip.by_index_raw(i).map_err(zip_err)?;
                out.raw_copy_file(f).map_err(zip_err)?;
            }
        }
    }
    for name in &parts.added {
        if let Some(text) = parts.changed.get(name) {
            out.start_file(name.as_str(), deflated).map_err(zip_err)?;
            std::io::Write::write_all(&mut out, text.as_bytes())?;
        }
    }
    Ok(out.finish().map_err(zip_err)?.into_inner())
}

// ----- XML positions ---------------------------------------------------

fn is_w(n: Node<'_, '_>) -> bool {
    n.is_element() && matches!(n.tag_name().namespace(), Some(W | W_STRICT))
}

fn is_w_named(n: Node<'_, '_>, local: &str) -> bool {
    is_w(n) && n.tag_name().name() == local
}

/// An attribute by local name, whatever its prefix.
fn attr<'a>(n: Node<'a, '_>, local: &str) -> Option<&'a str> {
    n.attributes()
        .find(|a| a.name() == local)
        .map(|a| a.value())
}

/// Where `n`'s start tag ends (just after its `>`).
fn start_tag_end(src: &str, n: Node<'_, '_>) -> usize {
    let r = n.range();
    let mut quote = None;
    for (i, &b) in src
        .as_bytes()
        .iter()
        .enumerate()
        .take(r.end)
        .skip(r.start + 1)
    {
        match (quote, b) {
            (None, b'"' | b'\'') => quote = Some(b),
            (Some(q), b) if q == b => quote = None,
            (None, b'>') => return i + 1,
            _ => {}
        }
    }
    r.end
}

fn self_closing(src: &str, n: Node<'_, '_>) -> bool {
    src[..start_tag_end(src, n)].ends_with("/>")
}

/// `n`'s content: after its start tag, before its end tag (empty for
/// `<a/>`).
fn inner(src: &str, n: Node<'_, '_>) -> Range<usize> {
    let start = start_tag_end(src, n);
    if self_closing(src, n) {
        return start..start;
    }
    let end = n.range().end;
    let close = src[start..end].rfind("</").map_or(end, |i| start + i);
    start..close
}

/// The element's qualified name as written (`w:p`).
fn qname<'s>(src: &'s str, n: Node<'_, '_>) -> &'s str {
    let rest = &src[n.range().start + 1..];
    let len = rest
        .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
        .unwrap_or(rest.len());
    &rest[..len]
}

/// `prefix:local`, or `local` for an empty prefix.
fn q(prefix: &str, local: &str) -> String {
    if prefix.is_empty() {
        local.to_owned()
    } else {
        format!("{prefix}:{local}")
    }
}

fn parse<'a>(name: &str, src: &'a str) -> Result<roxmltree::Document<'a>, WriteError> {
    roxmltree::Document::parse(src)
        .map_err(|e| WriteError::Zip(format!("{name} is not well-formed XML: {e}")))
}

// ----- Tracked changes ---------------------------------------------------

/// The move each `w:moveFrom` and `w:moveTo` belongs to (its id to a key),
/// and each move range's key (its range id to the key): the range's name,
/// or for moves outside any range, `#n` for the n-th such half of each
/// kind in reading order. The DOCX loader pairs the halves by this rule.
fn move_keys(root: Node<'_, '_>) -> (HashMap<String, String>, HashMap<String, String>) {
    let mut halves = HashMap::new();
    let mut ranges = HashMap::new();
    let mut active: [Vec<(String, String)>; 2] = [Vec::new(), Vec::new()];
    let mut unnamed = [0usize; 2];
    for n in root.descendants().filter(|n| is_w(*n)) {
        let name = n.tag_name().name();
        let id = attr(n, "id").unwrap_or_default().to_owned();
        let side = usize::from(name.starts_with("moveTo"));
        match name {
            "moveFromRangeStart" | "moveToRangeStart" => {
                let key = attr(n, "name").unwrap_or(&id).to_owned();
                ranges.insert(id.clone(), key.clone());
                active[side].push((id, key));
            }
            "moveFromRangeEnd" | "moveToRangeEnd" => active[side].retain(|(r, _)| *r != id),
            "moveFrom" | "moveTo" if !in_properties(n) => {
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
    (halves, ranges)
}

/// True for a revision mark inside properties (`w:rPr`, `w:trPr`): a
/// paragraph mark, a row, or a run mark, not a wrapper of runs.
fn in_properties(n: Node<'_, '_>) -> bool {
    n.parent_element()
        .is_some_and(|p| p.tag_name().name().ends_with("Pr"))
}

/// A formatting change (`w:rPrChange`, `w:pPrChange`, `w:tblGridChange`,
/// and the like): rejecting it puts its old properties back.
fn is_property_change(name: &str) -> bool {
    name.ends_with("PrChange") || name.ends_with("PrExChange") || name == "tblGridChange"
}

/// How an element is written back.
enum Act {
    Keep,
    Remove,
    /// Its content stays, the element goes.
    Unwrap,
    /// Unwrapped, with deleted text made text again.
    Restore,
}

struct Reviser<'s, 'u> {
    src: &'s str,
    update: &'u DocxUpdate,
    /// A move half's id to its move.
    move_of: HashMap<String, String>,
    /// A move range's id to its move.
    range_of: HashMap<String, String>,
    /// A move decided by one of its halves.
    move_decided: HashMap<String, bool>,
    /// Inside a rejected deletion: `w:delText` becomes `w:t`.
    restoring: usize,
    changed: bool,
}

/// `src` with `update`'s decisions applied, or `None` when nothing in it
/// changed.
fn revise(src: &str, update: &DocxUpdate) -> Result<Option<String>, WriteError> {
    let xml = parse("a document part", src)?;
    let root = xml.root_element();
    let (move_of, range_of) = move_keys(root);
    let move_decided = move_of
        .iter()
        .filter_map(|(id, key)| Some((key.clone(), *update.decisions.get(id)?)))
        .collect();
    let mut r = Reviser {
        src,
        update,
        move_of,
        range_of,
        move_decided,
        restoring: 0,
        changed: false,
    };
    let mut out = String::with_capacity(src.len());
    out.push_str(&src[..root.range().start]);
    r.element(root, &mut out, "");
    out.push_str(&src[root.range().end..]);
    Ok(r.changed.then_some(out))
}

impl Reviser<'_, '_> {
    /// A revision's decision: by its id, by the other half of its move, or
    /// the rest's.
    fn decision(&self, n: Node<'_, '_>) -> Option<bool> {
        let id = attr(n, "id");
        if let Some(&d) = id.and_then(|i| self.update.decisions.get(i)) {
            return Some(d);
        }
        if let Some(&d) = id
            .and_then(|i| self.move_of.get(i))
            .and_then(|k| self.move_decided.get(k))
        {
            return Some(d);
        }
        self.update.rest
    }

    /// A paragraph mark's decision: its own, else that of a decided change
    /// of the same kind in its paragraph by the same author at the same
    /// time.
    fn mark_decision(&self, mark: Node<'_, '_>, p: Node<'_, '_>) -> Option<bool> {
        if let Some(d) = self.decision(mark) {
            return Some(d);
        }
        let adds = matches!(mark.tag_name().name(), "ins" | "moveTo");
        p.descendants()
            .filter(|n| {
                is_w(*n)
                    && !in_properties(*n)
                    && match n.tag_name().name() {
                        "ins" | "moveTo" => adds,
                        "del" | "moveFrom" => !adds,
                        _ => false,
                    }
                    && attr(*n, "author") == attr(mark, "author")
                    && attr(*n, "date") == attr(mark, "date")
            })
            .find_map(|n| self.decision(n))
    }

    /// A paragraph's mark revision, if it has one.
    fn mark<'a, 'i>(p: Node<'a, 'i>) -> Option<Node<'a, 'i>> {
        let ppr = p.children().find(|c| is_w_named(*c, "pPr"))?;
        let rpr = ppr.children().find(|c| is_w_named(*c, "rPr"))?;
        rpr.children().find(|c| {
            is_w(*c) && matches!(c.tag_name().name(), "ins" | "del" | "moveFrom" | "moveTo")
        })
    }

    /// True when paragraph `p`'s mark goes (a deleted mark accepted, an
    /// inserted one rejected), so it joins the next paragraph. A paragraph
    /// ending a section keeps its mark.
    fn joins_next(&self, p: Node<'_, '_>) -> bool {
        let Some(mark) = Self::mark(p) else {
            return false;
        };
        let adds = matches!(mark.tag_name().name(), "ins" | "moveTo");
        let ends_section = p
            .children()
            .filter(|c| is_w_named(*c, "pPr"))
            .any(|ppr| ppr.children().any(|c| is_w_named(c, "sectPr")));
        !ends_section && self.mark_decision(mark, p) == Some(!adds)
    }

    /// Removed when decided, kept otherwise.
    fn removed_if(d: Option<bool>) -> Act {
        if d.is_some() { Act::Remove } else { Act::Keep }
    }

    fn act(&self, n: Node<'_, '_>) -> Act {
        if !is_w(n) {
            return Act::Keep;
        }
        let name = n.tag_name().name();
        match name {
            "ins" | "del" | "moveFrom" | "moveTo" if in_properties(n) => {
                // A paragraph mark follows its paragraph; any other mark
                // (a row's, a run's) its own decision.
                let para = n
                    .parent_element()
                    .and_then(|rpr| rpr.parent_element())
                    .filter(|ppr| ppr.tag_name().name() == "pPr")
                    .and_then(|ppr| ppr.parent_element());
                Self::removed_if(match para {
                    Some(p) => self.mark_decision(n, p),
                    None => self.decision(n),
                })
            }
            "ins" | "moveTo" => match self.decision(n) {
                Some(true) => Act::Unwrap,
                Some(false) => Act::Remove,
                None => Act::Keep,
            },
            "del" | "moveFrom" => match self.decision(n) {
                Some(true) => Act::Remove,
                Some(false) => Act::Restore,
                None => Act::Keep,
            },
            "moveFromRangeStart" | "moveFromRangeEnd" | "moveToRangeStart" | "moveToRangeEnd" => {
                let key = attr(n, "id").and_then(|i| self.range_of.get(i));
                Self::removed_if(
                    key.and_then(|k| self.move_decided.get(k).copied())
                        .or(self.update.rest),
                )
            }
            // shortcut: a rejected inserted cell keeps the cell (only its
            // mark goes); remove the cell when tables are reviewed.
            "cellIns" | "cellDel" | "cellMerge" | "numberingChange" => {
                Self::removed_if(self.decision(n))
            }
            "tr" => {
                let mark = n.children().find(|c| is_w_named(*c, "trPr")).and_then(|t| {
                    t.children()
                        .find(|c| is_w(*c) && matches!(c.tag_name().name(), "ins" | "del"))
                });
                match mark {
                    Some(m) if self.decision(m) == Some(m.tag_name().name() == "del") => {
                        Act::Remove
                    }
                    _ => Act::Keep,
                }
            }
            _ if is_property_change(name) => Self::removed_if(self.decision(n)),
            _ => Act::Keep,
        }
    }

    /// A rejected formatting change among `n`'s children.
    fn rejected_property_change<'a, 'i>(&self, n: Node<'a, 'i>) -> Option<Node<'a, 'i>> {
        n.children().find(|c| {
            is_w(*c) && is_property_change(c.tag_name().name()) && self.decision(*c) == Some(false)
        })
    }

    /// Writes element `n` as decided. `prefix` (a joined paragraph's
    /// content) goes after a paragraph's properties.
    fn element(&mut self, n: Node<'_, '_>, out: &mut String, prefix: &str) {
        match self.act(n) {
            Act::Keep => {}
            Act::Remove => {
                self.changed = true;
                return;
            }
            Act::Unwrap => {
                self.changed = true;
                self.children(n, out, "");
                return;
            }
            Act::Restore => {
                self.changed = true;
                self.restoring += 1;
                self.children(n, out, "");
                self.restoring -= 1;
                return;
            }
        }
        let src = self.src;
        let start = n.range().start;
        let st = start_tag_end(src, n);
        let closed = self_closing(src, n);
        let name = qname(src, n);
        let local = match n.tag_name().name() {
            "delText" if self.restoring > 0 && is_w(n) => Some("t"),
            "delInstrText" if self.restoring > 0 && is_w(n) => Some("instrText"),
            _ => None,
        };
        let renamed = local.map(|l| match name.split_once(':') {
            Some((p, _)) => q(p, l),
            None => l.to_owned(),
        });
        let property_change = self.rejected_property_change(n);
        if renamed.is_none() && property_change.is_none() && prefix.is_empty() {
            // The start tag as written.
            out.push_str(&src[start..st]);
            if !closed {
                self.children(n, out, "");
                out.push_str(&src[inner(src, n).end..n.range().end]);
            }
            return;
        }
        self.changed |= renamed.is_some();
        let tag = renamed.as_deref().unwrap_or(name);
        out.push('<');
        out.push_str(tag);
        let attrs_end = if closed { st - 2 } else { st - 1 };
        out.push_str(&src[start + 1 + name.len()..attrs_end]);
        if closed && prefix.is_empty() && property_change.is_none() {
            out.push_str("/>");
            return;
        }
        out.push('>');
        if let Some(change) = property_change {
            // The old properties, then the children the change does not
            // cover.
            self.changed = true;
            if let Some(old) = change.children().find(|c| c.is_element()) {
                out.push_str(&src[inner(src, old)]);
            }
            let kept: &[&str] = match n.tag_name().name() {
                "pPr" => &["rPr", "sectPr"],
                "rPr" => &["ins", "del", "moveFrom", "moveTo"],
                _ => &[],
            };
            for c in n
                .children()
                .filter(|c| is_w(*c) && kept.contains(&c.tag_name().name()))
            {
                self.element(c, out, "");
            }
        } else if closed {
            out.push_str(prefix);
        } else {
            self.children(n, out, prefix);
        }
        out.push_str("</");
        out.push_str(tag);
        out.push('>');
    }

    /// Writes `n`'s content: each child as decided, a paragraph whose mark
    /// goes joined to the next one, and `prefix` after a paragraph's
    /// properties (or first, when it has none).
    fn children(&mut self, n: Node<'_, '_>, out: &mut String, prefix: &str) {
        let src = self.src;
        let range = inner(src, n);
        let mut at = range.start;
        let mut prefix_due = !prefix.is_empty();
        if prefix_due && !n.children().any(|c| is_w_named(c, "pPr")) {
            out.push_str(prefix);
            prefix_due = false;
        }
        // A paragraph joined to the next: its whole self (written if no
        // paragraph follows) and its content (which goes into the next).
        let mut carry: Option<(String, String)> = None;
        for c in n.children() {
            let r = c.range();
            if r.start < at {
                continue;
            }
            out.push_str(&src[at..r.start]);
            at = r.end;
            if is_w_named(c, "p") {
                let before = carry.take().map(|(_, content)| content).unwrap_or_default();
                if self.joins_next(c) {
                    self.changed = true;
                    let mut whole = String::new();
                    self.element(c, &mut whole, &before);
                    let mut content = before;
                    for k in c.children().filter(|k| !is_w_named(*k, "pPr")) {
                        self.emit(k, &mut content);
                    }
                    carry = Some((whole, content));
                } else {
                    self.element(c, out, &before);
                }
                continue;
            }
            if c.is_element()
                && let Some((whole, _)) = carry.take()
            {
                out.push_str(&whole);
            }
            self.emit(c, out);
            if prefix_due && is_w_named(c, "pPr") {
                out.push_str(prefix);
                prefix_due = false;
            }
        }
        if let Some((whole, _)) = carry.take() {
            out.push_str(&whole);
        }
        out.push_str(&src[at.min(range.end)..range.end]);
    }

    fn emit(&mut self, n: Node<'_, '_>, out: &mut String) {
        if n.is_element() {
            self.element(n, out, "");
        } else {
            out.push_str(&self.src[n.range()]);
        }
    }
}

mod comments;
mod track;

pub use track::{TrackReport, TrackedEdit, track_edits};

#[cfg(test)]
mod tests;
