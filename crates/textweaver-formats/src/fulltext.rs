//! Full-text index over loaded documents: the index side of star's
//! `star/fulltext.py`, as a small on-disk inverted index.
//!
//! [`FullTextIndex`] keeps, under a directory the caller chooses (the
//! library's cache directory):
//!
//! - `index.json`: every indexed document (path, title, size, modification
//!   time, word count) and an inverted index from each lowercase word to the
//!   documents containing it with a count;
//! - `text/<id>.txt`: each document's canonical text (at most
//!   [`MAX_TEXT_CHARS`], as in star), for snippets and phrase counts.
//!
//! As in star, indexing is lazy and incremental ([`FullTextIndex::refresh`]
//! re-reads only files whose size or modification time changed and drops
//! files no longer listed), best-effort (a file that fails to load is
//! reported and keeps its stale entry), and cancellable between files.
//! Search is case-insensitive:
//!
//! - every query word must start some word of the document (so `read`
//!   finds `reading`; star matched substrings anywhere, which also found
//!   `bread`);
//! - documents containing the whole query as a phrase come first, ranked
//!   by how often it occurs, then documents with all the words, ranked by
//!   how often they occur; ties sort by title;
//! - each hit has a snippet of about 60 chars either side of the first
//!   phrase occurrence (or first word), with `…` where it is cut.
//!
//! The index never makes the caller fail: a missing or corrupt `index.json`
//! is an empty index, and saving is explicit ([`FullTextIndex::save`],
//! atomic).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use textweaver_core::Unit;
use textweaver_core::fs::write_atomic;
use textweaver_text::{Document, segments};

use crate::{LoadOptions, Registry, Source};

/// Most chars of text kept per document (star's `_MAX_TEXT_CHARS`).
pub const MAX_TEXT_CHARS: usize = 2_000_000;

/// Chars of context either side of a hit in a snippet.
pub const SNIPPET_CONTEXT: usize = 60;

const VERSION: u32 = 1;

/// One indexed document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedDocument {
    /// Internal id (names the text file).
    pub id: u32,
    /// The file.
    pub path: PathBuf,
    /// Display title.
    pub title: String,
    /// File size when indexed.
    pub size: u64,
    /// Modification time when indexed, in nanoseconds since the Unix epoch.
    pub mtime_ns: u64,
    /// Number of words.
    pub words: u32,
}

/// A search result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHit {
    /// The document's file.
    pub path: PathBuf,
    /// Its title.
    pub title: String,
    /// Phrase occurrences when [`exact`](Self::exact), else occurrences of
    /// the query words.
    pub count: usize,
    /// True when the whole query occurs as a phrase.
    pub exact: bool,
    /// Text around the first occurrence, on one line.
    pub snippet: String,
}

/// What [`FullTextIndex::refresh`] did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RefreshReport {
    /// Documents (re)indexed.
    pub indexed: usize,
    /// Documents unchanged since they were indexed.
    pub reused: usize,
    /// Documents dropped because they are no longer listed or no longer exist.
    pub removed: usize,
    /// Files that failed to load, with the reason (their old entries stay).
    pub failed: Vec<(PathBuf, String)>,
    /// True when `should_stop` ended the refresh early.
    pub stopped: bool,
}

#[derive(Default, Serialize, Deserialize)]
struct Stored {
    version: u32,
    next_id: u32,
    docs: Vec<IndexedDocument>,
    /// Word → (document id, count), sorted by id.
    terms: BTreeMap<String, Vec<(u32, u32)>>,
}

/// An inverted index of document text on disk.
pub struct FullTextIndex {
    dir: PathBuf,
    next_id: u32,
    docs: BTreeMap<u32, IndexedDocument>,
    by_path: HashMap<PathBuf, u32>,
    terms: BTreeMap<String, Vec<(u32, u32)>>,
    /// Texts added or changed since the last save.
    pending: HashMap<u32, String>,
    /// Text files to delete on save.
    deleted: BTreeSet<u32>,
}

/// The file's size and modification time (ns), if it exists.
fn stamp(path: &Path) -> Option<(u64, u64)> {
    let md = fs::metadata(path).ok()?;
    let mtime = md
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX));
    Some((md.len(), mtime))
}

/// Lowercases char by char, one char for one char, so offsets in the result
/// are offsets in the original.
fn fold(s: &str) -> Vec<char> {
    s.chars()
        .map(|c| c.to_lowercase().next().unwrap_or(c))
        .collect()
}

/// The lowercase words of `text`, as the reader segments words.
fn words(text: &str) -> Vec<String> {
    let doc = Document::from_plain_text(text);
    segments(&doc, Unit::Word)
        .into_iter()
        .map(|r| doc.slice(r).to_lowercase())
        .filter(|w| w.chars().count() <= 64)
        .collect()
}

impl FullTextIndex {
    /// The index stored under `dir` (empty when there is none or it cannot
    /// be read).
    pub fn open(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        let stored: Stored = fs::read(dir.join("index.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .filter(|s: &Stored| s.version == VERSION)
            .unwrap_or_default();
        let docs: BTreeMap<u32, IndexedDocument> =
            stored.docs.into_iter().map(|d| (d.id, d)).collect();
        let by_path = docs.values().map(|d| (d.path.clone(), d.id)).collect();
        FullTextIndex {
            dir,
            next_id: stored.next_id,
            docs,
            by_path,
            terms: stored.terms,
            pending: HashMap::new(),
            deleted: BTreeSet::new(),
        }
    }

    /// The index directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Number of indexed documents.
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    /// True when nothing is indexed.
    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    /// The indexed documents, by id.
    pub fn documents(&self) -> impl Iterator<Item = &IndexedDocument> {
        self.docs.values()
    }

    /// True when `path` is indexed and unchanged since (same size and
    /// modification time).
    pub fn is_current(&self, path: &Path) -> bool {
        let Some(d) = self.by_path.get(path).and_then(|id| self.docs.get(id)) else {
            return false;
        };
        stamp(path) == Some((d.size, d.mtime_ns))
    }

    /// Indexes a loaded document under `path` (replacing any earlier entry),
    /// stamped with the file's current size and modification time.
    pub fn add_document(&mut self, path: &Path, doc: &Document) {
        let title = doc
            .meta
            .title
            .clone()
            .or_else(|| path.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_default();
        let (size, mtime_ns) = stamp(path).unwrap_or((0, 0));
        self.add_text(path, &title, &doc.text().to_string(), size, mtime_ns);
    }

    /// Indexes `text` for `path` with the given title and file stamp.
    pub fn add_text(&mut self, path: &Path, title: &str, text: &str, size: u64, mtime_ns: u64) {
        self.remove(path);
        let text: String = if text.chars().count() > MAX_TEXT_CHARS {
            text.chars().take(MAX_TEXT_CHARS).collect()
        } else {
            text.to_owned()
        };
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        let mut counts: HashMap<String, u32> = HashMap::new();
        let all = words(&text);
        for w in &all {
            *counts.entry(w.clone()).or_default() += 1;
        }
        for (w, n) in counts {
            let postings = self.terms.entry(w).or_default();
            let at = postings.partition_point(|(d, _)| *d < id);
            postings.insert(at, (id, n));
        }
        self.docs.insert(
            id,
            IndexedDocument {
                id,
                path: path.to_owned(),
                title: title.to_owned(),
                size,
                mtime_ns,
                words: u32::try_from(all.len()).unwrap_or(u32::MAX),
            },
        );
        self.by_path.insert(path.to_owned(), id);
        self.deleted.remove(&id);
        self.pending.insert(id, text);
    }

    /// Drops `path` from the index; true when it was indexed.
    pub fn remove(&mut self, path: &Path) -> bool {
        let Some(id) = self.by_path.remove(path) else {
            return false;
        };
        self.docs.remove(&id);
        self.pending.remove(&id);
        self.deleted.insert(id);
        self.terms.retain(|_, postings| {
            postings.retain(|(d, _)| *d != id);
            !postings.is_empty()
        });
        true
    }

    /// Brings the index up to date with `paths` (the library's documents):
    /// new and changed files are loaded with `registry` and indexed,
    /// unchanged ones kept, unlisted or vanished ones dropped. `should_stop`
    /// is checked between files. Call [`save`](Self::save) afterwards.
    pub fn refresh(
        &mut self,
        paths: impl IntoIterator<Item = PathBuf>,
        registry: &Registry,
        options: &LoadOptions,
        should_stop: &dyn Fn() -> bool,
    ) -> RefreshReport {
        let mut report = RefreshReport::default();
        let mut listed: BTreeSet<PathBuf> = BTreeSet::new();
        for path in paths {
            if should_stop() {
                report.stopped = true;
                return report;
            }
            if !listed.insert(path.clone()) {
                continue;
            }
            if stamp(&path).is_none() {
                continue;
            }
            if self.is_current(&path) {
                report.reused += 1;
                continue;
            }
            match registry.load(&Source::Path(path.clone()), options) {
                Ok(doc) => {
                    self.add_document(&path, &doc);
                    report.indexed += 1;
                }
                Err(e) => report.failed.push((path, e.to_string())),
            }
        }
        let gone: Vec<PathBuf> = self
            .by_path
            .keys()
            .filter(|p| !listed.contains(*p) || stamp(p).is_none())
            .cloned()
            .collect();
        for p in gone {
            if self.remove(&p) {
                report.removed += 1;
            }
        }
        report
    }

    fn text_path(&self, id: u32) -> PathBuf {
        self.dir.join("text").join(format!("{id}.txt"))
    }

    fn text_of(&self, id: u32) -> Option<String> {
        if let Some(t) = self.pending.get(&id) {
            return Some(t.clone());
        }
        fs::read_to_string(self.text_path(id)).ok()
    }

    /// Documents matching `query`, best first, at most `limit`.
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchHit> {
        let qwords = words(query);
        if qwords.is_empty() {
            return Vec::new();
        }
        // Documents where every query word starts some indexed word, with
        // the summed counts of those words.
        let mut candidates: Option<HashMap<u32, usize>> = None;
        for w in &qwords {
            let mut found: HashMap<u32, usize> = HashMap::new();
            for (_, postings) in self
                .terms
                .range::<str, _>((
                    std::ops::Bound::Included(w.as_str()),
                    std::ops::Bound::Unbounded,
                ))
                .take_while(|(t, _)| t.starts_with(w.as_str()))
            {
                for &(d, n) in postings {
                    *found.entry(d).or_default() += n as usize;
                }
            }
            candidates = Some(match candidates {
                None => found,
                Some(prev) => prev
                    .into_iter()
                    .filter_map(|(d, n)| found.get(&d).map(|m| (d, n + m)))
                    .collect(),
            });
        }
        let phrase: Vec<char> = fold(&query.split_whitespace().collect::<Vec<_>>().join(" "));
        let mut hits = Vec::new();
        for (id, word_count) in candidates.unwrap_or_default() {
            let Some(doc) = self.docs.get(&id) else {
                continue;
            };
            let text = self.text_of(id).unwrap_or_default();
            let chars: Vec<char> = text.chars().collect();
            let folded = fold(&text);
            let occurrences = find_all(&folded, &phrase);
            let exact = !occurrences.is_empty();
            let at = occurrences.first().copied().or_else(|| {
                let first = fold(&qwords[0]);
                find_all(&folded, &first).first().copied()
            });
            let len = if exact {
                phrase.len()
            } else {
                qwords[0].chars().count()
            };
            hits.push(SearchHit {
                path: doc.path.clone(),
                title: doc.title.clone(),
                count: if exact { occurrences.len() } else { word_count },
                exact,
                snippet: at.map(|a| snippet(&chars, a, len)).unwrap_or_default(),
            });
        }
        hits.sort_by(|a, b| {
            b.exact
                .cmp(&a.exact)
                .then(b.count.cmp(&a.count))
                .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
                .then_with(|| a.path.cmp(&b.path))
        });
        hits.truncate(limit);
        hits
    }

    /// Writes the index atomically (and the texts added since the last
    /// save), removing texts of dropped documents.
    pub fn save(&mut self) -> std::io::Result<()> {
        fs::create_dir_all(self.dir.join("text"))?;
        for (id, text) in &self.pending {
            write_atomic(&self.text_path(*id), text.as_bytes())?;
        }
        for id in &self.deleted {
            let _ = fs::remove_file(self.text_path(*id));
        }
        let stored = Stored {
            version: VERSION,
            next_id: self.next_id,
            docs: self.docs.values().cloned().collect(),
            terms: self.terms.clone(),
        };
        let json = serde_json::to_vec(&stored).map_err(std::io::Error::other)?;
        write_atomic(&self.dir.join("index.json"), &json)?;
        self.pending.clear();
        self.deleted.clear();
        Ok(())
    }
}

/// Start offsets of the non-overlapping occurrences of `needle`.
fn find_all(hay: &[char], needle: &[char]) -> Vec<usize> {
    let mut out = Vec::new();
    if needle.is_empty() || needle.len() > hay.len() {
        return out;
    }
    let mut i = 0;
    while i + needle.len() <= hay.len() {
        if hay[i..i + needle.len()] == *needle {
            out.push(i);
            i += needle.len();
        } else {
            i += 1;
        }
    }
    out
}

/// star's snippet: the hit with context either side, whitespace collapsed,
/// `…` where cut.
fn snippet(chars: &[char], at: usize, len: usize) -> String {
    let start = at.saturating_sub(SNIPPET_CONTEXT);
    let end = (at + len + SNIPPET_CONTEXT).min(chars.len());
    let body: String = chars[start..end].iter().collect();
    let mut s = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if start > 0 {
        s.insert(0, '\u{2026}');
    }
    if end < chars.len() {
        s.push('\u{2026}');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_ranks_phrases_then_words_with_snippets() {
        let dir = tempfile::tempdir().unwrap();
        let mut idx = FullTextIndex::open(dir.path().join("fts"));
        idx.add_text(
            Path::new("/a.md"),
            "Alpha",
            "Reading aloud helps. Reading aloud again.",
            1,
            1,
        );
        idx.add_text(
            Path::new("/b.md"),
            "Beta",
            "Aloud, the students were reading.",
            1,
            1,
        );
        idx.add_text(
            Path::new("/c.md"),
            "Gamma",
            "Bread is not reading material aloud? It is.",
            1,
            1,
        );
        idx.add_text(Path::new("/d.md"), "Delta", "Nothing relevant here.", 1, 1);
        let hits = idx.search("reading ALOUD", 10);
        let order: Vec<&str> = hits.iter().map(|h| h.title.as_str()).collect();
        assert_eq!(order, ["Alpha", "Beta", "Gamma"]);
        assert!(hits[0].exact && hits[0].count == 2);
        assert!(!hits[1].exact);
        assert_eq!(hits[0].snippet, "Reading aloud helps. Reading aloud again.");
        // Prefixes match word starts only.
        assert_eq!(idx.search("read", 10).len(), 3);
        assert!(idx.search("ead", 10).is_empty());
        assert!(idx.search("   ", 10).is_empty());
        assert_eq!(idx.search("reading", 1).len(), 1);
    }

    #[test]
    fn saves_reopens_refreshes_and_removes() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path().join("lib");
        fs::create_dir_all(&lib).unwrap();
        let a = lib.join("a.md");
        let b = lib.join("b.txt");
        fs::write(&a, "# Photosynthesis\n\nPlants make sugar from light.").unwrap();
        fs::write(&b, "Light travels fast.").unwrap();
        let registry = Registry::with_builtins();
        let opts = LoadOptions::default();
        let store = dir.path().join("fts");
        let mut idx = FullTextIndex::open(&store);
        let r = idx.refresh([a.clone(), b.clone()], &registry, &opts, &|| false);
        assert_eq!((r.indexed, r.reused, r.removed), (2, 0, 0));
        idx.save().unwrap();

        let mut idx = FullTextIndex::open(&store);
        assert_eq!(idx.len(), 2);
        let hits = idx.search("light", 10);
        assert_eq!(hits.len(), 2);
        // Equal counts sort by title.
        assert_eq!(hits[0].title, "b.txt");
        assert_eq!(hits[1].title, "Photosynthesis");
        let r = idx.refresh([a.clone(), b.clone()], &registry, &opts, &|| false);
        assert_eq!((r.indexed, r.reused), (0, 2));

        // A changed file is reindexed; an unlisted one dropped.
        fs::write(&b, "Sound travels slower than light, much slower.").unwrap();
        let r = idx.refresh([b.clone()], &registry, &opts, &|| false);
        assert_eq!((r.indexed, r.removed), (1, 1));
        idx.save().unwrap();
        let idx = FullTextIndex::open(&store);
        assert_eq!(idx.search("sugar", 10).len(), 0);
        assert_eq!(idx.search("slower", 10)[0].count, 2);
        assert_eq!(fs::read_dir(store.join("text")).unwrap().count(), 1);

        // Stopping early, and a corrupt index is an empty one.
        let mut idx = FullTextIndex::open(&store);
        let r = idx.refresh([a], &registry, &opts, &|| true);
        assert!(r.stopped);
        fs::write(store.join("index.json"), "{broken").unwrap();
        assert!(FullTextIndex::open(&store).is_empty());
    }
}
