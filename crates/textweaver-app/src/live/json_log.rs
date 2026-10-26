//! The JSON log: a live document's entries as received, with their
//! arrival times, written to a dated file while the log runs and when it
//! closes, so a crash loses little. Reopening such a file shows it in the
//! log view again, read only.
//!
//! ```json
//! {
//!   "format": "textweaver live log",
//!   "version": 1,
//!   "source": "club",
//!   "title": "Club log",
//!   "started": "2026-10-26T20:10:00Z",
//!   "entries": [
//!     {"received": "2026-10-26T20:10:05Z", "entry": {"call": "N0CALL", "name": "Ada Example"}}
//!   ]
//! }
//! ```
//!
//! A corrected entry carries `"updated"` too, the time of its last
//! correction. A log whose source wraps its list carries `"head"` too, the
//! wrapper's plain values, shown as the heading line. Every object keeps
//! its keys in the order the source sent them.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::ser::{SerializeMap, SerializeSeq, Serializer};
use serde_json::Value;

use super::log::{LiveLog, LogEntry, LogOptions};
use super::order::{KeyOrder, child, parse_ordered};

/// The `format` value that marks a file as a textweaver live log.
pub const FORMAT: &str = "textweaver live log";

/// The format's version.
const VERSION: u64 = 1;

/// The largest file read back as a live log.
const MAX_BYTES: u64 = 64 * 1024 * 1024;

/// The log's file name for a log started at `stamp` (`YYYY-MM-DD-HHMM`,
/// local time), from source `source`: `live-log-2026-10-26-2010.json`,
/// prefixed with the source's id (`club-live-log-...`) when there is one.
/// The id keeps only letters, digits, `-` and `_`.
pub fn file_name(source: &str, stamp: &str) -> String {
    let id: String = source
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(40)
        .collect::<String>()
        .to_lowercase();
    if id.is_empty() {
        format!("live-log-{stamp}.json")
    } else {
        format!("{id}-live-log-{stamp}.json")
    }
}

/// A path in `folder` for `name` that no file has yet: `name`, else
/// `name` with `-2`, `-3` and so on before `.json`.
pub fn free_path(folder: &Path, name: &str) -> PathBuf {
    let first = folder.join(name);
    if !first.exists() {
        return first;
    }
    let stem = name.strip_suffix(".json").unwrap_or(name);
    (2..10_000)
        .map(|n| folder.join(format!("{stem}-{n}.json")))
        .find(|p| !p.exists())
        .unwrap_or(first)
}

/// A JSON value written with its objects' keys in message order.
struct Ordered<'a> {
    value: &'a Value,
    order: &'a KeyOrder,
    at: String,
}

impl Serialize for Ordered<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.value {
            Value::Object(map) => {
                let members = self.order.members(&self.at, map);
                let mut m = s.serialize_map(Some(members.len()))?;
                for (k, v) in members {
                    m.serialize_entry(
                        k,
                        &Ordered {
                            value: v,
                            order: self.order,
                            at: child(&self.at, k),
                        },
                    )?;
                }
                m.end()
            }
            Value::Array(items) => {
                let mut q = s.serialize_seq(Some(items.len()))?;
                for (i, v) in items.iter().enumerate() {
                    q.serialize_element(&Ordered {
                        value: v,
                        order: self.order,
                        at: format!("{}/{i}", self.at),
                    })?;
                }
                q.end()
            }
            other => other.serialize(s),
        }
    }
}

/// One entry as the file holds it.
#[derive(Serialize)]
struct EntryOut<'a> {
    received: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated: Option<&'a str>,
    entry: Ordered<'a>,
}

/// The file, its keys in the order of the example above.
#[derive(Serialize)]
struct FileOut<'a> {
    format: &'a str,
    version: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<&'a str>,
    title: &'a str,
    started: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id_field: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    head: Option<Ordered<'a>>,
    entries: Vec<EntryOut<'a>>,
}

/// The log as the file's JSON. Each entry's keys, and the heading's, keep
/// the order they came in.
pub fn to_json(log: &LiveLog, source: &str, started: &str) -> Vec<u8> {
    let (head, head_order) = log.head();
    let head_value = Value::Object(head.clone());
    let fields: Vec<Value> = log
        .entries()
        .iter()
        .map(|e| Value::Object(e.fields.clone()))
        .collect();
    let doc = FileOut {
        format: FORMAT,
        version: VERSION,
        source: (!source.is_empty()).then_some(source),
        title: &log.options.title,
        started,
        id_field: log.options.id_field.as_deref(),
        head: (!head.is_empty()).then(|| Ordered {
            value: &head_value,
            order: head_order,
            at: String::new(),
        }),
        entries: log
            .entries()
            .iter()
            .zip(&fields)
            .map(|(e, f)| EntryOut {
                received: &e.received,
                updated: e.updated.as_deref(),
                entry: Ordered {
                    value: f,
                    order: &e.order,
                    at: String::new(),
                },
            })
            .collect(),
    };
    let mut out = serde_json::to_vec_pretty(&doc).unwrap_or_default();
    out.push(b'\n');
    out
}

/// A saved log read back: its log and its source's id.
#[derive(Clone, Debug)]
pub struct SavedLog {
    /// The entries, title and id field.
    pub log: LiveLog,
    /// The source's id, if the file names one.
    pub source: String,
}

/// Reads `bytes` as a saved live log; `None` when it is not one.
pub fn parse(bytes: &[u8]) -> Option<SavedLog> {
    let (v, order) = parse_ordered(std::str::from_utf8(bytes).ok()?)?;
    if v.get("format")?.as_str()? != FORMAT {
        return None;
    }
    let text = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_owned);
    let entries = v
        .get("entries")?
        .as_array()?
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            Some(LogEntry {
                fields: e.get("entry")?.as_object()?.clone(),
                received: e.get("received")?.as_str()?.to_owned(),
                updated: e.get("updated").and_then(Value::as_str).map(str::to_owned),
                order: order.sub(&format!("/entries/{i}/entry")),
            })
        })
        .collect();
    let options = LogOptions {
        title: text("title").unwrap_or_default(),
        id_field: text("id_field"),
    };
    let mut log = LiveLog::with_entries(options, entries);
    if let Some(head) = v.get("head").and_then(Value::as_object) {
        log.set_head(head.clone(), order.sub("/head"));
    }
    Some(SavedLog {
        log,
        source: text("source").unwrap_or_default(),
    })
}

/// Reads the file at `path` as a saved live log, when its name ends in
/// `.json` and it is one; `None` otherwise (it opens as any other file).
pub fn read(path: &Path) -> Option<SavedLog> {
    let json = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("json"));
    if !json || std::fs::metadata(path).ok()?.len() > MAX_BYTES {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    // Cheap test before parsing a large file that is something else.
    let head = &bytes[..bytes.len().min(4096)];
    if !String::from_utf8_lossy(head).contains(FORMAT) {
        return None;
    }
    parse(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::log::entry_object;

    #[test]
    fn file_names_are_plain() {
        assert_eq!(
            file_name("", "2026-10-26-2010"),
            "live-log-2026-10-26-2010.json"
        );
        assert_eq!(
            file_name("Club Log/$x", "2026-10-26-2010"),
            "clublogx-live-log-2026-10-26-2010.json"
        );
    }

    #[test]
    fn a_log_survives_the_round_trip() {
        let mut log = LiveLog::new(LogOptions {
            title: "Club".into(),
            id_field: Some("row".into()),
        });
        log.upsert(
            entry_object(&[("call", "N0CALL"), ("name", "Ada Example")]),
            "t1",
        );
        log.upsert(entry_object(&[("call", "N0CALL"), ("name", "Ada")]), "t2");
        // An entry whose keys came name first, and a heading.
        let mut order = KeyOrder::default();
        order.set("", vec!["name".into(), "call".into()]);
        log.upsert_ordered(
            entry_object(&[("call", "N1TEST"), ("name", "Grace Sample")]),
            order.clone(),
            "t3",
        );
        let mut head_order = KeyOrder::default();
        head_order.set("", vec!["net".into(), "frequency".into()]);
        log.set_head(
            entry_object(&[("net", "Practice"), ("frequency", "147.040")]),
            head_order.clone(),
        );
        let bytes = to_json(&log, "club", "t0");
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(
            text.find("Grace Sample").unwrap() < text.find("N1TEST").unwrap(),
            "{text}"
        );
        assert!(text.find("Practice").unwrap() < text.find("147.040").unwrap());
        let back = parse(&bytes).expect("a live log");
        assert_eq!(back.source, "club");
        assert_eq!(back.log.options, log.options);
        let plain = |l: &LiveLog| -> Vec<_> {
            l.entries()
                .iter()
                .map(|e| (e.fields.clone(), e.received.clone(), e.updated.clone()))
                .collect()
        };
        assert_eq!(plain(&back.log), plain(&log));
        assert_eq!(back.log.entries()[1].order, order);
        assert_eq!(back.log.head(), log.head());
        assert_eq!(*back.log.head().1, head_order);
        assert!(parse(br#"{"entries": []}"#).is_none());
    }
}
