//! Tab completion of file paths in the Open, Save As, and Insert image
//! prompts (Wave 3: moved from the terminal reader, so every frontend's
//! prompts share it): the names in the folder typed so far that start with
//! what follows the last separator (ignoring case on Windows and macOS).

use std::path::{MAIN_SEPARATOR, Path};

/// Most names read out when several match.
const SPOKEN_NAMES: usize = 5;

/// Completes `typed` (relative to `cwd` unless absolute). Returns the new
/// prompt text, when it changed, and what to say: the completed name, "3
/// matches: a, b, c", or that nothing matches. A folder gets a separator
/// after it, so the next Tab lists what is inside.
pub fn complete(typed: &str, cwd: &Path) -> (Option<String>, String) {
    let unquoted = typed.trim_start_matches('"');
    // The folder part (with its separator) and the name being typed.
    let cut = unquoted.rfind(['/', '\\']).map_or(0, |i| i + 1);
    let (dir_part, prefix) = unquoted.split_at(cut);
    let dir = if dir_part.is_empty() {
        cwd.to_path_buf()
    } else if Path::new(dir_part).is_absolute() {
        Path::new(dir_part).to_path_buf()
    } else {
        cwd.join(dir_part)
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return (None, format!("There is no folder {}.", dir.display()));
    };
    let fold = |s: &str| {
        if cfg!(any(windows, target_os = "macos")) {
            s.to_lowercase()
        } else {
            s.to_owned()
        }
    };
    let want = fold(prefix);
    let mut names: Vec<(String, bool)> = entries
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let is_dir = e.file_type().is_ok_and(|t| t.is_dir());
            (fold(&name).starts_with(&want) && (!name.starts_with('.') || prefix.starts_with('.')))
                .then_some((name, is_dir))
        })
        .collect();
    names.sort_by_key(|(n, _)| fold(n));
    match names.as_slice() {
        [] => (None, format!("No file or folder starts with {prefix}.")),
        [(name, is_dir)] => {
            let mut text = format!("{dir_part}{name}");
            if *is_dir {
                text.push(MAIN_SEPARATOR);
            }
            let what = if *is_dir { "folder" } else { "file" };
            (Some(text), format!("{name}, {what}"))
        }
        many => {
            let common = common_prefix(many.iter().map(|(n, _)| n.as_str()));
            let text = (common.chars().count() > prefix.chars().count())
                .then(|| format!("{dir_part}{common}"));
            let shown: Vec<&str> = many
                .iter()
                .take(SPOKEN_NAMES)
                .map(|(n, _)| n.as_str())
                .collect();
            let more = if many.len() > SPOKEN_NAMES {
                ", and more"
            } else {
                ""
            };
            (
                text,
                format!("{} matches: {}{more}.", many.len(), shown.join(", ")),
            )
        }
    }
}

/// The longest start shared by every name (ignoring case where the file
/// system does; the first name's spelling is kept).
fn common_prefix<'a>(mut names: impl Iterator<Item = &'a str>) -> String {
    let Some(first) = names.next() else {
        return String::new();
    };
    let same = |a: char, b: char| {
        if cfg!(any(windows, target_os = "macos")) {
            a.to_lowercase().eq(b.to_lowercase())
        } else {
            a == b
        }
    };
    let mut len = first.chars().count();
    for n in names {
        len = first
            .chars()
            .zip(n.chars())
            .take(len)
            .take_while(|(a, b)| same(*a, *b))
            .count();
    }
    first.chars().take(len).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completes_files_and_folders() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("notes.md"), "").unwrap();
        std::fs::write(dir.path().join("novel.txt"), "").unwrap();
        std::fs::create_dir(dir.path().join("papers")).unwrap();
        std::fs::write(dir.path().join("papers").join("draft.md"), "").unwrap();
        std::fs::write(dir.path().join(".hidden"), "").unwrap();

        let (text, said) = complete("no", dir.path());
        assert_eq!(text, None);
        assert_eq!(said, "2 matches: notes.md, novel.txt.");
        let (text, said) = complete("not", dir.path());
        assert_eq!(text.as_deref(), Some("notes.md"));
        assert_eq!(said, "notes.md, file");
        let (text, said) = complete("pa", dir.path());
        assert_eq!(text, Some(format!("papers{MAIN_SEPARATOR}")));
        assert_eq!(said, "papers, folder");
        let (text, _) = complete("papers/d", dir.path());
        assert_eq!(text.as_deref(), Some("papers/draft.md"));
        let abs = format!("{}{MAIN_SEPARATOR}nov", dir.path().display());
        let (text, _) = complete(&abs, dir.path());
        assert!(text.unwrap().ends_with("novel.txt"));
        let (_, said) = complete("zz", dir.path());
        assert_eq!(said, "No file or folder starts with zz.");
        let (_, said) = complete("", dir.path());
        assert!(
            said.starts_with("3 matches"),
            "hidden files are left out: {said}"
        );
        let (_, said) = complete("missing/x", dir.path());
        assert!(said.starts_with("There is no folder"), "{said}");
    }

    #[test]
    fn several_matches_complete_to_what_they_share() {
        let dir = tempfile::tempdir().unwrap();
        for n in ["chapter-01.md", "chapter-02.md", "chapter-10.md"] {
            std::fs::write(dir.path().join(n), "").unwrap();
        }
        let (text, said) = complete("ch", dir.path());
        assert_eq!(text.as_deref(), Some("chapter-"));
        assert!(said.starts_with("3 matches: chapter-01.md"), "{said}");
    }
}
