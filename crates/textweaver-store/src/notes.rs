//! Notes and highlights: Star's annotations and user highlights
//! (the Star parity reference Part 3 §2.3 and §2.4), stored in [`DocState`].
//!
//! Changes from Star, all deliberate:
//!
//! - notes and highlights are anchored to a [`CharRange`] of the canonical
//!   text and move across edits with the same [`EditOutcome`] as bookmarks
//!   (Star stored a rendered-editor offset and a word index that edits and
//!   display transforms invalidated, §7 items 16 and 17);
//! - every note and highlight gets an id and a timestamp when it is created,
//!   so sidecar merges can match them by id (Star assigned note ids lazily
//!   and highlights never had one, items 17 and 18);
//! - timestamps are Unix seconds, UTC (item 19);
//! - one document key for everything (item 14);
//! - new ids are 64 bits (16 hex digits, [`new_id`]) so notes made on
//!   several computers do not collide; older 8-digit ids load and stay as
//!   they are.
//!
//! [`DocState`]: crate::DocState
//! [`EditOutcome`]: textweaver_core::EditOutcome

use std::fmt;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange, EditOutcome};

/// The relation types a note can have to another note (Star's
/// `RELATION_TYPES`, used by the knowledge graph and Obsidian links).
pub const RELATION_TYPES: [&str; 10] = [
    "CONFLICTS_WITH",
    "SUPPORTS",
    "IS_EXAMPLE_OF",
    "CITES",
    "CONTRADICTS",
    "DEFINES",
    "EXTENDS",
    "SEE_ALSO",
    "PRECEDES",
    "FOLLOWS",
];

/// The relation used for a link without a type (Star's default).
pub const DEFAULT_RELATION: &str = "SEE_ALSO";

/// How one note relates to another (Star's `RELATION_TYPES`,
/// `star/annotations.py:153-164`). [`Relation::rel_type`] keeps the name
/// as a string, so a type from a newer version survives a round trip;
/// [`Relation::relation_type`] parses it.
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

/// Star's default highlight color.
pub const DEFAULT_HIGHLIGHT_COLOR: &str = "#ffff00";

/// The named highlight colors Star offered (Ctrl+Shift+1 to 5), as
/// `(name, #rrggbb)`.
pub const HIGHLIGHT_COLORS: [(&str, &str); 5] = [
    ("yellow", "#ffff00"),
    ("green", "#90ee90"),
    ("cyan", "#00ffff"),
    ("pink", "#ffc0cb"),
    ("orange", "#ffa500"),
];

/// Longest anchor kept with a note, in chars (Star: 120).
pub const ANCHOR_MAX_CHARS: usize = 120;

/// Longest highlighted text kept with a highlight, in chars.
pub const HIGHLIGHT_TEXT_MAX_CHARS: usize = 500;

/// A relation type in canonical form: uppercase, with spaces and hyphens
/// turned into `_` (Star's normalization for Dataview fields). `None` when
/// it is not one of [`RELATION_TYPES`].
pub fn normalize_relation(name: &str) -> Option<&'static str> {
    RelationType::parse(name).map(RelationType::as_str)
}

/// The `#rrggbb` form of a named highlight color, or the input unchanged.
pub fn highlight_color(name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase();
    HIGHLIGHT_COLORS
        .iter()
        .find(|(n, _)| *n == lower)
        .map_or_else(|| name.trim().to_owned(), |(_, hex)| (*hex).to_owned())
}

/// The spoken name of a highlight color: "yellow" for `#ffff00`, else the
/// value as stored.
pub fn color_name(color: &str) -> String {
    let lower = color.trim().to_ascii_lowercase();
    if let Some((n, _)) = HIGHLIGHT_COLORS
        .iter()
        .find(|(n, hex)| *hex == lower || *n == lower)
    {
        return (*n).to_owned();
    }
    let named = match lower.as_str() {
        "#ff0" => "yellow",
        "#00ff00" | "#0f0" | "lime" | "lightgreen" => "green",
        "#0ff" | "aqua" => "cyan",
        "#ff00ff" | "#f0f" | "magenta" | "fuchsia" => "magenta",
        "#ff0000" | "#f00" | "red" => "red",
        "#0000ff" | "#00f" | "blue" | "#add8e6" | "lightblue" => "blue",
        _ => return color.trim().to_owned(),
    };
    named.to_owned()
}

/// Splits a tag string on commas and whitespace, dropping a leading `#` and
/// empty parts (Star's `_parse_tags`).
pub fn parse_tags(raw: &str) -> Vec<String> {
    raw.split(|c: char| c == ',' || c.is_whitespace())
        .map(|p| p.trim().trim_start_matches('#').trim())
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Collapses runs of whitespace to one space, trims, and keeps at most
/// `max` chars (Star's anchor rule).
pub fn collapse(text: &str, max: usize) -> String {
    let mut out = String::new();
    for word in text.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    if out.chars().count() > max {
        out = out
            .chars()
            .take(max)
            .collect::<String>()
            .trim_end()
            .to_owned();
    }
    out
}

/// A fresh 64-bit id as 16 hex digits, for notes, highlights, and
/// bookmarks.
///
/// Ids were 8 hex digits (32 bits, Star's `uuid4().hex[:8]` shape) until
/// the sync wave; with notes arriving from other computers, 32 bits made a
/// collision plausible in a large library. Old ids are kept as they are:
/// they are 8 digits long, so a new id can never equal one.
///
/// Never repeats within a process: the id is a bijective mix of a random
/// per-process seed plus a counter. Across computers, two ids collide only
/// if two random 64-bit seeds happen to line up.
pub fn new_id() -> String {
    format!("{:016x}", next_id64())
}

/// The next 64-bit id of this process ([`new_id`]).
fn next_id64() -> u64 {
    static SEED: OnceLock<u64> = OnceLock::new();
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seed = *SEED.get_or_init(process_seed);
    id_from_seed(seed, COUNTER.fetch_add(1, Ordering::Relaxed))
}

/// A random seed for this process: the standard library's randomly keyed
/// hasher (seeded by the operating system) over the time and process id.
fn process_seed() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(nanos);
    h.write_u32(std::process::id());
    h.finish()
}

/// The `n`th id from `seed`. Every step is a bijection on `u64` (adding an
/// odd multiple, then the SplitMix64 finalizer), so different `n` under one
/// seed never give the same id.
pub(crate) fn id_from_seed(seed: u64, n: u64) -> u64 {
    let mut z = seed.wrapping_add(n.wrapping_mul(0x9e37_79b9_7f4a_7c15));
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// A stable 64-bit id derived from `parts` (16 hex digits after `prefix`),
/// for items that are given an id when an older file is read, so reading
/// the same file twice gives the same ids.
pub fn stable_id64(prefix: &str, parts: &[&str]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for b in part.as_bytes().iter().chain(std::iter::once(&0u8)) {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{prefix}{:016x}", id_from_seed(h, 0))
}

/// A stable id derived from `parts`, for imported items that had none, so
/// importing the same data twice gives the same ids.
pub fn stable_id(prefix: &str, parts: &[&str]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for b in part.as_bytes().iter().chain(std::iter::once(&0u8)) {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{prefix}{:08x}", (h ^ (h >> 32)) & 0xffff_ffff)
}

/// A typed link from one note to another (Star's `relations` entries).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Relation {
    /// One of [`RELATION_TYPES`] ([`Relation::relation_type`] parses it).
    pub rel_type: String,
    /// The target note's document key (empty for this document).
    pub target_doc: String,
    /// The target note's id.
    pub target_id: String,
    /// Why the link exists.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
}

impl Relation {
    /// The relation's type, when it is one textweaver knows.
    pub fn relation_type(&self) -> Option<RelationType> {
        RelationType::parse(&self.rel_type)
    }
}

/// A note attached to a range of the text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Note {
    /// Stable id, assigned on creation.
    pub id: String,
    /// The text the note is about; an empty range marks a point.
    pub range: CharRange,
    /// The anchored text when the note was made: whitespace collapsed, at
    /// most [`ANCHOR_MAX_CHARS`] chars. It keeps the note meaningful if the
    /// text later changes.
    pub anchor: String,
    /// The note itself.
    pub note: String,
    /// Tags, without `#`.
    pub tags: Vec<String>,
    /// A citation key or reference, if any.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cite: String,
    /// A color, when the note is also shown as a highlight.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Links to other notes.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<Relation>,
    /// When the note was created (Unix seconds, UTC).
    pub created: i64,
    /// When the note was last changed (Unix seconds, UTC).
    pub ts: i64,
    /// Unknown fields (Star's `sr_state` and anything newer), preserved.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Default for Note {
    fn default() -> Self {
        Note {
            id: String::new(),
            range: CharRange::default(),
            anchor: String::new(),
            note: String::new(),
            tags: Vec::new(),
            cite: String::new(),
            color: None,
            relations: Vec::new(),
            created: 0,
            ts: 0,
            extra: serde_json::Map::new(),
        }
    }
}

impl Note {
    /// True when the note matches a search query (Star's
    /// `_annotation_matches`): every space-separated term must match,
    /// case-insensitively; `#term` matches a tag containing `term`; any
    /// other term must occur in the note, the anchor, or a tag. An empty
    /// query matches every note.
    pub fn matches(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return true;
        }
        let note = self.note.to_lowercase();
        let anchor = self.anchor.to_lowercase();
        let tags: Vec<String> = self.tags.iter().map(|t| t.to_lowercase()).collect();
        q.split_whitespace()
            .all(|term| match term.strip_prefix('#') {
                Some(tag) if !tag.is_empty() => tags.iter().any(|t| t.contains(tag)),
                _ => {
                    note.contains(term)
                        || anchor.contains(term)
                        || tags.iter().any(|t| t.contains(term))
                }
            })
    }

    /// A one-line description for lists and speech: the note, then the
    /// anchor in quotes, then the tags.
    pub fn summary(&self) -> String {
        let mut s = if self.note.is_empty() {
            "Empty note".to_owned()
        } else {
            collapse(&self.note, 200)
        };
        if !self.anchor.is_empty() {
            s.push_str(&format!(", on \u{201c}{}\u{201d}", self.anchor));
        }
        if !self.tags.is_empty() {
            s.push_str(&format!(", tagged {}", self.tags.join(", ")));
        }
        s
    }
}

/// A highlighted range of the text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Highlight {
    /// Stable id, assigned on creation.
    pub id: String,
    /// The highlighted text's range.
    pub range: CharRange,
    /// Color, `#rrggbb` or a CSS name (Star default `#ffff00`).
    pub color: String,
    /// The highlighted text when it was made (whitespace collapsed, at most
    /// [`HIGHLIGHT_TEXT_MAX_CHARS`] chars), for lists and exports.
    pub text: String,
    /// When it was made (Unix seconds, UTC).
    pub ts: i64,
    /// Unknown fields, preserved.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Default for Highlight {
    fn default() -> Self {
        Highlight {
            id: String::new(),
            range: CharRange::default(),
            color: DEFAULT_HIGHLIGHT_COLOR.to_owned(),
            text: String::new(),
            ts: 0,
            extra: serde_json::Map::new(),
        }
    }
}

/// A note or a highlight, for stepping through both in document order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Annotation<'a> {
    /// A note.
    Note(&'a Note),
    /// A highlight.
    Highlight(&'a Highlight),
}

impl Annotation<'_> {
    /// The annotated range.
    pub fn range(&self) -> CharRange {
        match self {
            Annotation::Note(n) => n.range,
            Annotation::Highlight(h) => h.range,
        }
    }

    /// The id.
    pub fn id(&self) -> &str {
        match self {
            Annotation::Note(n) => &n.id,
            Annotation::Highlight(h) => &h.id,
        }
    }

    /// What to say when the cursor reaches it: "Note: ..." or "Yellow
    /// highlight: ...".
    pub fn spoken(&self) -> String {
        match self {
            Annotation::Note(n) => format!("Note: {}", n.summary()),
            Annotation::Highlight(h) => {
                let mut name = color_name(&h.color);
                if let Some(first) = name.get(..1) {
                    name = format!("{}{}", first.to_uppercase(), &name[1..]);
                }
                if h.text.is_empty() {
                    format!("{name} highlight")
                } else {
                    format!("{name} highlight: {}", collapse(&h.text, 200))
                }
            }
        }
    }
}

/// Moves notes and highlights across an edit. Notes whose text was deleted
/// become point notes at the edit (their anchor keeps the old text);
/// highlights whose text was deleted are dropped.
pub(crate) fn shift(notes: &mut [Note], highlights: &mut Vec<Highlight>, outcome: &EditOutcome) {
    for n in notes.iter_mut() {
        n.range = outcome.map_range(n.range);
    }
    notes.sort_by_key(|n| (n.range.start, n.range.end));
    for h in highlights.iter_mut() {
        h.range = outcome.map_range(h.range);
    }
    highlights.retain(|h| !h.range.is_empty());
    highlights.sort_by_key(|h| (h.range.start, h.range.end));
}

/// Options for [`notes_markdown`].
#[derive(Clone, Debug, Default)]
pub struct NotesExport<'a> {
    /// Document title.
    pub title: &'a str,
    /// Author, if known.
    pub author: Option<&'a str>,
    /// Source file or URL.
    pub source: Option<&'a str>,
    /// Export time (Unix seconds); the date is written when given.
    pub exported: Option<i64>,
    /// Document length in chars, to give each note its percentage.
    pub doc_len: Option<usize>,
    /// Include highlights after the notes.
    pub highlights: bool,
}

/// Quotes `text` as a Markdown block quote, line by line.
fn block_quote(text: &str) -> String {
    text.lines()
        .map(|l| {
            if l.is_empty() {
                ">".to_owned()
            } else {
                format!("> {l}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Notes (and optionally highlights) as Markdown, after Star's default
/// export: a title, a short header, then one section per note with the
/// anchored text quoted. Written to read well aloud: no decorative symbols,
/// dates in words-friendly `YYYY-MM-DD` form. Agent J's vault export reuses
/// [`Note`] directly; this is the single-file export.
pub fn notes_markdown(notes: &[Note], highlights: &[Highlight], opts: &NotesExport<'_>) -> String {
    let mut md = vec![format!("# Notes: {}", opts.title), String::new()];
    if let Some(a) = opts.author.filter(|a| !a.is_empty()) {
        md.push(format!("- Author: {a}"));
    }
    if let Some(s) = opts.source.filter(|s| !s.is_empty()) {
        md.push(format!("- Source: `{s}`"));
    }
    if let Some(ts) = opts.exported {
        let date: String = crate::time::rfc3339(ts).chars().take(10).collect();
        md.push(format!("- Exported: {date}"));
    }
    md.push(format!("- Notes: {}", notes.len()));
    if opts.highlights {
        md.push(format!("- Highlights: {}", highlights.len()));
    }
    md.push(String::new());
    for (i, n) in notes.iter().enumerate() {
        md.push(format!("## Note {}", i + 1));
        md.push(String::new());
        if !n.anchor.is_empty() {
            md.push(block_quote(&n.anchor));
            md.push(String::new());
        }
        if !n.note.trim().is_empty() {
            md.push(n.note.trim().to_owned());
            md.push(String::new());
        }
        let mut facts = Vec::new();
        if let Some(len) = opts.doc_len {
            facts.push(format!("at {} percent", crate::percent(n.range.start, len)));
        }
        if !n.tags.is_empty() {
            facts.push(format!(
                "tags {}",
                n.tags
                    .iter()
                    .map(|t| format!("#{t}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
        if !n.cite.is_empty() {
            facts.push(format!("cites {}", n.cite));
        }
        if n.ts > 0 {
            facts.push(format!("saved {}", crate::time::human(n.ts)));
        }
        if !facts.is_empty() {
            let mut line = facts.join(", ");
            if let Some(first) = line.get(..1) {
                line = format!("{}{}", first.to_uppercase(), &line[1..]);
            }
            md.push(format!("*{line}*"));
            md.push(String::new());
        }
    }
    if opts.highlights && !highlights.is_empty() {
        md.push("## Highlights".to_owned());
        md.push(String::new());
        for h in highlights {
            let mut line = format!("- {}", color_name(&h.color));
            if let Some(len) = opts.doc_len {
                line.push_str(&format!(
                    ", at {} percent",
                    crate::percent(h.range.start, len)
                ));
            }
            if !h.text.is_empty() {
                line.push_str(&format!(": {}", h.text));
            }
            md.push(line);
        }
        md.push(String::new());
    }
    let mut out = md.join("\n");
    while out.ends_with("\n\n") {
        out.pop();
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// The first item after `pos` in document order, wrapping when asked.
pub(crate) fn step<'a>(
    items: &[Annotation<'a>],
    pos: CharPos,
    forward: bool,
    wrap: bool,
) -> Option<Annotation<'a>> {
    if forward {
        items
            .iter()
            .find(|a| a.range().start > pos)
            .or_else(|| if wrap { items.first() } else { None })
            .copied()
    } else {
        items
            .iter()
            .rev()
            .find(|a| a.range().start < pos)
            .or_else(|| if wrap { items.last() } else { None })
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_parse_like_star() {
        assert_eq!(parse_tags("#a, b  #c,,"), vec!["a", "b", "c"]);
        assert_eq!(parse_tags("  "), Vec::<String>::new());
        assert_eq!(parse_tags("#"), Vec::<String>::new());
    }

    #[test]
    fn matching_follows_star() {
        let n = Note {
            note: "Mitochondria make energy".into(),
            anchor: "the powerhouse of the cell".into(),
            tags: vec!["biology".into(), "exam".into()],
            ..Note::default()
        };
        assert!(n.matches(""));
        assert!(n.matches("ENERGY cell"));
        assert!(n.matches("#bio"));
        assert!(n.matches("exam"), "plain terms also search tags");
        assert!(!n.matches("#energy"), "tag terms search tags only");
        assert!(!n.matches("energy chemistry"), "terms are ANDed");
    }

    #[test]
    fn relations_normalize() {
        assert_eq!(normalize_relation("see also"), Some("SEE_ALSO"));
        assert_eq!(normalize_relation("is-example-of"), Some("IS_EXAMPLE_OF"));
        assert_eq!(normalize_relation("likes"), None);
    }

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
    fn colors_and_collapse() {
        assert_eq!(highlight_color("Green"), "#90ee90");
        assert_eq!(highlight_color("#123456"), "#123456");
        assert_eq!(color_name("#FFFF00"), "yellow");
        assert_eq!(color_name("teal"), "teal");
        assert_eq!(color_name("#00FF00"), "green");
        assert_eq!(color_name("#123456"), "#123456");
        assert_eq!(collapse("  a \n\t b  ", 120), "a b");
        assert_eq!(collapse("abcdef", 3), "abc");
    }

    #[test]
    fn ids_are_unique_and_stable_ids_repeat() {
        let a = new_id();
        let b = new_id();
        assert_eq!(a.len(), 16);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
        assert_eq!(stable_id("s", &["x", "y"]), stable_id("s", &["x", "y"]));
        assert_ne!(stable_id("s", &["xy", ""]), stable_id("s", &["x", "y"]));
        // Imports keep their 8-digit form, so importing twice matches.
        assert_eq!(stable_id("star-", &["a"]).len(), "star-".len() + 8);
        let s64 = stable_id64("bm-", &["mark1", "3", "5"]);
        assert_eq!(s64, stable_id64("bm-", &["mark1", "3", "5"]));
        assert_eq!(s64.len(), "bm-".len() + 16);
        assert_ne!(s64, stable_id64("bm-", &["mark1", "3", "6"]));
    }

    /// The owner's check for the sync wave: a million new ids, no repeat.
    #[test]
    fn a_million_ids_never_collide() {
        let mut seen = std::collections::HashSet::with_capacity(1_000_000);
        for _ in 0..1_000_000 {
            assert!(seen.insert(new_id()), "a new id repeated");
        }
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig::with_cases(32))]

        /// Two computers (two random seeds) making ids side by side: never
        /// the same id twice, and every id is 16 hex digits.
        #[test]
        fn ids_from_two_seeds_never_collide(a in proptest::prelude::any::<u64>(), b in proptest::prelude::any::<u64>(), start in 0u64..u64::MAX / 2) {
            let mut seen = std::collections::HashSet::new();
            for n in start..start + 5_000 {
                proptest::prop_assert!(seen.insert(id_from_seed(a, n)));
                if a != b {
                    proptest::prop_assert!(seen.insert(id_from_seed(b, n)));
                }
            }
            proptest::prop_assert_eq!(format!("{:016x}", id_from_seed(a, start)).len(), 16);
        }
    }

    #[test]
    fn spoken_forms() {
        let h = Highlight {
            color: "#ffff00".into(),
            text: "key idea".into(),
            ..Highlight::default()
        };
        assert_eq!(
            Annotation::Highlight(&h).spoken(),
            "Yellow highlight: key idea"
        );
        let n = Note {
            note: "check this".into(),
            anchor: "claim".into(),
            tags: vec!["todo".into()],
            ..Note::default()
        };
        assert_eq!(
            Annotation::Note(&n).spoken(),
            "Note: check this, on \u{201c}claim\u{201d}, tagged todo"
        );
    }
}
