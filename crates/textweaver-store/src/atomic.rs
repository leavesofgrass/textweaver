use std::io::Write;
use std::path::Path;

use crate::StoreError;

/// Writes `bytes` to `path` atomically: a temp file in the same directory,
/// flushed and synced, then renamed over the target. Creates parent
/// directories as needed.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let io = |source| StoreError::Io {
        path: path.to_owned(),
        source,
    };
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(io)?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    // Unique per process and per call, so two writers in one process
    // (threads, or two stores over one directory) never share a temp file.
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = dir.join(format!(".{name}.{}.{n}.tmp", std::process::id()));
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
    result.map_err(io)
}

/// Renames a file that exists but cannot be parsed to
/// `<name>.corrupt-<unix time>.bak` beside it, so the next save cannot
/// overwrite what it holds, and logs why. Returns the backup's path.
pub(crate) fn set_aside(path: &Path, why: &dyn std::fmt::Display) -> Option<std::path::PathBuf> {
    let backup = path.with_extension(format!("corrupt-{}.bak", crate::now_ts()));
    match std::fs::rename(path, &backup) {
        Ok(()) => {
            log::warn!(
                "{} could not be read ({why}); kept as {}",
                path.display(),
                backup.display()
            );
            Some(backup)
        }
        Err(e) => {
            log::warn!(
                "{} could not be read ({why}) or set aside ({e})",
                path.display()
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a").join("f.txt");
        atomic_write(&p, b"one").unwrap();
        atomic_write(&p, b"two").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "two");
        assert_eq!(std::fs::read_dir(p.parent().unwrap()).unwrap().count(), 1);
    }
}
