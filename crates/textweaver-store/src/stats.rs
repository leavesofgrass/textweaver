//! Reading statistics: time spent reading each document aloud, the
//! furthest point reached, and how many sessions, in `stats.json` in the
//! data folder. `[stats] enabled = false` stops the app recording.
//!
//! The app adds to the file in small steps ([`StatsDelta`]) through its
//! writer thread, so nothing waits on the disk; `tw stats` reads it.
//!
//! Differences from Star's `ReadingStats` (`stats.py`): the statistics live
//! in their own file, not in `settings.json`, so reading does not rewrite
//! the settings; a session is an opening of a document in which you read,
//! not every press of play; and the furthest point is kept as a position,
//! so a later version can go to it.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{Paths, StoreError, atomic_write};

/// Documents kept; the ones read longest ago are dropped past this.
pub const MAX_DOCUMENTS: usize = 1000;

/// One document's statistics.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DocStats {
    /// The title when last read.
    pub title: String,
    /// The file, if it has one.
    pub path: Option<PathBuf>,
    /// Seconds spent reading aloud.
    pub seconds: f64,
    /// Openings of the document in which it was read.
    pub sessions: u32,
    /// The furthest point read, in percent.
    pub furthest_percent: u8,
    /// The furthest point read, as a character position.
    pub furthest_char: u64,
    /// When it was first read (Unix seconds, UTC).
    pub first_read: i64,
    /// When it was last read (Unix seconds, UTC).
    pub last_read: i64,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Every document's statistics, by document key (`DocKey`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReadingStats {
    /// Format version, 1.
    pub version: u32,
    /// Document key to statistics.
    pub documents: BTreeMap<String, DocStats>,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Reading to add to a document's statistics.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StatsDelta {
    /// The document key.
    pub key: String,
    /// Its title.
    pub title: String,
    /// Its file.
    pub path: Option<PathBuf>,
    /// Seconds read since the last delta.
    pub seconds: f64,
    /// This delta starts a new session.
    pub new_session: bool,
    /// The furthest point reached, in percent.
    pub furthest_percent: u8,
    /// The furthest point reached, as a character position.
    pub furthest_char: u64,
    /// Now (Unix seconds, UTC).
    pub at: i64,
}

impl ReadingStats {
    /// Loads `stats.json`; a missing file gives empty statistics.
    pub fn load(paths: &Paths) -> Result<ReadingStats, StoreError> {
        let path = paths.stats_file();
        match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| StoreError::Parse {
                path,
                message: e.to_string(),
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ReadingStats::default()),
            Err(source) => Err(StoreError::Io { path, source }),
        }
    }

    /// Saves `stats.json` atomically.
    pub fn save(&self, paths: &Paths) -> Result<(), StoreError> {
        let path = paths.stats_file();
        let mut out = self.clone();
        out.version = 1;
        let text = serde_json::to_string_pretty(&out).map_err(|e| StoreError::Parse {
            path: path.clone(),
            message: e.to_string(),
        })?;
        atomic_write(&path, text.as_bytes())
    }

    /// Adds `d`, keeping the furthest point the furthest, and drops the
    /// documents read longest ago past [`MAX_DOCUMENTS`].
    pub fn add(&mut self, d: &StatsDelta) {
        let s = self.documents.entry(d.key.clone()).or_default();
        if !d.title.is_empty() {
            s.title.clone_from(&d.title);
        }
        if d.path.is_some() {
            s.path.clone_from(&d.path);
        }
        if d.seconds.is_finite() && d.seconds > 0.0 {
            s.seconds += d.seconds;
        }
        if d.new_session {
            s.sessions += 1;
        }
        s.furthest_percent = s.furthest_percent.max(d.furthest_percent.min(100));
        s.furthest_char = s.furthest_char.max(d.furthest_char);
        if s.first_read == 0 {
            s.first_read = d.at;
        }
        s.last_read = s.last_read.max(d.at);
        if self.documents.len() > MAX_DOCUMENTS {
            let mut by_age: Vec<(i64, String)> = self
                .documents
                .iter()
                .map(|(k, v)| (v.last_read, k.clone()))
                .collect();
            by_age.sort();
            let extra = self.documents.len() - MAX_DOCUMENTS;
            for (_, k) in by_age.into_iter().take(extra) {
                self.documents.remove(&k);
            }
        }
    }

    /// Loads, adds `deltas`, and saves: what the app's writer does.
    pub fn add_to_file(paths: &Paths, deltas: &[StatsDelta]) -> Result<(), StoreError> {
        // A damaged file is replaced rather than blocking new statistics.
        let mut s = ReadingStats::load(paths).unwrap_or_default();
        for d in deltas {
            s.add(d);
        }
        s.save(paths)
    }

    /// The documents read longest, at most `n`, longest first.
    pub fn most_read(&self, n: usize) -> Vec<(&str, &DocStats)> {
        let mut v: Vec<(&str, &DocStats)> = self
            .documents
            .iter()
            .map(|(k, s)| (k.as_str(), s))
            .collect();
        v.sort_by(|a, b| {
            b.1.seconds
                .total_cmp(&a.1.seconds)
                .then(b.1.last_read.cmp(&a.1.last_read))
        });
        v.truncate(n);
        v
    }

    /// Seconds read across every document.
    pub fn total_seconds(&self) -> f64 {
        self.documents.values().map(|d| d.seconds).sum()
    }

    /// Sessions across every document.
    pub fn total_sessions(&self) -> u64 {
        self.documents.values().map(|d| u64::from(d.sessions)).sum()
    }
}

/// Whole hours, minutes, and seconds in `seconds` (rounded to the second).
pub fn hms(seconds: f64) -> (u64, u64, u64) {
    let s = if seconds.is_finite() && seconds > 0.0 {
        seconds.round() as u64
    } else {
        0
    };
    (s / 3600, s % 3600 / 60, s % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delta(key: &str, seconds: f64, pct: u8, new_session: bool, at: i64) -> StatsDelta {
        StatsDelta {
            key: key.into(),
            title: format!("Title {key}"),
            path: Some(PathBuf::from(format!("{key}.md"))),
            seconds,
            new_session,
            furthest_percent: pct,
            furthest_char: u64::from(pct) * 10,
            at,
        }
    }

    #[test]
    fn adds_and_ranks() {
        let mut s = ReadingStats::default();
        s.add(&delta("a", 30.0, 10, true, 100));
        s.add(&delta("a", 30.0, 5, false, 200));
        s.add(&delta("b", 90.0, 50, true, 150));
        s.add(&delta("a", f64::NAN, 120, true, 300));
        let a = &s.documents["a"];
        assert_eq!(a.seconds, 60.0);
        assert_eq!(a.sessions, 2);
        assert_eq!(a.furthest_percent, 100);
        assert_eq!(a.furthest_char, 1200);
        assert_eq!((a.first_read, a.last_read), (100, 300));
        let top: Vec<&str> = s.most_read(5).iter().map(|(k, _)| *k).collect();
        assert_eq!(top, ["b", "a"]);
        assert_eq!(s.total_seconds(), 150.0);
        assert_eq!(s.total_sessions(), 3);
        assert_eq!(hms(3725.4), (1, 2, 5));
        assert_eq!(hms(-3.0), (0, 0, 0));
    }

    #[test]
    fn file_round_trip_and_limit() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::under(dir.path());
        assert!(ReadingStats::load(&paths).unwrap().documents.is_empty());
        ReadingStats::add_to_file(&paths, &[delta("a", 5.0, 1, true, 1)]).unwrap();
        ReadingStats::add_to_file(&paths, &[delta("a", 5.0, 2, false, 2)]).unwrap();
        let s = ReadingStats::load(&paths).unwrap();
        assert_eq!(s.version, 1);
        assert_eq!(s.documents["a"].seconds, 10.0);
        // A damaged file is replaced.
        std::fs::write(paths.stats_file(), "{broken").unwrap();
        assert!(ReadingStats::load(&paths).is_err());
        ReadingStats::add_to_file(&paths, &[delta("b", 1.0, 1, true, 3)]).unwrap();
        assert_eq!(ReadingStats::load(&paths).unwrap().documents.len(), 1);

        let mut s = ReadingStats::default();
        for i in 0..(MAX_DOCUMENTS + 5) {
            s.add(&delta(&format!("d{i}"), 1.0, 1, true, i as i64));
        }
        assert_eq!(s.documents.len(), MAX_DOCUMENTS);
        assert!(!s.documents.contains_key("d0"));
        assert!(s.documents.contains_key(&format!("d{}", MAX_DOCUMENTS + 4)));
    }
}
