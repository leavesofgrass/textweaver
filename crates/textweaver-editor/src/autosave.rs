//! Autosave snapshots and the save rule.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// When to write recovery snapshots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutosavePolicy {
    /// Snapshots enabled.
    pub enabled: bool,
    /// Interval between snapshots while dirty (Star: 20 s).
    pub interval: Duration,
}

impl Default for AutosavePolicy {
    fn default() -> Self {
        AutosavePolicy {
            enabled: true,
            interval: Duration::from_secs(20),
        }
    }
}

impl AutosavePolicy {
    /// True when a snapshot is due: enabled, dirty, and `interval` has passed
    /// since the last snapshot (or there has been none).
    pub fn due(&self, dirty: bool, since_last: Option<Duration>) -> bool {
        self.enabled && dirty && since_last.is_none_or(|d| d >= self.interval)
    }
}

/// A recovery snapshot, stored as `recovery/<doc-key>.json` by the app.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoverySnapshot {
    /// The document's key.
    pub doc_key: String,
    /// The file being edited, if it has one.
    pub path: Option<PathBuf>,
    /// The full text.
    pub text: String,
    /// When the snapshot was taken (Unix seconds, UTC).
    pub ts: i64,
}

/// Where Save writes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveTarget {
    /// Overwrite the source file (text and Markdown sources).
    InPlace(PathBuf),
    /// Ask for a new `.md` path (everything else, including converted
    /// formats Star overwrote in place by mistake).
    SaveAsMarkdown {
        /// Suggested path: the source with a `.md` extension.
        suggested: PathBuf,
    },
}

/// Star's save rule, fixed: save in place only for plain text and Markdown
/// sources; everything else becomes save-as-Markdown.
pub fn save_target(path: &Path, loader_id: &str) -> SaveTarget {
    match loader_id {
        "text" | "markdown" => SaveTarget::InPlace(path.to_owned()),
        _ => SaveTarget::SaveAsMarkdown {
            suggested: path.with_extension("md"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn due_rules() {
        let p = AutosavePolicy::default();
        assert!(p.due(true, None));
        assert!(!p.due(false, None));
        assert!(!p.due(true, Some(Duration::from_secs(5))));
        assert!(p.due(true, Some(Duration::from_secs(20))));
    }

    #[test]
    fn save_rule() {
        assert_eq!(
            save_target(Path::new("a.md"), "markdown"),
            SaveTarget::InPlace("a.md".into())
        );
        assert_eq!(
            save_target(Path::new("a.html"), "html"),
            SaveTarget::SaveAsMarkdown {
                suggested: "a.md".into()
            }
        );
    }
}
