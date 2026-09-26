//! The notes and highlights this crate reads and writes, and the store it
//! reads them from and writes them back to.
//!
//! These are this crate's own types. Agent C2 is adding notes and highlights
//! to `textweaver-store`'s `DocState` in parallel; at integration the
//! orchestrator maps C2's types onto these (or replaces them) and implements
//! [`AnnotationStore`] over the real store. Until then
//! [`StateStoreAnnotations`](crate::StateStoreAnnotations) keeps them in the
//! per-document state file's preserved extra keys.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange};

use crate::VaultError;

/// How one note relates to another (Star's `RELATION_TYPES`,
/// `star/annotations.py:153-164`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationType {
    /// The source conflicts with the target.
    ConflictsWith,
    /// The source supports the target.
    Supports,
    /// The source is an example of the target.
    IsExampleOf,
    /// The source cites the target.
    Cites,
    /// The source contradicts the target.
    Contradicts,
    /// The source defines the target.
    Defines,
    /// The source extends the target.
    Extends,
    /// A plain link: see also the target. The default for untyped links.
    SeeAlso,
    /// The source comes before the target.
    Precedes,
    /// The source comes after the target.
    Follows,
}

impl RelationType {
    /// Every relation type, in Star's order.
    pub const ALL: [RelationType; 10] = [
        RelationType::ConflictsWith,
        RelationType::Supports,
        RelationType::IsExampleOf,
        RelationType::Cites,
        RelationType::Contradicts,
        RelationType::Defines,
        RelationType::Extends,
        RelationType::SeeAlso,
        RelationType::Precedes,
        RelationType::Follows,
    ];

    /// The stored name, as Star wrote it: `SEE_ALSO`.
    pub fn as_str(self) -> &'static str {
        match self {
            RelationType::ConflictsWith => "CONFLICTS_WITH",
            RelationType::Supports => "SUPPORTS",
            RelationType::IsExampleOf => "IS_EXAMPLE_OF",
            RelationType::Cites => "CITES",
            RelationType::Contradicts => "CONTRADICTS",
            RelationType::Defines => "DEFINES",
            RelationType::Extends => "EXTENDS",
            RelationType::SeeAlso => "SEE_ALSO",
            RelationType::Precedes => "PRECEDES",
            RelationType::Follows => "FOLLOWS",
        }
    }

    /// The name as it reads aloud: `see also`.
    pub fn spoken(self) -> &'static str {
        match self {
            RelationType::ConflictsWith => "conflicts with",
            RelationType::Supports => "supports",
            RelationType::IsExampleOf => "is an example of",
            RelationType::Cites => "cites",
            RelationType::Contradicts => "contradicts",
            RelationType::Defines => "defines",
            RelationType::Extends => "extends",
            RelationType::SeeAlso => "see also",
            RelationType::Precedes => "precedes",
            RelationType::Follows => "follows",
        }
    }

    /// Parses a relation name the way Star's `_norm_rel` does: trimmed,
    /// upper-cased, spaces and hyphens turned into underscores, then matched
    /// against the known types. `see also`, `See-Also`, and `SEE_ALSO` all
    /// give [`RelationType::SeeAlso`].
    pub fn parse(name: &str) -> Option<Self> {
        let key: String = name
            .trim()
            .chars()
            .map(|c| if c == ' ' || c == '-' { '_' } else { c })
            .collect::<String>()
            .to_uppercase();
        RelationType::ALL.into_iter().find(|t| t.as_str() == key)
    }
}

impl fmt::Display for RelationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A typed link from one note to another (Star's `relations` entries).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Relation {
    /// The kind of link.
    pub rel_type: RelationType,
    /// The document holding the target note.
    pub target_doc: PathBuf,
    /// The target note's id.
    pub target_id: String,
    /// A comment on the link; empty when there is none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// A note attached to a place in a document (Star's annotation).
///
/// Unlike Star, every note has an id from the start (Star assigned ids
/// lazily, Part 3 §7 item 18), and positions are canonical char offsets
/// rather than rendered-editor positions (item 16).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Note {
    /// Stable id.
    pub id: String,
    /// Where in the document the note is attached.
    pub pos: CharPos,
    /// The text the note is about: the selection or the paragraph, with
    /// whitespace collapsed (Star kept at most 120 chars).
    pub anchor: String,
    /// The note itself.
    pub text: String,
    /// Tags, without a leading `#`.
    pub tags: Vec<String>,
    /// A citation; empty when there is none.
    pub cite: String,
    /// When the note was last changed (Unix seconds, UTC).
    pub ts: i64,
    /// Links to other notes.
    pub relations: Vec<Relation>,
}

/// A highlighted range of a document (Star's `user_highlights` entry).
///
/// Unlike Star, a highlight has an id and a timestamp, so it can sync and
/// round-trip through a vault (Part 3 §7 item 17), and its range is in
/// canonical chars, not rendered-editor offsets.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Highlight {
    /// Stable id.
    pub id: String,
    /// The highlighted chars.
    pub range: CharRange,
    /// The color, as CSS text (Star's default is `#ffff00`).
    pub color: String,
    /// When the highlight was made (Unix seconds, UTC).
    pub ts: i64,
}

/// The default highlight color, Star's `#ffff00`.
pub const DEFAULT_HIGHLIGHT_COLOR: &str = "#ffff00";

/// A spoken name for a highlight color: the common named colors, else the
/// CSS text as written. `#ffff00` reads as "yellow".
pub fn color_name(color: &str) -> String {
    let c = color.trim().to_ascii_lowercase();
    let named = match c.as_str() {
        "#ffff00" | "#ff0" | "yellow" => "yellow",
        "#00ff00" | "#0f0" | "lime" | "green" | "#90ee90" | "lightgreen" => "green",
        "#00ffff" | "#0ff" | "cyan" | "aqua" => "cyan",
        "#ff00ff" | "#f0f" | "magenta" | "fuchsia" => "magenta",
        "#ffc0cb" | "pink" => "pink",
        "#ff0000" | "#f00" | "red" => "red",
        "#0000ff" | "#00f" | "blue" | "#add8e6" | "lightblue" => "blue",
        "#ffa500" | "orange" => "orange",
        _ => return color.trim().to_owned(),
    };
    named.to_owned()
}

/// Everything annotated on one document.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DocAnnotations {
    /// Notes, in any order.
    pub notes: Vec<Note>,
    /// Highlights, in any order.
    pub highlights: Vec<Highlight>,
}

impl DocAnnotations {
    /// True when there are no notes and no highlights.
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty() && self.highlights.is_empty()
    }

    /// The note with `id`.
    pub fn note(&self, id: &str) -> Option<&Note> {
        self.notes.iter().find(|n| n.id == id)
    }

    /// The note with `id`, mutably.
    pub fn note_mut(&mut self, id: &str) -> Option<&mut Note> {
        self.notes.iter_mut().find(|n| n.id == id)
    }

    /// Notes in document order (by position, then id).
    pub fn sorted_notes(&self) -> Vec<&Note> {
        let mut v: Vec<&Note> = self.notes.iter().collect();
        v.sort_by(|a, b| a.pos.cmp(&b.pos).then_with(|| a.id.cmp(&b.id)));
        v
    }

    /// Highlights in document order (by start, then id).
    pub fn sorted_highlights(&self) -> Vec<&Highlight> {
        let mut v: Vec<&Highlight> = self.highlights.iter().collect();
        v.sort_by(|a, b| {
            a.range
                .start
                .cmp(&b.range.start)
                .then_with(|| a.id.cmp(&b.id))
        });
        v
    }
}

/// Where notes and highlights live, and where imported documents are
/// registered. The vault reads and writes only through this, so the app,
/// the CLI, and tests can each plug in their own store.
pub trait AnnotationStore {
    /// The notes and highlights of the document at `doc` (empty when none).
    fn load(&mut self, doc: &Path) -> Result<DocAnnotations, VaultError>;

    /// Replaces the notes and highlights of the document at `doc`.
    fn save(&mut self, doc: &Path, annotations: &DocAnnotations) -> Result<(), VaultError>;

    /// Adds the document at `doc` to the library (Star's `library[path]`
    /// entry: title and format), or refreshes its entry.
    fn register_document(
        &mut self,
        doc: &Path,
        title: &str,
        format: &str,
    ) -> Result<(), VaultError>;
}

/// A library entry recorded by [`MemoryStore`] or
/// [`StateStoreAnnotations`](crate::StateStoreAnnotations).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LibraryEntry {
    /// The document.
    pub path: PathBuf,
    /// Its title.
    pub title: String,
    /// Its format (`markdown` for vault notes).
    pub format: String,
}

/// An in-memory [`AnnotationStore`], for tests and dry runs.
#[derive(Clone, Debug, Default)]
pub struct MemoryStore {
    /// Annotations per document.
    pub docs: BTreeMap<PathBuf, DocAnnotations>,
    /// Registered documents, in registration order, one entry per path.
    pub library: Vec<LibraryEntry>,
}

impl MemoryStore {
    /// An empty store.
    pub fn new() -> Self {
        MemoryStore::default()
    }
}

impl AnnotationStore for MemoryStore {
    fn load(&mut self, doc: &Path) -> Result<DocAnnotations, VaultError> {
        Ok(self.docs.get(doc).cloned().unwrap_or_default())
    }

    fn save(&mut self, doc: &Path, annotations: &DocAnnotations) -> Result<(), VaultError> {
        if annotations.is_empty() {
            self.docs.remove(doc);
        } else {
            self.docs.insert(doc.to_owned(), annotations.clone());
        }
        Ok(())
    }

    fn register_document(
        &mut self,
        doc: &Path,
        title: &str,
        format: &str,
    ) -> Result<(), VaultError> {
        record_library(&mut self.library, doc, title, format);
        Ok(())
    }
}

/// Adds or refreshes a library entry in a list.
pub(crate) fn record_library(list: &mut Vec<LibraryEntry>, doc: &Path, title: &str, format: &str) {
    let entry = LibraryEntry {
        path: doc.to_owned(),
        title: title.to_owned(),
        format: format.to_owned(),
    };
    match list.iter_mut().find(|e| e.path == doc) {
        Some(e) => *e = entry,
        None => list.push(entry),
    }
}

/// A short stable id derived from `seed` (64-bit FNV-1a, 12 hex digits).
///
/// Star assigned random 8-hex-digit ids. Deriving ids from a stable seed
/// (a vault note's path, a highlight's range and color) makes re-imports
/// and re-exports idempotent without remembering anything between runs.
pub fn derive_id(seed: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in seed.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{:012x}", h >> 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relation_names_normalize_like_star() {
        assert_eq!(RelationType::parse("see also"), Some(RelationType::SeeAlso));
        assert_eq!(RelationType::parse("See-Also"), Some(RelationType::SeeAlso));
        assert_eq!(
            RelationType::parse(" supports "),
            Some(RelationType::Supports)
        );
        assert_eq!(
            RelationType::parse("is example of"),
            Some(RelationType::IsExampleOf)
        );
        assert_eq!(RelationType::parse("related"), None);
        assert_eq!(RelationType::parse(""), None);
        for t in RelationType::ALL {
            assert_eq!(RelationType::parse(t.as_str()), Some(t));
        }
    }

    #[test]
    fn relation_serializes_as_star_names() {
        let json = serde_json::to_string(&RelationType::IsExampleOf).unwrap();
        assert_eq!(json, "\"IS_EXAMPLE_OF\"");
    }

    #[test]
    fn derived_ids_are_stable_and_short() {
        let a = derive_id("/vault/a.md");
        assert_eq!(a, derive_id("/vault/a.md"));
        assert_ne!(a, derive_id("/vault/b.md"));
        assert_eq!(a.len(), 12);
    }

    #[test]
    fn colors_read_as_names() {
        assert_eq!(color_name("#FFFF00"), "yellow");
        assert_eq!(color_name("#123456"), "#123456");
    }

    #[test]
    fn memory_store_round_trips_and_forgets_empty() {
        let mut s = MemoryStore::new();
        let p = Path::new("/d.md");
        let mut a = DocAnnotations::default();
        a.notes.push(Note {
            id: "x".into(),
            ..Note::default()
        });
        s.save(p, &a).unwrap();
        assert_eq!(s.load(p).unwrap(), a);
        s.save(p, &DocAnnotations::default()).unwrap();
        assert!(s.docs.is_empty());
        s.register_document(p, "D", "markdown").unwrap();
        s.register_document(p, "D2", "markdown").unwrap();
        assert_eq!(s.library.len(), 1);
        assert_eq!(s.library[0].title, "D2");
    }
}
