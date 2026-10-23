//! The vault index: every note of a scope with its name, title, aliases,
//! tags, and outgoing links, for backlinks ("Notes that link here"),
//! unresolved links ("Links to notes that don't exist"), tags, and links
//! by title or alias.
//!
//! It is built when first needed ([`VaultIndex::build`]), bounded in size
//! ([`MAX_INDEX_NOTES`], [`MAX_INDEX_NOTE_BYTES`], [`MAX_INDEX_BYTES`]),
//! and brought up to date with [`VaultIndex::refresh`], which reads again
//! only the notes whose size or time changed. Nothing in a vault is ever
//! written.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Serialize;

use crate::frontmatter;
use crate::import::collect_tags;
use crate::links::note_links;
use crate::scope::{LinkScope, Resolved, ScopeFiles, distance, resolve, split_target};
use crate::walk::stem;

/// Most notes indexed; past it the index says it is partial.
pub const MAX_INDEX_NOTES: usize = 20_000;

/// Largest note read for its links and tags; a larger one is indexed by
/// its name only.
pub const MAX_INDEX_NOTE_BYTES: u64 = 4 << 20;

/// Most bytes read for one index; notes past it are indexed by name only.
pub const MAX_INDEX_BYTES: u64 = 256 << 20;

/// One outgoing link of an indexed note.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IndexLink {
    /// The target as written (`Note#Heading`), without its shown text.
    pub target: String,
    /// True for an embed.
    pub embed: bool,
    /// The note or file it reaches, when it resolves.
    pub resolved: Option<PathBuf>,
}

/// One note in the index.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IndexedNote {
    /// The note's file, resolved.
    pub path: PathBuf,
    /// Its name: the file name without `.md`.
    pub name: String,
    /// Front matter `title`.
    pub title: Option<String>,
    /// Front matter `aliases` (and the older `alias`).
    pub aliases: Vec<String>,
    /// Front matter `tags` (and `tag`) and inline `#tags`, without `#`.
    pub tags: Vec<String>,
    /// Its links to other files, in order.
    pub links: Vec<IndexLink>,
    /// False when it was too large, or past the index's byte limit, to
    /// read: only its name is known.
    pub read: bool,
    #[serde(skip)]
    stamp: Option<(u64, Option<SystemTime>)>,
}

/// A note linking to another, for "Notes that link here".
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Backlink {
    /// The linking note.
    pub from: PathBuf,
    /// How many of its links reach the note.
    pub links: usize,
}

/// A link to a note that does not exist.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Unresolved {
    /// The note holding the link.
    pub from: PathBuf,
    /// The link's target as written.
    pub target: String,
}

/// A tag and how many notes carry it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TagCount {
    /// The tag, without `#`, as first written in the vault.
    pub tag: String,
    /// The notes that carry it (or a tag nested under it, `tag/sub`).
    pub notes: usize,
}

/// The index of one scope (a vault, or a folder without one).
#[derive(Clone, Debug)]
pub struct VaultIndex {
    scope: LinkScope,
    files: ScopeFiles,
    notes: Vec<IndexedNote>,
    truncated: bool,
}

impl VaultIndex {
    /// Reads every note of `scope`, within the size limits.
    pub fn build(scope: LinkScope) -> Self {
        let mut index = VaultIndex {
            scope,
            files: ScopeFiles::default(),
            notes: Vec::new(),
            truncated: false,
        };
        index.refresh();
        index
    }

    /// The index of the note at `note`'s scope.
    pub fn for_note(note: &Path) -> Self {
        Self::build(LinkScope::for_note(note))
    }

    /// Brings the index up to date with the files: new notes are read,
    /// removed ones dropped, and changed ones (another size or time) read
    /// again; every link is then resolved again. True when anything
    /// changed.
    pub fn refresh(&mut self) -> bool {
        let files = self.scope.files();
        let mut old: HashMap<PathBuf, IndexedNote> = std::mem::take(&mut self.notes)
            .into_iter()
            .map(|n| (n.path.clone(), n))
            .collect();
        let mut changed = files != self.files;
        let mut notes = Vec::new();
        let mut bytes = 0u64;
        let mut truncated = files.truncated;
        for path in files.files.iter().filter(|p| is_note(p)) {
            if notes.len() >= MAX_INDEX_NOTES {
                truncated = true;
                break;
            }
            let meta = std::fs::metadata(path).ok();
            let stamp = meta.as_ref().map(|m| (m.len(), m.modified().ok()));
            let len = stamp.map_or(0, |s| s.0);
            if let Some(prev) = old.remove(path)
                && prev.stamp == stamp
                && (prev.read || len > MAX_INDEX_NOTE_BYTES)
            {
                bytes += if prev.read { len } else { 0 };
                notes.push(prev);
                continue;
            }
            changed = true;
            let fits = len <= MAX_INDEX_NOTE_BYTES && bytes + len <= MAX_INDEX_BYTES;
            if !fits && len <= MAX_INDEX_NOTE_BYTES {
                truncated = true;
            }
            let text = fits
                .then(|| std::fs::read(path).ok())
                .flatten()
                .map(|b| String::from_utf8_lossy(&b).into_owned());
            if let Some(t) = &text {
                bytes += t.len() as u64;
            }
            let mut note = index_note(path, text.as_deref());
            note.stamp = stamp;
            notes.push(note);
        }
        changed |= !old.is_empty();
        self.files = files;
        self.notes = notes;
        self.truncated = truncated;
        self.resolve_links();
        changed
    }

    /// The scope indexed.
    pub fn scope(&self) -> &LinkScope {
        &self.scope
    }

    /// True when a limit stopped the index short: some notes, or their
    /// links and tags, are missing.
    pub fn is_partial(&self) -> bool {
        self.truncated
    }

    /// The notes, sorted by path.
    pub fn notes(&self) -> &[IndexedNote] {
        &self.notes
    }

    /// The note at `path`, when indexed.
    pub fn note(&self, path: &Path) -> Option<&IndexedNote> {
        let path = textweaver_store::library::resolve_path(path);
        self.notes.iter().find(|n| n.path == path)
    }

    /// The file the link `target` (`Note`, `Note#Heading`, `Alias`) names
    /// for the note at `from`: by path and name ([`resolve`]), then by a
    /// note's front matter title, then by an alias, the nearest first.
    pub fn resolve(&self, from: &Path, target: &str) -> Option<Resolved> {
        let from = textweaver_store::library::resolve_path(from);
        self.resolve_in(from.parent().unwrap_or(&from), target)
    }

    /// [`resolve`](Self::resolve) for a note in `folder`.
    pub fn resolve_in(&self, folder: &Path, target: &str) -> Option<Resolved> {
        let (file, _) = split_target(target);
        let folder = textweaver_store::library::resolve_path(folder);
        resolve(&self.scope, &self.files, &folder, &file)
            .or_else(|| self.by_title_or_alias(&folder, &file))
    }

    fn by_title_or_alias(&self, folder: &Path, name: &str) -> Option<Resolved> {
        let want = name.trim().to_lowercase();
        if want.is_empty() {
            return None;
        }
        self.nearest(folder, |n| {
            n.title.as_deref().is_some_and(|t| t.to_lowercase() == want)
        })
        .or_else(|| {
            self.nearest(folder, |n| {
                n.aliases.iter().any(|a| a.to_lowercase() == want)
            })
        })
    }

    /// The note nearest `folder` among those `pick` accepts.
    fn nearest(&self, folder: &Path, pick: impl Fn(&IndexedNote) -> bool) -> Option<Resolved> {
        let matches: Vec<&IndexedNote> = self.notes.iter().filter(|n| pick(n)).collect();
        let near = matches
            .iter()
            .min_by_key(|n| distance(folder, n.path.parent().unwrap_or(&n.path)))?;
        Some(Resolved {
            path: near.path.clone(),
            same_name: matches.len(),
        })
    }

    fn resolve_links(&mut self) {
        let resolved: Vec<Vec<Option<PathBuf>>> = self
            .notes
            .iter()
            .map(|n| {
                n.links
                    .iter()
                    .map(|l| self.resolve(&n.path, &l.target).map(|r| r.path))
                    .collect()
            })
            .collect();
        for (note, links) in self.notes.iter_mut().zip(resolved) {
            for (link, r) in note.links.iter_mut().zip(links) {
                link.resolved = r;
            }
        }
    }

    /// The notes that link to (or embed) the note at `path`, in path
    /// order, each with how many of its links reach it. The note's links
    /// to itself are left out.
    pub fn backlinks(&self, path: &Path) -> Vec<Backlink> {
        let path = textweaver_store::library::resolve_path(path);
        self.notes
            .iter()
            .filter(|n| n.path != path)
            .filter_map(|n| {
                let links = n
                    .links
                    .iter()
                    .filter(|l| l.resolved.as_deref() == Some(path.as_path()))
                    .count();
                (links > 0).then(|| Backlink {
                    from: n.path.clone(),
                    links,
                })
            })
            .collect()
    }

    /// The links that reach no file: those of the note at `path`, or of
    /// every note with `None`. Each target is listed once per note.
    pub fn unresolved(&self, path: Option<&Path>) -> Vec<Unresolved> {
        let path = path.map(textweaver_store::library::resolve_path);
        let mut out = Vec::new();
        for n in &self.notes {
            if path.as_ref().is_some_and(|p| *p != n.path) {
                continue;
            }
            let mut seen: Vec<String> = Vec::new();
            for l in n.links.iter().filter(|l| l.resolved.is_none()) {
                let key = l.target.to_lowercase();
                if !seen.contains(&key) {
                    seen.push(key);
                    out.push(Unresolved {
                        from: n.path.clone(),
                        target: l.target.clone(),
                    });
                }
            }
        }
        out
    }

    /// Every tag in the scope with the number of notes carrying it,
    /// sorted by tag ignoring case. Tags differing only in case are one
    /// tag, as in Obsidian. A note tagged `a/b` counts for `a/b` only;
    /// [`notes_with_tag`](Self::notes_with_tag) finds it under `a` too.
    pub fn tags(&self) -> Vec<TagCount> {
        let mut counts: BTreeMap<String, TagCount> = BTreeMap::new();
        for n in &self.notes {
            let mut mine: Vec<String> = Vec::new();
            for t in &n.tags {
                let key = t.to_lowercase();
                if mine.contains(&key) {
                    continue;
                }
                mine.push(key.clone());
                counts
                    .entry(key)
                    .or_insert_with(|| TagCount {
                        tag: t.clone(),
                        notes: 0,
                    })
                    .notes += 1;
            }
        }
        counts.into_values().collect()
    }

    /// The notes carrying `tag` (with or without `#`, any case), or a tag
    /// nested under it (`tag/sub`), in path order.
    pub fn notes_with_tag(&self, tag: &str) -> Vec<PathBuf> {
        let want = tag.trim().trim_start_matches('#').to_lowercase();
        if want.is_empty() {
            return Vec::new();
        }
        self.notes
            .iter()
            .filter(|n| {
                n.tags.iter().any(|t| {
                    let t = t.to_lowercase();
                    t == want || t.starts_with(&format!("{want}/"))
                })
            })
            .map(|n| n.path.clone())
            .collect()
    }

    /// The note's title for lists: its front matter title, else its name.
    pub fn title_of(&self, path: &Path) -> String {
        self.note(path).map_or_else(
            || stem(path),
            |n| n.title.clone().unwrap_or_else(|| n.name.clone()),
        )
    }
}

/// True for a Markdown note.
fn is_note(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("md"))
}

/// One note's entry, from its text (`None` when it was not read).
fn index_note(path: &Path, text: Option<&str>) -> IndexedNote {
    let mut note = IndexedNote {
        path: path.to_owned(),
        name: stem(path),
        title: None,
        aliases: Vec::new(),
        tags: Vec::new(),
        links: Vec::new(),
        read: text.is_some(),
        stamp: None,
    };
    let Some(text) = text else {
        return note;
    };
    let (fm, body) = frontmatter::split(text);
    note.title = fm.text("title").map(|t| t.trim().to_owned());
    note.aliases = fm.items("aliases");
    note.aliases.extend(fm.items("alias"));
    note.tags = collect_tags(&fm, body);
    note.links = note_links(body)
        .into_iter()
        .map(|l| IndexLink {
            target: l.target,
            embed: l.embed,
            resolved: None,
        })
        .collect();
    note
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, text: &str) -> PathBuf {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, text).unwrap();
        textweaver_store::library::resolve_path(&p)
    }

    fn vault() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("vault");
        std::fs::create_dir_all(root.join(".obsidian")).unwrap();
        write(
            &root,
            "Notes/Cell.md",
            "---\ntitle: The Cell\naliases:\n  - Cytology\n  - Cell biology\ntags: [biology, exam]\n---\n# Cell\n\nSee [[Mitochondria]] and [[Mitochondria#Energy]].\n\n## Parts ^parts\n",
        );
        write(
            &root,
            "Projects/Mitochondria.md",
            "# Mitochondria\n\nPart of the [[Cytology]]. Also [[Ghost]] and [[ghost|again]].\n\n`[[Not a link]]` %% [[Hidden]] %%\n\n#biology/organelles\n\n## Energy\n",
        );
        write(
            &root,
            "Daily/today.md",
            "Read ![[Cell#^parts]] and [the cell](../Notes/Cell.md). #Exam\n",
        );
        (tmp, textweaver_store::library::resolve_path(&root))
    }

    fn rel(index: &VaultIndex, p: &Path) -> String {
        index.scope().relative(p)
    }

    #[test]
    fn backlinks_count_wikilinks_embeds_and_markdown_links() {
        let (_tmp, root) = vault();
        let index = VaultIndex::for_note(&root.join("Notes/Cell.md"));
        assert!(index.scope().is_vault());
        let back: Vec<(String, usize)> = index
            .backlinks(&root.join("Notes/Cell.md"))
            .iter()
            .map(|b| (rel(&index, &b.from), b.links))
            .collect();
        // today.md embeds a block and links by path; Mitochondria links by
        // an alias.
        assert_eq!(
            back,
            vec![
                ("Daily/today.md".to_owned(), 2),
                ("Projects/Mitochondria.md".to_owned(), 1)
            ]
        );
        let back: Vec<(String, usize)> = index
            .backlinks(&root.join("Projects/Mitochondria.md"))
            .iter()
            .map(|b| (rel(&index, &b.from), b.links))
            .collect();
        assert_eq!(back, vec![("Notes/Cell.md".to_owned(), 2)]);
    }

    #[test]
    fn unresolved_links_skip_code_and_comments_and_repeat_once() {
        let (_tmp, root) = vault();
        let index = VaultIndex::for_note(&root);
        let missing: Vec<String> = index
            .unresolved(Some(&root.join("Projects/Mitochondria.md")))
            .into_iter()
            .map(|u| u.target)
            .collect();
        assert_eq!(missing, vec!["Ghost"]);
        assert_eq!(index.unresolved(None).len(), 1);
        assert!(
            index
                .unresolved(Some(&root.join("Notes/Cell.md")))
                .is_empty()
        );
    }

    #[test]
    fn titles_and_aliases_resolve_after_names() {
        let (_tmp, root) = vault();
        let index = VaultIndex::for_note(&root);
        let from = root.join("Daily/today.md");
        let get = |t: &str| index.resolve(&from, t).map(|r| rel(&index, &r.path));
        assert_eq!(get("Cell biology"), Some("Notes/Cell.md".into()));
        assert_eq!(get("the cell"), Some("Notes/Cell.md".into()));
        assert_eq!(
            get("Mitochondria#Energy"),
            Some("Projects/Mitochondria.md".into())
        );
        assert_eq!(get("Nobody"), None);
        assert_eq!(index.title_of(&root.join("Notes/Cell.md")), "The Cell");
        assert_eq!(index.title_of(&from), "today");
    }

    #[test]
    fn tags_count_notes_and_nested_tags_find_their_parents() {
        let (_tmp, root) = vault();
        let index = VaultIndex::for_note(&root);
        let tags: Vec<(String, usize)> =
            index.tags().into_iter().map(|t| (t.tag, t.notes)).collect();
        assert_eq!(
            tags,
            vec![
                ("biology".to_owned(), 1),
                ("biology/organelles".to_owned(), 1),
                ("Exam".to_owned(), 2),
            ]
        );
        let names = |t: &str| -> Vec<String> {
            index
                .notes_with_tag(t)
                .iter()
                .map(|p| rel(&index, p))
                .collect()
        };
        assert_eq!(
            names("#Biology"),
            vec!["Notes/Cell.md", "Projects/Mitochondria.md"]
        );
        assert_eq!(names("EXAM"), vec!["Daily/today.md", "Notes/Cell.md"]);
        assert!(names("bio").is_empty());
    }

    #[test]
    fn refresh_reads_new_changed_and_removed_notes() {
        let (_tmp, root) = vault();
        let mut index = VaultIndex::for_note(&root);
        assert!(!index.refresh(), "nothing changed");
        assert_eq!(index.unresolved(None).len(), 1);
        // The missing note appears: the link resolves.
        write(&root, "Ghost.md", "# Ghost\n");
        assert!(index.refresh());
        assert!(index.unresolved(None).is_empty());
        // A note changes its links.
        write(&root, "Daily/today.md", "Now only [[Ghost]].\n");
        assert!(index.refresh());
        assert_eq!(index.backlinks(&root.join("Notes/Cell.md")).len(), 1);
        assert_eq!(index.backlinks(&root.join("Ghost.md")).len(), 2);
        // A renamed target: the old name no longer resolves.
        std::fs::rename(root.join("Ghost.md"), root.join("Spirit.md")).unwrap();
        assert!(index.refresh());
        assert_eq!(index.unresolved(None).len(), 2);
    }

    #[test]
    fn large_notes_are_indexed_by_name_only() {
        let (_tmp, root) = vault();
        let big = "x".repeat(usize::try_from(MAX_INDEX_NOTE_BYTES).unwrap() + 1);
        write(&root, "Big.md", &format!("[[Cell]] {big}"));
        let index = VaultIndex::for_note(&root);
        let note = index.note(&root.join("Big.md")).unwrap();
        assert!(!note.read);
        assert!(note.links.is_empty());
        let from = root.join("Notes/Cell.md");
        assert!(index.resolve(&from, "Big").is_some());
    }
}
