//! Where a note's links lead: the vault root, and the one resolver that
//! the reader's links and embeds, the converter, and the vault commands
//! share.
//!
//! A note's **scope** is its vault: the nearest folder at or above the
//! note holding a `.obsidian` folder ([`find_vault_root`]). Links resolve
//! anywhere inside it and never outside it, as in Obsidian. A note with no
//! `.obsidian` above it keeps the older rule: its own folder and the
//! folders below it.
//!
//! A link target (`Note`, `folder/Note`, `Note.md`, `picture.png`)
//! resolves, in order:
//!
//! 1. as a path from the linking note's folder (`../Other.md` included,
//!    but inside a vault only when it stays inside);
//! 2. inside a vault, as a path from the vault root;
//! 3. by name: a file whose name, or whose path ending, is the target,
//!    with `.md` added when the target has no `.md`. When several match,
//!    the nearest to the linking note wins (fewest folders between them),
//!    then the first in path order, and [`Resolved::same_name`] says how
//!    many there were.
//!
//! Titles and aliases from front matter come after these, through
//! [`VaultIndex::resolve`](crate::VaultIndex::resolve), since they need
//! every note read.

use std::path::{Component, Path, PathBuf};

use textweaver_store::library::resolve_path;

/// Most folder entries one scope walk looks at; past it the walk stops
/// and says it was cut short ([`ScopeFiles::truncated`]).
pub const MAX_SCOPE_ENTRIES: usize = 100_000;

/// The nearest folder at or above `path` (a note or a folder) that holds
/// a `.obsidian` folder: the vault the note belongs to. `None` when there
/// is none. The path is resolved first (absolute, links followed, no
/// `\\?\` prefix), so the answer never depends on the current folder.
pub fn find_vault_root(path: &Path) -> Option<PathBuf> {
    let resolved = resolve_path(path);
    let start = if resolved.is_dir() {
        resolved.as_path()
    } else {
        resolved.parent()?
    };
    start
        .ancestors()
        .find(|dir| dir.join(".obsidian").is_dir())
        .map(Path::to_owned)
}

/// The folder a note's links may reach.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkScope {
    root: PathBuf,
    vault: bool,
}

impl LinkScope {
    /// The scope of the note at `note`: its vault, else its own folder.
    pub fn for_note(note: &Path) -> Self {
        let resolved = resolve_path(note);
        let folder = if resolved.is_dir() {
            resolved.clone()
        } else {
            resolved
                .parent()
                .map_or_else(|| resolved.clone(), Path::to_owned)
        };
        Self::for_folder(&folder)
    }

    /// The scope of a note in `folder`: the vault holding the folder, else
    /// the folder itself.
    pub fn for_folder(folder: &Path) -> Self {
        let folder = resolve_path(folder);
        match find_vault_root(&folder) {
            Some(root) => LinkScope { root, vault: true },
            None => LinkScope {
                root: folder,
                vault: false,
            },
        }
    }

    /// The scope's top folder: the vault root, or the note's folder.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// True when the scope is a vault (a `.obsidian` folder was found).
    pub fn is_vault(&self) -> bool {
        self.vault
    }

    /// True when `path` (resolved) lies inside the scope.
    pub fn contains(&self, path: &Path) -> bool {
        resolve_path(path).starts_with(&self.root)
    }

    /// Every file in the scope ([`walk`]).
    pub fn files(&self) -> ScopeFiles {
        walk(&self.root, MAX_SCOPE_ENTRIES)
    }

    /// `path` relative to the root, with `/` between folders, for lists
    /// and messages. A path outside the scope is shown whole.
    pub fn relative(&self, path: &Path) -> String {
        let resolved = resolve_path(path);
        rel_str(&self.root, &resolved).unwrap_or_else(|| resolved.display().to_string())
    }
}

/// `path` relative to `root` with `/` between folders, when inside it;
/// nothing is resolved (the walk's paths already are).
fn rel_str(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    Some(
        rel.components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
    )
}

/// The files of a scope, found once and reused for many links.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScopeFiles {
    /// Every file, resolved, sorted by path.
    pub files: Vec<PathBuf>,
    /// True when the walk stopped at its entry limit.
    pub truncated: bool,
}

/// Every file under `root`, sorted, skipping folders whose names start
/// with a dot (`.obsidian`, `.trash`, `.git`), as Obsidian does, and not
/// following links to folders, so a loop cannot hang the walk. Stops after
/// `max_entries` folder entries.
pub fn walk(root: &Path, max_entries: usize) -> ScopeFiles {
    let mut out = ScopeFiles::default();
    let mut stack = vec![root.to_owned()];
    let mut seen = 0usize;
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            seen += 1;
            if seen > max_entries {
                out.truncated = true;
                out.files.sort();
                return out;
            }
            let Ok(kind) = e.file_type() else { continue };
            let name = e.file_name();
            if kind.is_dir() {
                if !name.to_string_lossy().starts_with('.') {
                    stack.push(e.path());
                }
            } else if kind.is_file() || (kind.is_symlink() && e.path().is_file()) {
                out.files.push(e.path());
            }
        }
    }
    out.files.sort();
    out
}

/// What part of a note a link names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Anchor {
    /// A heading: `Note#Heading`, or a path of headings, `Note#A#B`.
    Heading(String),
    /// A block: `Note#^id`, the id without its `^`.
    Block(String),
}

impl Anchor {
    /// The heading texts to look for, most specific first: the whole
    /// anchor (a heading may itself hold `#`), then the last heading of a
    /// nested path (`A#B` finds `B`). Empty for a block.
    pub fn heading_candidates(&self) -> Vec<&str> {
        match self {
            Anchor::Block(_) => Vec::new(),
            Anchor::Heading(h) => {
                let mut out = vec![h.as_str()];
                if let Some(last) = h.rsplit('#').map(str::trim).find(|s| !s.is_empty())
                    && last != h.as_str()
                {
                    out.push(last);
                }
                out
            }
        }
    }
}

/// A link target split into the file it names and the part of it:
/// `Note#Heading` is `("Note", Heading)`, `Note#^id` is `("Note",
/// Block)`, `#Heading` names a heading of the linking note itself (an
/// empty file).
pub fn split_target(target: &str) -> (String, Option<Anchor>) {
    let target = target.split('|').next().unwrap_or(target);
    match target.split_once('#') {
        None => (target.trim().to_owned(), None),
        Some((file, part)) => {
            let part = part.trim();
            let anchor = match part.strip_prefix('^') {
                Some(id) if !id.trim().is_empty() => Some(Anchor::Block(id.trim().to_owned())),
                Some(_) => None,
                None if part.is_empty() => None,
                None => Some(Anchor::Heading(part.to_owned())),
            };
            (file.trim().to_owned(), anchor)
        }
    }
}

/// A link's file, found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    /// The file.
    pub path: PathBuf,
    /// How many files the name matched (1 when it was unambiguous, or when
    /// the link gave a path). More than 1: the nearest was chosen.
    pub same_name: usize,
}

/// The file the link `name` (the file part of a target, without `#`)
/// names, for a note in `from_folder`, inside `scope`; `files` is the
/// scope's walk. See the module documentation for the order.
pub fn resolve(
    scope: &LinkScope,
    files: &ScopeFiles,
    from_folder: &Path,
    name: &str,
) -> Option<Resolved> {
    let name = name.trim().replace('\\', "/");
    if name.is_empty() {
        return None;
    }
    let from_folder = resolve_path(from_folder);
    let one = |path: PathBuf| Resolved { path, same_name: 1 };
    // A path from the linking note's folder, then (in a vault) from the
    // root.
    // A plain name (no folder) is looked for by name only, so a duplicate
    // elsewhere is counted even when the nearest is beside the note.
    let mut bases = Vec::new();
    if name.contains('/') {
        bases.push(from_folder.clone());
        if scope.vault {
            bases.push(scope.root.clone());
        }
    }
    let direct = |bases: &[PathBuf]| {
        bases.iter().find_map(|base| {
            with_md(&base.join(&name), &name).into_iter().find_map(|c| {
                let real = c.is_file().then(|| resolve_path(&c))?;
                (!scope.vault || real.starts_with(&scope.root)).then(|| one(real))
            })
        })
    };
    if let Some(found) = direct(&bases) {
        return Some(found);
    }
    // By name: only plain names and paths inside the scope.
    let rel = Path::new(&name);
    if !rel.components().all(|c| matches!(c, Component::Normal(_))) {
        return None;
    }
    let lower = name.to_lowercase();
    let wanted: Vec<String> = if lower.ends_with(".md") {
        vec![lower]
    } else {
        vec![format!("{lower}.md"), lower]
    };
    // Exact name with `.md` first: `Note` finds `Note.md` before a file
    // called `Note` with no extension.
    for w in &wanted {
        let matches: Vec<&PathBuf> = files
            .files
            .iter()
            .filter(|p| {
                rel_str(&scope.root, p).is_some_and(|r| {
                    let r = r.to_lowercase();
                    r == *w || r.ends_with(&format!("/{w}"))
                })
            })
            .collect();
        if matches.is_empty() {
            continue;
        }
        let nearest = matches
            .iter()
            .min_by_key(|p| distance(&from_folder, p.parent().unwrap_or(p)))
            .map(|p| (*p).clone())?;
        return Some(Resolved {
            path: nearest,
            same_name: matches.len(),
        });
    }
    // A plain name beside the note, when a very large folder cut the walk
    // short before reaching it.
    if bases.is_empty() && files.truncated {
        return direct(std::slice::from_ref(&from_folder));
    }
    None
}

/// `path`, and `path` with `.md` added when `name` does not end in `.md`.
fn with_md(path: &Path, name: &str) -> Vec<PathBuf> {
    let mut out = vec![path.to_owned()];
    if !name.to_lowercase().ends_with(".md") {
        let mut md = path.as_os_str().to_owned();
        md.push(".md");
        out.insert(0, PathBuf::from(md));
    }
    out
}

/// How many folders lie between two folders: up from `a` to their common
/// folder, then down to `b`.
pub fn distance(a: &Path, b: &Path) -> usize {
    let a: Vec<Component<'_>> = a.components().collect();
    let b: Vec<Component<'_>> = b.components().collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    (a.len() - common) + (b.len() - common)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, text: &str) -> PathBuf {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, text).unwrap();
        p
    }

    /// A vault with notes in sibling and nested folders, and two notes
    /// named `Dup`.
    fn vault() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("vault");
        std::fs::create_dir_all(root.join(".obsidian")).unwrap();
        write(&root, "Notes/a.md", "a");
        write(&root, "Notes/deep/nested/c.md", "c");
        write(&root, "Projects/b.md", "b");
        write(&root, "Projects/Dup.md", "dup near b");
        write(&root, "Archive/old/Dup.md", "dup far");
        write(&root, "Notes/pic.png", "png");
        write(&root, ".trash/b.md", "trashed");
        write(tmp.path(), "outside.md", "outside");
        (tmp, resolve_path(&root))
    }

    fn found(root: &Path, from: &str, name: &str) -> Option<(String, usize)> {
        let scope = LinkScope::for_note(&root.join(from));
        let files = scope.files();
        let from_folder = root.join(from).parent().unwrap().to_owned();
        resolve(&scope, &files, &from_folder, name).map(|r| (scope.relative(&r.path), r.same_name))
    }

    #[test]
    fn the_vault_root_is_the_nearest_folder_with_obsidian() {
        let (tmp, root) = vault();
        assert_eq!(
            find_vault_root(&root.join("Notes/deep/nested/c.md")),
            Some(root.clone())
        );
        assert_eq!(find_vault_root(&root), Some(root.clone()));
        // A nested vault is its own vault.
        std::fs::create_dir_all(root.join("Projects/.obsidian")).unwrap();
        assert_eq!(
            find_vault_root(&root.join("Projects/b.md")),
            Some(root.join("Projects"))
        );
        // Above the vault there is none.
        assert_eq!(find_vault_root(&tmp.path().join("outside.md")), None);
    }

    #[test]
    fn links_reach_sibling_and_nested_folders_in_a_vault() {
        let (_tmp, root) = vault();
        assert_eq!(
            found(&root, "Notes/a.md", "b"),
            Some(("Projects/b.md".into(), 1))
        );
        assert_eq!(
            found(&root, "Projects/b.md", "c"),
            Some(("Notes/deep/nested/c.md".into(), 1))
        );
        // By a path from the root, and by a path ending.
        assert_eq!(
            found(&root, "Notes/a.md", "Projects/b"),
            Some(("Projects/b.md".into(), 1))
        );
        assert_eq!(
            found(&root, "Projects/b.md", "nested/c.md"),
            Some(("Notes/deep/nested/c.md".into(), 1))
        );
        // Relative paths, attachments, and case.
        assert_eq!(
            found(&root, "Notes/a.md", "../Projects/B.md"),
            Some(("Projects/b.md".into(), 1))
        );
        assert_eq!(
            found(&root, "Projects/b.md", "pic.png"),
            Some(("Notes/pic.png".into(), 1))
        );
        // Never a note in a dot folder.
        assert_eq!(found(&root, "Notes/a.md", "missing"), None);
    }

    #[test]
    fn duplicates_pick_the_nearest_and_count() {
        let (_tmp, root) = vault();
        assert_eq!(
            found(&root, "Projects/b.md", "Dup"),
            Some(("Projects/Dup.md".into(), 2))
        );
        assert_eq!(
            found(&root, "Archive/old/x.md", "dup"),
            Some(("Archive/old/Dup.md".into(), 2))
        );
        // A path names one of them exactly.
        assert_eq!(
            found(&root, "Notes/a.md", "Archive/old/Dup"),
            Some(("Archive/old/Dup.md".into(), 1))
        );
    }

    #[test]
    fn links_never_leave_the_vault() {
        let (_tmp, root) = vault();
        assert_eq!(found(&root, "Notes/a.md", "../../outside.md"), None);
        assert_eq!(found(&root, "Notes/a.md", "../../outside"), None);
        assert_eq!(found(&root, "Notes/a.md", "outside"), None);
    }

    #[test]
    fn without_a_vault_the_note_folder_and_below_as_before() {
        let tmp = tempfile::tempdir().unwrap();
        let top = resolve_path(tmp.path());
        write(&top, "docs/a.md", "a");
        write(&top, "docs/sub/b.md", "b");
        write(&top, "other/c.md", "c");
        write(&top, "chapter-2.md", "two");
        let scope = LinkScope::for_note(&top.join("docs/a.md"));
        assert!(!scope.is_vault());
        assert_eq!(scope.root(), top.join("docs"));
        let files = scope.files();
        let from = top.join("docs");
        let get = |n: &str| resolve(&scope, &files, &from, n).map(|r| r.path);
        assert_eq!(get("b"), Some(top.join("docs/sub/b.md")));
        // A sibling folder is out of reach by name...
        assert_eq!(get("c"), None);
        // ...but an ordinary relative link works, as it always did.
        assert_eq!(get("../chapter-2.md"), Some(top.join("chapter-2.md")));
    }

    #[test]
    fn targets_split_into_file_heading_and_block() {
        assert_eq!(split_target("Note"), ("Note".into(), None));
        assert_eq!(
            split_target("Note#Part two|shown"),
            ("Note".into(), Some(Anchor::Heading("Part two".into())))
        );
        assert_eq!(
            split_target("Note#^key-point"),
            ("Note".into(), Some(Anchor::Block("key-point".into())))
        );
        assert_eq!(
            split_target("#Here"),
            (String::new(), Some(Anchor::Heading("Here".into())))
        );
        let nested = Anchor::Heading("A#B".into());
        assert_eq!(nested.heading_candidates(), vec!["A#B", "B"]);
        assert!(Anchor::Block("x".into()).heading_candidates().is_empty());
    }
}
