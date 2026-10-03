use std::path::Path;

use crate::StoreError;

/// Writes `bytes` to `path` atomically: a temp file in the same directory,
/// synced, then renamed over the target, retrying while Windows reports
/// the file in use ([`textweaver_core::fs::write_atomic`]). Creates parent
/// directories as needed.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    textweaver_core::fs::write_atomic(path, bytes).map_err(|source| StoreError::Io {
        path: path.to_owned(),
        source,
    })
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
