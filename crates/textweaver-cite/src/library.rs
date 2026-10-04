//! The reference library: CSL-JSON on disk, one per user and optionally one
//! per document folder.
//!
//! - The **user library** lives at `<data dir>/references.json` (see
//!   [`user_library_path`]).
//! - A **folder library** is `references.json` next to the documents that
//!   cite it (see [`folder_library_path`]), the name Pandoc users already
//!   pass to `--bibliography`, so a project folder is self-contained.
//!
//! When a key is looked up for a document, the folder library is consulted
//! first, then the user library ([`Layered`]).
//!
//! star kept citations inside `settings.json`; importing a file with a key
//! that already existed overwrote that entry even when it was a different
//! work. Here an import updates an entry only when it is the same work
//! (same DOI, same ISBN, or same key and title); a different work with a
//! taken key is added under a new key, and the report says so.

use std::path::{Path, PathBuf};

use crate::error::{CiteError, Result};
use crate::key::{base_key, unique_key};
use crate::reference::Reference;

/// File name of a library.
pub const LIBRARY_FILE: &str = "references.json";

/// The user library path under textweaver's data directory
/// (`textweaver_store::Paths::data_dir`).
pub fn user_library_path(data_dir: &Path) -> PathBuf {
    data_dir.join(LIBRARY_FILE)
}

/// The library path for a document folder.
pub fn folder_library_path(folder: &Path) -> PathBuf {
    folder.join(LIBRARY_FILE)
}

/// Something citation keys can be resolved against.
pub trait ReferenceSource {
    /// The reference with this key.
    fn get(&self, key: &str) -> Option<&Reference>;
}

/// A set of references with unique keys, optionally tied to a file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Library {
    path: Option<PathBuf>,
    items: Vec<Reference>,
}

/// What happened to one reference added to a library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AddOutcome {
    /// Added under this key.
    Added {
        /// The key.
        key: String,
    },
    /// Added, but under a new key because the requested one belonged to a
    /// different work.
    Renamed {
        /// The key that was asked for.
        requested: String,
        /// The key it got.
        key: String,
    },
    /// The same work was already present and was updated.
    Updated {
        /// The existing key.
        key: String,
    },
}

impl AddOutcome {
    /// The key the reference has now.
    pub fn key(&self) -> &str {
        match self {
            AddOutcome::Added { key }
            | AddOutcome::Renamed { key, .. }
            | AddOutcome::Updated { key } => key,
        }
    }

    /// A sentence for the status line and screen reader.
    pub fn announcement(&self) -> String {
        match self {
            AddOutcome::Added { key } => format!("Added reference {key}."),
            AddOutcome::Renamed { requested, key } => {
                format!(
                    "Added reference {key}; the key {requested} was already used by a different work."
                )
            }
            AddOutcome::Updated { key } => {
                format!("Updated reference {key}, which was already in the library.")
            }
        }
    }
}

/// The result of merging many references.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MergeReport {
    /// One outcome per incoming reference, in order.
    pub outcomes: Vec<AddOutcome>,
}

impl MergeReport {
    /// Number of new references.
    pub fn added(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| !matches!(o, AddOutcome::Updated { .. }))
            .count()
    }

    /// Number of existing references updated.
    pub fn updated(&self) -> usize {
        self.outcomes.len() - self.added()
    }

    /// Renamed keys, as `(requested, given)`.
    pub fn renamed(&self) -> Vec<(&str, &str)> {
        self.outcomes
            .iter()
            .filter_map(|o| match o {
                AddOutcome::Renamed { requested, key } => Some((requested.as_str(), key.as_str())),
                _ => None,
            })
            .collect()
    }

    /// A summary sentence: "Imported 3 references: 2 new, 1 updated. Key
    /// doe2020 was taken, so the new reference is doe2020a."
    pub fn announcement(&self) -> String {
        let n = self.outcomes.len();
        if n == 0 {
            return "The file had no references; nothing was imported.".to_owned();
        }
        let mut s = format!(
            "Imported {} {}: {} new, {} updated.",
            n,
            plural(n, "reference", "references"),
            self.added(),
            self.updated()
        );
        for (requested, key) in self.renamed() {
            s.push_str(&format!(
                " Key {requested} was taken, so the new reference is {key}."
            ));
        }
        s
    }
}

pub(crate) fn plural<'a>(n: usize, one: &'a str, many: &'a str) -> &'a str {
    if n == 1 { one } else { many }
}

impl Library {
    /// An empty library not tied to a file.
    pub fn new() -> Self {
        Library::default()
    }

    /// A library from references (keys are made unique; missing keys are
    /// generated).
    pub fn from_references(items: Vec<Reference>) -> Self {
        let mut lib = Library::new();
        lib.merge(items);
        lib
    }

    /// Loads a library; a missing file is an empty library at that path.
    pub fn load(path: &Path) -> Result<Self> {
        let items = match std::fs::read(path) {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                if text.trim().is_empty() {
                    Vec::new()
                } else {
                    crate::csljson::parse(&text)?
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(CiteError::io("read", path, e)),
        };
        let mut lib = Library::from_references(items);
        lib.path = Some(path.to_owned());
        Ok(lib)
    }

    /// The file this library saves to.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Ties the library to a file.
    pub fn set_path(&mut self, path: impl Into<PathBuf>) {
        self.path = Some(path.into());
    }

    /// Saves to [`Library::path`] (atomically: a temporary file is written
    /// and renamed over the old one). A library without a path is not
    /// saved.
    pub fn save(&self) -> Result<()> {
        match &self.path {
            Some(p) => self.save_to(p),
            None => Ok(()),
        }
    }

    /// Saves as CSL-JSON to `path` atomically, creating its folder.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)
                .map_err(|e| CiteError::io("create the folder for", path, e))?;
        }
        let text = crate::csljson::write(&self.items)?;
        textweaver_core::fs::write_atomic(path, text.as_bytes())
            .map_err(|e| CiteError::io("write", path, e))
    }

    /// All references, in the order they were added.
    pub fn references(&self) -> &[Reference] {
        &self.items
    }

    /// References sorted for reading: by first creator, then year, then title.
    pub fn sorted(&self) -> Vec<&Reference> {
        let mut v: Vec<&Reference> = self.items.iter().collect();
        v.sort_by_cached_key(|r| {
            (
                r.creators()
                    .first()
                    .map(|n| n.sort_form().to_lowercase())
                    .unwrap_or_else(|| {
                        r.title
                            .as_deref()
                            .map(crate::text::plain_title)
                            .unwrap_or_default()
                            .to_lowercase()
                    }),
                r.year().unwrap_or(i32::MAX),
                r.title.clone().unwrap_or_default().to_lowercase(),
            )
        });
        v
    }

    /// Number of references.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the library is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The reference with this key.
    pub fn get(&self, key: &str) -> Option<&Reference> {
        self.items.iter().find(|r| r.id == key)
    }

    /// Whether a key is used.
    pub fn contains(&self, key: &str) -> bool {
        self.items.iter().any(|r| r.id == key)
    }

    /// Mutable access to one reference.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Reference> {
        self.items.iter_mut().find(|r| r.id == key)
    }

    /// Removes a reference by key.
    pub fn remove(&mut self, key: &str) -> Option<Reference> {
        let i = self.items.iter().position(|r| r.id == key)?;
        Some(self.items.remove(i))
    }

    /// The index of an existing reference for the same work: same DOI, same
    /// ISBN, or same key and same title.
    pub fn find_same_work(&self, r: &Reference) -> Option<usize> {
        if let Some(doi) = r.doi_key()
            && let Some(i) = self
                .items
                .iter()
                .position(|x| x.doi_key().as_deref() == Some(&doi))
        {
            return Some(i);
        }
        if let Some(isbn) = r.isbn_key()
            && let Some(i) = self
                .items
                .iter()
                .position(|x| x.isbn_key().as_deref() == Some(&isbn))
        {
            return Some(i);
        }
        let title = normalized_title(r);
        if r.id.is_empty() || title.is_empty() {
            return None;
        }
        self.items
            .iter()
            .position(|x| x.id == r.id && normalized_title(x) == title)
    }

    /// Adds one reference (see the module docs for the update rules).
    pub fn add(&mut self, mut r: Reference) -> AddOutcome {
        if let Some(i) = self.find_same_work(&r) {
            let key = self.items[i].id.clone();
            self.items[i].update_from(r);
            return AddOutcome::Updated { key };
        }
        let requested = r.id.trim().to_owned();
        let base = if requested.is_empty() {
            base_key(&r)
        } else {
            requested.clone()
        };
        let key = unique_key(&base, |k| self.contains(k));
        r.id = key.clone();
        self.items.push(r);
        if !requested.is_empty() && requested != key {
            AddOutcome::Renamed { requested, key }
        } else {
            AddOutcome::Added { key }
        }
    }

    /// Adds many references.
    pub fn merge(&mut self, refs: Vec<Reference>) -> MergeReport {
        MergeReport {
            outcomes: refs.into_iter().map(|r| self.add(r)).collect(),
        }
    }

    /// References whose key, creators, title, container, year, DOI, or
    /// ISBN contain every word of `query` (case-insensitive).
    pub fn search(&self, query: &str) -> Vec<&Reference> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        self.sorted()
            .into_iter()
            .filter(|r| {
                let hay = haystack(r);
                words.iter().all(|w| hay.contains(w.as_str()))
            })
            .collect()
    }
}

fn haystack(r: &Reference) -> String {
    let mut parts: Vec<String> = vec![r.id.clone()];
    parts.extend(r.author.iter().chain(&r.editor).map(|n| n.natural_form()));
    for s in [&r.title, &r.container_title, &r.doi, &r.isbn]
        .into_iter()
        .flatten()
    {
        parts.push(crate::text::plain_title(s));
    }
    if let Some(y) = r.year() {
        parts.push(y.to_string());
    }
    parts.join(" ").to_lowercase()
}

fn normalized_title(r: &Reference) -> String {
    r.title
        .as_deref()
        .map(crate::text::plain_title)
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

impl ReferenceSource for Library {
    fn get(&self, key: &str) -> Option<&Reference> {
        Library::get(self, key)
    }
}

/// Libraries searched in order: a document folder's library, then the
/// user's.
#[derive(Clone, Copy, Debug)]
pub struct Layered<'a> {
    /// The libraries, first match wins.
    pub layers: &'a [&'a Library],
}

impl ReferenceSource for Layered<'_> {
    fn get(&self, key: &str) -> Option<&Reference> {
        self.layers.iter().find_map(|l| l.get(key))
    }
}

/// Gives every reference without a key a generated one, unique within the
/// slice and against `taken`.
pub fn fill_missing_keys(refs: &mut [Reference], taken: impl Fn(&str) -> bool) {
    let mut used: Vec<String> = refs
        .iter()
        .filter(|r| !r.id.is_empty())
        .map(|r| r.id.clone())
        .collect();
    for r in refs.iter_mut().filter(|r| r.id.trim().is_empty()) {
        let key = unique_key(&base_key(r), |k| taken(k) || used.iter().any(|u| u == k));
        used.push(key.clone());
        r.id = key;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::{CslDate, Name};

    fn doe(title: &str) -> Reference {
        let mut r = Reference::new("", "article-journal");
        r.author = vec![Name::new("Doe", "Jane")];
        r.issued = Some(CslDate::year(2020));
        r.title = Some(title.into());
        r
    }

    #[test]
    fn keys_are_generated_and_disambiguated() {
        let mut lib = Library::new();
        assert_eq!(
            lib.add(doe("One")),
            AddOutcome::Added {
                key: "doe2020".into()
            }
        );
        assert_eq!(
            lib.add(doe("Two")),
            AddOutcome::Added {
                key: "doe2020a".into()
            }
        );
    }

    #[test]
    fn same_work_updates_and_different_work_is_renamed() {
        let mut lib = Library::new();
        let mut a = doe("One");
        a.id = "k".into();
        lib.add(a.clone());
        a.volume = Some("3".into());
        assert_eq!(lib.add(a), AddOutcome::Updated { key: "k".into() });
        assert_eq!(lib.get("k").and_then(|r| r.volume.as_deref()), Some("3"));

        let mut b = doe("Something else");
        b.id = "k".into();
        let out = lib.add(b);
        assert_eq!(
            out,
            AddOutcome::Renamed {
                requested: "k".into(),
                key: "ka".into()
            }
        );
        assert!(
            out.announcement()
                .contains("already used by a different work")
        );
    }

    #[test]
    fn doi_matches_regardless_of_prefix_and_case() {
        let mut lib = Library::new();
        let mut a = doe("One");
        a.doi = Some("10.1000/ABC".into());
        lib.add(a);
        let mut b = doe("One (preprint title)");
        b.doi = Some("https://doi.org/10.1000/abc".into());
        assert!(matches!(lib.add(b), AddOutcome::Updated { .. }));
        assert_eq!(lib.len(), 1);
    }

    #[test]
    fn load_save_round_trip_and_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join(LIBRARY_FILE);
        let mut lib = Library::load(&path).unwrap();
        assert!(lib.is_empty());
        lib.add(doe("One"));
        lib.save().unwrap();
        let back = Library::load(&path).unwrap();
        assert_eq!(back.references(), lib.references());
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn search_and_layers() {
        let mut user = Library::new();
        user.add(doe("Reading machines"));
        let mut folder = Library::new();
        let mut x = doe("Folder copy");
        x.id = "doe2020".into();
        folder.add(x);
        assert_eq!(user.search("doe machines").len(), 1);
        assert!(user.search("nothing").is_empty());
        let layers = [&folder, &user];
        let set = Layered { layers: &layers };
        assert_eq!(
            set.get("doe2020").and_then(|r| r.title.as_deref()),
            Some("Folder copy")
        );
    }

    #[test]
    fn merge_report_reads_well() {
        let mut lib = Library::new();
        let report = lib.merge(vec![doe("A"), doe("B")]);
        assert_eq!(
            report.announcement(),
            "Imported 2 references: 2 new, 0 updated."
        );
    }
}
