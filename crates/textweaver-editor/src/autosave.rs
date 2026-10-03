//! Autosave snapshots, crash recovery, and the save rule.
//!
//! Ported from `star/gui/mixin_autosave.py` and `_editing.py`
//! (the Star parity reference Part 3 §4.2 and §5) with these fixes:
//!
//! - snapshots are written to a unique temp file, synced, and renamed
//!   (Star used a fixed `<key>.tmp` and no fsync, §7 item 40);
//! - saves are atomic and keep the file's byte-order mark and line endings
//!   (Star used `write_text`, dropped the BOM, and wrote CRLF on Windows,
//!   item 29);
//! - Save never writes Markdown over a converted source (`.rst`, `.org`,
//!   `.adoc`, `.html`, ...): only Markdown and plain-text sources are saved
//!   in place (item 28), and Save As turns such an extension into `.md`;
//! - an instance editing a document holds a lock on its snapshot
//!   ([`SnapshotLock`], `File::try_lock`), so a second instance neither
//!   offers to recover a snapshot that is still being written nor
//!   overwrites it (item 39).

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
/// directory, synced, then renamed over the target, retrying while Windows
/// reports the file in use ([`textweaver_core::fs::write_atomic`]).
pub use textweaver_core::fs::write_atomic;

/// Waits between attempts to rename a saved file into place while another
/// program has the target open (Windows only).
pub use textweaver_core::fs::RENAME_RETRY_DELAYS_MS;

/// Renames `from` over `to`, retrying while Windows reports the target as
/// in use.
pub use textweaver_core::fs::rename_with_retry;

use textweaver_core::fs::{retry_while_in_use, write_atomic_with_permissions};

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

/// `<dir>/<doc_key>.lock`, the lock file guarding a snapshot.
pub fn lock_file(dir: &Path, doc_key: &str) -> PathBuf {
    dir.join(format!("{doc_key}.lock"))
}

/// An exclusive lock on one document's snapshot, held by the instance
/// editing it for as long as it edits (Star had no lock, so a second
/// instance offered to "recover" the first one's live work, item 39).
///
/// The lock is an operating-system file lock (`File::try_lock`), so it
/// ends when the holder exits, even after a crash: a crashed instance's
/// snapshot is offered again, a running instance's is not. Dropping the
/// lock releases it and removes the lock file.
#[derive(Debug)]
pub struct SnapshotLock {
    file: std::fs::File,
    path: PathBuf,
}

impl SnapshotLock {
    /// Takes the lock for `doc_key`'s snapshot in `dir`. `Ok(None)` when
    /// another instance (or another session in this one) holds it.
    pub fn acquire(dir: &Path, doc_key: &str) -> std::io::Result<Option<SnapshotLock>> {
        std::fs::create_dir_all(dir)?;
        let path = lock_file(dir, doc_key);
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)?;
        match file.try_lock() {
            Ok(()) => {
                let mut f = &file;
                let _ = f.set_len(0);
                let _ = write!(f, "{}", std::process::id());
                Ok(Some(SnapshotLock { file, path }))
            }
            Err(std::fs::TryLockError::WouldBlock) => Ok(None),
            Err(std::fs::TryLockError::Error(e)) => Err(e),
        }
    }

    /// The lock file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SnapshotLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
        let _ = std::fs::remove_file(&self.path);
    }
}

/// True when a running instance holds the lock on `doc_key`'s snapshot.
/// A missing lock file means nobody does; a lock file that cannot be
/// opened counts as held, to be safe.
pub fn snapshot_in_use(dir: &Path, doc_key: &str) -> bool {
    let path = lock_file(dir, doc_key);
    let file = match std::fs::OpenOptions::new().write(true).open(&path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return false,
        Err(_) => return true,
    };
    match file.try_lock() {
        Ok(()) => {
            let _ = file.unlock();
            false
        }
        Err(_) => true,
    }
}

/// Deletes a snapshot file; a missing file is not an error.
pub fn delete_snapshot(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// Snapshots worth offering at startup, in file-name order (Star's
/// `_scan_snapshots`). Unreadable and malformed files are skipped, and so
/// are snapshots another running instance is still writing (it holds their
/// [`SnapshotLock`]). A snapshot whose file already holds exactly its text
/// was saved after all: it is deleted and skipped. A snapshot whose file is
/// missing is offered.
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
        if snapshot_in_use(dir, &snap.doc_key) {
            continue;
        }
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

/// The file name suggested for a new document with no title of its own.
pub const DEFAULT_FILE_NAME: &str = "document.md";

/// The title a Markdown text gives itself: the front matter's `title:`,
/// else the first heading (ATX `# Title` or a setext heading underlined
/// with `===` or `---`), skipping fenced code. `None` when it has neither.
pub fn document_title(text: &str) -> Option<String> {
    let mut lines = text.lines().peekable();
    // Front matter: a first line of `---`, closed by `---` or `...`.
    if lines.peek().is_some_and(|l| l.trim_end() == "---") {
        lines.next();
        let mut title = None;
        for line in lines.by_ref() {
            let t = line.trim_end();
            if t == "---" || t == "..." {
                break;
            }
            if title.is_none()
                && let Some(v) = t.strip_prefix("title:")
            {
                let v = v.trim().trim_matches(|c| c == '"' || c == '\'').trim();
                if !v.is_empty() {
                    title = Some(v.to_owned());
                }
            }
        }
        if title.is_some() {
            return title;
        }
    }
    let mut fence: Option<&str> = None;
    let mut previous: Option<&str> = None;
    for line in lines {
        let t = line.trim();
        if let Some(f) = fence {
            if t.starts_with(f) {
                fence = None;
            }
            previous = None;
            continue;
        }
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = Some(&t[..3]);
            previous = None;
            continue;
        }
        let hashes = t.chars().take_while(|&c| c == '#').count();
        if (1..=6).contains(&hashes) {
            let rest = &t[hashes..];
            if rest.is_empty() || rest.starts_with(' ') {
                let title = rest.trim().trim_end_matches('#').trim();
                if !title.is_empty() {
                    return Some(title.to_owned());
                }
            }
        }
        if let Some(p) = previous
            && !t.is_empty()
            && (t.chars().all(|c| c == '=') || (t.len() >= 2 && t.chars().all(|c| c == '-')))
        {
            return Some(p.to_owned());
        }
        previous = (!t.is_empty() && !t.starts_with(['>', '-', '*', '|'])).then_some(t);
    }
    None
}

/// A file name made from a title: letters and digits kept, lowercase, every
/// other run of characters a single hyphen, at most 60 characters, with
/// `.md`. "Methods and Results" becomes `methods-and-results.md`. `None`
/// when nothing usable is left.
pub fn file_name_for_title(title: &str) -> Option<String> {
    let mut slug = String::new();
    let mut gap = false;
    for c in title.chars() {
        if c.is_alphanumeric() {
            if gap && !slug.is_empty() {
                slug.push('-');
            }
            gap = false;
            slug.extend(c.to_lowercase());
        } else {
            gap = true;
        }
        if slug.chars().count() >= 60 {
            break;
        }
    }
    let slug: String = slug.chars().take(60).collect();
    let slug = slug.trim_matches('-');
    (!slug.is_empty()).then(|| format!("{slug}.md"))
}

/// The file name to suggest when saving `text` for the first time: from
/// its front matter title or first heading, else [`DEFAULT_FILE_NAME`].
pub fn suggest_file_name(text: &str) -> String {
    document_title(text)
        .and_then(|t| file_name_for_title(&t))
        .unwrap_or_else(|| DEFAULT_FILE_NAME.to_owned())
}

/// How a text file was encoded on disk, so a save can keep it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextFormat {
    /// Starts with a UTF-8 byte-order mark.
    pub bom: bool,
    /// Uses CRLF line endings.
    pub crlf: bool,
    /// Uses lone CR line endings (classic Mac OS text), which a save used to
    /// turn into LF.
    pub cr: bool,
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
            cr: lf == 0 && bytes.contains(&b'\r'),
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
        } else if self.cr {
            out.extend_from_slice(text.replace("\r\n", "\n").replace('\n', "\r").as_bytes());
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
///
/// An existing file is protected:
///
/// - a symbolic link is written through to its target (replacing the link
///   with a plain file would silently stop updating the real file);
/// - the new file keeps the old one's permissions (Unix mode bits);
/// - a read-only file is refused ([`std::io::ErrorKind::PermissionDenied`];
///   renaming over it would succeed on Unix and overwrite it anyway);
/// - a file that is not UTF-8 (Windows-1252, UTF-16) is refused
///   ([`std::io::ErrorKind::InvalidData`]): it was read with replacement
///   characters, and saving would write them back for good.
pub fn save_text(path: &Path, text: &str) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    let target = match std::fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => std::fs::canonicalize(path)?,
        _ => path.to_owned(),
    };
    let existing = match std::fs::metadata(&target) {
        Ok(m) => Some(m),
        Err(e) if e.kind() == ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };
    let mut format = TextFormat::default();
    let mut permissions = None;
    if let Some(meta) = existing {
        if meta.permissions().readonly() {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                "the file is read-only; use Save As to write a copy",
            ));
        }
        let bytes = retry_while_in_use(|| std::fs::read(&target))?;
        format = TextFormat::detect(&bytes);
        let body = if format.bom { &bytes[3..] } else { &bytes[..] };
        if std::str::from_utf8(body).is_err() {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "the file is not UTF-8 text, and saving would replace the characters \
                 that could not be read; use Save As to write a copy",
            ));
        }
        permissions = Some(meta.permissions());
    }
    write_atomic_with_permissions(&target, &format.encode(text), permissions)
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
                crlf: true,
                cr: false
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
    fn save_text_keeps_classic_mac_line_endings() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("mac.txt");
        std::fs::write(&p, b"one\rtwo\r").unwrap();
        let (text, format) = decode(&std::fs::read(&p).unwrap());
        assert!(format.cr && !format.crlf);
        assert_eq!(text, "one\rtwo\r", "decode leaves lone CRs to the loader");
        save_text(&p, "one\ntwo\nthree\n").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"one\rtwo\rthree\r");
    }

    #[test]
    fn save_text_refuses_read_only_and_non_utf8_files() {
        let dir = tempfile::tempdir().unwrap();
        // Windows-1252 smart quotes: read as replacement characters.
        let legacy = dir.path().join("legacy.md");
        let bytes = b"\x93quoted\x94 caf\xe9\n".to_vec();
        std::fs::write(&legacy, &bytes).unwrap();
        let e = save_text(&legacy, "\u{fffd}quoted\u{fffd} caf\u{fffd}\n").unwrap_err();
        assert_eq!(e.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(std::fs::read(&legacy).unwrap(), bytes, "left untouched");
        // UTF-16 with a byte-order mark.
        let wide = dir.path().join("wide.txt");
        std::fs::write(&wide, b"\xFF\xFEh\x00i\x00").unwrap();
        assert!(save_text(&wide, "hi").is_err());
        // Read-only.
        let ro = dir.path().join("ro.md");
        std::fs::write(&ro, "keep").unwrap();
        let mut perm = std::fs::metadata(&ro).unwrap().permissions();
        perm.set_readonly(true);
        std::fs::set_permissions(&ro, perm.clone()).unwrap();
        let e = save_text(&ro, "changed").unwrap_err();
        assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied);
        assert_eq!(std::fs::read_to_string(&ro).unwrap(), "keep");
        #[allow(clippy::permissions_set_readonly_false)]
        perm.set_readonly(false);
        std::fs::set_permissions(&ro, perm).unwrap();
    }

    /// Another program holding the file without sharing (as an antivirus
    /// scanner or OneDrive may for a moment) makes the rename fail with a
    /// sharing violation; the save retries and succeeds once it lets go,
    /// and gives up with the error, leaving no temporary file, when it
    /// does not.
    #[cfg(windows)]
    #[test]
    fn save_retries_while_another_program_holds_the_file() {
        use std::os::windows::fs::OpenOptionsExt;
        use std::sync::mpsc;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("held.md");
        std::fs::write(&p, "old").unwrap();
        let hold = |path: std::path::PathBuf| {
            std::fs::OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(path)
                .unwrap()
        };
        // Released while the save is retrying: it succeeds.
        let (tx, rx) = mpsc::channel();
        let path = p.clone();
        let holder = std::thread::spawn(move || {
            let f = hold(path);
            tx.send(()).unwrap();
            std::thread::sleep(Duration::from_millis(60));
            drop(f);
        });
        rx.recv().unwrap();
        save_text(&p, "new").unwrap();
        holder.join().unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "new");
        // Held for longer than every retry: the error comes back.
        let f = hold(p.clone());
        let e = write_atomic(&p, b"newer").unwrap_err();
        drop(f);
        assert!(matches!(e.raw_os_error(), Some(5 | 32 | 33)), "{e:?}");
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "new");
        let left: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(left.len(), 1, "no temporary file left behind");
    }

    #[test]
    fn other_rename_errors_are_not_retried() {
        let dir = tempfile::tempdir().unwrap();
        let started = std::time::Instant::now();
        let e = rename_with_retry(&dir.path().join("missing"), &dir.path().join("x")).unwrap_err();
        assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
        assert!(started.elapsed() < Duration::from_millis(20));
    }

    #[cfg(unix)]
    #[test]
    fn save_text_writes_through_links_and_keeps_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real.md");
        std::fs::write(&real, "old").unwrap();
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o600)).unwrap();
        let link = dir.path().join("link.md");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        save_text(&link, "new").unwrap();
        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(std::fs::read_to_string(&real).unwrap(), "new");
        let mode = std::fs::metadata(&real).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn snapshot_lock_is_exclusive_and_hides_live_snapshots() {
        let dir = tempfile::tempdir().unwrap();
        let snap = RecoverySnapshot {
            doc_key: "doc-1".into(),
            path: None,
            text: "unsaved".into(),
            ts: 1,
            title: None,
        };
        write_snapshot(dir.path(), &snap).unwrap();
        assert!(!snapshot_in_use(dir.path(), "doc-1"));
        let lock = SnapshotLock::acquire(dir.path(), "doc-1").unwrap().unwrap();
        assert!(lock.path().exists());
        assert!(snapshot_in_use(dir.path(), "doc-1"));
        assert!(
            SnapshotLock::acquire(dir.path(), "doc-1")
                .unwrap()
                .is_none(),
            "a second holder is refused"
        );
        assert!(
            scan_snapshots(dir.path()).is_empty(),
            "a snapshot still being written is not offered"
        );
        let lock_path = lock.path().to_owned();
        drop(lock);
        assert!(!lock_path.exists());
        assert!(!snapshot_in_use(dir.path(), "doc-1"));
        let offered = scan_snapshots(dir.path());
        assert_eq!(offered.len(), 1, "after the holder exits it is offered");
        assert!(
            SnapshotLock::acquire(dir.path(), "doc-1")
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn suggested_names_come_from_the_title() {
        assert_eq!(
            suggest_file_name("---\ntitle: \"My Essay: Draft 2\"\n---\n# Other\n"),
            "my-essay-draft-2.md"
        );
        assert_eq!(
            suggest_file_name("intro\n\n## Methods and Results ##\n"),
            "methods-and-results.md"
        );
        assert_eq!(
            suggest_file_name("Setext Title\n=====\n"),
            "setext-title.md"
        );
        assert_eq!(
            suggest_file_name("```\n# not a heading\n```\n# Real\n"),
            "real.md"
        );
        assert_eq!(suggest_file_name("#hashtag only\n"), DEFAULT_FILE_NAME);
        assert_eq!(suggest_file_name(""), DEFAULT_FILE_NAME);
        assert_eq!(suggest_file_name("# !!!\n"), DEFAULT_FILE_NAME);
        assert_eq!(
            suggest_file_name("---\nauthor: x\n---\n# Café Notes\n"),
            "café-notes.md"
        );
        let long = format!("# {}\n", "word ".repeat(40));
        let name = suggest_file_name(&long);
        assert!(name.chars().count() <= 63, "{name}");
        assert!(!name.contains("-.md"), "{name}");
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
