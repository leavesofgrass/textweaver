//! Reading a vault's notes and applying them to the annotation store
//! (Star's `import_vault`, `star/obsidian.py:142-252`).
//!
//! Every `*.md` note under the vault (except in `.obsidian` and `.trash`)
//! is read. What happens to it depends on the mode and on what the note is:
//!
//! - **Library mode** registers every note as a document in the library,
//!   and nothing else.
//! - **Graph mode** (the default):
//!   - A note textweaver exported for a note on a document (front matter
//!     `textweaver_id` and `source`) updates that note on that document:
//!     its text, tags, citation, and links. This is the round trip: export,
//!     edit in Obsidian, import.
//!   - A document note textweaver exported (`textweaver_doc` and `source`)
//!     brings back its highlights.
//!   - Any other note becomes a document in the library plus one node note
//!     (tag `obsidian-note`) summarizing it, whose links become relations
//!     to other notes (Star's behavior).
//!
//! Links are resolved by file name, title, or alias, ignoring case; the
//! first note registered under a name wins. A typed Dataview field
//! (`supports:: [[X]]`) gives that relation; a plain `[[X]]` gives the
//! chosen default (Star: `link_relation`, then `vault.default_link_relation`,
//! then `SEE_ALSO`). Embeds are ignored; duplicate (type, target) pairs
//! give one relation. Relations are rebuilt from the files on every import,
//! so importing twice changes nothing.
//!
//! Import never deletes: a note or highlight removed from the vault stays
//! on its document.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;
use textweaver_core::{CharPos, CharRange};

use crate::export::{parse_highlights, plural};
use crate::frontmatter::{self, FrontMatter};
use crate::links::{Link, extract_links, first_line, inline_tags, strip_link_syntax};
use crate::model::{AnnotationStore, Highlight, Note, Relation, RelationType, derive_id};
use crate::walk::{note_files, stem};
use crate::{NODE_TAG, VaultError};

/// What an import creates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportMode {
    /// Documents, notes, and relations (Star's `graph`).
    #[default]
    Graph,
    /// Only documents in the library (Star's `library`).
    Library,
}

impl ImportMode {
    /// Parses `graph` or `library`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "graph" => Some(ImportMode::Graph),
            "library" => Some(ImportMode::Library),
            _ => None,
        }
    }

    /// `graph` or `library`.
    pub fn as_str(self) -> &'static str {
        match self {
            ImportMode::Graph => "graph",
            ImportMode::Library => "library",
        }
    }
}

/// Import choices.
#[derive(Clone, Debug, Default)]
pub struct ImportOptions {
    /// What to create.
    pub mode: ImportMode,
    /// The relation for plain `[[links]]`; `None` for `SEE_ALSO` (the
    /// caller passes the `vault.default_link_relation` setting here).
    pub link_relation: Option<RelationType>,
}

/// What a vault note is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum NoteKind {
    /// A note of the user's own: a document with one node note.
    Plain,
    /// A note textweaver exported for a note on a document.
    Annotation {
        /// The document the note belongs to.
        source: PathBuf,
        /// Where on it (used when the note no longer exists there).
        position: CharPos,
    },
    /// A document note textweaver exported, with its highlights.
    Document {
        /// The document.
        source: PathBuf,
        /// Its highlights.
        highlights: Vec<Highlight>,
    },
}

/// One note read from the vault.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VaultNote {
    /// The note's file.
    pub path: PathBuf,
    /// Its title: front matter `title`, else the file name.
    pub title: String,
    /// Front matter `aliases` (or `alias`).
    pub aliases: Vec<String>,
    /// Front matter tags and inline `#tags`, without `#`, deduplicated.
    pub tags: Vec<String>,
    /// Its id: `textweaver_id`, `star_id`, or one derived from its path.
    pub id: String,
    /// Its first line of content, link syntax removed (Star's summary).
    pub summary: String,
    /// Its text without front matter, without the `## Links` section and
    /// comments, with link syntax reduced to plain words.
    pub text: String,
    /// Front matter `cite`.
    pub cite: String,
    /// Front matter `updated`, as Unix seconds; 0 when absent.
    pub ts: i64,
    /// What it is.
    #[serde(flatten)]
    pub kind: NoteKind,
    /// The links as written.
    #[serde(skip)]
    pub links: Vec<Link>,
    /// The links resolved to notes (graph mode only).
    pub relations: Vec<Relation>,
}

/// A vault as read, before anything is stored.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VaultRead {
    /// The vault folder.
    pub vault: PathBuf,
    /// The mode it was read for.
    pub mode: ImportMode,
    /// The notes, sorted by path.
    pub notes: Vec<VaultNote>,
    /// Relations found (graph mode).
    pub relations: usize,
    /// Links whose target could not be found (graph mode).
    pub unresolved: usize,
    /// Notes that could not be read, with the reason.
    pub unreadable: Vec<(PathBuf, String)>,
}

/// Reads every note of the vault at `vault`.
pub fn read_vault(vault: &Path, options: &ImportOptions) -> Result<VaultRead, VaultError> {
    let files = note_files(vault)?;
    let mut notes = Vec::with_capacity(files.len());
    let mut unreadable = Vec::new();
    for path in files {
        match std::fs::read(&path) {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                notes.push(parse_note(&path, &text));
            }
            Err(e) => unreadable.push((path, e.to_string())),
        }
    }
    let mut read = VaultRead {
        vault: vault.to_owned(),
        mode: options.mode,
        notes,
        relations: 0,
        unresolved: 0,
        unreadable,
    };
    if options.mode == ImportMode::Graph {
        resolve(
            &mut read,
            options.link_relation.unwrap_or(RelationType::SeeAlso),
        );
    }
    Ok(read)
}

/// Parses one note.
pub fn parse_note(path: &Path, text: &str) -> VaultNote {
    let (fm, body) = frontmatter::split(text);
    let title = fm.text("title").unwrap_or_else(|| stem(path));
    let mut aliases = fm.list("aliases");
    aliases.extend(fm.list("alias"));
    let tags = collect_tags(&fm, body);
    let source = fm.text("source").map(PathBuf::from);
    let kind = match (fm.text("textweaver_doc"), fm.text("textweaver_id"), source) {
        (Some(_), _, Some(source)) => NoteKind::Document {
            source,
            highlights: parse_highlights(body),
        },
        (None, Some(_), Some(source)) => NoteKind::Annotation {
            source,
            position: CharPos(
                fm.int("position")
                    .and_then(|p| usize::try_from(p).ok())
                    .unwrap_or(0),
            ),
        },
        _ => NoteKind::Plain,
    };
    let id = fm
        .text("textweaver_doc")
        .or_else(|| fm.text("textweaver_id"))
        .or_else(|| fm.text("star_id"))
        .unwrap_or_else(|| derive_id(&path.to_string_lossy()));
    VaultNote {
        path: path.to_owned(),
        title,
        aliases,
        tags,
        id,
        summary: first_line(body),
        text: note_text(body),
        cite: fm.text("cite").unwrap_or_default(),
        ts: fm
            .text("updated")
            .and_then(|t| textweaver_store::time::parse_timestamp(&t))
            .unwrap_or(0),
        kind,
        links: extract_links(body),
        relations: Vec::new(),
    }
}

/// Front matter `tags` (or `tag`) then inline tags, `#` stripped, without
/// duplicates (Star's `_collect_tags`).
fn collect_tags(fm: &FrontMatter, body: &str) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    let mut raw = fm.list("tags");
    raw.extend(fm.list("tag"));
    raw.extend(inline_tags(body));
    for t in raw {
        let t = t.trim().trim_start_matches('#').trim().to_owned();
        if !t.is_empty() && !tags.contains(&t) {
            tags.push(t);
        }
    }
    tags
}

/// The body without its `## Links` section and `%% comments %%`, link
/// syntax reduced to words, trimmed.
fn note_text(body: &str) -> String {
    let mut kept = Vec::new();
    let mut skip_level: Option<usize> = None;
    let mut in_comment = false;
    for line in body.lines() {
        let level = heading_level(line);
        if let Some(skip) = skip_level {
            match level {
                Some(l) if l <= skip => skip_level = None,
                _ => continue,
            }
        }
        if let Some(l) = level
            && line.trim_start()[l..].trim().eq_ignore_ascii_case("links")
        {
            skip_level = Some(l);
            continue;
        }
        let without = strip_comments(line, &mut in_comment);
        if without.trim().is_empty() && line.contains("%%") {
            continue;
        }
        kept.push(without);
    }
    let joined = kept.join("\n");
    strip_link_syntax(joined.trim()).trim().to_owned()
}

/// The level of an ATX heading line (`## x` is 2).
fn heading_level(line: &str) -> Option<usize> {
    let t = line.trim_start();
    let n = t.bytes().take_while(|&b| b == b'#').count();
    ((1..=6).contains(&n) && t[n..].starts_with([' ', '\t'])).then_some(n)
}

/// `line` without `%% ... %%` comments, which may span lines.
fn strip_comments(line: &str, in_comment: &mut bool) -> String {
    let mut out = String::new();
    let mut rest = line;
    loop {
        if *in_comment {
            match rest.find("%%") {
                Some(at) => {
                    rest = &rest[at + 2..];
                    *in_comment = false;
                }
                None => return out,
            }
        } else {
            match rest.find("%%") {
                Some(at) => {
                    out.push_str(&rest[..at]);
                    rest = &rest[at + 2..];
                    *in_comment = true;
                }
                None => {
                    out.push_str(rest);
                    return out;
                }
            }
        }
    }
}

/// Resolves every note's links into relations.
fn resolve(read: &mut VaultRead, default_rel: RelationType) {
    // Lower-cased name, title, or alias to note index; first one wins.
    let mut index: HashMap<String, usize> = HashMap::new();
    for (i, n) in read.notes.iter().enumerate() {
        index.entry(stem(&n.path).to_lowercase()).or_insert(i);
        index.entry(n.title.to_lowercase()).or_insert(i);
        for a in &n.aliases {
            index.entry(a.to_lowercase()).or_insert(i);
        }
    }
    // Where each note's relations point: (document, id).
    let targets: Vec<Option<(PathBuf, String)>> = read
        .notes
        .iter()
        .map(|n| match &n.kind {
            NoteKind::Plain => Some((n.path.clone(), n.id.clone())),
            NoteKind::Annotation { source, .. } => Some((source.clone(), n.id.clone())),
            NoteKind::Document { .. } => None,
        })
        .collect();
    let (mut relations, mut unresolved) = (0, 0);
    for note in &mut read.notes {
        if matches!(note.kind, NoteKind::Document { .. }) {
            continue;
        }
        let mut seen: HashSet<(RelationType, String)> = HashSet::new();
        let mut out = Vec::new();
        for link in &note.links {
            let target = index
                .get(&link.target.to_lowercase())
                .and_then(|&i| targets[i].as_ref());
            let Some((doc, id)) = target else {
                unresolved += 1;
                continue;
            };
            let rel_type = link.rel_type.unwrap_or(default_rel);
            if !seen.insert((rel_type, id.clone())) {
                continue;
            }
            out.push(Relation {
                rel_type: rel_type.as_str().to_owned(),
                target_doc: doc.to_string_lossy().into_owned(),
                target_id: id.clone(),
                note: link.note.clone(),
            });
            relations += 1;
        }
        note.relations = out;
    }
    read.relations = relations;
    read.unresolved = unresolved;
}

/// What applying an import changed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ImportReport {
    /// The vault folder.
    pub vault: PathBuf,
    /// The mode.
    pub mode: ImportMode,
    /// Notes read.
    pub notes: usize,
    /// Documents registered in the library.
    pub documents: usize,
    /// Node notes created or refreshed for the user's own notes.
    pub nodes: usize,
    /// Notes on documents updated from their exported copies.
    pub notes_updated: usize,
    /// Notes on documents re-created from exported copies.
    pub notes_added: usize,
    /// Highlights added back to documents.
    pub highlights_added: usize,
    /// Highlights whose range or color changed.
    pub highlights_updated: usize,
    /// Relations found.
    pub relations: usize,
    /// Links whose target could not be found.
    pub unresolved: usize,
    /// Notes that could not be read.
    pub unreadable: usize,
}

impl ImportReport {
    /// A one-sentence summary that reads well aloud.
    pub fn summary(&self) -> String {
        let vault = self.vault.file_name().map_or_else(
            || self.vault.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let mut s = format!("Imported {} from {vault}", plural(self.notes, "note"));
        let mut parts = Vec::new();
        if self.documents > 0 {
            parts.push(format!(
                "{} added to the library",
                plural(self.documents, "document")
            ));
        }
        if self.notes_updated + self.notes_added > 0 {
            let n = self.notes_updated + self.notes_added;
            parts.push(if n == 1 {
                "1 note on a document updated".to_owned()
            } else {
                format!("{n} notes on documents updated")
            });
        }
        if self.highlights_added > 0 {
            parts.push(format!(
                "{} restored",
                plural(self.highlights_added, "highlight")
            ));
        }
        if self.mode == ImportMode::Graph {
            parts.push(plural(self.relations, "link").to_string());
        }
        if !parts.is_empty() {
            s.push_str(": ");
            s.push_str(&parts.join(", "));
        }
        if self.unresolved > 0 {
            s.push_str(&format!(
                "; {} could not be found",
                plural(self.unresolved, "linked note")
            ));
        }
        if self.unreadable > 0 {
            s.push_str(&format!(
                "; {} could not be read",
                plural(self.unreadable, "note")
            ));
        }
        s.push('.');
        s
    }
}

/// Applies a read vault to `store`.
pub fn apply(
    read: &VaultRead,
    store: &mut dyn AnnotationStore,
) -> Result<ImportReport, VaultError> {
    let mut report = ImportReport {
        vault: read.vault.clone(),
        mode: read.mode,
        notes: read.notes.len(),
        relations: read.relations,
        unresolved: read.unresolved,
        unreadable: read.unreadable.len(),
        ..ImportReport::default()
    };
    let now = textweaver_store::now_ts();
    // Every note id in the vault: a relation to a note that is not here
    // could not have been written to the vault, so an import keeps it.
    let in_vault: std::collections::HashSet<&str> =
        read.notes.iter().map(|n| n.id.as_str()).collect();
    for note in &read.notes {
        if read.mode == ImportMode::Library {
            store.register_document(&note.path, &note.title, "markdown")?;
            report.documents += 1;
            continue;
        }
        match &note.kind {
            NoteKind::Plain => {
                store.register_document(&note.path, &note.title, "markdown")?;
                report.documents += 1;
                apply_node(note, store, now)?;
                report.nodes += 1;
            }
            NoteKind::Annotation { source, position } => {
                let mut ann = store.load(source)?;
                let tags: Vec<String> = note
                    .tags
                    .iter()
                    .filter(|t| *t != NODE_TAG)
                    .cloned()
                    .collect();
                match ann.note_mut(&note.id) {
                    Some(existing) => {
                        let relations =
                            merged_relations(&note.relations, &existing.relations, &in_vault);
                        let changed = existing.note != note.text
                            || existing.tags != tags
                            || existing.cite != note.cite
                            || existing.relations != relations
                            || existing.anchor != note.title;
                        if changed {
                            existing.note = note.text.clone();
                            existing.tags = tags;
                            existing.cite = note.cite.clone();
                            existing.relations = relations;
                            existing.anchor = note.title.clone();
                            existing.ts = now;
                            report.notes_updated += 1;
                        }
                    }
                    None => {
                        let ts = if note.ts > 0 { note.ts } else { now };
                        ann.notes.push(Note {
                            id: note.id.clone(),
                            range: CharRange::new(*position, *position),
                            anchor: note.title.clone(),
                            note: note.text.clone(),
                            tags,
                            cite: note.cite.clone(),
                            created: ts,
                            ts,
                            relations: note.relations.clone(),
                            ..Note::default()
                        });
                        report.notes_added += 1;
                    }
                }
                store.save(source, &ann)?;
            }
            NoteKind::Document { source, highlights } => {
                let mut ann = store.load(source)?;
                for h in highlights {
                    match ann.highlights.iter_mut().find(|x| x.id == h.id) {
                        Some(x) => {
                            if x.range != h.range || x.color != h.color {
                                x.range = h.range;
                                x.color = h.color.clone();
                                report.highlights_updated += 1;
                            }
                        }
                        None => {
                            ann.highlights.push(h.clone());
                            report.highlights_added += 1;
                        }
                    }
                }
                store.save(source, &ann)?;
            }
        }
    }
    Ok(report)
}

/// Creates or refreshes the node note of a plain vault note (Star's graph
/// node: position 0, anchor the title, text the summary, tag
/// `obsidian-note`).
/// The relations a note has after an import: the ones its file lists, plus
/// the ones it had to notes that are not in the vault. Those were left out
/// of the export ("links left out because the linked note is not in the
/// vault"), so their absence from the file does not mean they were deleted.
fn merged_relations(
    from_file: &[Relation],
    existing: &[Relation],
    in_vault: &std::collections::HashSet<&str>,
) -> Vec<Relation> {
    let mut out = from_file.to_vec();
    for r in existing {
        if !in_vault.contains(r.target_id.as_str()) && !out.contains(r) {
            out.push(r.clone());
        }
    }
    out
}

fn apply_node(
    note: &VaultNote,
    store: &mut dyn AnnotationStore,
    now: i64,
) -> Result<(), VaultError> {
    let mut ann = store.load(&note.path)?;
    let mut tags = note.tags.clone();
    if !tags.iter().any(|t| t == NODE_TAG) {
        tags.push(NODE_TAG.to_owned());
    }
    let text = if note.summary.is_empty() {
        note.title.clone()
    } else {
        note.summary.clone()
    };
    let at = ann.notes.iter().position(|n| n.id == note.id).or_else(|| {
        ann.notes
            .iter()
            .position(|n| n.tags.iter().any(|t| t == NODE_TAG))
    });
    let node = match at {
        Some(i) => &mut ann.notes[i],
        None => {
            ann.notes.push(Note {
                created: now,
                ts: now,
                ..Note::default()
            });
            let last = ann.notes.len() - 1;
            &mut ann.notes[last]
        }
    };
    node.id = note.id.clone();
    node.anchor = note.title.clone();
    node.note = text;
    node.tags = tags;
    node.relations = note.relations.clone();
    if node.ts == 0 {
        node.ts = now;
    }
    store.save(&note.path, &ann)
}

/// Reads the vault and applies it to `store`.
pub fn import_vault(
    vault: &Path,
    options: &ImportOptions,
    store: &mut dyn AnnotationStore,
) -> Result<ImportReport, VaultError> {
    let read = read_vault(vault, options)?;
    apply(&read, store)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::MemoryStore;

    fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, text).unwrap();
        p
    }

    /// A small vault in Star's test style.
    fn vault() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_owned();
        write(
            &root,
            "Alpha.md",
            "---\ntitle: Alpha Note\naliases: [First]\ntags: [intro]\nstar_id: aaaa1111\n---\n# Alpha\nLinks to [[Beta]] and supports:: [[Gamma]], also [[Beta|B again]].\n![[Beta]]\n#exam\n",
        );
        write(
            &root,
            "sub/Beta.md",
            "Beta body mentions [[first]] and [[Nowhere]].\n",
        );
        write(&root, "Gamma.md", "---\ntags: solo\n---\n\n");
        write(&root, ".obsidian/workspace.md", "[[Alpha]]");
        (dir, root)
    }

    #[test]
    fn graph_import_builds_nodes_and_relations() {
        let (_d, root) = vault();
        let mut store = MemoryStore::new();
        let report = import_vault(&root, &ImportOptions::default(), &mut store).unwrap();
        assert_eq!(report.notes, 3);
        assert_eq!(report.documents, 3);
        assert_eq!(report.nodes, 3);
        // Alpha: SEE_ALSO Beta (once; the alias link is a duplicate and the
        // embed is ignored), SUPPORTS Gamma. Beta: SEE_ALSO Alpha via its
        // alias. `Nowhere` is unresolved.
        assert_eq!(report.relations, 3);
        assert_eq!(report.unresolved, 1);
        assert_eq!(
            report.summary(),
            format!(
                "Imported 3 notes from {}: 3 documents added to the library, 3 links; 1 linked note could not be found.",
                root.file_name().unwrap().to_string_lossy()
            )
        );

        let alpha = store.load(&root.join("Alpha.md")).unwrap();
        assert_eq!(alpha.notes.len(), 1);
        let node = &alpha.notes[0];
        assert_eq!(node.id, "aaaa1111");
        assert_eq!(node.anchor, "Alpha Note");
        assert_eq!(node.note, "Alpha");
        assert_eq!(node.tags, vec!["intro", "exam", NODE_TAG]);
        let rels: Vec<(RelationType, String)> = node
            .relations
            .iter()
            .map(|r| {
                (
                    r.relation_type().unwrap(),
                    Path::new(&r.target_doc)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                )
            })
            .collect();
        assert_eq!(
            rels,
            vec![
                (RelationType::Supports, "Gamma.md".to_owned()),
                (RelationType::SeeAlso, "Beta.md".to_owned()),
            ]
        );
        let beta = store.load(&root.join("sub/Beta.md")).unwrap();
        assert_eq!(beta.notes[0].relations[0].target_id, "aaaa1111");
        let gamma = store.load(&root.join("Gamma.md")).unwrap();
        // An empty note's node text falls back to its title (the stem).
        assert_eq!(gamma.notes[0].note, "Gamma");
        assert_eq!(gamma.notes[0].tags, vec!["solo", NODE_TAG]);
        assert_eq!(store.library.len(), 3);
        assert_eq!(store.library[0].title, "Alpha Note");
    }

    #[test]
    fn reimport_is_idempotent() {
        let (_d, root) = vault();
        let mut store = MemoryStore::new();
        import_vault(&root, &ImportOptions::default(), &mut store).unwrap();
        let first = store.docs.clone();
        import_vault(&root, &ImportOptions::default(), &mut store).unwrap();
        assert_eq!(store.docs, first);
        assert_eq!(store.library.len(), 3);
    }

    #[test]
    fn link_relation_option_types_plain_links() {
        let (_d, root) = vault();
        let mut store = MemoryStore::new();
        let opts = ImportOptions {
            mode: ImportMode::Graph,
            link_relation: Some(RelationType::Cites),
        };
        import_vault(&root, &opts, &mut store).unwrap();
        let beta = store.load(&root.join("sub/Beta.md")).unwrap();
        assert_eq!(beta.notes[0].relations[0].rel_type, "CITES");
    }

    #[test]
    fn library_mode_only_registers_documents() {
        let (_d, root) = vault();
        let mut store = MemoryStore::new();
        let opts = ImportOptions {
            mode: ImportMode::Library,
            link_relation: None,
        };
        let report = import_vault(&root, &opts, &mut store).unwrap();
        assert_eq!(report.documents, 3);
        assert_eq!(report.relations, 0);
        assert_eq!(report.unresolved, 0);
        assert!(store.docs.is_empty());
        assert_eq!(store.library.len(), 3);
    }

    #[test]
    fn note_text_drops_links_section_and_comments() {
        let body = "First line with [[Link|alias]].\n%% hidden %%\nSecond.\n\n## Links\n\n- SUPPORTS:: [[X]]\n\n## After\nkept";
        assert_eq!(
            note_text(body),
            "First line with alias.\nSecond.\n\n## After\nkept"
        );
    }

    #[test]
    fn mode_names() {
        assert_eq!(ImportMode::parse("Library"), Some(ImportMode::Library));
        assert_eq!(ImportMode::parse("graph"), Some(ImportMode::Graph));
        assert_eq!(ImportMode::parse("x"), None);
        assert_eq!(ImportMode::Graph.as_str(), "graph");
    }
}
