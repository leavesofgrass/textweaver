//! Edits written as tracked changes (task B1-t3): text deleted and
//! inserted in textweaver's edit mode goes into the Word file as `w:del`
//! and `w:ins`, with the author and the time, so a reviewer in Word sees
//! them in the Review tab.
//!
//! Each [`TrackedEdit`] names its place by the words around it, as they
//! read in the document; the words are found in one paragraph, and the
//! runs there are split where the change starts and ends, keeping each
//! run's formatting. shortcut: a change across paragraphs, or inside a
//! hyperlink, a field, or a run holding more than its text, is not
//! written (it is counted in [`TrackReport::unplaced`]); widen this when
//! edit mode edits Word files directly instead of their Markdown.

use std::collections::HashMap;
use std::io::Cursor;
use std::ops::Range;

use roxmltree::{Node, NodeId};
use zip::ZipArchive;

use super::{
    DOCUMENT, Parts, assemble, is_w, is_w_named, parse, q, qname, read_part, start_tag_end, zip_err,
};
use crate::{WriteError, xml};

/// One change made while editing, placed by the words around it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrackedEdit {
    /// The words just before the change in the same paragraph (may be
    /// empty at its start).
    pub before: String,
    /// The words deleted (empty for an insertion).
    pub deleted: String,
    /// The words inserted (empty for a deletion).
    pub inserted: String,
    /// The words just after the change in the same paragraph (may be
    /// empty at its end).
    pub after: String,
}

/// What [`track_edits`] wrote.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrackReport {
    /// Changes written as tracked changes.
    pub written: usize,
    /// Changes whose place was not found or could not take a tracked
    /// change; they are left out of the file.
    pub unplaced: usize,
}

/// The package `package` with `edits` written into `word/document.xml` as
/// tracked changes by `author` (empty: "textweaver") at `date` (ISO
/// 8601). Every other part is copied unchanged.
pub fn track_edits(
    package: &[u8],
    edits: &[TrackedEdit],
    author: &str,
    date: &str,
) -> Result<(Vec<u8>, TrackReport), WriteError> {
    let mut zip = ZipArchive::new(Cursor::new(package)).map_err(zip_err)?;
    let names: Vec<String> = zip.file_names().map(str::to_owned).collect();
    if !names.iter().any(|n| n == DOCUMENT) {
        return Err(WriteError::Zip(
            "this is not a Word document: it has no word/document.xml".into(),
        ));
    }
    let mut parts = Parts::default();
    let mut src = read_part(&mut zip, DOCUMENT)?;
    parts.original.insert(DOCUMENT.to_owned(), src.clone());
    let author = match author.trim() {
        "" => "textweaver",
        a => a,
    };
    let mut report = TrackReport::default();
    let mut next_id = next_revision_id(&src);
    for e in edits {
        match track_one(&src, e, author, date.trim(), &mut next_id)? {
            Some(new) => {
                src = new;
                report.written += 1;
            }
            None => report.unplaced += 1,
        }
    }
    if report.written > 0 {
        parts.set(DOCUMENT, src);
    }
    Ok((assemble(&mut zip, &names, &parts)?, report))
}

/// One more than the largest `w:id` in the part.
fn next_revision_id(src: &str) -> u64 {
    src.match_indices(":id=\"")
        .filter_map(|(i, _)| {
            let rest = &src[i + 5..];
            rest[..rest.find('"')?].parse::<u64>().ok()
        })
        .max()
        .map_or(1, |m| m + 1)
}

/// Where a character of the paragraph text comes from: the run (a child
/// of the paragraph), and the character's place in its `w:t`.
#[derive(Clone, Copy)]
struct At {
    run: NodeId,
    offset: usize,
}

/// The document's paragraph text, whitespace runs as one space, with each
/// character's source; `None` marks text that cannot take a change.
struct Index {
    text: Vec<char>,
    from: Vec<Option<At>>,
}

/// True for a run a change can split: its formatting and one `w:t`.
fn simple_run(r: Node<'_, '_>) -> bool {
    let mut texts = 0;
    for c in r.children().filter(|c| c.is_element()) {
        match c.tag_name().name() {
            "rPr" if is_w(c) => {}
            "t" if is_w(c) => texts += 1,
            _ => return false,
        }
    }
    texts == 1
}

impl Index {
    fn of(root: Node<'_, '_>) -> Index {
        let mut ix = Index {
            text: Vec::new(),
            from: Vec::new(),
        };
        for p in root.descendants().filter(|n| is_w_named(*n, "p")) {
            if !ix.text.is_empty() {
                ix.text.push('\n');
                ix.from.push(None);
            }
            let mut space = true;
            for t in p.descendants().filter(|n| is_w_named(*n, "t")) {
                if t.ancestors().skip(1).find(|a| is_w_named(*a, "p")) != Some(p)
                    || t.ancestors()
                        .any(|a| is_w(a) && matches!(a.tag_name().name(), "del" | "moveFrom"))
                {
                    continue;
                }
                let run = t.parent_element().filter(|r| is_w_named(*r, "r"));
                let usable = run.filter(|r| r.parent() == Some(p) && simple_run(*r));
                for (offset, c) in t.text().unwrap_or_default().chars().enumerate() {
                    let c = if c.is_whitespace() { ' ' } else { c };
                    if c == ' ' && space {
                        continue;
                    }
                    space = c == ' ';
                    ix.text.push(c);
                    ix.from.push(usable.map(|r| At {
                        run: r.id(),
                        offset,
                    }));
                }
            }
        }
        ix
    }

    /// Where `needle` first occurs.
    fn find(&self, needle: &[char]) -> Option<usize> {
        if needle.is_empty() || needle.len() > self.text.len() {
            return None;
        }
        (0..=self.text.len() - needle.len()).find(|&i| self.text[i..i + needle.len()] == *needle)
    }

    /// The source of the place just before character `pos` (or, at a
    /// paragraph's end, just after the character before it).
    fn place(&self, pos: usize) -> Option<At> {
        if let Some(Some(at)) = self.from.get(pos) {
            return Some(*at);
        }
        let mut at = (*self.from.get(pos.checked_sub(1)?)?)?;
        at.offset += 1;
        Some(at)
    }
}

/// Whitespace runs as one space, trimmed.
fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// What one run becomes: pieces of its text kept or deleted, and the
/// insertion, if it goes in this run.
#[derive(Default)]
struct Cut {
    deleted: Option<Range<usize>>,
    insert_at: Option<usize>,
}

/// `src` with edit `e` written as a tracked change, or `None` when its
/// place is not found or cannot take one.
fn track_one(
    src: &str,
    e: &TrackedEdit,
    author: &str,
    date: &str,
    next_id: &mut u64,
) -> Result<Option<String>, WriteError> {
    let xml = parse(DOCUMENT, src)?;
    let ix = Index::of(xml.root_element());
    let (b, d, i, a) = (
        collapse(&e.before),
        collapse(&e.deleted),
        collapse(&e.inserted),
        collapse(&e.after),
    );
    if d.is_empty() && i.is_empty() {
        return Ok(None);
    }
    let needle: Vec<char> = [b.as_str(), d.as_str(), a.as_str()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .collect();
    let Some(start) = ix.find(&needle) else {
        return Ok(None);
    };
    let (b_len, d_len, a_len) = (b.chars().count(), d.chars().count(), a.chars().count());
    let gap = usize::from(b_len > 0);
    let del_start = start + b_len + gap;
    // The deletion, where the insertion goes, and its text.
    let (mut del, ins_at, ins_text) = if d.is_empty() {
        if a_len > 0 {
            (
                del_start..del_start,
                start + needle.len() - a_len,
                format!("{i} "),
            )
        } else {
            (del_start..del_start, start + b_len, format!(" {i}"))
        }
    } else {
        let del = del_start..del_start + d_len;
        (del.clone(), del.end, i.clone())
    };
    if i.is_empty() {
        // A deleted word takes one space with it.
        if a_len > 0 {
            del.end += 1;
        } else if b_len > 0 {
            del.start -= 1;
        }
    }
    // Each run touched, with what happens in it.
    let mut cuts: HashMap<NodeId, Cut> = HashMap::new();
    for k in del.clone() {
        let Some(Some(at)) = ix.from.get(k) else {
            return Ok(None);
        };
        let cut = cuts.entry(at.run).or_default();
        cut.deleted = Some(match cut.deleted.take() {
            Some(r) => r.start..at.offset + 1,
            None => at.offset..at.offset + 1,
        });
    }
    if !ins_text.trim().is_empty() {
        let Some(at) = ix.place(ins_at) else {
            return Ok(None);
        };
        cuts.entry(at.run).or_default().insert_at = Some(at.offset);
    }
    let runs: Vec<Node<'_, '_>> = cuts.keys().filter_map(|id| xml.get_node(*id)).collect();
    let Some(para) = runs.first().and_then(|r| r.parent()) else {
        return Ok(None);
    };
    if runs.iter().any(|r| r.parent() != Some(para)) {
        return Ok(None);
    }
    let first = runs.iter().map(|r| r.range().start).min().unwrap_or(0);
    let last = runs.iter().map(|r| r.range().end).max().unwrap_or(0);
    let mut stamp = |kind: &str, w: &str| {
        let id = *next_id;
        *next_id += 1;
        let when = if date.is_empty() {
            String::new()
        } else {
            format!(" {}=\"{}\"", q(w, "date"), xml::attr(date))
        };
        format!(
            "<{} {}=\"{id}\" {}=\"{}\"{when}>",
            q(w, kind),
            q(w, "id"),
            q(w, "author"),
            xml::attr(author)
        )
    };
    let mut out = String::new();
    let mut at = first;
    for c in para.children() {
        let r = c.range();
        if r.start < first || r.end > last {
            continue;
        }
        out.push_str(&src[at..r.start]);
        at = r.end;
        match cuts.get(&c.id()) {
            Some(cut) => split_run(src, c, cut, &ins_text, &mut stamp, &mut out),
            None => out.push_str(&src[r]),
        }
    }
    let mut new = String::with_capacity(src.len() + out.len());
    new.push_str(&src[..first]);
    new.push_str(&out);
    new.push_str(&src[last..]);
    Ok(Some(new))
}

/// Writes run `r` split as `cut` says: its text kept, deleted (in a
/// `w:del`), and the insertion (in a `w:ins`), each piece a run with the
/// run's own formatting.
fn split_run(
    src: &str,
    r: Node<'_, '_>,
    cut: &Cut,
    ins_text: &str,
    stamp: &mut impl FnMut(&str, &str) -> String,
    out: &mut String,
) {
    let name = qname(src, r);
    let w = name.split_once(':').map_or("", |(p, _)| p);
    let open = &src[r.range().start..start_tag_end(src, r)];
    let rpr = r
        .children()
        .find(|c| is_w_named(*c, "rPr"))
        .map_or("", |c| &src[c.range()]);
    let text: Vec<char> = r
        .children()
        .find(|c| is_w_named(*c, "t"))
        .and_then(|t| t.text())
        .unwrap_or_default()
        .chars()
        .collect();
    let piece = |tag: &str, s: &[char]| {
        let s: String = s.iter().collect();
        format!(
            "{open}{rpr}<{t} xml:space=\"preserve\">{}</{t}></{name}>",
            xml::text(&s),
            t = q(w, tag)
        )
    };
    let len = text.len();
    let del = cut
        .deleted
        .clone()
        .map(|d| d.start.min(len)..d.end.min(len));
    let mut bounds = vec![0, len];
    bounds.extend(del.iter().flat_map(|d| [d.start, d.end]));
    bounds.extend(cut.insert_at.map(|i| i.min(len)));
    bounds.sort_unstable();
    bounds.dedup();
    let insert = |out: &mut String, stamp: &mut dyn FnMut(&str, &str) -> String| {
        let s: Vec<char> = ins_text.chars().collect();
        out.push_str(&stamp("ins", w));
        out.push_str(&piece("t", &s));
        out.push_str(&format!("</{}>", q(w, "ins")));
    };
    for pair in bounds.windows(2) {
        let (x, y) = (pair[0], pair[1]);
        if cut.insert_at.map(|i| i.min(len)) == Some(x) {
            insert(out, &mut *stamp);
        }
        if x == y {
            continue;
        }
        let deleted = del.as_ref().is_some_and(|d| d.start <= x && y <= d.end);
        if deleted {
            out.push_str(&stamp("del", w));
            out.push_str(&piece("delText", &text[x..y]));
            out.push_str(&format!("</{}>", q(w, "del")));
        } else {
            out.push_str(&piece("t", &text[x..y]));
        }
    }
    if len > 0 && cut.insert_at.map(|i| i.min(len)) == Some(len) {
        insert(out, &mut *stamp);
    }
}
