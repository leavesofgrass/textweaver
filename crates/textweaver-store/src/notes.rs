//! Notes and highlights: Star's annotations and user highlights
//! (docs/star-parity.md Part 3 §2.3 and §2.4), stored in [`DocState`].
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
//! - one document key for everything (item 14).
//!
//! [`DocState`]: crate::DocState
//! [`EditOutcome`]: textweaver_core::EditOutcome

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
    let canon: String = name
        .trim()
        .chars()
        .map(|c| match c {
            ' ' | '-' => '_',
            c => c.to_ascii_uppercase(),
        })
        .collect();
    RELATION_TYPES.iter().copied().find(|t| *t == canon)
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
    HIGHLIGHT_COLORS
        .iter()
        .find(|(n, hex)| *hex == lower || *n == lower)
        .map_or_else(|| color.trim().to_owned(), |(n, _)| (*n).to_owned())
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

/// A fresh 8-hex-digit id (Star's `uuid4().hex[:8]` shape), unique within
/// a process and very unlikely to collide across devices.
pub fn new_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let seed = format!("{nanos}-{n}-{}", std::process::id());
    for b in seed.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{:08x}", (h ^ (h >> 32)) & 0xffff_ffff)
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
    /// One of [`RELATION_TYPES`].
    pub rel_type: String,
    /// The target note's document key (empty for this document).
    pub target_doc: String,
    /// The target note's id.
    pub target_id: String,
    /// Why the link exists.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// A note attached to a range of the text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    fn colors_and_collapse() {
        assert_eq!(highlight_color("Green"), "#90ee90");
        assert_eq!(highlight_color("#123456"), "#123456");
        assert_eq!(color_name("#FFFF00"), "yellow");
        assert_eq!(color_name("teal"), "teal");
        assert_eq!(collapse("  a \n\t b  ", 120), "a b");
        assert_eq!(collapse("abcdef", 3), "abc");
    }

    #[test]
    fn ids_are_unique_and_stable_ids_repeat() {
        let a = new_id();
        let b = new_id();
        assert_eq!(a.len(), 8);
        assert_ne!(a, b);
        assert_eq!(stable_id("s", &["x", "y"]), stable_id("s", &["x", "y"]));
        assert_ne!(stable_id("s", &["xy", ""]), stable_id("s", &["x", "y"]));
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
