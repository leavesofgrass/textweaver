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
    ///
    /// The path is stored resolved, as document keys and library entries
    /// are ([`resolve_path`](crate::library::resolve_path)), so an entry
    /// recorded from `textweaver notes.md` still opens from another working
    /// directory, and the same file opened by a relative and an absolute
    /// path is listed once.
    pub fn touch(&mut self, path: &Path, title: Option<String>, limit: usize) {
        let path = crate::library::resolve_path(path);
        self.entries
            .retain(|e| e.path != path && crate::library::resolve_path(&e.path) != path);
        self.entries.insert(
            0,
            RecentEntry {
                path,
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
        let names: Vec<_> = r
            .entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_owned())
            .collect();
        assert_eq!(names, vec![PathBuf::from("a"), PathBuf::from("b")]);
        r.touch(Path::new("c"), None, 2);
        assert_eq!(r.entries.len(), 2);
    }

    #[test]
    fn relative_and_absolute_paths_are_one_resolved_entry() {
        // Before: the path was stored as typed, so `textweaver notes.md`
        // left an entry that did not open from another directory.
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("notes.md");
        std::fs::write(&file, "# Notes").unwrap();
        let mut r = Recent::default();
        // An older entry stored as typed (relative, with `..`).
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        r.entries.push(RecentEntry {
            path: sub.join("..").join("notes.md"),
            title: None,
            opened: 0,
        });
        r.touch(&file, Some("Notes".into()), 10);
        assert_eq!(r.entries.len(), 1);
        let stored = &r.entries[0].path;
        assert!(stored.is_absolute());
        assert_eq!(stored, &crate::library::resolve_path(&file));
        assert!(!stored.to_string_lossy().starts_with(r"\\?\"));
    }
}
