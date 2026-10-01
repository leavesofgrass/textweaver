//! The library: folders scanned for documents, the bookshelf in
//! `library.json`, and the folder sidecars that carry reading positions
//! between devices (Star's `star/library.py` and `star/stats.py`,
//! the Star parity reference Part 3 §2.6 and §3).
//!
//! A library folder is an ordinary directory; pointing textweaver at a
//! folder synced by Dropbox, OneDrive, Syncthing, or iCloud makes the whole
//! library travel between machines. [`LibrarySync`] writes each document's
//! reading position into `<folder>/.textweaver/progress.json`, keyed by the
//! path relative to the folder, and chooses between that and the local
//! position when a document opens.
//!
//! Changes from Star, all deliberate:
//!
//! - bookshelf keys are absolute paths, so one file reached through
//!   `./a.md` and `a.md` shares one entry (Part 3 §7 item 15);
//! - opening a document always updates the bookshelf and recents, in both
//!   frontends (item 21);
//! - resuming honors the conflict policy instead of comparing timestamps
//!   as strings (item 25), and conflicts are returned, not dropped
//!   (item 24);
//! - the terminal reader reads and writes sidecars too (item 26).
//!
//! Library search goes through the [`FullTextIndex`](crate::fulltext::FullTextIndex)
//! trait; [`SimpleIndex`](crate::fulltext::SimpleIndex) is the in-crate
//! implementation until the formats crate's index is wired in.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use textweaver_core::CharPos;

use crate::sync::{self, Conflict, ConflictPolicy, Prefer, ProgressEntry, Recorded, SidecarStore};
use crate::{DocState, LibrarySettings, Recent, StoreError, atomic_write};

/// Directory names never scanned (Star's `_SKIP_DIRS`, plus textweaver's
/// own sidecar directory). Hidden directories are skipped as well.
pub const SKIP_DIRS: [&str; 13] = [
    ".git",
    ".svn",
    ".hg",
    "__pycache__",
    ".obsidian",
    ".star",
    ".textweaver",
    "node_modules",
    ".cache",
    ".Trash",
    ".trash",
    "$RECYCLE.BIN",
    "System Volume Information",
];

/// Most files one folder scan returns (Star 20,000).
pub const MAX_SCAN_FILES: usize = 20_000;

/// Bookshelf size that triggers eviction (Star 500).
pub const LIBRARY_CAP: usize = 500;

/// Entries evicted, oldest first, when the bookshelf passes
/// [`LIBRARY_CAP`] (Star 100).
pub const LIBRARY_EVICT: usize = 100;

/// Longest stored title, in chars (Star 200).
pub const TITLE_MAX_CHARS: usize = 200;

/// `path` made absolute, with Windows' `\\?\` prefix removed and, when the
/// path exists, symbolic links resolved. The form used for library folders
/// and bookshelf keys.
pub fn resolve_path(path: &Path) -> PathBuf {
    let p = std::fs::canonicalize(path)
        .or_else(|_| std::path::absolute(path))
        .unwrap_or_else(|_| path.to_owned());
    strip_verbatim(p)
}

/// Removes the `\\?\` prefix `canonicalize` adds on Windows, when what
/// follows is an ordinary drive path.
fn strip_verbatim(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\")
        && rest.as_bytes().get(1) == Some(&b':')
    {
        return PathBuf::from(rest);
    }
    p
}

impl LibrarySettings {
    /// Adds a library folder (resolved; adding the same folder again does
    /// nothing). Returns the stored path and whether it was new.
    pub fn add_folder(&mut self, folder: &Path) -> (PathBuf, bool) {
        let resolved = resolve_path(folder);
        if self.folders.iter().any(|f| resolve_path(f) == resolved) {
            return (resolved, false);
        }
        self.folders.push(resolved.clone());
        (resolved, true)
    }

    /// Removes a library folder, matched by resolved path. Returns whether
    /// it was there.
    pub fn remove_folder(&mut self, folder: &Path) -> bool {
        let target = resolve_path(folder);
        let before = self.folders.len();
        self.folders.retain(|f| resolve_path(f) != target);
        self.folders.len() != before
    }
}

/// One document found in a library folder.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannedDoc {
    /// The file.
    pub path: PathBuf,
    /// The path relative to its library folder, with `/` separators (the
    /// sidecar key).
    pub rel: String,
    /// The file stem.
    pub title: String,
    /// The extension, lowercase, without the dot.
    pub ext: String,
    /// Size in bytes.
    pub size: u64,
    /// Modification time (Unix seconds).
    pub mtime: i64,
    /// The library folder it was found in.
    pub folder: PathBuf,
}

fn mtime_secs(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// Every supported document under `folder` (Star's `scan_folder`): hidden
/// files and directories and [`SKIP_DIRS`] are skipped, symbolic links to
/// directories are not followed, and at most `max_files` are returned.
/// `supported` receives a lowercase extension without the dot.
pub fn scan_folder(
    folder: &Path,
    recursive: bool,
    max_files: usize,
    supported: &dyn Fn(&str) -> bool,
) -> Vec<ScannedDoc> {
    scan_folder_with(folder, recursive, max_files, supported, &|_| {})
}

/// [`scan_folder`], calling `found` with the number of documents found so
/// far after each one (a progress count for a scan on another thread).
pub fn scan_folder_with(
    folder: &Path,
    recursive: bool,
    max_files: usize,
    supported: &dyn Fn(&str) -> bool,
    found: &dyn Fn(usize),
) -> Vec<ScannedDoc> {
    let mut out = Vec::new();
    if !folder.is_dir() {
        return out;
    }
    let mut stack = vec![folder.to_owned()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
        entries.sort_by_key(|e| e.file_name());
        let mut subdirs = Vec::new();
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                if recursive && !SKIP_DIRS.contains(&name.as_str()) {
                    subdirs.push(entry.path());
                }
                continue;
            }
            let path = entry.path();
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if ext.is_empty() || !supported(&ext) {
                continue;
            }
            let Ok(meta) = std::fs::metadata(&path) else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            let rel = path
                .strip_prefix(folder)
                .map(|r| {
                    r.components()
                        .map(|c| c.as_os_str().to_string_lossy().into_owned())
                        .collect::<Vec<_>>()
                        .join("/")
                })
                .unwrap_or_else(|_| name.clone());
            out.push(ScannedDoc {
                title: path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                path,
                rel,
                ext,
                size: meta.len(),
                mtime: mtime_secs(&meta),
                folder: folder.to_owned(),
            });
            found(out.len());
            if out.len() >= max_files {
                return out;
            }
        }
        // Depth-first in name order.
        stack.extend(subdirs.into_iter().rev());
    }
    out
}

/// Scans every library folder, drops duplicates (a folder nested in
/// another), and sorts by folder, then relative path, case-insensitively
/// (Star's `scan_library`).
pub fn scan_library(folders: &[PathBuf], supported: &dyn Fn(&str) -> bool) -> Vec<ScannedDoc> {
    scan_library_with(folders, supported, &|_| {})
}

/// [`scan_library`], calling `found` with the number of documents found so
/// far across the folders.
pub fn scan_library_with(
    folders: &[PathBuf],
    supported: &dyn Fn(&str) -> bool,
    found: &dyn Fn(usize),
) -> Vec<ScannedDoc> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for folder in folders {
        let before = out.len();
        let counted = |n: usize| found(before + n);
        for doc in scan_folder_with(folder, true, MAX_SCAN_FILES, supported, &counted) {
            if seen.insert(doc.path.clone()) {
                out.push(doc);
            }
        }
    }
    out.sort_by_key(|d| {
        (
            d.folder.to_string_lossy().to_lowercase(),
            d.rel.to_lowercase(),
        )
    });
    out
}

/// How far into a document's text [`DocMetadata::from_document`] looks for a
/// DOI or an ISBN (a paper prints its DOI on the first page, a book its
/// ISBN on the copyright page).
pub const METADATA_SCAN_CHARS: usize = 20_000;

/// Longest stored author, in chars.
pub const AUTHOR_MAX_CHARS: usize = 200;

/// A document's bibliographic facts, for searching the library by them
/// (Star's `discovery.py`): the author, the DOI, and the ISBN. Each comes
/// from the document's own metadata (front matter, DOCX and EPUB
/// properties, HTML `<meta>`), from the start of its text, or from the
/// reference library's record of the same work (`tw cite`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocMetadata {
    /// The author or authors, as the document gives them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// The DOI, lowercase, without `doi:` or a `doi.org` link
    /// (`10.1000/xyz`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    /// The ISBN, digits only (and a final `X` for an ISBN-10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isbn: Option<String>,
}

/// Property names that carry a DOI: Markdown front matter, HTML `<meta>`
/// (Highwire, Dublin Core, PRISM), and DOCX and EPUB identifiers.
const DOI_PROPERTIES: [&str; 7] = [
    "doi",
    "citation_doi",
    "dc.identifier",
    "dc.identifier.doi",
    "prism.doi",
    "identifier",
    "dcterms.identifier",
];

/// Property names that carry an ISBN.
const ISBN_PROPERTIES: [&str; 5] = [
    "isbn",
    "citation_isbn",
    "dc.identifier",
    "identifier",
    "dcterms.identifier",
];

/// Property names that carry an author when the loader has not set one.
const AUTHOR_PROPERTIES: [&str; 4] = ["author", "authors", "citation_author", "dc.creator"];

impl DocMetadata {
    /// True when nothing is known.
    pub fn is_empty(&self) -> bool {
        self.author.is_none() && self.doi.is_none() && self.isbn.is_none()
    }

    /// The metadata of a loaded document: `author` and `properties` as the
    /// loader found them, and `text` searched (its first
    /// [`METADATA_SCAN_CHARS`] chars) for a DOI or ISBN the properties
    /// lack.
    pub fn from_document(
        author: Option<&str>,
        properties: &std::collections::BTreeMap<String, String>,
        text: &str,
    ) -> Self {
        let prop = |names: &[&str], parse: fn(&str) -> Option<String>| {
            properties
                .iter()
                .filter(|(k, _)| names.iter().any(|n| k.eq_ignore_ascii_case(n)))
                .find_map(|(_, v)| parse(v))
        };
        let author = author
            .map(str::to_owned)
            .or_else(|| prop(&AUTHOR_PROPERTIES, |v| Some(v.to_owned())))
            .map(|a| clean_author(&a))
            .filter(|a| !a.is_empty());
        let head: String = text.chars().take(METADATA_SCAN_CHARS).collect();
        DocMetadata {
            author,
            doi: prop(&DOI_PROPERTIES, find_doi).or_else(|| find_doi(&head)),
            isbn: prop(&ISBN_PROPERTIES, find_isbn_value).or_else(|| find_isbn(&head)),
        }
    }

    /// Fills what `self` lacks from `other`; returns true when anything
    /// changed.
    pub fn fill_from(&mut self, other: &DocMetadata) -> bool {
        let mut changed = false;
        for (mine, theirs) in [
            (&mut self.author, &other.author),
            (&mut self.doi, &other.doi),
            (&mut self.isbn, &other.isbn),
        ] {
            if mine.is_none() && theirs.is_some() {
                mine.clone_from(theirs);
                changed = true;
            }
        }
        changed
    }
}

/// Whitespace collapsed, at most [`AUTHOR_MAX_CHARS`] chars.
fn clean_author(a: &str) -> String {
    a.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(AUTHOR_MAX_CHARS)
        .collect()
}

/// `s` as a DOI when it is one: `10.` and a registrant of four to nine
/// digits, a slash, and a suffix; a `doi:` prefix or a `doi.org` link is
/// removed. Lowercase, since DOIs are case-insensitive.
pub fn normalize_doi(s: &str) -> Option<String> {
    let t = s.trim();
    let lower = t.to_lowercase();
    let mut rest = lower.as_str();
    for prefix in [
        "https://doi.org/",
        "http://doi.org/",
        "https://dx.doi.org/",
        "http://dx.doi.org/",
        "doi.org/",
        "doi:",
        "doi ",
        "urn:doi:",
    ] {
        if let Some(r) = rest.strip_prefix(prefix) {
            rest = r.trim_start();
            break;
        }
    }
    let rest = rest.trim_end_matches(['.', ',', ';', ':', ')', ']', '"', '\'']);
    let (registrant, suffix) = rest.strip_prefix("10.")?.split_once('/')?;
    let registrant_ok = (4..=9).contains(&registrant.len())
        && registrant
            .split('.')
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    let suffix_ok = !suffix.is_empty() && !suffix.chars().any(char::is_whitespace);
    (registrant_ok && suffix_ok).then(|| format!("10.{registrant}/{suffix}"))
}

/// The first DOI in `text` ([`normalize_doi`]).
pub fn find_doi(text: &str) -> Option<String> {
    text.split(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '"' | '(' | '['))
        .filter(|w| w.contains("10."))
        .find_map(|w| {
            let at = w.find("10.")?;
            normalize_doi(&w[at..])
        })
}

/// `s` as an ISBN when it is one: ten or thirteen digits (a final `X` for
/// ISBN-10), hyphens and spaces ignored, with a valid check digit. An
/// `ISBN` or `urn:isbn:` prefix is removed. Returns the digits only.
pub fn normalize_isbn(s: &str) -> Option<String> {
    let t = s.trim();
    let lower = t.to_ascii_lowercase();
    let body = ["urn:isbn:", "isbn-13:", "isbn-10:", "isbn:", "isbn"]
        .iter()
        .find_map(|p| lower.strip_prefix(p))
        .unwrap_or(&lower);
    let digits: String = body
        .chars()
        .filter(|c| !matches!(c, '-' | ' ' | '\u{2010}' | '\u{2011}'))
        .map(|c| c.to_ascii_uppercase())
        .collect();
    isbn_checks(&digits).then_some(digits)
}

/// True when `d` is a valid ISBN-10 or ISBN-13.
fn isbn_checks(d: &str) -> bool {
    let b = d.as_bytes();
    match b.len() {
        10 => {
            if !b[..9].iter().all(u8::is_ascii_digit) || !(b[9].is_ascii_digit() || b[9] == b'X') {
                return false;
            }
            let sum: u32 = b
                .iter()
                .enumerate()
                .map(|(i, &c)| {
                    let v = if c == b'X' { 10 } else { u32::from(c - b'0') };
                    v * (10 - i as u32)
                })
                .sum();
            sum.is_multiple_of(11)
        }
        13 => {
            if !b.iter().all(u8::is_ascii_digit)
                || !(b.starts_with(b"978") || b.starts_with(b"979"))
            {
                return false;
            }
            let sum: u32 = b
                .iter()
                .enumerate()
                .map(|(i, &c)| u32::from(c - b'0') * if i % 2 == 0 { 1 } else { 3 })
                .sum();
            sum.is_multiple_of(10)
        }
        _ => false,
    }
}

/// An ISBN in a property value (which may hold other identifiers).
fn find_isbn_value(v: &str) -> Option<String> {
    normalize_isbn(v).or_else(|| find_isbn(v))
}

/// The first ISBN in `text` that follows the word "ISBN" (a bare number
/// is too often something else).
pub fn find_isbn(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let mut from = 0;
    while let Some(at) = lower[from..].find("isbn") {
        let start = from + at + 4;
        let candidate: String = text[start..]
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit() || matches!(c, '-' | ' ' | 'X' | 'x'))
            .take(20)
            .collect();
        // The longest valid prefix: "978-0-306-40615-7 (paper)".
        let parts: Vec<&str> = candidate.trim().split(' ').collect();
        for n in (1..=parts.len()).rev() {
            if let Some(i) = normalize_isbn(&parts[..n].join(" ")) {
                return Some(i);
            }
        }
        from = start;
    }
    None
}

/// A document's details as the owner typed them (Wave 7, W7m): each field
/// set here wins over what the document says about itself, in the library
/// list, its filter, and `tw library --search`. A field not set shows the
/// document's own value.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditedDetails {
    /// The title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The author or authors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// The DOI, in [`normalize_doi`]'s form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    /// The ISBN, in [`normalize_isbn`]'s form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isbn: Option<String>,
    /// When the owner last saved an edit (milliseconds since 1970, UTC): a
    /// newer edit from another computer wins over this one.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub at_ms: u64,
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

/// A detail the owner can edit by hand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetailField {
    /// The title.
    Title,
    /// The author or authors.
    Author,
    /// The DOI.
    Doi,
    /// The ISBN.
    Isbn,
}

impl DetailField {
    /// Every field, in the edit form's order.
    pub const ALL: [DetailField; 4] = [
        DetailField::Title,
        DetailField::Author,
        DetailField::Doi,
        DetailField::Isbn,
    ];

    /// The field's name, as the sync folder's details and `--json` use it.
    pub fn name(self) -> &'static str {
        match self {
            DetailField::Title => "title",
            DetailField::Author => "author",
            DetailField::Doi => "doi",
            DetailField::Isbn => "isbn",
        }
    }

    /// `value` as stored: whitespace collapsed (a title or author at most
    /// [`TITLE_MAX_CHARS`] or [`AUTHOR_MAX_CHARS`] chars), a DOI or ISBN in
    /// its usual form. `None` for a blank value; `Err` with the value for a
    /// DOI or ISBN that is not one.
    pub fn clean(self, value: &str) -> Result<Option<String>, String> {
        let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
        if collapsed.is_empty() {
            return Ok(None);
        }
        match self {
            DetailField::Title => Ok(Some(collapsed.chars().take(TITLE_MAX_CHARS).collect())),
            DetailField::Author => Ok(Some(clean_author(&collapsed))),
            DetailField::Doi => normalize_doi(&collapsed).map(Some).ok_or(collapsed),
            DetailField::Isbn => normalize_isbn(&collapsed).map(Some).ok_or(collapsed),
        }
    }
}

impl EditedDetails {
    /// Nothing edited.
    pub fn is_empty(&self) -> bool {
        self.title.is_none() && self.author.is_none() && self.doi.is_none() && self.isbn.is_none()
    }

    /// A field's hand-edited value.
    pub fn get(&self, field: DetailField) -> Option<&str> {
        match field {
            DetailField::Title => self.title.as_deref(),
            DetailField::Author => self.author.as_deref(),
            DetailField::Doi => self.doi.as_deref(),
            DetailField::Isbn => self.isbn.as_deref(),
        }
    }

    /// Sets a field's hand-edited value; `None` clears it.
    pub fn set(&mut self, field: DetailField, value: Option<String>) {
        let slot = match field {
            DetailField::Title => &mut self.title,
            DetailField::Author => &mut self.author,
            DetailField::Doi => &mut self.doi,
            DetailField::Isbn => &mut self.isbn,
        };
        *slot = value;
    }

    /// `meta` with the hand-edited author, DOI, and ISBN in place of the
    /// document's own.
    pub fn apply_to(&self, meta: &mut DocMetadata) {
        for (mine, edited) in [
            (&mut meta.author, &self.author),
            (&mut meta.doi, &self.doi),
            (&mut meta.isbn, &self.isbn),
        ] {
            if edited.is_some() {
                mine.clone_from(edited);
            }
        }
    }
}

/// One bookshelf entry (Star's `library[path]`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LibraryEntry {
    /// The document (absolute).
    pub path: PathBuf,
    /// Its title, at most [`TITLE_MAX_CHARS`] chars.
    pub title: String,
    /// The loader that opened it (`markdown`, `text`, `html`, ...).
    #[serde(default)]
    pub format: String,
    /// When it was first opened (Unix seconds, UTC).
    #[serde(default)]
    pub added: i64,
    /// When it was last opened (Unix seconds, UTC).
    #[serde(default)]
    pub last_opened: i64,
    /// Author, DOI, and ISBN, when known.
    #[serde(flatten)]
    pub meta: DocMetadata,
    /// Details the owner typed (W7m), which win over the document's own.
    #[serde(default, skip_serializing_if = "EditedDetails::is_empty")]
    pub edited: EditedDetails,
    /// Unknown fields, preserved.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

impl LibraryEntry {
    /// The title the library shows: the owner's, else the document's.
    pub fn shown_title(&self) -> &str {
        self.edited.title.as_deref().unwrap_or(&self.title)
    }

    /// The author, DOI, and ISBN the library shows: the owner's where set,
    /// else the document's.
    pub fn shown_meta(&self) -> DocMetadata {
        let mut m = self.meta.clone();
        self.edited.apply_to(&mut m);
        m
    }
}

/// The bookshelf, `library.json`: every document opened, newest first.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Library {
    /// The entries, most recently opened first.
    pub entries: Vec<LibraryEntry>,
}

impl Library {
    /// Loads `library.json`. A missing file is an empty library; an
    /// unreadable one is an error, so the caller does not overwrite it.
    pub fn load(file: &Path) -> Result<Self, StoreError> {
        match std::fs::read_to_string(file) {
            Ok(text) => serde_json::from_str(&text).map_err(|e| StoreError::Parse {
                path: file.to_owned(),
                message: e.to_string(),
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Library::default()),
            Err(source) => Err(StoreError::Io {
                path: file.to_owned(),
                source,
            }),
        }
    }

    /// Saves `library.json` atomically.
    pub fn save(&self, file: &Path) -> Result<(), StoreError> {
        let text = serde_json::to_string_pretty(self).map_err(|e| StoreError::Parse {
            path: file.to_owned(),
            message: e.to_string(),
        })?;
        atomic_write(file, text.as_bytes())
    }

    /// The entry for `path`.
    pub fn get(&self, path: &Path) -> Option<&LibraryEntry> {
        let key = resolve_path(path);
        self.entries.iter().find(|e| e.path == key)
    }

    /// Records that `path` was opened now (Star's `_record_library`): adds
    /// it or updates its title, format, and time, and moves it to the
    /// front. Past [`LIBRARY_CAP`] entries, the [`LIBRARY_EVICT`] least
    /// recently opened are dropped. Returns the entry as recorded.
    pub fn record_open(&mut self, path: &Path, title: &str, format: &str) -> LibraryEntry {
        self.record_open_at(path, title, format, crate::now_ts())
    }

    /// [`record_open`](Self::record_open) at a given time (imports, tests).
    pub fn record_open_at(
        &mut self,
        path: &Path,
        title: &str,
        format: &str,
        when: i64,
    ) -> LibraryEntry {
        let key = resolve_path(path);
        let title: String = title.trim().chars().take(TITLE_MAX_CHARS).collect();
        let mut entry = match self.entries.iter().position(|e| e.path == key) {
            Some(i) => self.entries.remove(i),
            None => LibraryEntry {
                path: key,
                title: String::new(),
                format: String::new(),
                added: when,
                last_opened: when,
                meta: DocMetadata::default(),
                edited: EditedDetails::default(),
                extra: serde_json::Map::new(),
            },
        };
        if !title.is_empty() {
            entry.title = title;
        }
        if !format.is_empty() {
            entry.format = format.to_owned();
        }
        entry.last_opened = entry.last_opened.max(when);
        let recorded = entry.clone();
        let at = self
            .entries
            .partition_point(|e| e.last_opened > entry.last_opened);
        self.entries.insert(at, entry);
        if self.entries.len() > LIBRARY_CAP {
            self.entries
                .sort_by_key(|e| std::cmp::Reverse(e.last_opened));
            self.entries
                .truncate(self.entries.len().saturating_sub(LIBRARY_EVICT));
        }
        recorded
    }

    /// Stores what a document's own metadata says about `path` (an entry
    /// recorded before): each field `meta` has replaces the stored one, and
    /// a field it lacks keeps its value. Returns true when anything
    /// changed; false too when `path` has no entry.
    pub fn record_metadata(&mut self, path: &Path, meta: &DocMetadata) -> bool {
        let key = resolve_path(path);
        let Some(entry) = self.entries.iter_mut().find(|e| e.path == key) else {
            return false;
        };
        let before = entry.meta.clone();
        for (mine, new) in [
            (&mut entry.meta.author, &meta.author),
            (&mut entry.meta.doi, &meta.doi),
            (&mut entry.meta.isbn, &meta.isbn),
        ] {
            if new.is_some() {
                mine.clone_from(new);
            }
        }
        entry.meta != before
    }

    /// Stores the owner's hand edits of `path`'s details (W7m): each
    /// `(field, value)` sets that field, or clears it with `None`; fields
    /// not listed keep their value. A document not on the bookshelf yet (a
    /// library folder's document never opened here) is added without
    /// changing when it was last opened. `at_ms` is when the edit was
    /// made. Returns true when anything changed.
    pub fn record_edits(
        &mut self,
        path: &Path,
        edits: &[(DetailField, Option<String>)],
        at_ms: u64,
    ) -> bool {
        let key = resolve_path(path);
        let i = match self.entries.iter().position(|e| e.path == key) {
            Some(i) => i,
            None => {
                self.entries.push(LibraryEntry {
                    path: key,
                    title: String::new(),
                    format: String::new(),
                    added: crate::now_ts(),
                    last_opened: 0,
                    meta: DocMetadata::default(),
                    edited: EditedDetails::default(),
                    extra: serde_json::Map::new(),
                });
                self.entries.len() - 1
            }
        };
        let entry = &mut self.entries[i];
        let before = entry.edited.clone();
        for (field, value) in edits {
            entry.edited.set(*field, value.clone());
        }
        let changed = entry.edited != before;
        if changed {
            entry.edited.at_ms = at_ms;
        }
        changed
    }

    /// Removes `path`. Returns whether it was there.
    pub fn remove(&mut self, path: &Path) -> bool {
        let key = resolve_path(path);
        let before = self.entries.len();
        self.entries.retain(|e| e.path != key);
        self.entries.len() != before
    }
}

/// Where a [`LibraryItem`] came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemSource {
    /// Found in a library folder.
    Folder,
    /// Opened before, outside every library folder (Star's "recent").
    Recent,
}

/// One row of the library view.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryItem {
    /// The document.
    pub path: PathBuf,
    /// Title: the bookshelf title when known, else the file stem.
    pub title: String,
    /// Library folder and relative path, for folder items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<PathBuf>,
    /// Path relative to the folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rel: Option<String>,
    /// Reading progress, when known: the synced sidecar value first, else
    /// the caller's local value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pct: Option<u8>,
    /// When it was last opened on this device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_opened: Option<i64>,
    /// Folder or recent.
    pub source: ItemSource,
    /// Author, DOI, and ISBN, when known (from the bookshelf): the
    /// owner's hand-edited values where there are some.
    #[serde(flatten)]
    pub meta: DocMetadata,
    /// The details the owner typed (W7m), already in `title` and `meta`;
    /// kept so the edit form knows which values are the owner's.
    #[serde(default, skip_serializing_if = "EditedDetails::is_empty")]
    pub edited: EditedDetails,
}

impl LibraryItem {
    /// One line for lists and speech: "Chapter 3, by Ada Example, 42
    /// percent, in Readings". The title comes first, so it is in the first
    /// cells of a Braille line.
    pub fn describe(&self) -> String {
        let mut s = self.title.clone();
        if let Some(a) = &self.meta.author {
            s.push_str(&format!(", by {a}"));
        }
        if let Some(p) = self.pct {
            s.push_str(&format!(", {p} percent"));
        }
        match (&self.folder, self.source) {
            (Some(f), ItemSource::Folder) => {
                let name = f.file_name().map_or_else(
                    || f.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                s.push_str(&format!(", in {name}"));
            }
            _ => s.push_str(", recent"),
        }
        s
    }
}

/// The library view (Star's Library dialog): every document in the library
/// folders, with its synced progress, followed by recently opened
/// documents outside the folders, newest first. `local_pct` supplies a
/// position for documents without a synced one (the per-document state).
pub fn library_view(
    scanned: &[ScannedDoc],
    library: &Library,
    recent: &Recent,
    sidecars: &SidecarStore,
    local_pct: &dyn Fn(&Path) -> Option<u8>,
) -> Vec<LibraryItem> {
    let mut items = Vec::new();
    let mut sidecar_cache: std::collections::HashMap<PathBuf, sync::SidecarMap> =
        std::collections::HashMap::new();
    let mut seen = std::collections::HashSet::new();
    for doc in scanned {
        let side = sidecar_cache
            .entry(doc.folder.clone())
            .or_insert_with(|| sidecars.load(&doc.folder));
        let synced = side
            .get(&doc.rel)
            .and_then(ProgressEntry::from_value)
            .map(|e| e.pct);
        let entry = library.get(&doc.path);
        seen.insert(resolve_path(&doc.path));
        items.push(LibraryItem {
            path: doc.path.clone(),
            title: entry
                .map(|e| e.shown_title().to_owned())
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| doc.title.clone()),
            folder: Some(doc.folder.clone()),
            rel: Some(doc.rel.clone()),
            pct: synced.or_else(|| local_pct(&doc.path)),
            last_opened: entry.map(|e| e.last_opened),
            source: ItemSource::Folder,
            meta: entry.map(LibraryEntry::shown_meta).unwrap_or_default(),
            edited: entry.map(|e| e.edited.clone()).unwrap_or_default(),
        });
    }
    let mut recents: Vec<LibraryItem> = Vec::new();
    for e in &library.entries {
        if seen.insert(e.path.clone()) {
            recents.push(LibraryItem {
                path: e.path.clone(),
                title: if e.shown_title().is_empty() {
                    stem(&e.path)
                } else {
                    e.shown_title().to_owned()
                },
                folder: None,
                rel: None,
                pct: local_pct(&e.path),
                last_opened: Some(e.last_opened),
                source: ItemSource::Recent,
                meta: e.shown_meta(),
                edited: e.edited.clone(),
            });
        }
    }
    for r in &recent.entries {
        if seen.insert(resolve_path(&r.path)) {
            recents.push(LibraryItem {
                path: r.path.clone(),
                title: r.title.clone().unwrap_or_else(|| stem(&r.path)),
                folder: None,
                rel: None,
                pct: local_pct(&r.path),
                last_opened: Some(r.opened),
                source: ItemSource::Recent,
                meta: DocMetadata::default(),
                edited: EditedDetails::default(),
            });
        }
    }
    recents.sort_by_key(|i| std::cmp::Reverse(i.last_opened));
    items.extend(recents);
    items
}

fn stem(p: &Path) -> String {
    p.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.display().to_string())
}

/// True when every word of `query` is in `item`'s title, path, author,
/// DOI, or ISBN, or in `text` (the document's text, when the caller has
/// it), ignoring case (Star's `discovery.py` search). A word that is a DOI
/// or an ISBN matches however it is written: `doi:10.1000/XYZ`, a
/// `doi.org` link, `978-0-306-40615-7`, or `0306406152` for the same
/// book's ISBN-13.
pub fn item_matches(item: &LibraryItem, query: &str, text: Option<&str>) -> bool {
    let hay = format!(
        "{} {} {} {} {}",
        item.title,
        item.path.to_string_lossy(),
        item.meta.author.as_deref().unwrap_or(""),
        item.meta.doi.as_deref().unwrap_or(""),
        item.meta.isbn.as_deref().unwrap_or(""),
    )
    .to_lowercase();
    let text = text.map(str::to_lowercase);
    query.split_whitespace().all(|word| {
        let w = word.to_lowercase();
        if hay.contains(&w) || text.as_deref().is_some_and(|t| t.contains(&w)) {
            return true;
        }
        if let Some(doi) = normalize_doi(word) {
            return item.meta.doi.as_deref() == Some(doi.as_str())
                || text.as_deref().is_some_and(|t| t.contains(&doi));
        }
        if let (Some(isbn), Some(have)) = (normalize_isbn(word), item.meta.isbn.as_deref()) {
            return isbn13(&isbn) == isbn13(have);
        }
        false
    })
}

/// An ISBN ([`normalize_isbn`]'s form) in its 13-digit form, so an ISBN-10
/// and its ISBN-13 compare equal.
pub fn isbn13(isbn: &str) -> String {
    if isbn.len() != 10 {
        return isbn.to_owned();
    }
    let body = format!("978{}", &isbn[..9]);
    let sum: u32 = body
        .bytes()
        .enumerate()
        .map(|(i, c)| u32::from(c - b'0') * if i % 2 == 0 { 1 } else { 3 })
        .sum();
    format!("{body}{}", (10 - sum % 10) % 10)
}

/// Items matching `query` by title, path, author, DOI, or ISBN
/// ([`item_matches`]; the library's filter without document text).
pub fn filter_items<'a>(items: &'a [LibraryItem], query: &str) -> Vec<&'a LibraryItem> {
    items
        .iter()
        .filter(|i| item_matches(i, query, None))
        .collect()
}

/// Where a resumed position came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResumeSource {
    /// This device's saved state.
    Local,
    /// The library folder's sidecar (another device, or this one).
    Sidecar,
}

/// The position to resume at when a document opens.
#[derive(Clone, Debug, PartialEq)]
pub struct Resume {
    /// Char offset.
    pub pos: CharPos,
    /// Percentage through the document.
    pub pct: u8,
    /// When it was saved (Unix seconds; 0 when unknown).
    pub ts: i64,
    /// Where it came from.
    pub source: ResumeSource,
    /// Set when the local and synced positions differed; under the
    /// `manual` policy the local one is used and the user should be told.
    pub conflict: Option<Conflict>,
}

/// Connects per-document state to the library folders' sidecars.
#[derive(Clone, Debug)]
pub struct LibrarySync {
    folders: Vec<PathBuf>,
    sidecars: SidecarStore,
}

impl LibrarySync {
    /// Sync for `folders` under `policy`.
    pub fn new(folders: &[PathBuf], policy: ConflictPolicy) -> Self {
        LibrarySync::with_store(folders, SidecarStore::new(policy))
    }

    /// Sync through an existing sidecar store (shared with other readers,
    /// or with a custom debounce for tests).
    pub fn with_store(folders: &[PathBuf], sidecars: SidecarStore) -> Self {
        LibrarySync {
            folders: folders.to_vec(),
            sidecars,
        }
    }

    /// The sidecar store.
    pub fn sidecars(&self) -> &SidecarStore {
        &self.sidecars
    }

    /// Replaces the folder list (the user added or removed one).
    pub fn set_folders(&mut self, folders: &[PathBuf]) {
        self.folders = folders.to_vec();
    }

    /// The library folder holding `doc` and its sidecar key, or `None`
    /// outside the library.
    pub fn location(&self, doc: &Path) -> Option<(PathBuf, String)> {
        sync::folder_for(&self.folders, doc)
    }

    /// The sidecar's `_meta` entry for a document: the portable reading
    /// stats Star synced (`seconds`, `pct`, `last_ts` from `stats`, when
    /// given) and the number of notes. `None` when there is nothing to say.
    pub fn meta_entry(state: &DocState, stats: Option<&Value>) -> Option<Value> {
        let mut meta = serde_json::Map::new();
        if let Some(Value::Object(s)) = stats {
            for field in ["seconds", "pct", "last_ts"] {
                if let Some(v) = s.get(field) {
                    meta.insert(field.to_owned(), v.clone());
                }
            }
        }
        if !state.notes.is_empty() {
            meta.insert("annotations".to_owned(), Value::from(state.notes.len()));
        }
        (!meta.is_empty()).then_some(Value::Object(meta))
    }

    /// Mirrors `state`'s reading position (and its `_meta` entry) into the
    /// sidecar of the library folder holding `doc`. `Ok(None)` when the
    /// document is outside the library or has no saved position.
    pub fn record(
        &self,
        doc: &Path,
        state: &DocState,
        stats: Option<&Value>,
    ) -> Result<Option<Recorded>, StoreError> {
        if !state.has_position() {
            return Ok(None);
        }
        let Some((folder, rel)) = self.location(doc) else {
            return Ok(None);
        };
        let entry = ProgressEntry::new(state.position, state.pct, state.ts).to_value();
        self.sidecars
            .record_progress(&folder, &rel, entry, Self::meta_entry(state, stats))
            .map(Some)
    }

    /// The synced position of `doc`, if any.
    pub fn synced(&self, doc: &Path) -> Option<ProgressEntry> {
        let (folder, rel) = self.location(doc)?;
        ProgressEntry::from_value(&self.sidecars.progress_for(&folder, &rel)?)
    }

    /// Where to resume `doc`: the local saved position or the synced one,
    /// chosen by the sidecar store's policy (newest, highest progress, or
    /// manual, which keeps local and reports the conflict). `None` when
    /// neither exists.
    pub fn resume(&self, doc: &Path, local: Option<&DocState>) -> Option<Resume> {
        let local_entry = local
            .filter(|s| s.has_position())
            .map(|s| ProgressEntry::new(s.position, s.pct, s.ts));
        let synced = self.synced(doc);
        let key = self
            .location(doc)
            .map_or_else(|| doc.display().to_string(), |(_, rel)| rel);
        let lv = local_entry.as_ref().map(ProgressEntry::to_value);
        let rv = synced.as_ref().map(ProgressEntry::to_value);
        let (winner, conflict) = sync::resolve_entry(
            &key,
            lv.as_ref(),
            rv.as_ref(),
            self.sidecars.policy(),
            Prefer::Local,
        );
        let entry = ProgressEntry::from_value(&winner)?;
        let source = match (&lv, &rv) {
            (Some(l), _) if *l == winner => ResumeSource::Local,
            (_, Some(_)) => ResumeSource::Sidecar,
            _ => ResumeSource::Local,
        };
        Some(Resume {
            pos: entry.offset,
            pct: entry.pct,
            ts: entry.ts_secs().unwrap_or(0),
            source,
            conflict,
        })
    }

    /// Writes every pending sidecar. Call on quit and document switch.
    pub fn flush(&self) -> Result<Vec<Conflict>, StoreError> {
        self.sidecars.flush_all()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn supported(ext: &str) -> bool {
        matches!(ext, "md" | "txt" | "html")
    }

    fn touch(p: &Path, text: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    #[test]
    fn scan_skips_hidden_junk_and_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        touch(&root.join("b.md"), "b");
        touch(&root.join("A.txt"), "a");
        touch(&root.join("image.png"), "x");
        touch(&root.join(".hidden.md"), "h");
        touch(&root.join("sub").join("c.html"), "c");
        touch(&root.join(".git").join("d.md"), "d");
        touch(&root.join("node_modules").join("e.md"), "e");
        touch(&root.join(".textweaver").join("progress.md"), "p");
        let docs = scan_library(&[root.to_owned()], &supported);
        let rels: Vec<&str> = docs.iter().map(|d| d.rel.as_str()).collect();
        assert_eq!(rels, vec!["A.txt", "b.md", "sub/c.html"]);
        assert_eq!(docs[2].title, "c");
        assert_eq!(docs[2].ext, "html");
        let flat = scan_folder(root, false, 10, &supported);
        assert_eq!(flat.len(), 2);
        assert_eq!(scan_folder(root, true, 1, &supported).len(), 1, "max_files");
        assert!(scan_folder(&root.join("missing"), true, 10, &supported).is_empty());
    }

    #[test]
    fn nested_folders_are_deduplicated() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_owned();
        touch(&root.join("sub").join("x.md"), "x");
        let docs = scan_library(&[root.clone(), root.join("sub")], &supported);
        assert_eq!(docs.len(), 1);
    }

    #[test]
    fn folders_add_once_and_remove() {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = LibrarySettings::default();
        let (p, added) = lib.add_folder(dir.path());
        assert!(added);
        assert!(p.is_absolute());
        assert!(!p.to_string_lossy().starts_with(r"\\?\"));
        let (_, again) = lib.add_folder(&dir.path().join("."));
        assert!(!again, "the same folder by another spelling");
        assert_eq!(lib.folders.len(), 1);
        assert!(lib.remove_folder(dir.path()));
        assert!(!lib.remove_folder(dir.path()));
    }

    #[test]
    fn bookshelf_records_moves_to_front_and_evicts() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("library.json");
        let mut lib = Library::load(&file).unwrap();
        lib.record_open_at(&dir.path().join("a.md"), "A", "markdown", 10);
        lib.record_open_at(&dir.path().join("b.md"), "", "text", 20);
        let e = lib.record_open_at(&dir.path().join("a.md"), &"T".repeat(300), "", 30);
        assert_eq!(e.title.chars().count(), TITLE_MAX_CHARS);
        assert_eq!((e.added, e.last_opened), (10, 30));
        assert_eq!(e.format, "markdown", "an empty format keeps the old one");
        assert!(lib.entries[0].path.ends_with("a.md"));
        lib.save(&file).unwrap();
        assert_eq!(Library::load(&file).unwrap(), lib);
        assert!(lib.get(&dir.path().join("b.md")).is_some());
        assert!(lib.remove(&dir.path().join("b.md")));

        let mut big = Library::default();
        for i in 0..=LIBRARY_CAP {
            big.record_open_at(&dir.path().join(format!("{i}.md")), "", "", i as i64);
        }
        assert_eq!(big.entries.len(), LIBRARY_CAP + 1 - LIBRARY_EVICT);
        assert!(
            big.get(&dir.path().join("0.md")).is_none(),
            "oldest evicted"
        );
        assert!(
            big.get(&dir.path().join(format!("{LIBRARY_CAP}.md")))
                .is_some()
        );

        std::fs::write(&file, "not json").unwrap();
        assert!(Library::load(&file).is_err(), "never silently replaced");
    }

    fn state_at(pos: usize, pct: u8, ts: i64) -> DocState {
        DocState {
            position: CharPos(pos),
            pct,
            ts,
            ..DocState::default()
        }
    }

    #[test]
    fn sync_records_and_resumes_by_policy() {
        let dir = tempfile::tempdir().unwrap();
        let folder = resolve_path(dir.path());
        let doc = folder.join("sub").join("book.md");
        touch(&doc, "text");
        let store = SidecarStore::with_debounce(ConflictPolicy::Newest, Duration::ZERO);
        let sync = LibrarySync::with_store(std::slice::from_ref(&folder), store);
        assert_eq!(
            sync.location(&doc),
            Some((folder.clone(), "sub/book.md".to_owned()))
        );

        // Nothing saved anywhere.
        assert!(sync.resume(&doc, None).is_none());
        // Another device saved 80% later than our local 5%.
        let mut remote = state_at(800, 80, 2_000);
        remote.add_note(textweaver_core::CharRange::new(1, 2), "t", "n", "");
        let r = sync.record(&doc, &remote, None).unwrap().unwrap();
        assert!(r.written);
        let meta = sync
            .sidecars()
            .metadata_for(&folder, "sub/book.md")
            .unwrap();
        assert_eq!(meta["annotations"], 1);
        let local = state_at(50, 5, 1_000);
        let resume = sync.resume(&doc, Some(&local)).unwrap();
        assert_eq!((resume.pos, resume.pct), (CharPos(800), 80));
        assert_eq!(resume.source, ResumeSource::Sidecar);
        assert!(resume.conflict.is_some());

        // Local newer: local wins.
        let local = state_at(900, 90, 3_000);
        let resume = sync.resume(&doc, Some(&local)).unwrap();
        assert_eq!(resume.source, ResumeSource::Local);

        // Highest progress: the further position wins even when older.
        sync.sidecars().set_policy(ConflictPolicy::HighestProgress);
        let local = state_at(10, 1, 9_000);
        let resume = sync.resume(&doc, Some(&local)).unwrap();
        assert_eq!(resume.pos, CharPos(800));

        // Manual: keep local, report the conflict.
        sync.sidecars().set_policy(ConflictPolicy::Manual);
        let resume = sync.resume(&doc, Some(&local)).unwrap();
        assert_eq!(resume.pos, CharPos(10));
        assert_eq!(resume.source, ResumeSource::Local);
        assert_eq!(
            resume.conflict.unwrap().resolution,
            sync::Resolution::Unresolved
        );

        // Outside the library: no sidecar, local only.
        let outside = dir.path().parent().unwrap().join("elsewhere.md");
        assert!(sync.record(&outside, &local, None).unwrap().is_none());
        assert_eq!(
            sync.resume(&outside, Some(&local)).unwrap().pos,
            CharPos(10)
        );
        // No saved position: nothing recorded.
        assert!(
            sync.record(&doc, &DocState::default(), None)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn dois_and_isbns_are_recognized_however_written() {
        for s in [
            "10.1000/xyz",
            "doi:10.1000/XYZ",
            "https://doi.org/10.1000/xyz",
            "DOI 10.1000/xyz.",
        ] {
            assert_eq!(normalize_doi(s).as_deref(), Some("10.1000/xyz"), "{s}");
        }
        for s in ["10.10/xyz", "10.1000/", "11.1000/x", "10.1000 /x"] {
            assert_eq!(normalize_doi(s), None, "{s}");
        }
        assert_eq!(
            find_doi("Published as https://doi.org/10.1038/nature12373. Read it.").as_deref(),
            Some("10.1038/nature12373")
        );
        assert_eq!(
            normalize_isbn("978-0-306-40615-7").as_deref(),
            Some("9780306406157")
        );
        assert_eq!(
            normalize_isbn("ISBN 0-306-40615-2").as_deref(),
            Some("0306406152")
        );
        assert_eq!(
            normalize_isbn("urn:isbn:080442957X").as_deref(),
            Some("080442957X")
        );
        assert_eq!(normalize_isbn("978-0-306-40615-8"), None, "check digit");
        assert_eq!(
            find_isbn("Copyright 2020.\nISBN: 978-0-306-40615-7 (paperback)").as_deref(),
            Some("9780306406157")
        );
        assert_eq!(find_isbn("Call 978-0-306-40615-7"), None, "no ISBN label");
        assert_eq!(isbn13("0306406152"), "9780306406157");
    }

    #[test]
    fn metadata_comes_from_properties_then_text() {
        let mut props = std::collections::BTreeMap::new();
        props.insert("citation_doi".to_owned(), "doi:10.5555/ABC".to_owned());
        let m = DocMetadata::from_document(
            Some("  Ada   Example "),
            &props,
            "ISBN 978-0-306-40615-7 and 10.9999/other",
        );
        assert_eq!(m.author.as_deref(), Some("Ada Example"));
        assert_eq!(m.doi.as_deref(), Some("10.5555/abc"), "the property wins");
        assert_eq!(m.isbn.as_deref(), Some("9780306406157"), "from the text");
        let none = DocMetadata::from_document(None, &Default::default(), "Plain text.");
        assert!(none.is_empty());
        let mut a = DocMetadata {
            doi: Some("10.1/x".into()),
            ..DocMetadata::default()
        };
        assert!(a.fill_from(&m));
        assert_eq!(
            a.doi.as_deref(),
            Some("10.1/x"),
            "filling keeps what is known"
        );
        assert_eq!(a.author.as_deref(), Some("Ada Example"));
        assert!(!a.fill_from(&m));
    }

    #[test]
    fn bookshelf_keeps_metadata_and_the_filter_finds_it() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("library.json");
        let paper = dir.path().join("paper.pdf");
        let mut lib = Library::default();
        assert!(
            !lib.record_metadata(&paper, &DocMetadata::default()),
            "no entry yet"
        );
        lib.record_open_at(&paper, "A Paper", "pdf", 5);
        let meta = DocMetadata {
            author: Some("Ada Example".into()),
            doi: Some("10.1000/xyz".into()),
            isbn: Some("0306406152".into()),
        };
        assert!(lib.record_metadata(&paper, &meta));
        assert!(
            !lib.record_metadata(&paper, &DocMetadata::default()),
            "nothing new"
        );
        // Opening again keeps the metadata.
        lib.record_open_at(&paper, "A Paper", "pdf", 6);
        lib.save(&file).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("\"doi\": \"10.1000/xyz\""), "{text}");
        let back = Library::load(&file).unwrap();
        assert_eq!(back, lib);
        assert_eq!(back.entries[0].meta, meta);
        assert!(back.entries[0].extra.is_empty(), "not duplicated in extra");

        let items = library_view(
            &[],
            &back,
            &Recent::default(),
            &SidecarStore::new(ConflictPolicy::Newest),
            &|_| None,
        );
        assert_eq!(items[0].describe(), "A Paper, by Ada Example, recent");
        for q in [
            "10.1000/xyz",
            "https://doi.org/10.1000/XYZ",
            "978-0-306-40615-7",
            "0-306-40615-2",
            "example",
            "paper ada",
        ] {
            assert_eq!(filter_items(&items, q).len(), 1, "{q}");
        }
        for q in ["10.1000/other", "9780306406164", "grace"] {
            assert!(filter_items(&items, q).is_empty(), "{q}");
        }
        assert!(item_matches(
            &items[0],
            "mitochondria",
            Some("The Mitochondria.")
        ));
        assert!(!item_matches(&items[0], "mitochondria", None));
    }

    /// Wave 7 (W7m): hand-edited details win over the document's own,
    /// survive the document opening again, and are found by the filter;
    /// clearing one brings the document's own value back.
    #[test]
    fn hand_edits_win_and_survive_a_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("library.json");
        let paper = dir.path().join("scan0042.pdf");
        let folder_doc = dir.path().join("Readings").join("week1.md");
        let mut lib = Library::default();
        lib.record_open_at(&paper, "scan0042", "pdf", 5);
        lib.record_metadata(
            &paper,
            &DocMetadata {
                author: Some("Scanner Operator".into()),
                ..DocMetadata::default()
            },
        );
        let edits = [
            (
                DetailField::Title,
                Some("Cell Biology, Chapter 3".to_owned()),
            ),
            (DetailField::Author, Some("Ada Example".to_owned())),
            (DetailField::Doi, Some("10.1000/cells".to_owned())),
        ];
        assert!(lib.record_edits(&paper, &edits, 1_000));
        assert!(!lib.record_edits(&paper, &edits, 2_000), "no change");
        // A document never opened here (a library folder's) is added
        // without counting as opened.
        assert!(lib.record_edits(
            &folder_doc,
            &[(DetailField::Isbn, Some("9780306406157".into()))],
            1_500
        ));
        assert_eq!(lib.get(&folder_doc).unwrap().last_opened, 0);
        // Opening again records the document's own details; the edits stay.
        lib.record_open_at(&paper, "scan0042", "pdf", 9);
        lib.record_metadata(
            &paper,
            &DocMetadata {
                author: Some("Scanner Operator".into()),
                ..DocMetadata::default()
            },
        );
        lib.save(&file).unwrap();
        let back = Library::load(&file).unwrap();
        assert_eq!(back, lib);
        let e = back.get(&paper).unwrap();
        assert_eq!(e.title, "scan0042");
        assert_eq!(e.shown_title(), "Cell Biology, Chapter 3");
        assert_eq!(e.edited.at_ms, 1_000);
        assert!(e.extra.is_empty(), "not duplicated in extra");
        let items = library_view(
            &[],
            &back,
            &Recent::default(),
            &SidecarStore::new(ConflictPolicy::Newest),
            &|_| None,
        );
        let item = items
            .iter()
            .find(|i| i.path == resolve_path(&paper))
            .unwrap();
        assert_eq!(
            item.describe(),
            "Cell Biology, Chapter 3, by Ada Example, recent"
        );
        assert_eq!(item.edited.get(DetailField::Doi), Some("10.1000/cells"));
        for q in ["chapter 3", "ada", "doi:10.1000/CELLS"] {
            assert_eq!(filter_items(&items, q).len(), 1, "{q}");
        }
        assert!(filter_items(&items, "scanner").is_empty());

        // Clearing the author shows the document's own again.
        lib.record_edits(&paper, &[(DetailField::Author, None)], 3_000);
        assert_eq!(
            lib.get(&paper).unwrap().shown_meta().author.as_deref(),
            Some("Scanner Operator")
        );
    }

    #[test]
    fn detail_fields_clean_their_values() {
        assert_eq!(
            DetailField::Title.clean("  Cell \n Biology "),
            Ok(Some("Cell Biology".into()))
        );
        assert_eq!(DetailField::Author.clean("   "), Ok(None));
        assert_eq!(
            DetailField::Doi.clean("https://doi.org/10.1000/XYZ"),
            Ok(Some("10.1000/xyz".into()))
        );
        assert_eq!(
            DetailField::Isbn.clean("978-0-306-40615-7"),
            Ok(Some("9780306406157".into()))
        );
        assert_eq!(DetailField::Isbn.clean("12345"), Err("12345".into()));
        assert_eq!(DetailField::Doi.clean("not a doi"), Err("not a doi".into()));
    }

    #[test]
    fn stats_subset_goes_to_meta() {
        let stats =
            serde_json::json!({"seconds": 12.5, "pct": 40, "last_ts": "x", "words_read": 9});
        let meta = LibrarySync::meta_entry(&DocState::default(), Some(&stats)).unwrap();
        assert_eq!(
            meta,
            serde_json::json!({"seconds": 12.5, "pct": 40, "last_ts": "x"})
        );
        assert!(LibrarySync::meta_entry(&DocState::default(), None).is_none());
    }

    #[test]
    fn view_merges_folders_sidecars_and_recents() {
        let dir = tempfile::tempdir().unwrap();
        let folder = resolve_path(&dir.path().join("lib"));
        touch(&folder.join("a.md"), "a");
        touch(&folder.join("b.md"), "b");
        let folder = resolve_path(&folder);
        let outside = resolve_path(dir.path()).join("other.txt");
        touch(&outside, "o");
        let store = SidecarStore::with_debounce(ConflictPolicy::Newest, Duration::ZERO);
        store
            .record_progress(
                &folder,
                "a.md",
                ProgressEntry::new(CharPos(1), 33, 5).to_value(),
                None,
            )
            .unwrap();
        let mut lib = Library::default();
        lib.record_open_at(&folder.join("b.md"), "Book B", "markdown", 7);
        lib.record_open_at(&outside, "Other", "text", 9);
        let mut recent = Recent::default();
        recent.touch(&dir.path().join("gone.md"), Some("Gone".into()), 5);
        let scanned = scan_library(std::slice::from_ref(&folder), &supported);
        let local = |p: &Path| p.ends_with("b.md").then_some(12);
        let items = library_view(&scanned, &lib, &recent, &store, &local);
        let titles: Vec<&str> = items.iter().map(|i| i.title.as_str()).collect();
        assert_eq!(titles, vec!["a", "Book B", "Gone", "Other"]);
        assert_eq!(items[0].pct, Some(33), "sidecar progress");
        assert_eq!(items[1].pct, Some(12), "local progress");
        assert_eq!(items[3].source, ItemSource::Recent);
        assert_eq!(items[0].describe(), "a, 33 percent, in lib");
        assert_eq!(items[3].describe(), "Other, recent");
        assert_eq!(filter_items(&items, "BOOK b").len(), 1);
        assert_eq!(filter_items(&items, "").len(), 4);
    }
}
