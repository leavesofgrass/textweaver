//! Autosave snapshots, crash recovery, and the save rule.
//!
//! Ported from `star/gui/mixin_autosave.py` and `_editing.py`
//! (docs/star-parity.md Part 3 §4.2 and §5) with these fixes:
//!
//! - snapshots are written to a unique temp file, synced, and renamed
//!   (Star used a fixed `<key>.tmp` and no fsync, §7 item 40);
//! - saves are atomic and keep the file's byte-order mark and line endings
//!   (Star used `write_text`, dropped the BOM, and wrote CRLF on Windows,
//!   item 29);
//! - Save never writes Markdown over a converted source (`.rst`, `.org`,
//!   `.adoc`, `.html`, ...): only Markdown and plain-text sources are saved
//!   in place (item 28), and Save As turns such an extension into `.md`.

use std::io::Write as _;
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
    /// A policy from the `[editing]` settings.
    pub fn new(enabled: bool, interval_secs: u32) -> Self {
        AutosavePolicy {
            enabled,
            interval: Duration::from_secs(u64::from(interval_secs.max(1))),
        }
    }

    /// True when a snapshot is due: enabled, dirty, and `interval` has passed
    /// since the last snapshot (or there has been none).
    pub fn due(&self, dirty: bool, since_last: Option<Duration>) -> bool {
        self.enabled && dirty && since_last.is_none_or(|d| d >= self.interval)
    }
}

/// A recovery snapshot, stored as `recovery/<doc-key>.json`.
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
    /// The document's title, for the recovery prompt.
    #[serde(default)]
    pub title: Option<String>,
}

impl RecoverySnapshot {
    /// The title to show: the stored title, else the file name, else
    /// "Recovered document" (Star's fallback).
    pub fn display_title(&self) -> String {
        self.title
            .clone()
            .filter(|t| !t.is_empty())
            .or_else(|| {
                self.path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "Recovered document".to_owned())
    }
}

/// Current time as Unix seconds.
pub(crate) fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// Writes `bytes` to `path` through a unique temp file in the same
/// directory, synced, then renamed over the target.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let tmp = dir.join(format!(".{name}.{}.{nanos}.tmp", std::process::id()));
    let result = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// `<dir>/<doc_key>.json`.
pub fn snapshot_file(dir: &Path, doc_key: &str) -> PathBuf {
    dir.join(format!("{doc_key}.json"))
}

/// Writes a snapshot to `<dir>/<doc_key>.json` atomically. Returns its path.
pub fn write_snapshot(dir: &Path, snapshot: &RecoverySnapshot) -> std::io::Result<PathBuf> {
    let path = snapshot_file(dir, &snapshot.doc_key);
    let text = serde_json::to_string(snapshot).map_err(std::io::Error::other)?;
    write_atomic(&path, text.as_bytes())?;
    Ok(path)
}

/// Deletes a snapshot file; a missing file is not an error.
pub fn delete_snapshot(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// Snapshots worth offering at startup, in file-name order (Star's
/// `_scan_snapshots`). Unreadable and malformed files are skipped. A
/// snapshot whose file already holds exactly its text was saved after all:
/// it is deleted and skipped. A snapshot whose file is missing is offered.
pub fn scan_snapshots(dir: &Path) -> Vec<(PathBuf, RecoverySnapshot)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for file in files {
        let Some(snap) = std::fs::read_to_string(&file)
            .ok()
            .and_then(|t| serde_json::from_str::<RecoverySnapshot>(&t).ok())
        else {
            continue;
        };
        let saved = snap
            .path
            .as_ref()
            .and_then(|p| std::fs::read(p).ok())
            .is_some_and(|bytes| decode(&bytes).0 == snap.text);
        if saved {
            let _ = std::fs::remove_file(&file);
            continue;
        }
        out.push((file, snap));
    }
    out
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

/// Extensions of files whose text is edited as-is and saved in place.
pub const IN_PLACE_EXTENSIONS: [&str; 7] =
    ["md", "markdown", "mdown", "mkd", "mkdn", "txt", "text"];

/// Star's save rule, fixed: save in place only for plain-text and Markdown
/// sources (by loader and by extension); everything else, including
/// `.rst`, `.org`, `.adoc`, and `.html`, becomes save-as-Markdown.
pub fn save_target(path: &Path, loader_id: &str) -> SaveTarget {
    let ext_ok = path.extension().is_none_or(|e| {
        let e = e.to_string_lossy().to_ascii_lowercase();
        IN_PLACE_EXTENSIONS.contains(&e.as_str())
    });
    match loader_id {
        "text" | "markdown" if ext_ok => SaveTarget::InPlace(path.to_owned()),
        _ => SaveTarget::SaveAsMarkdown {
            suggested: path.with_extension("md"),
        },
    }
}

/// The path Save As writes: the chosen path, with a converted-format
/// extension replaced by `.md` so Markdown never lands in an `.rst` or
/// `.html` file. No extension gets `.md`.
pub fn save_as_path(chosen: &Path) -> PathBuf {
    let keep = chosen.extension().is_some_and(|e| {
        let e = e.to_string_lossy().to_ascii_lowercase();
        IN_PLACE_EXTENSIONS.contains(&e.as_str())
    });
    if keep {
        chosen.to_owned()
    } else {
        chosen.with_extension("md")
    }
}

/// How a text file was encoded on disk, so a save can keep it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextFormat {
    /// Starts with a UTF-8 byte-order mark.
    pub bom: bool,
    /// Uses CRLF line endings.
    pub crlf: bool,
}

impl TextFormat {
    /// The format of `bytes`: a BOM, and CRLF when most line breaks are.
    pub fn detect(bytes: &[u8]) -> Self {
        let bom = bytes.starts_with(b"\xEF\xBB\xBF");
        let lf = bytes.iter().filter(|&&b| b == b'\n').count();
        let crlf = bytes.windows(2).filter(|w| w == b"\r\n").count();
        TextFormat {
            bom,
            crlf: lf > 0 && crlf * 2 > lf,
        }
    }

    /// `text` (with `\n` line breaks) encoded in this format.
    pub fn encode(self, text: &str) -> Vec<u8> {
        let mut out = Vec::with_capacity(text.len() + 3);
        if self.bom {
            out.extend_from_slice(b"\xEF\xBB\xBF");
        }
        if self.crlf {
            out.extend_from_slice(text.replace("\r\n", "\n").replace('\n', "\r\n").as_bytes());
        } else {
            out.extend_from_slice(text.as_bytes());
        }
        out
    }
}

/// Decodes file bytes for editing: strips a BOM and turns CRLF into `\n`.
/// Invalid UTF-8 is replaced. Returns the text and the format to save with.
pub fn decode(bytes: &[u8]) -> (String, TextFormat) {
    let format = TextFormat::detect(bytes);
    let body = if format.bom { &bytes[3..] } else { bytes };
    let text = String::from_utf8_lossy(body).replace("\r\n", "\n");
    (text, format)
}

/// Saves `text` to `path` atomically, keeping the existing file's BOM and
/// line endings (a new file gets `\n` and no BOM).
pub fn save_text(path: &Path, text: &str) -> std::io::Result<()> {
    let format = std::fs::read(path)
        .map(|b| TextFormat::detect(&b))
        .unwrap_or_default();
    write_atomic(path, &format.encode(text))
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
        assert!(!AutosavePolicy::new(false, 20).due(true, None));
        assert_eq!(
            AutosavePolicy::new(true, 0).interval,
            Duration::from_secs(1)
        );
    }

    #[test]
    fn save_rule() {
        assert_eq!(
            save_target(Path::new("a.md"), "markdown"),
            SaveTarget::InPlace("a.md".into())
        );
        assert_eq!(
            save_target(Path::new("a.TXT"), "text"),
            SaveTarget::InPlace("a.TXT".into())
        );
        assert_eq!(
            save_target(Path::new("README"), "text"),
            SaveTarget::InPlace("README".into())
        );
        for (p, loader) in [
            ("a.html", "html"),
            ("a.rst", "text"),
            ("a.org", "markdown"),
            ("a.adoc", "text"),
            ("a.pdf", "pdf"),
            ("a.md", "html"),
        ] {
            assert_eq!(
                save_target(Path::new(p), loader),
                SaveTarget::SaveAsMarkdown {
                    suggested: Path::new(p).with_extension("md")
                },
                "{p}"
            );
        }
        assert_eq!(save_as_path(Path::new("x.rst")), PathBuf::from("x.md"));
        assert_eq!(save_as_path(Path::new("x")), PathBuf::from("x.md"));
        assert_eq!(save_as_path(Path::new("x.txt")), PathBuf::from("x.txt"));
        assert_eq!(
            save_as_path(Path::new("x.markdown")),
            PathBuf::from("x.markdown")
        );
    }

    #[test]
    fn formats_round_trip() {
        let (t, f) = decode(b"\xEF\xBB\xBFa\r\nb\r\n");
        assert_eq!(t, "a\nb\n");
        assert_eq!(
            f,
            TextFormat {
                bom: true,
                crlf: true
            }
        );
        assert_eq!(f.encode(&t), b"\xEF\xBB\xBFa\r\nb\r\n");
        let (t, f) = decode(b"a\nb");
        assert_eq!((t.as_str(), f), ("a\nb", TextFormat::default()));
    }

    #[test]
    fn save_text_keeps_crlf_and_bom() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("w.txt");
        std::fs::write(&p, b"\xEF\xBB\xBFold\r\nline\r\n").unwrap();
        save_text(&p, "new\ntext\n").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"\xEF\xBB\xBFnew\r\ntext\r\n");
        let fresh = dir.path().join("n.md");
        save_text(&fresh, "a\nb").unwrap();
        assert_eq!(std::fs::read(&fresh).unwrap(), b"a\nb");
        let names: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(names.len(), 2, "no temp files left behind");
    }

    #[test]
    fn snapshot_title_fallbacks() {
        let mut s = RecoverySnapshot {
            doc_key: "k".into(),
            path: None,
            text: String::new(),
            ts: 0,
            title: None,
        };
        assert_eq!(s.display_title(), "Recovered document");
        s.path = Some("dir/notes.md".into());
        assert_eq!(s.display_title(), "notes.md");
        s.title = Some("Notes".into());
        assert_eq!(s.display_title(), "Notes");
    }
}
