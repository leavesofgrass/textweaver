//! The log view's model: entries made from a source's JSON, matched,
//! corrected in place, and rendered as Markdown, as a list (one entry per
//! line, meaning first) or a table with real headers.
//!
//! Three message shapes are understood ([`Shape::of`]):
//!
//! - one JSON object adds an entry at the end, or updates the earlier
//!   entry it matches;
//! - a JSON list is the whole log: it replaces the entries;
//! - an object wrapping one list (under `checkins`, `entries`, `log`,
//!   `stations`, `items` or `records`, or its only list of objects) is the
//!   whole log too.
//!
//! An entry matches an earlier one by its id field (the source's
//! configured `id_field`, else the first of `id`, `uuid`, `serial`,
//! `serialno`, `seq`) or, with none, by its callsign, compared without
//! spaces and case. Common field names are recognized with no setup
//! ([`FIELD_GROUPS`]); fields not recognized are kept and shown after the
//! known ones, so nothing is dropped.

use serde_json::{Map, Value};
use textweaver_core::{CharPos, CharRange, MarkerKind};
use textweaver_text::Document;

/// An entry's fields, as a JSON object.
pub type Fields = Map<String, Value>;

/// How a log is shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LogView {
    /// One entry per line, meaning first.
    #[default]
    List,
    /// A table with real headers.
    Table,
}

impl LogView {
    /// The other view.
    pub fn other(self) -> LogView {
        match self {
            LogView::List => LogView::Table,
            LogView::Table => LogView::List,
        }
    }
}

/// A field the log view knows, in display order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    /// A callsign.
    Callsign,
    /// A person's name.
    Name,
    /// Where they are.
    Location,
    /// Their status (mobile, short time).
    Status,
    /// Whether they have traffic.
    Traffic,
    /// Notes or remarks.
    Notes,
    /// A time the source sent.
    Time,
}

/// The names recognized for an entry's id, first match first. The id is
/// used for matching, not shown.
pub const ID_NAMES: &[&str] = &["id", "uuid", "serial", "serialno", "seq"];

/// The field names recognized with no setup, compared without case, in
/// display order. Within a group the first name present wins; another
/// name of the same group is shown as an unknown field.
pub const FIELD_GROUPS: &[(Field, &[&str])] = &[
    (
        Field::Callsign,
        &["callsign", "call", "call_sign", "cs", "col_call"],
    ),
    (
        Field::Name,
        &[
            "name",
            "first_name",
            "firstname",
            "preferredname",
            "operator",
            "op",
        ],
    ),
    (
        Field::Location,
        &[
            "location",
            "loc",
            "qth",
            "city",
            "citycountry",
            "state",
            "county",
            "grid",
            "gridsquare",
        ],
    ),
    (
        Field::Status,
        &["status", "state_of_checkin", "checkin_type", "role"],
    ),
    (
        Field::Traffic,
        &["traffic", "has_traffic", "msgs", "messages"],
    ),
    (
        Field::Notes,
        &[
            "notes", "note", "remarks", "comment", "comments", "text", "message",
        ],
    ),
    (
        Field::Time,
        &[
            "time",
            "timestamp",
            "ts",
            "utc",
            "checked_in_at",
            "time_on",
            "col_time_on",
        ],
    ),
];

/// Keys whose value is the whole log, in an object wrapping it.
const WRAPPER_NAMES: &[&str] = &["checkins", "entries", "log", "stations", "items", "records"];

/// The most entries a log keeps; more are left out, and said.
pub const MAX_ENTRIES: usize = 20_000;

/// The most fields an entry keeps.
const MAX_FIELDS: usize = 64;

/// The longest value shown, in chars.
const MAX_VALUE_CHARS: usize = 500;

/// The words a log is rendered with, in the interface's language
/// ([`App::live_labels`](crate::App::live_labels)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogLabels {
    /// Shown when the log has no entries yet.
    pub empty: String,
    /// An entry with no fields.
    pub empty_entry: String,
    /// Column headers: callsign, name, location, status, traffic, notes,
    /// time.
    pub headers: [String; 7],
    /// An entry with traffic, in the list.
    pub traffic: String,
    /// True and false values.
    pub yes: String,
    /// See [`yes`](Self::yes).
    pub no: String,
}

impl Default for LogLabels {
    /// The English words (tests).
    fn default() -> Self {
        LogLabels {
            empty: "No entries yet.".into(),
            empty_entry: "Empty entry.".into(),
            headers: [
                "Callsign", "Name", "Location", "Status", "Traffic", "Notes", "Time",
            ]
            .map(String::from),
            traffic: "Traffic".into(),
            yes: "yes".into(),
            no: "no".into(),
        }
    }
}

impl LogLabels {
    fn header(&self, f: Field) -> &str {
        let i = FIELD_GROUPS
            .iter()
            .position(|(g, _)| *g == f)
            .unwrap_or_default();
        &self.headers[i]
    }
}

/// One entry: its fields as received, and when.
#[derive(Clone, Debug, PartialEq)]
pub struct LogEntry {
    /// The fields, as the source sent them.
    pub fields: Map<String, Value>,
    /// When it first arrived (RFC 3339, UTC).
    pub received: String,
    /// When it was last corrected, if it was.
    pub updated: Option<String>,
}

/// What a message is.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// One entry.
    One(Map<String, Value>),
    /// The whole log.
    Whole(Vec<Map<String, Value>>),
}

impl Shape {
    /// The shape of a JSON message: a list, or an object wrapping one, is
    /// the whole log; any other object is one entry; a bare value is an
    /// entry with one field, `value`.
    pub fn of(value: Value) -> Shape {
        match value {
            Value::Array(items) => Shape::Whole(items.into_iter().map(as_object).collect()),
            Value::Object(mut map) => match wrapped_list(&map) {
                Some(key) => match map.remove(&key) {
                    Some(Value::Array(items)) => {
                        Shape::Whole(items.into_iter().map(as_object).collect())
                    }
                    _ => Shape::One(map),
                },
                None => Shape::One(map),
            },
            other => Shape::One(as_object(other)),
        }
    }
}

/// The key of the list an object wraps: one of [`WRAPPER_NAMES`] holding a
/// list, else its only value that is a list of objects, when the object
/// has no field of an entry (a callsign or a name).
fn wrapped_list(map: &Map<String, Value>) -> Option<String> {
    let named = map
        .iter()
        .find(|(k, v)| v.is_array() && WRAPPER_NAMES.contains(&k.to_lowercase().as_str()));
    if let Some((k, _)) = named {
        return Some(k.clone());
    }
    let lists: Vec<&String> = map
        .iter()
        .filter(|(_, v)| {
            v.as_array()
                .is_some_and(|a| !a.is_empty() && a.iter().all(Value::is_object))
        })
        .map(|(k, _)| k)
        .collect();
    let entry_like = [Field::Callsign, Field::Name]
        .iter()
        .any(|f| find_field(map, *f).is_some());
    match lists.as_slice() {
        [one] if !entry_like => Some((*one).clone()),
        _ => None,
    }
}

fn as_object(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        other => {
            let mut m = Map::new();
            m.insert("value".into(), other);
            m
        }
    }
}

/// An object from string pairs: a convenience for tests and examples.
pub fn entry_object(pairs: &[(&str, &str)]) -> Map<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), Value::String((*v).to_owned())))
        .collect()
}

/// The key of `map` that is field `f`, first name first.
fn find_field(map: &Map<String, Value>, f: Field) -> Option<&String> {
    let (_, names) = FIELD_GROUPS.iter().find(|(g, _)| *g == f)?;
    names
        .iter()
        .find_map(|n| map.keys().find(|k| k.eq_ignore_ascii_case(n)))
}

/// Options a source gives its log.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LogOptions {
    /// The log's title (the source's name).
    pub title: String,
    /// The field that identifies an entry, when the source names one.
    pub id_field: Option<String>,
}

/// The log view's entries.
#[derive(Clone, Debug, Default)]
pub struct LiveLog {
    /// Title and id field.
    pub options: LogOptions,
    entries: Vec<LogEntry>,
    /// Entries left out at [`MAX_ENTRIES`].
    pub dropped: usize,
}

/// An entry's fields sorted for showing.
struct Shown {
    known: Vec<(Field, String)>,
    unknown: Vec<(String, String)>,
}

impl LiveLog {
    /// An empty log.
    pub fn new(options: LogOptions) -> Self {
        LiveLog {
            options,
            entries: Vec::new(),
            dropped: 0,
        }
    }

    /// A log holding `entries` (a saved log reopened).
    pub fn with_entries(options: LogOptions, entries: Vec<LogEntry>) -> Self {
        LiveLog {
            options,
            entries,
            dropped: 0,
        }
    }

    /// The entries, in order.
    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    /// How many entries there are.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True with no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The key an entry is matched by: its id, else its callsign; `None`
    /// with neither.
    pub fn key(&self, fields: &Map<String, Value>) -> Option<String> {
        let id_key = match &self.options.id_field {
            Some(f) => fields.keys().find(|k| k.eq_ignore_ascii_case(f)),
            None => None,
        }
        .or_else(|| {
            ID_NAMES
                .iter()
                .find_map(|n| fields.keys().find(|k| k.eq_ignore_ascii_case(n)))
        });
        if let Some(k) = id_key {
            let v = plain(&fields[k]);
            if !v.is_empty() {
                return Some(format!("id:{v}"));
            }
        }
        let call = find_field(fields, Field::Callsign).map(|k| plain(&fields[k]))?;
        let call: String = call
            .chars()
            .filter(|c| !c.is_whitespace())
            .flat_map(char::to_uppercase)
            .collect();
        (!call.is_empty()).then(|| format!("call:{call}"))
    }

    /// The key of entry `i`.
    pub fn key_of(&self, i: usize) -> Option<String> {
        self.entries.get(i).and_then(|e| self.key(&e.fields))
    }

    /// The index of the entry with key `key`.
    pub fn find_key(&self, key: &str) -> Option<usize> {
        self.entries
            .iter()
            .position(|e| self.key(&e.fields).as_deref() == Some(key))
    }

    /// Applies a message: one entry added or updated, or the whole log
    /// replaced. Returns the indices of the entries added or changed, in
    /// order (empty when nothing changed). `now` is the arrival time.
    pub fn apply(&mut self, shape: Shape, now: &str) -> Vec<usize> {
        match shape {
            Shape::One(fields) => self.upsert(fields, now).into_iter().collect(),
            Shape::Whole(items) => self.replace(items, now),
        }
    }

    /// Adds an entry at the end, or updates the one it matches in place.
    /// Returns its index, or `None` when nothing changed (the same entry
    /// sent again, or the log is full).
    pub fn upsert(&mut self, mut fields: Map<String, Value>, now: &str) -> Option<usize> {
        cap_fields(&mut fields);
        let found = self.key(&fields).and_then(|k| self.find_key(&k));
        match found {
            Some(i) if self.entries[i].fields == fields => None,
            Some(i) => {
                let e = &mut self.entries[i];
                e.fields = fields;
                e.updated = Some(now.to_owned());
                Some(i)
            }
            None if self.entries.len() >= MAX_ENTRIES => {
                self.dropped += 1;
                None
            }
            None => {
                self.entries.push(LogEntry {
                    fields,
                    received: now.to_owned(),
                    updated: None,
                });
                Some(self.entries.len() - 1)
            }
        }
    }

    /// Replaces the entries with `items`, the whole log. An item matching
    /// an entry already here keeps its arrival time. Returns the indices
    /// (in the new log) of items new or changed.
    pub fn replace(&mut self, items: Vec<Map<String, Value>>, now: &str) -> Vec<usize> {
        let mut changed = Vec::new();
        let mut next = Vec::with_capacity(items.len().min(MAX_ENTRIES));
        let mut dropped = 0;
        for (i, mut fields) in items.into_iter().enumerate() {
            if i >= MAX_ENTRIES {
                dropped += 1;
                continue;
            }
            cap_fields(&mut fields);
            let old = self
                .key(&fields)
                .and_then(|k| self.find_key(&k))
                .or_else(|| {
                    // No key: the same fields at the same place.
                    self.entries
                        .get(i)
                        .filter(|e| e.fields == fields && self.key(&fields).is_none())
                        .map(|_| i)
                })
                .map(|j| &self.entries[j]);
            let entry = match old {
                Some(e) if e.fields == fields => e.clone(),
                Some(e) => LogEntry {
                    fields,
                    received: e.received.clone(),
                    updated: Some(now.to_owned()),
                },
                None => LogEntry {
                    fields,
                    received: now.to_owned(),
                    updated: None,
                },
            };
            if self.entries.get(i) != Some(&entry) {
                changed.push(i);
            }
            next.push(entry);
        }
        let shrank = next.len() < self.entries.len();
        self.entries = next;
        self.dropped = dropped;
        if changed.is_empty() && shrank {
            // Entries removed at the end: nothing new to say, but the view
            // changes. The last entry stands for the change.
            changed.extend(self.entries.len().checked_sub(1));
        }
        changed
    }

    fn shown(&self, e: &LogEntry, labels: &LogLabels) -> Shown {
        let mut used: Vec<&String> = Vec::new();
        let mut known = Vec::new();
        for (f, _) in FIELD_GROUPS {
            if let Some(k) = find_field(&e.fields, *f) {
                used.push(k);
                let v = &e.fields[k];
                let text = match (f, v) {
                    (Field::Traffic, Value::Bool(true)) => labels.traffic.clone(),
                    (Field::Traffic, Value::Bool(false)) => String::new(),
                    _ => value_text(v, labels),
                };
                if !text.is_empty() {
                    known.push((*f, text));
                }
            }
        }
        // The id is for matching, not for reading.
        let id = match &self.options.id_field {
            Some(f) => e.fields.keys().find(|k| k.eq_ignore_ascii_case(f)),
            None => None,
        }
        .or_else(|| {
            ID_NAMES
                .iter()
                .find_map(|n| e.fields.keys().find(|k| k.eq_ignore_ascii_case(n)))
        });
        used.extend(id);
        let unknown = e
            .fields
            .iter()
            .filter(|(k, _)| !used.contains(k))
            .map(|(k, v)| (clean(k), value_text(v, labels)))
            .filter(|(_, v)| !v.is_empty())
            .collect();
        Shown { known, unknown }
    }

    /// Entry `i` as one line, meaning first: "N0CALL, Ada Example.
    /// Mobile, portable antenna." The callsign and name come first, then
    /// location, status, traffic, notes and time, then any other fields
    /// as "key: value".
    pub fn line(&self, i: usize, labels: &LogLabels) -> String {
        let Some(e) = self.entries.get(i) else {
            return String::new();
        };
        let shown = self.shown(e, labels);
        let mut head = Vec::new();
        let mut rest = Vec::new();
        for (f, v) in shown.known {
            match f {
                Field::Callsign | Field::Name => head.push(v),
                Field::Time => rest.push(format!("{}: {v}", labels.header(f))),
                Field::Traffic if v != labels.traffic => {
                    rest.push(format!("{}: {v}", labels.header(f)));
                }
                _ => rest.push(v),
            }
        }
        rest.extend(shown.unknown.into_iter().map(|(k, v)| format!("{k}: {v}")));
        let mut sentences = Vec::new();
        if !head.is_empty() {
            sentences.push(head.join(", "));
        }
        sentences.extend(rest);
        if sentences.is_empty() {
            return labels.empty_entry.clone();
        }
        sentences
            .into_iter()
            .map(|s| sentence(&s))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The log as Markdown in `view`: a heading with the title, then the
    /// entries.
    pub fn render(&self, view: LogView, labels: &LogLabels) -> String {
        let mut md = format!("# {}\n\n", escape(&clean(&self.options.title)));
        if self.entries.is_empty() {
            md.push_str(&escape(&labels.empty));
            md.push('\n');
            return md;
        }
        match view {
            LogView::List => {
                for i in 0..self.entries.len() {
                    md.push_str("- ");
                    md.push_str(&escape(&self.line(i, labels)));
                    md.push('\n');
                }
            }
            LogView::Table => self.render_table(&mut md, labels),
        }
        md
    }

    fn render_table(&self, md: &mut String, labels: &LogLabels) {
        let rows: Vec<Shown> = self.entries.iter().map(|e| self.shown(e, labels)).collect();
        let mut fields: Vec<Field> = Vec::new();
        for (f, _) in FIELD_GROUPS {
            if rows.iter().any(|r| r.known.iter().any(|(g, _)| g == f)) {
                fields.push(*f);
            }
        }
        let mut others: Vec<String> = Vec::new();
        for r in &rows {
            for (k, _) in &r.unknown {
                if !others.contains(k) {
                    others.push(k.clone());
                }
            }
        }
        let headers: Vec<String> = fields
            .iter()
            .map(|f| labels.header(*f).to_owned())
            .chain(others.iter().cloned())
            .collect();
        let cell = |s: &str| {
            let e = escape(s);
            if e.is_empty() { " ".to_owned() } else { e }
        };
        md.push_str("| ");
        md.push_str(
            &headers
                .iter()
                .map(|h| cell(h))
                .collect::<Vec<_>>()
                .join(" | "),
        );
        md.push_str(" |\n|");
        md.push_str(&"---|".repeat(headers.len()));
        md.push('\n');
        for r in &rows {
            let cells = fields
                .iter()
                .map(|f| {
                    r.known
                        .iter()
                        .find(|(g, _)| g == f)
                        .map(|(_, v)| {
                            if *f == Field::Traffic && *v == labels.traffic {
                                labels.yes.clone()
                            } else {
                                v.clone()
                            }
                        })
                        .unwrap_or_default()
                })
                .chain(others.iter().map(|k| {
                    r.unknown
                        .iter()
                        .find(|(u, _)| u == k)
                        .map(|(_, v)| v.clone())
                        .unwrap_or_default()
                }));
            md.push_str("| ");
            md.push_str(&cells.map(|c| cell(&c)).collect::<Vec<_>>().join(" | "));
            md.push_str(" |\n");
        }
    }

    /// Where entry `i` starts in `doc`, rendered in `view`.
    pub fn entry_start(doc: &Document, view: LogView, i: usize) -> Option<CharPos> {
        Self::entry_ranges(doc, view).get(i).map(|r| r.start)
    }

    /// Which entry `pos` is in, in `doc` rendered in `view`.
    pub fn entry_at(doc: &Document, view: LogView, pos: CharPos) -> Option<usize> {
        Self::entry_ranges(doc, view)
            .iter()
            .position(|r| r.start <= pos && pos < r.end.max(CharPos(r.start.0 + 1)))
    }

    /// The entries' ranges in `doc`, rendered in `view`: its list items,
    /// or its table rows after the header.
    pub fn entry_ranges(doc: &Document, view: LogView) -> Vec<CharRange> {
        let (kind, skip) = match view {
            LogView::List => (MarkerKind::ListItem, 0),
            LogView::Table => (MarkerKind::TableRow, 1),
        };
        doc.markers()
            .iter()
            .filter(|m| m.kind == kind)
            .skip(skip)
            .map(|m| m.range)
            .collect()
    }

    /// The cursor's place in `old` (shown in `from`) moved to the same
    /// entry in `new` (shown in `to`): the entry's start.
    pub fn same_entry(
        old: &Document,
        from: LogView,
        new: &Document,
        to: LogView,
        pos: CharPos,
    ) -> Option<CharPos> {
        let i = Self::entry_at(old, from, pos)?;
        Self::entry_start(new, to, i)
    }
}

fn cap_fields(fields: &mut Map<String, Value>) {
    while fields.len() > MAX_FIELDS {
        let last = fields.keys().next_back().cloned();
        if let Some(k) = last {
            fields.remove(&k);
        }
    }
}

/// A value as text, for matching: strings as they are, other values as
/// JSON.
fn plain(v: &Value) -> String {
    match v {
        Value::String(s) => clean(s),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// A value as text, for showing: true and false in words, lists and
/// objects as compact JSON.
fn value_text(v: &Value, labels: &LogLabels) -> String {
    match v {
        Value::Bool(true) => labels.yes.clone(),
        Value::Bool(false) => labels.no.clone(),
        other => plain(other),
    }
}

/// `s` with control, escape and direction characters dropped, runs of
/// white space made one space, and at most [`MAX_VALUE_CHARS`] chars.
pub fn clean(s: &str) -> String {
    let mut out = String::with_capacity(s.len().min(MAX_VALUE_CHARS * 4));
    let mut space = false;
    let mut n = 0;
    for c in s.chars() {
        let bidi = matches!(c, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{feff}');
        if bidi {
            continue;
        }
        if c.is_whitespace() || c.is_control() {
            space = !out.is_empty();
            continue;
        }
        if space {
            out.push(' ');
            space = false;
            n += 1;
        }
        if n >= MAX_VALUE_CHARS {
            out.push_str("...");
            break;
        }
        out.push(c);
        n += 1;
    }
    out
}

/// `s` as a sentence: ending in a full stop unless it ends in one already.
fn sentence(s: &str) -> String {
    if s.ends_with(['.', '!', '?']) {
        s.to_owned()
    } else {
        format!("{s}.")
    }
}

/// `s` as literal Markdown text: every ASCII punctuation char escaped, so
/// data never becomes markup.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        if c.is_ascii_punctuation() {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn log() -> LiveLog {
        LiveLog::new(LogOptions {
            title: "Log".into(),
            id_field: None,
        })
    }

    fn obj(v: Value) -> Map<String, Value> {
        as_object(v)
    }

    #[test]
    fn three_shapes_are_told_apart() {
        assert!(matches!(
            Shape::of(json!({"call": "N0CALL"})),
            Shape::One(_)
        ));
        assert!(matches!(Shape::of(json!([{"call": "N0CALL"}])), Shape::Whole(v) if v.len() == 1));
        let wrapped = json!({"net": "Example", "freq": "147.040", "checkins": [{"call": "N0CALL"}, {"call": "N1TEST"}]});
        assert!(matches!(Shape::of(wrapped), Shape::Whole(v) if v.len() == 2));
        // An entry with a list of objects inside stays one entry.
        let entry = json!({"call": "N0CALL", "contacts": [{"call": "N1TEST"}]});
        assert!(matches!(Shape::of(entry), Shape::One(_)));
        // An unnamed single list of objects is the log.
        let other = json!({"meta": 1, "rows": [{"call": "N0CALL"}]});
        assert!(matches!(Shape::of(other), Shape::Whole(_)));
    }

    #[test]
    fn a_line_puts_the_meaning_first() {
        let mut l = log();
        l.upsert(
            obj(json!({"callsign": "N0CALL", "name": "Ada Example", "notes": "Mobile, portable antenna"})),
            "t",
        );
        assert_eq!(
            l.line(0, &LogLabels::default()),
            "N0CALL, Ada Example. Mobile, portable antenna."
        );
        l.upsert(
            obj(json!({"id": 7, "call": "N1TEST", "location": "Portland, OR", "traffic": true, "zzz": "x", "time": "20:10"})),
            "t",
        );
        assert_eq!(
            l.line(1, &LogLabels::default()),
            "N1TEST. Portland, OR. Traffic. Time: 20:10. zzz: x."
        );
    }

    #[test]
    fn corrections_match_by_id_then_callsign() {
        let mut l = log();
        assert_eq!(
            l.upsert(obj(json!({"id": 1, "call": "N0CALL"})), "a"),
            Some(0)
        );
        assert_eq!(
            l.upsert(obj(json!({"id": 2, "call": "N0CALL"})), "b"),
            Some(1)
        );
        assert_eq!(
            l.upsert(obj(json!({"id": 1, "call": "N0CALL", "notes": "x"})), "c"),
            Some(0)
        );
        assert_eq!(l.entries()[0].received, "a");
        assert_eq!(l.entries()[0].updated.as_deref(), Some("c"));
        assert_eq!(l.upsert(obj(json!({"call": "n1 test"})), "d"), Some(2));
        assert_eq!(
            l.upsert(obj(json!({"call": "N1TEST", "name": "Grace"})), "e"),
            Some(2)
        );
        // The same entry again changes nothing.
        assert_eq!(
            l.upsert(obj(json!({"call": "N1TEST", "name": "Grace"})), "f"),
            None
        );
        // A configured id field.
        let mut l = LiveLog::new(LogOptions {
            title: String::new(),
            id_field: Some("row".into()),
        });
        l.upsert(obj(json!({"row": 1, "call": "A"})), "a");
        assert_eq!(l.upsert(obj(json!({"row": 1, "call": "B"})), "b"), Some(0));
        assert_eq!(l.len(), 1);
    }

    #[test]
    fn a_whole_log_keeps_arrival_times() {
        let mut l = log();
        l.upsert(obj(json!({"call": "A"})), "1");
        l.upsert(obj(json!({"call": "B"})), "2");
        let changed = l.replace(
            vec![
                obj(json!({"call": "A"})),
                obj(json!({"call": "B", "notes": "n"})),
                obj(json!({"call": "C"})),
            ],
            "3",
        );
        assert_eq!(changed, vec![1, 2]);
        assert_eq!(l.entries()[0].received, "1");
        assert_eq!(l.entries()[1].received, "2");
        assert_eq!(l.entries()[2].received, "3");
    }

    #[test]
    fn markup_and_control_characters_stay_text() {
        let mut l = log();
        l.upsert(
            obj(json!({"call": "# N0CALL", "notes": "*bold* [x](y) | 1. \u{1b}[31mred\u{202e}"})),
            "t",
        );
        let md = l.render(LogView::List, &LogLabels::default());
        let doc = super::super::load_markdown(&md);
        let text = doc.text().to_string();
        assert!(
            text.contains("# N0CALL. *bold* [x](y) | 1. [31mred."),
            "{text:?}"
        );
        assert_eq!(LiveLog::entry_ranges(&doc, LogView::List).len(), 1);
    }

    #[test]
    fn the_table_has_real_headers_and_unknown_columns_last() {
        let mut l = log();
        l.upsert(obj(json!({"call": "A", "name": "Ada", "zone": "3"})), "t");
        l.upsert(obj(json!({"call": "B", "notes": "x"})), "t");
        let md = l.render(LogView::Table, &LogLabels::default());
        assert!(md.contains("| Callsign | Name | Notes | zone |"), "{md}");
        let doc = super::super::load_markdown(&md);
        assert_eq!(LiveLog::entry_ranges(&doc, LogView::Table).len(), 2);
        assert!(doc.markers().iter().any(|m| m.kind == MarkerKind::Table));
    }
}
