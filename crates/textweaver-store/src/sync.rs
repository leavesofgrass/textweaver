//! Folder sidecar sync (`<folder>/.textweaver/progress.json`), ported from
//! `star/sync.py` by Agent C with its 45 tests. Phase 0 holds only the
//! policy type the settings refer to.

use serde::{Deserialize, Serialize};

/// How conflicting positions from two devices are resolved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictPolicy {
    /// The newest timestamp wins (Star default).
    #[default]
    Newest,
    /// The furthest position wins.
    HighestProgress,
    /// Keep both and ask.
    Manual,
}
