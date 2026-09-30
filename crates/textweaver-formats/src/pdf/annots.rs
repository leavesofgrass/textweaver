//! Links and comments on PDF pages (ADR-0048).
//!
//! A PDF keeps both outside the page's text, as annotations placed by
//! rectangle:
//!
//! - **Links** (`/Link`) become `Link` markers over the text they cover.
//!   A web or mail address is the marker's reference, so following the
//!   link says the address and offers to open it. A link inside the
//!   document goes to the heading at its destination when one starts
//!   there (`#methods`, the same form Markdown uses), else to the page
//!   (`#page=12`, by printed page label, as PDF viewers write it); a link
//!   to another file names the file. Actions that run programs or scripts
//!   are ignored.
//! - **Comments** (sticky notes, typed comments, highlights, underlines,
//!   strike-outs, and drawn marks, with their `/Contents`) become
//!   [`DocumentComment`]s, as Word comments do (ADR-0031): anchored at the
//!   text they mark, or, for a note beside the text, at the start of the
//!   nearest line. Replies (`/IRT`) go under the comment they answer, and a
//!   review state of Completed or Accepted marks it resolved. A highlight
//!   with no comment typed into it is kept as "Highlighted", since a
//!   lecturer's marking is itself a comment.
//!
//! Everything is bounded: at most [`MAX_ANNOTS`] annotations are read, name
//! trees and reply chains stop at fixed depths, and texts are cut at
//! [`MAX_COMMENT_CHARS`](crate::annotations::MAX_COMMENT_CHARS).

use std::collections::{HashMap, HashSet};

use lopdf::{Dictionary, Object, ObjectId};
use textweaver_core::{CharPos, CharRange, MarkerKind};
use textweaver_text::{Document, Marker};

use super::interp::PageContent;
use super::locate::{self, Area, TextIndex};
use crate::annotations::{CommentReply, DocumentComment, clean_text};

/// Most annotations read from one document.
pub const MAX_ANNOTS: usize = 20_000;

/// The anchor prefix of a link to a page: `#page=12`.
const PAGE_ANCHOR: &str = "page=";

/// Where a link goes.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Target {
    /// A web or mail address.
    Uri(String),
    /// A page of this document (from 0), and the top of the destination in
    /// the page's own space, when given.
    Page { index: usize, top: Option<f32> },
    /// Another file.
    File(String),
}

/// A link annotation.
#[derive(Clone, Debug)]
pub(super) struct Link {
    pub page: usize,
    pub rect: [f32; 4],
    pub target: Target,
}

/// A comment annotation, with its replies.
#[derive(Clone, Debug, Default)]
pub(super) struct Note {
    pub page: usize,
    /// The marked areas (a highlight's quadrilaterals), else the
    /// annotation's rectangle.
    pub areas: Vec<[f32; 4]>,
    /// True when the note marks text (a highlight, a box around words);
    /// false when it sits beside it (a sticky note).
    pub marks_text: bool,
    /// What a markup without a typed comment says ("Highlighted").
    pub markup_word: Option<&'static str>,
    pub author: String,
    pub date: String,
    pub text: String,
    pub replies: Vec<CommentReply>,
    pub resolved: bool,
}

/// What a document's pages carry.
#[derive(Debug, Default)]
pub(super) struct Annots {
    pub links: Vec<Link>,
    pub notes: Vec<Note>,
    /// True when some were left out because there were too many.
    pub truncated: bool,
}

/// One annotation as read, before replies are gathered.
struct Raw {
    id: Option<ObjectId>,
    page: usize,
    subtype: Vec<u8>,
    dict: Dictionary,
}

fn name<'a>(d: &'a Dictionary, key: &[u8]) -> &'a [u8] {
    d.get(key).and_then(Object::as_name).unwrap_or(b"")
}

fn text(pdf: &lopdf::Document, d: &Dictionary, key: &[u8]) -> String {
    d.get(key)
        .ok()
        .and_then(|o| pdf.dereference(o).ok())
        .and_then(|(_, o)| lopdf::decode_text_string(o).ok())
        .map(|s| clean_text(&s))
        .unwrap_or_default()
}

fn rect(pdf: &lopdf::Document, d: &Dictionary) -> Option<[f32; 4]> {
    let a = pdf
        .dereference(d.get(b"Rect").ok()?)
        .ok()?
        .1
        .as_array()
        .ok()?;
    let v = numbers(pdf, a);
    (v.len() == 4).then(|| {
        [
            v[0].min(v[2]),
            v[1].min(v[3]),
            v[0].max(v[2]),
            v[1].max(v[3]),
        ]
    })
}

fn numbers(pdf: &lopdf::Document, a: &[Object]) -> Vec<f32> {
    a.iter()
        .take(4_096)
        .filter_map(|o| match pdf.dereference(o).ok()?.1 {
            Object::Integer(i) => Some(*i as f32),
            Object::Real(r) => Some(*r),
            _ => None,
        })
        .filter(|v| v.is_finite())
        .collect()
}

/// A highlight's quadrilaterals as rectangles.
fn quads(pdf: &lopdf::Document, d: &Dictionary) -> Vec<[f32; 4]> {
    let Some(a) = d
        .get(b"QuadPoints")
        .ok()
        .and_then(|o| pdf.dereference(o).ok())
        .and_then(|(_, o)| o.as_array().ok())
    else {
        return Vec::new();
    };
    numbers(pdf, a)
        .chunks_exact(8)
        .map(|q| {
            let xs = [q[0], q[2], q[4], q[6]];
            let ys = [q[1], q[3], q[5], q[7]];
            [
                xs.into_iter().fold(f32::MAX, f32::min),
                ys.into_iter().fold(f32::MAX, f32::min),
                xs.into_iter().fold(f32::MIN, f32::max),
                ys.into_iter().fold(f32::MIN, f32::max),
            ]
        })
        .collect()
}

/// A PDF date (`D:20260901103000+02'00'`) as ISO 8601
/// (`2026-09-01T10:30:00+02:00`); anything else as written.
pub(super) fn iso_date(s: &str) -> String {
    let d = s.trim().strip_prefix("D:").unwrap_or(s.trim());
    let digits: String = d.chars().take_while(char::is_ascii_digit).collect();
    if digits.len() < 4 {
        return s.trim().to_owned();
    }
    let part = |a: usize, b: usize, default: &str| {
        digits
            .get(a..b)
            .map_or_else(|| default.to_owned(), str::to_owned)
    };
    let mut out = format!(
        "{}-{}-{}",
        part(0, 4, "0000"),
        part(4, 6, "01"),
        part(6, 8, "01")
    );
    if digits.len() >= 10 {
        out.push_str(&format!(
            "T{}:{}:{}",
            part(8, 10, "00"),
            part(10, 12, "00"),
            part(12, 14, "00")
        ));
        let zone: String = d[digits.len()..].chars().filter(|c| *c != '\'').collect();
        match zone.chars().next() {
            Some('Z') => out.push('Z'),
            Some(sign @ ('+' | '-')) if zone.len() >= 3 => {
                let hh = zone.get(1..3).unwrap_or("00");
                let mm = zone.get(3..5).unwrap_or("00");
                if hh.chars().chain(mm.chars()).all(|c| c.is_ascii_digit()) {
                    out.push_str(&format!("{sign}{hh}:{mm}"));
                }
            }
            _ => {}
        }
    }
    out
}

/// Rich text (`/RC`, a little XHTML) as plain text.
fn strip_markup(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => {
                in_tag = true;
                out.push(' ');
            }
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    let out = out
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&");
    clean_text(&out)
}

/// The comment typed into an annotation.
fn contents(pdf: &lopdf::Document, d: &Dictionary) -> String {
    let t = text(pdf, d, b"Contents");
    if !t.is_empty() {
        return t;
    }
    strip_markup(&text(pdf, d, b"RC"))
}

/// What a markup annotation does to the text, said when no comment was
/// typed into it.
fn markup_word(subtype: &[u8]) -> Option<&'static str> {
    Some(match subtype {
        b"Highlight" => "Highlighted",
        b"Underline" => "Underlined",
        b"Squiggly" => "Underlined with a wavy line",
        b"StrikeOut" => "Struck out",
        _ => return None,
    })
}

/// Reads every page's links and comments.
pub(super) fn read(pdf: &lopdf::Document, page_ids: &[ObjectId]) -> Annots {
    let index: HashMap<ObjectId, usize> = page_ids
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, i))
        .collect();
    let mut out = Annots::default();
    let mut raw: Vec<Raw> = Vec::new();
    'pages: for (page, &id) in page_ids.iter().enumerate() {
        let Some(list) = pdf
            .get_dictionary(id)
            .ok()
            .and_then(|p| p.get(b"Annots").ok())
            .and_then(|o| pdf.dereference(o).ok())
            .and_then(|(_, o)| o.as_array().ok())
        else {
            continue;
        };
        for item in list {
            if raw.len() >= MAX_ANNOTS {
                out.truncated = true;
                break 'pages;
            }
            let own = item.as_reference().ok();
            let Some(dict) = pdf
                .dereference(item)
                .ok()
                .and_then(|(_, o)| o.as_dict().ok())
            else {
                continue;
            };
            // Hidden annotations (flag 2) are not shown, so not read; a
            // review state is hidden as a rule and still counts.
            let flags = dict.get(b"F").and_then(Object::as_i64).unwrap_or(0);
            if flags & 2 != 0 && dict.get(b"State").is_err() {
                continue;
            }
            raw.push(Raw {
                id: own,
                page,
                subtype: name(dict, b"Subtype").to_vec(),
                dict: dict.clone(),
            });
        }
    }
    let dests = Dests::new(pdf);
    for r in &raw {
        if r.subtype == b"Link"
            && let Some(rect) = rect(pdf, &r.dict)
            && let Some(target) = link_target(pdf, &r.dict, &index, &dests)
        {
            out.links.push(Link {
                page: r.page,
                rect,
                target,
            });
        }
    }
    out.notes = notes(pdf, &raw);
    out
}

/// Annotations that are not comments.
const NOT_COMMENTS: &[&[u8]] = &[
    b"Link",
    b"Popup",
    b"Widget",
    b"Screen",
    b"PrinterMark",
    b"TrapNet",
    b"Watermark",
    b"3D",
    b"Movie",
    b"RichMedia",
];

/// Comments with their replies and states, in page order.
fn notes(pdf: &lopdf::Document, raw: &[Raw]) -> Vec<Note> {
    let by_id: HashMap<ObjectId, usize> = raw
        .iter()
        .enumerate()
        .filter_map(|(i, r)| r.id.map(|id| (id, i)))
        .collect();
    let is_comment = |r: &Raw| !NOT_COMMENTS.contains(&r.subtype.as_slice());
    // The comment a reply belongs to: its thread's first comment.
    let root_of = |mut i: usize| {
        let mut seen = HashSet::new();
        while seen.insert(i) && seen.len() <= 64 {
            let parent = raw[i]
                .dict
                .get(b"IRT")
                .and_then(Object::as_reference)
                .ok()
                .and_then(|p| by_id.get(&p).copied());
            match parent {
                Some(p) if p != i && is_comment(&raw[p]) => i = p,
                _ => break,
            }
        }
        i
    };
    let mut out: Vec<Note> = Vec::new();
    let mut slot: HashMap<usize, usize> = HashMap::new();
    // Replies and states, gathered after their thread's comment exists.
    let mut later: Vec<(usize, usize)> = Vec::new();
    for (i, r) in raw.iter().enumerate() {
        if !is_comment(r) {
            continue;
        }
        let d = &r.dict;
        let reply = d.get(b"IRT").is_ok();
        // A grouped annotation is part of its parent, not a reply.
        if reply && name(d, b"RT") == b"Group" {
            continue;
        }
        let root = root_of(i);
        if reply && root != i {
            later.push((i, root));
            continue;
        }
        let quad = quads(pdf, d);
        let marks = markup_word(&r.subtype);
        let marks_text = marks.is_some() || matches!(r.subtype.as_slice(), b"Square" | b"Circle");
        let areas = if quad.is_empty() {
            rect(pdf, d).into_iter().collect()
        } else {
            quad
        };
        if areas.is_empty() {
            continue;
        }
        slot.insert(i, out.len());
        out.push(Note {
            page: r.page,
            areas,
            marks_text,
            markup_word: marks,
            author: text(pdf, d, b"T"),
            date: iso_date(&text(pdf, d, b"M")),
            text: contents(pdf, d),
            replies: Vec::new(),
            resolved: false,
        });
    }
    for (i, root) in later {
        let Some(&k) = slot.get(&root) else {
            continue;
        };
        let d = &raw[i].dict;
        if d.get(b"State").is_ok() || d.get(b"StateModel").is_ok() {
            // A review state: the latest one decides.
            let model = text(pdf, d, b"StateModel");
            if model.is_empty() || model == "Review" {
                let state = text(pdf, d, b"State");
                out[k].resolved = matches!(state.as_str(), "Completed" | "Accepted");
            }
            continue;
        }
        let t = contents(pdf, d);
        if t.is_empty() {
            continue;
        }
        out[k].replies.push(CommentReply {
            author: text(pdf, d, b"T"),
            date: iso_date(&text(pdf, d, b"M")),
            text: t,
        });
    }
    // A markup with nothing typed and no replies is a plain highlight: kept
    // with its word; other empty notes say nothing and are dropped.
    out.retain(|n| !n.text.is_empty() || !n.replies.is_empty() || n.markup_word.is_some());
    out
}

/// Named destinations: the catalog's `/Dests` and the `/Names` tree.
struct Dests<'a> {
    pdf: &'a lopdf::Document,
    old: Option<&'a Dictionary>,
    tree: Option<&'a Dictionary>,
}

impl<'a> Dests<'a> {
    fn new(pdf: &'a lopdf::Document) -> Self {
        let cat = pdf.catalog().ok();
        let dict = |o: Option<&'a Object>| {
            o.and_then(|o| pdf.dereference(o).ok())
                .and_then(|(_, o)| o.as_dict().ok())
        };
        let old = dict(cat.and_then(|c| c.get(b"Dests").ok()));
        let tree =
            dict(dict(cat.and_then(|c| c.get(b"Names").ok())).and_then(|n| n.get(b"Dests").ok()));
        Dests { pdf, old, tree }
    }

    fn lookup(&self, key: &[u8]) -> Option<&'a Object> {
        if let Some(o) = self.old.and_then(|d| d.get(key).ok()) {
            return Some(o);
        }
        let mut visits = 0;
        self.search(self.tree?, key, 0, &mut visits)
    }

    fn search(
        &self,
        node: &'a Dictionary,
        key: &[u8],
        depth: usize,
        visits: &mut usize,
    ) -> Option<&'a Object> {
        *visits += 1;
        if depth > 32 || *visits > 10_000 {
            return None;
        }
        if let Ok(names) = node.get(b"Names").and_then(Object::as_array) {
            for pair in names.chunks(2) {
                if let [k, v] = pair
                    && let Ok(k) = k.as_str()
                    && k == key
                {
                    return Some(v);
                }
            }
        }
        let kids = node.get(b"Kids").and_then(Object::as_array).ok()?;
        kids.iter().find_map(|k| {
            let d = self.pdf.dereference(k).ok()?.1.as_dict().ok()?;
            self.search(d, key, depth + 1, visits)
        })
    }
}

/// A destination (`[page /XYZ left top zoom]`, a name, or a dictionary
/// with `/D`) as a page and the top of the view.
fn destination(
    pdf: &lopdf::Document,
    o: &Object,
    index: &HashMap<ObjectId, usize>,
    dests: &Dests<'_>,
    depth: usize,
) -> Option<Target> {
    if depth > 8 {
        return None;
    }
    let (_, o) = pdf.dereference(o).ok()?;
    match o {
        Object::Array(a) => {
            let page = match a.first()? {
                Object::Reference(id) => *index.get(id)?,
                Object::Integer(n) => usize::try_from(*n).ok()?,
                _ => return None,
            };
            let num = |k: usize| match a.get(k).and_then(|o| pdf.dereference(o).ok()) {
                Some((_, Object::Integer(i))) => Some(*i as f32),
                Some((_, Object::Real(r))) if r.is_finite() => Some(*r),
                _ => None,
            };
            let top = match a.get(1).and_then(|o| o.as_name().ok()) {
                Some(b"XYZ") => num(3),
                Some(b"FitH" | b"FitBH") => num(2),
                Some(b"FitR") => num(5),
                _ => None,
            };
            Some(Target::Page { index: page, top })
        }
        Object::Name(n) => destination(pdf, dests.lookup(n)?, index, dests, depth + 1),
        Object::String(s, _) => destination(pdf, dests.lookup(s)?, index, dests, depth + 1),
        Object::Dictionary(d) => destination(pdf, d.get(b"D").ok()?, index, dests, depth + 1),
        _ => None,
    }
}

/// A web or mail address worth offering: http, https, mailto, or ftp;
/// `www.` names get `https://`. Scripts, data, and file addresses are
/// dropped.
pub(super) fn clean_uri(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() || s.len() > 2_048 || s.chars().any(char::is_control) {
        return None;
    }
    let lower = s.to_ascii_lowercase();
    if ["http://", "https://", "mailto:", "ftp://"]
        .iter()
        .any(|p| lower.starts_with(p))
    {
        return Some(s.to_owned());
    }
    lower.starts_with("www.").then(|| format!("https://{s}"))
}

/// A file named by a file specification (a string, or a dictionary with
/// `/UF` or `/F`), without folders that climb out (`..`).
fn file_spec(pdf: &lopdf::Document, o: &Object) -> Option<String> {
    let (_, o) = pdf.dereference(o).ok()?;
    let name = match o {
        Object::Dictionary(d) => d
            .get(b"UF")
            .or_else(|_| d.get(b"F"))
            .ok()
            .and_then(|f| lopdf::decode_text_string(f).ok())?,
        other => lopdf::decode_text_string(other).ok()?,
    };
    let name = name.trim().replace('\\', "/");
    let ok = !name.is_empty()
        && name.len() <= 1_024
        && !name.chars().any(char::is_control)
        && !name.split('/').any(|p| p == "..")
        && !name.contains(':');
    ok.then_some(name)
}

fn link_target(
    pdf: &lopdf::Document,
    d: &Dictionary,
    index: &HashMap<ObjectId, usize>,
    dests: &Dests<'_>,
) -> Option<Target> {
    if let Ok(dest) = d.get(b"Dest") {
        return destination(pdf, dest, index, dests, 0);
    }
    let action = pdf.dereference(d.get(b"A").ok()?).ok()?.1.as_dict().ok()?;
    match name(action, b"S") {
        b"URI" => {
            let uri = action.get(b"URI").ok()?;
            let (_, uri) = pdf.dereference(uri).ok()?;
            let bytes = uri.as_str().ok()?;
            clean_uri(&String::from_utf8_lossy(bytes)).map(Target::Uri)
        }
        b"GoTo" => destination(pdf, action.get(b"D").ok()?, index, dests, 0),
        b"GoToR" => file_spec(pdf, action.get(b"F").ok()?).map(Target::File),
        // Launch, JavaScript, and the rest run things or change the view.
        _ => None,
    }
}

/// The link reference for a place in this document: its heading's anchor
/// when a heading starts there, else its page's.
fn page_reference(
    ix: &TextIndex,
    markers: &[Marker],
    contents: &[PageContent],
    page: usize,
    top: Option<f32>,
) -> Option<String> {
    let range = ix.page_range(page)?;
    // Where on the page the destination is.
    let at = top
        .and_then(|t| {
            let c = contents.get(page)?;
            let (_, y, ..) = c.rect_on_page([0.0, t, 0.0, t])?;
            let line = locate::line_below(c, y);
            ix.find(page, &line).map(|r| r.start)
        })
        .unwrap_or(range.start);
    let headings: Vec<&Marker> = markers
        .iter()
        .filter(|m| m.kind == MarkerKind::Heading)
        .collect();
    let slug_of = |m: &Marker| {
        let t: String = ix.chars[m.range.start.0..m.range.end.0.min(ix.chars.len())]
            .iter()
            .collect();
        textweaver_text::slug::slugify(t.trim())
    };
    // A heading within a few lines after the destination, on its page.
    let near = headings.iter().find(|m| {
        m.range.start >= at && m.range.start < range.end && m.range.start.0 - at.0 <= 200
    });
    if let Some(h) = near {
        let slug = slug_of(h);
        let unique = headings.iter().filter(|m| slug_of(m) == slug).count() == 1;
        if unique && !slug.is_empty() {
            return Some(format!("#{slug}"));
        }
    }
    let label = ix.page_label(page)?;
    Some(format!("#{PAGE_ANCHOR}{}", encode_label(&label)))
}

/// A page label safe in a link: spaces, `%`, and `#` percent-encoded.
fn encode_label(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    for c in label.chars() {
        match c {
            ' ' => out.push_str("%20"),
            '%' => out.push_str("%25"),
            '#' => out.push_str("%23"),
            c => out.push(c),
        }
    }
    out
}

/// Adds `Link` markers for the links and returns the comments, anchored in
/// the canonical text.
pub(super) fn apply(
    annots: &Annots,
    contents: &[PageContent],
    ix: &TextIndex,
    markers: &mut Vec<Marker>,
) -> Vec<DocumentComment> {
    let area = |page: usize, r: [f32; 4]| -> Option<Area> { contents.get(page)?.rect_on_page(r) };
    let mut new = Vec::new();
    for l in &annots.links {
        let Some(a) = area(l.page, l.rect) else {
            continue;
        };
        let Some(content) = contents.get(l.page) else {
            continue;
        };
        let covered = locate::text_in(content, &[a]);
        let Some(range) = ix.find(l.page, &covered) else {
            continue;
        };
        let range = ix.first_line(range);
        if range.is_empty() {
            continue;
        }
        let reference = match &l.target {
            Target::Uri(u) => Some(u.clone()),
            Target::File(f) => Some(f.clone()),
            Target::Page { index, top } => page_reference(ix, markers, contents, *index, *top),
        };
        if let Some(r) = reference {
            new.push(Marker::new(MarkerKind::Link, range).with_reference(r));
        }
    }
    markers.extend(new);
    let mut comments = Vec::new();
    for (k, n) in annots.notes.iter().enumerate() {
        let Some(content) = contents.get(n.page) else {
            continue;
        };
        let areas: Vec<Area> = n
            .areas
            .iter()
            .filter_map(|&r| content.rect_on_page(r))
            .collect();
        let Some(&first) = areas.first() else {
            continue;
        };
        let covered = if n.marks_text {
            locate::text_in(content, &areas)
        } else {
            String::new()
        };
        let range = if covered.trim().is_empty() {
            if n.markup_word.is_some() && n.text.is_empty() && n.replies.is_empty() {
                // A highlight over nothing readable: nothing to say.
                continue;
            }
            let line = locate::line_near(content, first);
            ix.find(n.page, &line)
                .map(|r| CharRange::empty(r.start.0))
                .or_else(|| ix.page_start(n.page).map(|p| CharRange::empty(p.0)))
        } else {
            ix.find(n.page, &covered)
                .or_else(|| ix.page_start(n.page).map(|p| CharRange::empty(p.0)))
        };
        let Some(range) = range else {
            continue;
        };
        let text = if n.text.is_empty() {
            n.markup_word.unwrap_or_default().to_owned()
        } else {
            n.text.clone()
        };
        comments.push(DocumentComment {
            id: format!("pdf-{}", k + 1),
            range,
            author: n.author.clone(),
            date: n.date.clone(),
            text,
            replies: n.replies.clone(),
            resolved: n.resolved,
        });
    }
    comments
}

/// True for a link anchor naming a page (`page=12`, `#page=iv`).
pub fn is_page_anchor(anchor: &str) -> bool {
    anchor
        .trim()
        .trim_start_matches('#')
        .to_ascii_lowercase()
        .starts_with(PAGE_ANCHOR)
}

/// Where the page an anchor names (`page=12`, by printed page label, as
/// links inside a PDF are written) starts in `doc`, if it has that page.
pub fn page_anchor(doc: &Document, anchor: &str) -> Option<CharPos> {
    let a = anchor.trim().trim_start_matches('#');
    if !is_page_anchor(a) {
        return None;
    }
    let want = a[PAGE_ANCHOR.len()..].trim();
    if want.is_empty() {
        return None;
    }
    doc.marker_index()
        .iter(MarkerKind::PageBreak, None)
        .find(|m| {
            m.label
                .as_deref()
                .is_some_and(|l| l.trim().eq_ignore_ascii_case(want))
        })
        .map(|m| m.range.start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_dates_read_as_iso() {
        assert_eq!(iso_date("D:20260901103000Z"), "2026-09-01T10:30:00Z");
        assert_eq!(
            iso_date("D:20260901103000+02'00'"),
            "2026-09-01T10:30:00+02:00"
        );
        assert_eq!(iso_date("D:2026"), "2026-01-01");
        assert_eq!(iso_date("yesterday"), "yesterday");
    }

    #[test]
    fn only_safe_addresses_are_kept() {
        assert_eq!(
            clean_uri(" https://example.org/a "),
            Some("https://example.org/a".into())
        );
        assert_eq!(
            clean_uri("www.example.org"),
            Some("https://www.example.org".into())
        );
        assert_eq!(
            clean_uri("mailto:ada@example.org"),
            Some("mailto:ada@example.org".into())
        );
        for bad in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "data:text/html,x",
            "",
            "https://a\u{7}b",
        ] {
            assert_eq!(clean_uri(bad), None, "{bad}");
        }
        assert_eq!(markup_word(b"Highlight"), Some("Highlighted"));
        assert_eq!(
            strip_markup("<body><p>Check &amp; fix</p></body>"),
            "Check & fix"
        );
    }

    #[test]
    fn page_anchors_find_their_page() {
        use ropey::Rope;
        use textweaver_text::DocumentMeta;
        let doc = Document::new(
            DocumentMeta::default(),
            Rope::from_str("One\n\nTwo"),
            vec![
                Marker::new(MarkerKind::PageBreak, CharRange::new(0, 3)).with_label("iv"),
                Marker::new(MarkerKind::PageBreak, CharRange::new(5, 8)).with_label("A 2"),
            ],
        );
        assert!(is_page_anchor("#page=3"));
        assert!(!is_page_anchor("#methods"));
        assert_eq!(page_anchor(&doc, "#page=IV"), Some(CharPos(0)));
        assert_eq!(page_anchor(&doc, "page=A 2"), Some(CharPos(5)));
        assert_eq!(page_anchor(&doc, "page=9"), None);
        assert_eq!(encode_label("A 2#%"), "A%202%23%25");
    }
}
