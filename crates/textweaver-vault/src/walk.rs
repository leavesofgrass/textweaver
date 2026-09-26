//! Finding the notes in a vault.

use std::path::{Path, PathBuf};

use crate::VaultError;

/// True for folders Obsidian keeps for itself: `.obsidian` (settings) and
/// `.trash` (deleted notes), compared without regard to case (Star's
/// `_skip`).
fn skipped_dir(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower == ".obsidian" || lower == ".trash"
}

/// Every `*.md` file under `dir`, sorted by path, skipping `.obsidian` and
/// `.trash`. Symbolic links to folders are not followed, so a link loop
/// cannot hang the walk. Unreadable subfolders are skipped; an unreadable
/// `dir` is an error.
pub fn note_files(dir: &Path) -> Result<Vec<PathBuf>, VaultError> {
    if !dir.is_dir() {
        return Err(VaultError::NotAFolder(dir.to_owned()));
    }
    let mut out = Vec::new();
    let mut stack = vec![dir.to_owned()];
    let mut first = true;
    while let Some(folder) = stack.pop() {
        let entries = match std::fs::read_dir(&folder) {
            Ok(e) => e,
            Err(source) if first => {
                return Err(VaultError::Io {
                    path: folder,
                    source,
                });
            }
            Err(_) => continue,
        };
        first = false;
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            if kind.is_dir() {
                if !skipped_dir(&name) {
                    stack.push(path);
                }
            } else if (kind.is_file() || kind.is_symlink())
                && path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("md"))
                && path.is_file()
            {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// The note's name as Obsidian links to it: the file stem.
pub fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_notes_and_skips_obsidian_folders() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("sub/deeper")).unwrap();
        std::fs::create_dir_all(root.join(".obsidian")).unwrap();
        std::fs::create_dir_all(root.join(".Trash")).unwrap();
        for f in [
            "a.md",
            "sub/b.MD",
            "sub/deeper/c.md",
            "notes.txt",
            ".obsidian/app.md",
            ".Trash/old.md",
        ] {
            std::fs::write(root.join(f), "x").unwrap();
        }
        let found: Vec<String> = note_files(root)
            .unwrap()
            .iter()
            .map(|p| {
                p.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        assert_eq!(found, vec!["a.md", "sub/b.MD", "sub/deeper/c.md"]);
    }

    #[test]
    fn a_missing_folder_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            note_files(&dir.path().join("nope")),
            Err(VaultError::NotAFolder(_))
        ));
    }
}
