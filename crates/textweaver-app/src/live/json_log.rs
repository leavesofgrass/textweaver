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
//! correction.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use super::log::{LiveLog, LogEntry, LogOptions};

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

/// The log as the file's JSON.
pub fn to_json(log: &LiveLog, source: &str, started: &str) -> Vec<u8> {
    let entries: Vec<Value> = log
        .entries()
        .iter()
        .map(|e| {
            let mut m = Map::new();
            m.insert("received".into(), Value::String(e.received.clone()));
            if let Some(u) = &e.updated {
                m.insert("updated".into(), Value::String(u.clone()));
            }
            m.insert("entry".into(), Value::Object(e.fields.clone()));
            Value::Object(m)
        })
        .collect();
    let mut doc = json!({
        "format": FORMAT,
        "version": VERSION,
        "title": log.options.title,
        "started": started,
        "entries": entries,
    });
    if !source.is_empty() {
        doc["source"] = Value::String(source.to_owned());
    }
    if let Some(id) = &log.options.id_field {
        doc["id_field"] = Value::String(id.clone());
    }
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
    let v: Value = serde_json::from_slice(bytes).ok()?;
    if v.get("format")?.as_str()? != FORMAT {
        return None;
    }
    let text = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_owned);
    let entries = v
        .get("entries")?
        .as_array()?
        .iter()
        .filter_map(|e| {
            Some(LogEntry {
                fields: e.get("entry")?.as_object()?.clone(),
                received: e.get("received")?.as_str()?.to_owned(),
                updated: e.get("updated").and_then(Value::as_str).map(str::to_owned),
            })
        })
        .collect();
    let options = LogOptions {
        title: text("title").unwrap_or_default(),
        id_field: text("id_field"),
    };
    Some(SavedLog {
        log: LiveLog::with_entries(options, entries),
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
        let bytes = to_json(&log, "club", "t0");
        let back = parse(&bytes).expect("a live log");
        assert_eq!(back.source, "club");
        assert_eq!(back.log.options, log.options);
        assert_eq!(back.log.entries(), log.entries());
        assert!(parse(br#"{"entries": []}"#).is_none());
    }
}
