use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{StoreError, atomic_write};

/// One recently opened document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentEntry {
    /// The file.
    pub path: PathBuf,
    /// Title at the time it was opened.
    pub title: Option<String>,
    /// When it was last opened (Unix seconds, UTC).
    pub opened: i64,
}

/// Most-recently-used documents, newest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recent {
    /// The entries, newest first.
    pub entries: Vec<RecentEntry>,
}

impl Recent {
    /// Records an open: moves the path to the front (Star did not move
    /// existing entries; fixed) and trims to `limit`.
    pub fn touch(&mut self, path: &Path, title: Option<String>, limit: usize) {
        self.entries.retain(|e| e.path != path);
        self.entries.insert(
            0,
            RecentEntry {
                path: path.to_owned(),
                title,
                opened: crate::now_ts(),
            },
        );
        self.entries.truncate(limit.max(1));
    }

    /// Loads from `file` (empty when missing or unreadable).
    pub fn load(file: &Path) -> Self {
        std::fs::read_to_string(file)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    /// Saves to `file` atomically.
    pub fn save(&self, file: &Path) -> Result<(), StoreError> {
        let text = serde_json::to_string_pretty(self).map_err(|e| StoreError::Parse {
            path: file.to_owned(),
            message: e.to_string(),
        })?;
        atomic_write(file, text.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_moves_to_front_and_trims() {
        let mut r = Recent::default();
        r.touch(Path::new("a"), None, 2);
        r.touch(Path::new("b"), None, 2);
        r.touch(Path::new("a"), None, 2);
        let names: Vec<_> = r.entries.iter().map(|e| e.path.clone()).collect();
        assert_eq!(names, vec![PathBuf::from("a"), PathBuf::from("b")]);
        r.touch(Path::new("c"), None, 2);
        assert_eq!(r.entries.len(), 2);
    }
}
