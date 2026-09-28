//! The diagnostic log (`--log`): announcements, commands, keys, caret sync,
//! and load timing, one line each, to standard error or to `--log-file`.
//! The UI Automation report reads it.

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

static FILE: OnceLock<Mutex<File>> = OnceLock::new();

/// Sends log lines to `path` instead of standard error.
pub fn to_file(path: &Path) -> std::io::Result<()> {
    let file = File::create(path)?;
    let _ = FILE.set(Mutex::new(file));
    Ok(())
}

/// True when log lines go to a `--log-file`.
pub fn to_file_active() -> bool {
    FILE.get().is_some()
}

/// Writes one line.
pub fn line(text: &str) {
    match FILE.get() {
        Some(file) => {
            if let Ok(mut f) = file.lock() {
                let _ = writeln!(f, "{text}");
                let _ = f.flush();
            }
        }
        None => eprintln!("{text}"),
    }
}
