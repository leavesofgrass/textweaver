//! Library search over document text (the query side of Star's
//! `star/fulltext.py`).
//!
//! [`FullTextIndex`] is the interface the library searches through. The
//! formats crate owns text extraction and its own on-disk index; until the
//! orchestrator wires that in, [`SimpleIndex`] is a small implementation
//! here: extracted text per document in one JSON cache file, refreshed only
//! for files whose size or modification time changed, searched as Star did
//! (case-insensitive substring, ranked by match count).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::library::ScannedDoc;
use crate::{StoreError, atomic_write};

/// Characters of context on each side of a snippet's match (Star 60).
pub const SNIPPET_CONTEXT: usize = 60;

/// One document that matched a search.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHit {
    /// The document.
    pub path: PathBuf,
    /// Its title.
    pub title: String,
    /// Number of matches (non-overlapping).
    pub count: usize,
    /// The text around the first match, on one line, with "..." where it
    /// was cut.
    pub snippet: String,
}

impl SearchHit {
    /// One line for lists and speech: "Chapter 3, 4 matches: ...snippet...".
    pub fn describe(&self) -> String {
        format!(
            "{}, {} {}: {}",
            self.title,
            self.count,
            if self.count == 1 { "match" } else { "matches" },
            self.snippet
        )
    }
}

/// A searchable index of document text.
pub trait FullTextIndex {
    /// Documents whose text contains `query` (case-insensitive), most
    /// matches first, then by title; at most `limit`. An empty or blank
    /// query finds nothing.
    fn search(&self, query: &str, limit: usize) -> Vec<SearchHit>;

    /// Number of documents indexed.
    fn len(&self) -> usize;

    /// True when nothing is indexed.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// One indexed document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedText {
    /// Title shown in results.
    pub title: String,
    /// File size when indexed.
    pub size: u64,
    /// Modification time when indexed (Unix seconds).
    pub mtime: i64,
    /// The document's text.
    pub text: String,
}

/// What [`SimpleIndex::refresh`] did.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshReport {
    /// Newly indexed documents.
    pub added: usize,
    /// Re-indexed because they changed.
    pub updated: usize,
    /// Dropped because they are no longer in the library.
    pub removed: usize,
    /// Documents whose text could not be extracted.
    pub failed: Vec<PathBuf>,
}

impl RefreshReport {
    /// True when the index changed.
    pub fn changed(&self) -> bool {
        self.added + self.updated + self.removed > 0
    }
}

/// The in-crate [`FullTextIndex`]: document text in memory, cached in one
/// JSON file (for example `<cache>/fulltext.json`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimpleIndex {
    /// Indexed documents by path.
    pub entries: BTreeMap<PathBuf, IndexedText>,
}

impl SimpleIndex {
    /// Loads the cache; a missing or unreadable cache is an empty index
    /// (it is only a cache, rebuilt by [`refresh`](Self::refresh)).
    pub fn load(file: &Path) -> Self {
        std::fs::read_to_string(file)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    /// Saves the cache atomically.
    pub fn save(&self, file: &Path) -> Result<(), StoreError> {
        let text = serde_json::to_string(self).map_err(|e| StoreError::Parse {
            path: file.to_owned(),
            message: e.to_string(),
        })?;
        atomic_write(file, text.as_bytes())
    }

    /// Brings the index up to date with `docs`: extracts text for new and
    /// changed documents (by size and modification time) with `extract`,
    /// and drops documents no longer listed.
    pub fn refresh(
        &mut self,
        docs: &[ScannedDoc],
        extract: &mut dyn FnMut(&Path) -> Option<String>,
    ) -> RefreshReport {
        let mut report = RefreshReport::default();
        let wanted: std::collections::HashSet<&PathBuf> = docs.iter().map(|d| &d.path).collect();
        let before = self.entries.len();
        self.entries.retain(|p, _| wanted.contains(p));
        report.removed = before - self.entries.len();
        for doc in docs {
            let fresh = self
                .entries
                .get(&doc.path)
                .is_some_and(|e| e.size == doc.size && e.mtime == doc.mtime);
            if fresh {
                continue;
            }
            let existed = self.entries.contains_key(&doc.path);
            match extract(&doc.path) {
                Some(text) => {
                    self.entries.insert(
                        doc.path.clone(),
                        IndexedText {
                            title: doc.title.clone(),
                            size: doc.size,
                            mtime: doc.mtime,
                            text,
                        },
                    );
                    if existed {
                        report.updated += 1;
                    } else {
                        report.added += 1;
                    }
                }
                None => {
                    self.entries.remove(&doc.path);
                    report.failed.push(doc.path.clone());
                }
            }
        }
        report
    }

    /// Indexes one document's text directly (tests, or a document just
    /// loaded by the reader).
    pub fn insert(&mut self, path: PathBuf, title: &str, text: String) {
        self.entries.insert(
            path,
            IndexedText {
                title: title.to_owned(),
                size: 0,
                mtime: 0,
                text,
            },
        );
    }
}

/// Finds `query` in `text` case-insensitively: the number of
/// non-overlapping matches and the char range of the first, in `text`.
fn find_all(text: &str, query: &str) -> Option<(usize, usize, usize)> {
    // Lowercase char by char, remembering which original char each
    // lowercase char came from, so the snippet is cut from the original
    // text even where lowercasing changes the length.
    let mut low = String::with_capacity(text.len());
    let mut origin: Vec<usize> = Vec::with_capacity(text.len());
    for (i, c) in text.chars().enumerate() {
        for l in c.to_lowercase() {
            low.push(l);
            origin.push(i);
        }
    }
    let mut count = 0;
    let mut first: Option<usize> = None;
    for (byte, _) in low.match_indices(query) {
        if first.is_none() {
            first = Some(byte);
        }
        count += 1;
    }
    let byte = first?;
    let start_low = low[..byte].chars().count();
    let end_low = start_low + query.chars().count();
    let start = origin.get(start_low).copied().unwrap_or(0);
    let end = origin
        .get(end_low.saturating_sub(1))
        .map_or(start, |i| i + 1);
    Some((count, start, end))
}

fn snippet(text: &str, start: usize, end: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let from = start.saturating_sub(SNIPPET_CONTEXT);
    let to = (end + SNIPPET_CONTEXT).min(chars.len());
    let body: String = chars[from..to].iter().collect();
    let mut s = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if from > 0 {
        s = format!("...{s}");
    }
    if to < chars.len() {
        s.push_str("...");
    }
    s
}

impl FullTextIndex for SimpleIndex {
    fn search(&self, query: &str, limit: usize) -> Vec<SearchHit> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        let mut hits: Vec<SearchHit> = self
            .entries
            .iter()
            .filter_map(|(path, e)| {
                let (count, start, end) = find_all(&e.text, &q)?;
                Some(SearchHit {
                    path: path.clone(),
                    title: e.title.clone(),
                    count,
                    snippet: snippet(&e.text, start, end),
                })
            })
            .collect();
        hits.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
        });
        hits.truncate(limit);
        hits
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(path: &str, size: u64, mtime: i64) -> ScannedDoc {
        ScannedDoc {
            path: PathBuf::from(path),
            rel: path.to_owned(),
            title: path.trim_end_matches(".md").to_owned(),
            ext: "md".into(),
            size,
            mtime,
            folder: PathBuf::from("/lib"),
        }
    }

    #[test]
    fn search_ranks_by_count_then_title() {
        let mut idx = SimpleIndex::default();
        idx.insert("b.md".into(), "Beta", "cell cell membrane".into());
        idx.insert("a.md".into(), "alpha", "One Cell.".into());
        idx.insert("c.md".into(), "Gamma", "nothing here".into());
        let hits = idx.search("CELL", 10);
        let titles: Vec<&str> = hits.iter().map(|h| h.title.as_str()).collect();
        assert_eq!(titles, vec!["Beta", "alpha"]);
        assert_eq!(hits[0].count, 2);
        assert_eq!(hits[1].snippet, "One Cell.");
        assert!(idx.search("   ", 10).is_empty());
        assert_eq!(idx.search("cell", 1).len(), 1);
        assert_eq!(idx.len(), 3);
        assert_eq!(hits[0].describe(), "Beta, 2 matches: cell cell membrane");
    }

    #[test]
    fn snippets_cut_from_the_original_text() {
        let mut idx = SimpleIndex::default();
        let long = format!(
            "{}İstanbul target words\nnext line{}",
            "x ".repeat(50),
            " y".repeat(50)
        );
        idx.insert("d.md".into(), "D", long);
        let hit = &idx.search("TARGET", 5)[0];
        assert!(hit.snippet.starts_with("..."), "{}", hit.snippet);
        assert!(hit.snippet.ends_with("..."), "{}", hit.snippet);
        assert!(
            hit.snippet.contains("İstanbul target words next line"),
            "{}",
            hit.snippet
        );
        // A query that lowercases to more chars than it has still matches.
        assert_eq!(idx.search("i\u{307}stanbul", 5).len(), 1);
    }

    #[test]
    fn refresh_extracts_only_changes_and_persists() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("fulltext.json");
        let mut idx = SimpleIndex::load(&file);
        let mut calls = Vec::new();
        let mut extract = |p: &Path| {
            calls.push(p.to_owned());
            (!p.ends_with("bad.md")).then(|| format!("text of {}", p.display()))
        };
        let docs = vec![doc("a.md", 1, 1), doc("b.md", 1, 1), doc("bad.md", 1, 1)];
        let r = idx.refresh(&docs, &mut extract);
        assert_eq!((r.added, r.updated, r.removed), (2, 0, 0));
        assert_eq!(r.failed, vec![PathBuf::from("bad.md")]);
        let docs = vec![doc("a.md", 2, 1), doc("b.md", 1, 1)];
        let r = idx.refresh(&docs, &mut extract);
        assert_eq!((r.added, r.updated, r.removed), (0, 1, 0));
        assert!(r.changed());
        assert_eq!(calls.len(), 4, "b.md was not extracted again");
        idx.save(&file).unwrap();
        let back = SimpleIndex::load(&file);
        assert_eq!(back, idx);
        let r = idx.refresh(&[doc("a.md", 2, 1)], &mut |_| None);
        assert_eq!(r.removed, 1);
        std::fs::write(&file, "junk").unwrap();
        assert!(SimpleIndex::load(&file).is_empty());
    }
}
