//! Saving files: one atomic write for every crate.
//!
//! Before this module the workspace had seven atomic-write helpers with
//! different guarantees: only the editor's retried while Windows reported
//! the file in use, one flushed but never synced, and one named its
//! temporary file by process id alone, so two threads could collide. Every
//! crate that replaces a file now calls [`write_atomic`]:
//!
//! - the bytes go to a temporary file in the same folder, named uniquely
//!   per process and per call (`.<name>.<pid>.<n>.tmp`);
//! - the temporary file is synced to disk before it is renamed;
//! - the rename over the target is retried with a short backoff while
//!   Windows reports the target in use ([`RENAME_RETRY_DELAYS_MS`]), as it
//!   does while Obsidian, OneDrive, an antivirus scanner or the search
//!   indexer holds it;
//! - on any error the temporary file is removed and the target is left as
//!   it was.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Waits between attempts to rename a saved file into place while another
/// program has the target open (Windows only): four retries, about 0.4 s
/// in all.
pub const RENAME_RETRY_DELAYS_MS: [u64; 4] = [25, 50, 100, 200];

/// True for the errors Windows gives while another program holds a file:
/// access denied (5), sharing violation (32), and lock violation (33).
/// Always false elsewhere.
pub fn is_in_use(e: &std::io::Error) -> bool {
    cfg!(windows) && matches!(e.raw_os_error(), Some(5 | 32 | 33))
}

/// Runs `op`, retrying with a short backoff ([`RENAME_RETRY_DELAYS_MS`])
/// while it fails because Windows reports a file in use ([`is_in_use`]).
/// Other errors, and every error on other systems, are returned at once.
pub fn retry_while_in_use<T>(mut op: impl FnMut() -> std::io::Result<T>) -> std::io::Result<T> {
    let mut delays = RENAME_RETRY_DELAYS_MS.iter();
    loop {
        match op() {
            Err(e) if is_in_use(&e) => match delays.next() {
                Some(&ms) => std::thread::sleep(Duration::from_millis(ms)),
                None => return Err(e),
            },
            other => return other,
        }
    }
}

/// Renames `from` over `to`, retrying while Windows reports the target as
/// in use (see [`RENAME_RETRY_DELAYS_MS`]).
pub fn rename_with_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    retry_while_in_use(|| std::fs::rename(from, to))
}

/// Writes `bytes` to `path` atomically, creating its folder as needed: a
/// unique temporary file in the same folder, synced, then renamed over the
/// target with [`rename_with_retry`]. On error the target is unchanged and
/// no temporary file is left behind.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    write_atomic_with_permissions(path, bytes, None)
}

/// [`write_atomic`], giving the new file `permissions` (usually those of
/// the file it replaces) before it is renamed into place.
pub fn write_atomic_with_permissions(
    path: &Path,
    bytes: &[u8],
    permissions: Option<std::fs::Permissions>,
) -> std::io::Result<()> {
    let tmp = temp_path(path);
    if let Some(dir) = tmp.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let result = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        if let Some(p) = permissions {
            std::fs::set_permissions(&tmp, p)?;
        }
        rename_with_retry(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// The temporary file [`write_atomic`] writes before renaming it over
/// `path`: `.<name>.<pid>.<n>.tmp` in the same folder, unique per process
/// and per call, so two writers in one process never share one.
pub fn temp_path(path: &Path) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    dir.join(format!(".{name}.{}.{n}.tmp", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    #[test]
    fn writes_creates_folders_and_leaves_no_temp_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let p = dir.join("a").join("f.txt");
        write_atomic(&p, b"one").unwrap();
        write_atomic(&p, b"two").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"two");
        assert_eq!(entries(&dir.join("a")), ["f.txt"]);
    }

    #[test]
    fn temp_names_are_unique_and_beside_the_target() {
        let p = Path::new("folder").join("doc.md");
        let a = temp_path(&p);
        let b = temp_path(&p);
        assert_ne!(a, b);
        assert_eq!(a.parent(), Some(Path::new("folder")));
        let name = a.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            name.starts_with(".doc.md.") && name.ends_with(".tmp"),
            "{name}"
        );
        assert_eq!(temp_path(Path::new("bare")).parent(), Some(Path::new(".")));
    }

    #[test]
    fn a_failed_rename_leaves_the_target_and_no_temp_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        // A folder in the target's place: the rename fails on every system.
        let target = dir.join("taken");
        std::fs::create_dir_all(target.join("inside")).unwrap();
        assert!(write_atomic(&target, b"x").is_err());
        assert!(target.join("inside").is_dir());
        assert_eq!(entries(dir), ["taken"]);
    }

    #[test]
    fn other_errors_are_not_retried() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let started = std::time::Instant::now();
        let e = rename_with_retry(&dir.join("missing"), &dir.join("x")).unwrap_err();
        assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
        assert!(started.elapsed() < Duration::from_millis(20));
    }

    #[test]
    fn in_use_errors_are_windows_only() {
        let e = std::io::Error::from_raw_os_error(32);
        assert_eq!(is_in_use(&e), cfg!(windows));
        assert!(!is_in_use(&std::io::Error::other("other")));
    }

    #[test]
    fn retries_until_the_file_is_free() {
        let mut calls = 0;
        let r = retry_while_in_use(|| {
            calls += 1;
            if calls < 3 {
                Err(std::io::Error::from_raw_os_error(32))
            } else {
                Ok(calls)
            }
        });
        if cfg!(windows) {
            assert_eq!(r.unwrap(), 3);
        } else {
            assert_eq!(r.unwrap_err().raw_os_error(), Some(32));
            assert_eq!(calls, 1);
        }
    }
}
