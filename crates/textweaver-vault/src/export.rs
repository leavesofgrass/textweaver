//! Writing documents' notes and highlights into a vault (star's
//! `export_vault`, `star/obsidian.py:263-324`).
//!
//! Every note becomes one vault note, named after its anchor, with front
//! matter and its relations as Dataview fields under `## Links` (star's
//! format, which round-trips relation types and shows the edges in
//! Obsidian's graph). Each document also gets a document note listing its
//! highlights as quotes with block ids and linking to its notes.
//!
//! Differences from star:
//! - Front matter values are quoted when needed (Part 3 §7 item 42).
//! - The id key is `textweaver_id`; star's `star_id` is still read on
//!   import.
//! - Re-exporting updates the notes written before (found by id anywhere in
//!   the vault, even after the user renamed or moved them) instead of
//!   adding numbered copies, and never overwrites a note textweaver did not
//!   write: a name clash with the user's own note gets a numbered name.
//! - A relation whose target is not being exported still links when the
//!   target was exported earlier (found by id in the vault).
//! - Files are written atomically.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use textweaver_core::CharPos;
use textweaver_store::{atomic_write, percent, time};

use crate::frontmatter::{self, FmValue, FrontMatter};
use crate::links::first_line;
use crate::model::{DocAnnotations, Highlight, Note, color_name, derive_id};
use crate::names::{NameAllocator, sanitize};
use crate::walk::{note_files, stem};
use crate::{DOCUMENT_TAG, NODE_TAG, VaultError};

/// A document whose notes and highlights to export.
#[derive(Clone, Copy, Debug)]
pub struct ExportDocument<'a> {
    /// The document's file.
    pub path: &'a Path,
    /// Its title.
    pub title: &'a str,
    /// Its canonical text, for highlight quotes and percentages; `None`
    /// when the document could not be loaded.
    pub text: Option<&'a str>,
    /// Its notes and highlights.
    pub annotations: &'a DocAnnotations,
}

/// Export choices.
#[derive(Clone, Debug)]
pub struct ExportOptions {
    /// Folder inside the vault for new notes; `None` for the vault's top
    /// folder. Notes exported before stay where they are.
    pub folder: Option<PathBuf>,
    /// Also write one document note per document, with its highlights and
    /// links to its notes.
    pub document_notes: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions {
            folder: None,
            document_notes: true,
        }
    }
}

/// What an export did.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ExportReport {
    /// The vault folder.
    pub vault: PathBuf,
    /// Every file written, in order.
    pub written: Vec<PathBuf>,
    /// Note files written.
    pub notes: usize,
    /// Document notes written.
    pub documents: usize,
    /// Highlights written into document notes.
    pub highlights: usize,
    /// Files that existed and were updated in place.
    pub updated: usize,
    /// Relations left out because their target note is neither exported
    /// now nor found in the vault.
    pub unresolved: usize,
}

impl ExportReport {
    /// A one-sentence summary that reads well aloud.
    pub fn summary(&self) -> String {
        let vault = self.vault.file_name().map_or_else(
            || self.vault.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let mut s = format!(
            "Exported {} and {} to {vault}",
            plural(self.notes, "note"),
            plural(self.highlights, "highlight"),
        );
        if self.updated > 0 {
            s.push_str(&format!(
                ", updating {}",
                plural(self.updated, "existing file")
            ));
        }
        if self.unresolved > 0 {
            s.push_str(&format!(
                "; {} left out because the linked note is not in the vault",
                plural(self.unresolved, "link")
            ));
        }
        s.push('.');
        s
    }
}

/// `1 note`, `2 notes`.
pub(crate) fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

/// The id of a document's document note.
pub fn document_id(path: &Path) -> String {
    derive_id(&format!("doc:{}", path.to_string_lossy()))
}

/// The id of a highlight: its own, else one derived from its range and
/// color.
pub fn highlight_id(h: &Highlight) -> String {
    if h.id.trim().is_empty() {
        derive_id(&format!("hl:{}:{}:{}", h.range.start, h.range.end, h.color))
    } else {
        h.id.trim().to_owned()
    }
}

/// Notes textweaver wrote into the vault before, found by id.
#[derive(Debug, Default)]
struct Existing {
    /// Note id (`textweaver_id`, or star's `star_id`) to file.
    notes: HashMap<String, PathBuf>,
    /// Document id (`textweaver_doc`) to file.
    docs: HashMap<String, PathBuf>,
}

fn scan_existing(vault: &Path) -> Result<Existing, VaultError> {
    let mut ex = Existing::default();
    if !vault.is_dir() {
        return Ok(ex);
    }
    for path in note_files(vault)? {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let (fm, _) = frontmatter::split(&text);
        if let Some(id) = fm.text("textweaver_doc") {
            ex.docs.entry(id).or_insert(path);
        } else if let Some(id) = fm.text("textweaver_id").or_else(|| fm.text("star_id")) {
            ex.notes.entry(id).or_insert(path);
        }
    }
    Ok(ex)
}

/// The names already used by files in `folder` (textweaver's own notes
/// included; they are reused by id, not re-allocated).
fn reserve_folder(names: &mut NameAllocator, folder: &Path) {
    if let Ok(entries) = std::fs::read_dir(folder) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("md")) {
                names.reserve(&stem(&p));
            }
        }
    }
}

/// Where each exported note goes.
struct Plan {
    /// Note id to (file, link name).
    notes: HashMap<String, (PathBuf, String)>,
    /// Document path to (file, link name) of its document note.
    docs: HashMap<PathBuf, (PathBuf, String)>,
}

/// Writes the notes and highlights of `docs` into the vault at `vault`
/// (created if missing).
pub fn export_documents(
    vault: &Path,
    docs: &[ExportDocument<'_>],
    options: &ExportOptions,
) -> Result<ExportReport, VaultError> {
    std::fs::create_dir_all(vault).map_err(|source| VaultError::Io {
        path: vault.to_owned(),
        source,
    })?;
    let folder = match &options.folder {
        Some(f) => vault.join(f),
        None => vault.to_owned(),
    };
    std::fs::create_dir_all(&folder).map_err(|source| VaultError::Io {
        path: folder.clone(),
        source,
    })?;
    let existing = scan_existing(vault)?;
    let mut names = NameAllocator::new();
    reserve_folder(&mut names, &folder);

    let mut plan = Plan {
        notes: HashMap::new(),
        docs: HashMap::new(),
    };
    // Documents first, so a document note keeps the document's own title.
    for doc in docs {
        if !options.document_notes {
            break;
        }
        let id = document_id(doc.path);
        let entry = match existing.docs.get(&id) {
            Some(p) => (p.clone(), stem(p)),
            None => {
                let name = names.allocate(&sanitize(doc.title));
                (folder.join(format!("{name}.md")), name)
            }
        };
        plan.docs.insert(doc.path.to_owned(), entry);
    }
    for doc in docs {
        for note in doc.annotations.sorted_notes() {
            if plan.notes.contains_key(&note.id) {
                continue;
            }
            let entry = match existing.notes.get(&note.id) {
                Some(p) => (p.clone(), stem(p)),
                None => {
                    let name = names.allocate(&sanitize(&note_base_name(note)));
                    (folder.join(format!("{name}.md")), name)
                }
            };
            plan.notes.insert(note.id.clone(), entry);
        }
    }

    let mut report = ExportReport {
        vault: vault.to_owned(),
        ..ExportReport::default()
    };
    for doc in docs {
        let doc_link = plan.docs.get(doc.path).map(|(_, name)| name.as_str());
        let len = doc.text.map(|t| t.chars().count());
        for note in doc.annotations.sorted_notes() {
            let Some((file, _)) = plan.notes.get(&note.id) else {
                continue;
            };
            let (text, unresolved) = render_note(doc, note, doc_link, len, &plan, &existing);
            report.unresolved += unresolved;
            write(file, &text, &mut report)?;
            report.notes += 1;
        }
        if let Some((file, _)) = plan.docs.get(doc.path) {
            let text = render_document(doc, len, &plan);
            write(file, &text, &mut report)?;
            report.documents += 1;
            report.highlights += doc.annotations.highlights.len();
        }
    }
    Ok(report)
}

/// The base of a note's file name: its anchor, else its first line, else
/// its id (star's order).
fn note_base_name(note: &Note) -> String {
    let anchor = note.anchor.trim();
    if !anchor.is_empty() {
        return anchor.to_owned();
    }
    let first = first_line(&note.note);
    if first.is_empty() {
        note.id.clone()
    } else {
        first
    }
}

fn write(file: &Path, text: &str, report: &mut ExportReport) -> Result<(), VaultError> {
    if file.exists() {
        report.updated += 1;
    }
    atomic_write(file, text.as_bytes())?;
    report.written.push(file.to_owned());
    Ok(())
}

/// The link name of the note with `id`: exported now, or found in the vault.
fn link_name(id: &str, plan: &Plan, existing: &Existing) -> Option<String> {
    plan.notes
        .get(id)
        .map(|(_, name)| name.clone())
        .or_else(|| existing.notes.get(id).map(|p| stem(p)))
}

/// A note's file. Returns the text and the number of relations left out.
fn render_note(
    doc: &ExportDocument<'_>,
    note: &Note,
    doc_link: Option<&str>,
    len: Option<usize>,
    plan: &Plan,
    existing: &Existing,
) -> (String, usize) {
    let mut fm = FrontMatter::new();
    fm.set_text("textweaver_id", note.id.clone());
    fm.set_text("title", note_title(note));
    fm.set_text("source", doc.path.to_string_lossy().into_owned());
    if let Some(name) = doc_link {
        fm.set_text("document", format!("[[{name}]]"));
    }
    fm.set("position", FmValue::Int(to_i64(note.range.start.0)));
    if let Some(len) = len {
        fm.set(
            "pct",
            FmValue::Int(i64::from(percent(note.range.start, len))),
        );
    }
    let tags: Vec<String> = note
        .tags
        .iter()
        .filter(|t| *t != NODE_TAG && !t.trim().is_empty())
        .cloned()
        .collect();
    fm.set_list("tags", tags);
    if !note.cite.trim().is_empty() {
        fm.set_text("cite", note.cite.clone());
    }
    if note.ts > 0 {
        fm.set_text("updated", time::rfc3339(note.ts));
    }

    let mut out = fm.render();
    out.push('\n');
    let body = note.note.trim();
    if !body.is_empty() {
        out.push_str(body);
        out.push_str("\n\n");
    }
    let mut unresolved = 0;
    let mut lines = Vec::new();
    for rel in &note.relations {
        match link_name(&rel.target_id, plan, existing) {
            Some(name) => {
                let mut line = format!("- {}:: [[{name}]]", rel.rel_type);
                let comment = rel.note.split_whitespace().collect::<Vec<_>>().join(" ");
                if !comment.is_empty() {
                    line.push_str(" - ");
                    line.push_str(&comment);
                }
                lines.push(line);
            }
            None => unresolved += 1,
        }
    }
    if !lines.is_empty() {
        out.push_str("## Links\n\n");
        for l in lines {
            out.push_str(&l);
            out.push('\n');
        }
    }
    (out, unresolved)
}

/// A note's title: its anchor, else its first line.
fn note_title(note: &Note) -> String {
    let anchor = note.anchor.trim();
    if anchor.is_empty() {
        first_line(&note.note)
    } else {
        anchor.to_owned()
    }
}

fn to_i64(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// Longest highlight quote, in chars.
const MAX_QUOTE_CHARS: usize = 600;

/// The highlighted text, whitespace collapsed, shortened with an ellipsis.
fn quote(text: &str, h: &Highlight) -> Option<String> {
    let start = h.range.start.0;
    let end = h.range.end.0.max(start);
    let slice: String = text.chars().skip(start).take(end - start).collect();
    let collapsed = slice.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }
    if collapsed.chars().count() > MAX_QUOTE_CHARS {
        let short: String = collapsed.chars().take(MAX_QUOTE_CHARS).collect();
        Some(format!("{}\u{2026}", short.trim_end()))
    } else {
        Some(collapsed)
    }
}

/// A document's document note.
fn render_document(doc: &ExportDocument<'_>, len: Option<usize>, plan: &Plan) -> String {
    let mut fm = FrontMatter::new();
    fm.set_text("textweaver_doc", document_id(doc.path));
    fm.set_text("title", doc.title.to_owned());
    fm.set_text("source", doc.path.to_string_lossy().into_owned());
    fm.set_list("tags", vec![DOCUMENT_TAG.to_owned()]);
    let mut out = fm.render();
    out.push('\n');
    out.push_str(&format!("# {}\n\n", doc.title.trim()));

    let highlights = doc.annotations.sorted_highlights();
    if !highlights.is_empty() {
        out.push_str("## Highlights\n\n");
        for h in highlights {
            let id = highlight_id(h);
            out.push_str(&format!(
                "%% textweaver-highlight id={id} start={} end={} color={} ts={} %%\n",
                h.range.start,
                h.range.end,
                h.color.trim().replace(' ', ""),
                h.ts
            ));
            let text = doc
                .text
                .and_then(|t| quote(t, h))
                .unwrap_or_else(|| format!("Characters {} to {}", h.range.start, h.range.end));
            out.push_str(&format!("> {text} ^hl-{id}\n\n"));
            let mut about = color_name(&h.color);
            if let Some(len) = len {
                about.push_str(&format!(", {}%", percent(h.range.start, len)));
            }
            out.push_str(&format!("Highlighted {about}.\n\n"));
        }
    }
    let notes = doc.annotations.sorted_notes();
    if !notes.is_empty() {
        out.push_str("## Notes\n\n");
        for note in notes {
            let Some((_, name)) = plan.notes.get(&note.id) else {
                continue;
            };
            let mut line = format!("- [[{name}]]");
            if let Some(len) = len {
                line.push_str(&format!(" ({}%)", percent(note.range.start, len)));
            }
            let summary = first_line(&note.note);
            if !summary.is_empty() && summary != *name {
                line.push_str(": ");
                line.push_str(&summary);
            }
            out.push_str(&line);
            out.push('\n');
        }
    }
    out
}

/// Parses the highlight comments of a document note body.
pub(crate) fn parse_highlights(body: &str) -> Vec<Highlight> {
    let mut out = Vec::new();
    for line in body.lines() {
        let t = line.trim();
        let Some(inner) = t
            .strip_prefix("%%")
            .and_then(|r| r.strip_suffix("%%"))
            .map(str::trim)
            .and_then(|r| r.strip_prefix("textweaver-highlight"))
        else {
            continue;
        };
        let mut h = Highlight::default();
        let (mut start, mut end) = (None, None);
        for pair in inner.split_whitespace() {
            let Some((k, v)) = pair.split_once('=') else {
                continue;
            };
            match k {
                "id" => h.id = v.to_owned(),
                "start" => start = v.parse::<usize>().ok(),
                "end" => end = v.parse::<usize>().ok(),
                "color" => h.color = v.to_owned(),
                "ts" => h.ts = v.parse().unwrap_or(0),
                _ => {}
            }
        }
        if let (Some(s), Some(e)) = (start, end)
            && !h.id.is_empty()
        {
            h.range = textweaver_core::CharRange::new(CharPos(s), CharPos(e));
            out.push(h);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Relation, RelationType};

    fn rel(t: RelationType, doc: &str, id: &str, note: &str) -> Relation {
        Relation {
            rel_type: t.as_str().into(),
            target_doc: doc.into(),
            target_id: id.into(),
            note: note.into(),
        }
    }
    use textweaver_core::CharRange;

    fn note(id: &str, pos: usize, anchor: &str, text: &str) -> Note {
        Note {
            id: id.into(),
            range: CharRange::new(CharPos(pos), CharPos(pos)),
            anchor: anchor.into(),
            note: text.into(),
            tags: vec!["exam".into()],
            ts: 1_790_000_000,
            ..Note::default()
        }
    }

    const TEXT: &str =
        "The cell is the unit of life. Mitochondria make energy. Ribosomes build proteins.";

    fn sample() -> DocAnnotations {
        let mut a = note("n1", 4, "The cell", "Cells are the basic unit.");
        a.relations.push(rel(
            RelationType::Supports,
            "/docs/bio.md",
            "n2",
            "same chapter",
        ));
        a.relations
            .push(rel(RelationType::Cites, "/docs/other.md", "missing", ""));
        let b = note("n2", 30, "Mitochondria: the powerhouse #1", "Energy.");
        DocAnnotations {
            notes: vec![b, a],
            highlights: vec![Highlight {
                id: "h1".into(),
                range: CharRange::new(CharPos(30), CharPos(55)),
                color: "#ffff00".into(),
                ts: 5,
                ..Highlight::default()
            }],
        }
    }

    #[test]
    fn writes_notes_links_and_a_document_note() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("Vault");
        let ann = sample();
        let doc = ExportDocument {
            path: Path::new("/docs/bio.md"),
            title: "Biology: Chapter 1",
            text: Some(TEXT),
            annotations: &ann,
        };
        let report = export_documents(&vault, &[doc], &ExportOptions::default()).unwrap();
        assert_eq!(report.notes, 2);
        assert_eq!(report.documents, 1);
        assert_eq!(report.highlights, 1);
        assert_eq!(report.unresolved, 1);
        assert_eq!(report.updated, 0);
        assert_eq!(
            report.summary(),
            "Exported 2 notes and 1 highlight to Vault; 1 link left out because the linked note is not in the vault."
        );

        let a = std::fs::read_to_string(vault.join("The cell.md")).unwrap();
        assert!(
            a.starts_with("---\ntextweaver_id: n1\ntitle: The cell\n"),
            "{a}"
        );
        assert!(a.contains("document: \"[[Biology Chapter 1]]\"\n"), "{a}");
        assert!(a.contains("position: 4\npct: 4\ntags: [exam]\n"), "{a}");
        assert!(a.contains("\n\nCells are the basic unit.\n\n## Links\n\n- SUPPORTS:: [[Mitochondria the powerhouse 1]] - same chapter\n"), "{a}");
        let b = std::fs::read_to_string(vault.join("Mitochondria the powerhouse 1.md")).unwrap();
        assert!(
            b.contains("title: \"Mitochondria: the powerhouse #1\"\n"),
            "{b}"
        );
        assert!(!b.contains("## Links"));

        let d = std::fs::read_to_string(vault.join("Biology Chapter 1.md")).unwrap();
        assert!(d.contains("tags: [textweaver-document]\n"), "{d}");
        assert!(d.contains("%% textweaver-highlight id=h1 start=30 end=55 color=#ffff00 ts=5 %%\n> Mitochondria make energy. ^hl-h1\n\nHighlighted yellow, 37%.\n"), "{d}");
        assert!(
            d.contains("- [[The cell]] (4%): Cells are the basic unit.\n"),
            "{d}"
        );
        assert!(
            d.contains("- [[Mitochondria the powerhouse 1]] (37%): Energy.\n"),
            "{d}"
        );
        assert_eq!(parse_highlights(&d), ann.highlights);
    }

    #[test]
    fn re_export_updates_in_place_and_spares_foreign_notes() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path();
        // The user's own note has the name a textweaver note would take.
        std::fs::write(vault.join("The cell.md"), "my own note").unwrap();
        let ann = sample();
        let doc = ExportDocument {
            path: Path::new("/docs/bio.md"),
            title: "Bio",
            text: None,
            annotations: &ann,
        };
        let first = export_documents(vault, &[doc], &ExportOptions::default()).unwrap();
        assert_eq!(
            std::fs::read_to_string(vault.join("The cell.md")).unwrap(),
            "my own note"
        );
        assert!(vault.join("The cell 2.md").exists());
        assert_eq!(first.updated, 0);
        // The user renames the exported note and moves it into a folder.
        std::fs::create_dir(vault.join("moved")).unwrap();
        std::fs::rename(vault.join("The cell 2.md"), vault.join("moved/Renamed.md")).unwrap();
        let second = export_documents(vault, &[doc], &ExportOptions::default()).unwrap();
        assert_eq!(second.updated, 3);
        assert!(!vault.join("The cell 2.md").exists());
        let moved = std::fs::read_to_string(vault.join("moved/Renamed.md")).unwrap();
        assert!(moved.contains("textweaver_id: n1"));
        // The document note links to the note under its new name.
        let d = std::fs::read_to_string(vault.join("Bio.md")).unwrap();
        assert!(
            d.contains("- [[Renamed]]: Cells are the basic unit.\n"),
            "{d}"
        );
        // Without text there are no percentages, and quotes name the range.
        assert!(d.contains("> Characters 30 to 55 ^hl-h1"), "{d}");
    }

    #[test]
    fn relations_to_notes_exported_earlier_still_link() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path();
        let first = DocAnnotations {
            notes: vec![note("t1", 0, "Target", "t")],
            highlights: vec![],
        };
        let mut src = note("s1", 0, "Source", "s");
        src.relations
            .push(rel(RelationType::Defines, "/a.md", "t1", ""));
        let second = DocAnnotations {
            notes: vec![src],
            highlights: vec![],
        };
        let opts = ExportOptions {
            folder: Some(PathBuf::from("textweaver")),
            document_notes: false,
        };
        export_documents(
            vault,
            &[ExportDocument {
                path: Path::new("/a.md"),
                title: "A",
                text: None,
                annotations: &first,
            }],
            &opts,
        )
        .unwrap();
        let r = export_documents(
            vault,
            &[ExportDocument {
                path: Path::new("/b.md"),
                title: "B",
                text: None,
                annotations: &second,
            }],
            &opts,
        )
        .unwrap();
        assert_eq!(r.unresolved, 0);
        assert_eq!(r.documents, 0);
        let s = std::fs::read_to_string(vault.join("textweaver/Source.md")).unwrap();
        assert!(s.contains("- DEFINES:: [[Target]]\n"), "{s}");
    }

    #[test]
    fn long_quotes_are_shortened() {
        let text = "word ".repeat(400);
        let h = Highlight {
            id: "x".into(),
            range: CharRange::new(CharPos(0), CharPos(2000)),
            color: String::new(),
            ts: 0,
            ..Highlight::default()
        };
        let q = quote(&text, &h).unwrap();
        assert!(q.ends_with('\u{2026}'));
        assert!(q.chars().count() <= MAX_QUOTE_CHARS + 1);
    }

    #[test]
    fn highlight_ids_are_derived_when_missing() {
        let h = Highlight {
            id: String::new(),
            range: CharRange::new(CharPos(1), CharPos(3)),
            color: "#ffff00".into(),
            ts: 0,
            ..Highlight::default()
        };
        assert_eq!(highlight_id(&h), highlight_id(&h.clone()));
        assert_eq!(highlight_id(&h).len(), 12);
    }
}
