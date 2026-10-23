//! CW practice progress: the Koch lesson reached and the latest scores, in
//! `cw-practice.json` in the data folder, so each person's data folder
//! (their profile on the computer) keeps its own.
//!
//! Like `stats.json`, it is this computer's: sync carries per-document
//! data, and this file belongs to no document. A later version may sync
//! it.
//!
//! This is a new file format, a proposal for the owner's review; it is not
//! frozen ([`CW_PRACTICE_FORMAT`]).

use serde::{Deserialize, Serialize};

use crate::{Paths, StoreError, atomic_write};

/// The format of `cw-practice.json` this version writes.
pub const CW_PRACTICE_FORMAT: u32 = 1;

/// The scores kept; older ones are dropped.
pub const RECENT_MAX: usize = 20;

/// What a round practiced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Drill {
    /// A Koch lesson's characters.
    Koch,
    /// Random characters from the whole Koch course.
    Characters,
    /// Random words.
    Words,
}

/// One finished round.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PracticeScore {
    /// When it finished (Unix seconds, UTC).
    pub at: i64,
    /// What it practiced.
    pub drill: Drill,
    /// The Koch lesson practiced, for a Koch round.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lesson: Option<u8>,
    /// Characters copied right.
    pub correct: u32,
    /// Characters sent.
    pub total: u32,
}

/// `cw-practice.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CwPractice {
    /// Format version ([`CW_PRACTICE_FORMAT`]).
    pub version: u32,
    /// The Koch lesson reached: 1 is K and M, and each later lesson adds a
    /// character (40 has all 41).
    pub koch_lesson: u8,
    /// The latest rounds, oldest first, at most [`RECENT_MAX`].
    pub recent: Vec<PracticeScore>,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Default for CwPractice {
    fn default() -> Self {
        CwPractice {
            version: CW_PRACTICE_FORMAT,
            koch_lesson: 1,
            recent: Vec::new(),
            extra: serde_json::Map::new(),
        }
    }
}

impl CwPractice {
    /// Loads `cw-practice.json`; a missing file is a learner at lesson 1.
    pub fn load(paths: &Paths) -> Result<CwPractice, StoreError> {
        let path = paths.cw_practice_file();
        match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| StoreError::Parse {
                path,
                message: e.to_string(),
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(CwPractice::default()),
            Err(source) => Err(StoreError::Io { path, source }),
        }
    }

    /// Saves `cw-practice.json` atomically.
    pub fn save(&self, paths: &Paths) -> Result<(), StoreError> {
        let path = paths.cw_practice_file();
        let mut out = self.clone();
        out.version = CW_PRACTICE_FORMAT;
        let text = serde_json::to_string_pretty(&out).map_err(|e| StoreError::Parse {
            path: path.clone(),
            message: e.to_string(),
        })?;
        atomic_write(&path, text.as_bytes())
    }

    /// Adds a finished round, dropping the oldest past [`RECENT_MAX`].
    pub fn record(&mut self, score: PracticeScore) {
        self.recent.push(score);
        let over = self.recent.len().saturating_sub(RECENT_MAX);
        self.recent.drain(..over);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score(at: i64) -> PracticeScore {
        PracticeScore {
            at,
            drill: Drill::Koch,
            lesson: Some(3),
            correct: 46,
            total: 50,
        }
    }

    #[test]
    fn progress_round_trips_and_a_missing_file_is_lesson_one() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::under(dir.path());
        assert_eq!(CwPractice::load(&paths).unwrap(), CwPractice::default());
        let mut p = CwPractice {
            koch_lesson: 3,
            ..CwPractice::default()
        };
        p.record(score(10));
        p.extra
            .insert("future".into(), serde_json::Value::Bool(true));
        p.save(&paths).unwrap();
        assert_eq!(CwPractice::load(&paths).unwrap(), p);
        let text = std::fs::read_to_string(paths.cw_practice_file()).unwrap();
        assert!(text.contains("\"koch_lesson\": 3"), "{text}");
        assert!(text.contains("\"drill\": \"koch\""), "{text}");
    }

    #[test]
    fn only_the_latest_scores_are_kept() {
        let mut p = CwPractice::default();
        for at in 0..25 {
            p.record(score(at));
        }
        assert_eq!(p.recent.len(), RECENT_MAX);
        assert_eq!(p.recent[0].at, 5);
        assert_eq!(p.recent[RECENT_MAX - 1].at, 24);
    }
}
