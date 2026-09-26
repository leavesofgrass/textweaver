//! A small rotating log file for the `log` crate.
//!
//! Failures that are not worth interrupting the reader for (a position
//! save that failed, a corrupt state file set aside, a document out of step
//! with the editor) go to `log::warn!`; without a logger they were lost
//! (docs/audit-2026-09.md, finding D5). Frontends call [`init`] once: the
//! log goes to `textweaver.log` in the state directory, rotated at
//! [`MAX_BYTES`] with [`KEEP`] older files (`textweaver.log.1`, ...). The
//! level is low by default (warnings and errors) and set by the `--log`
//! flag or `TEXTWEAVER_LOG` (`off`, `error`, `warn`, `info`, `debug`,
//! `trace`).

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use log::{LevelFilter, Log, Metadata, Record};
use textweaver_store::Paths;

/// Size at which the log file is rotated.
pub const MAX_BYTES: u64 = 1024 * 1024;

/// Older log files kept (`textweaver.log.1` is the newest of them).
pub const KEEP: usize = 3;

/// The level used when neither `--log` nor `TEXTWEAVER_LOG` sets one.
pub const DEFAULT_LEVEL: LevelFilter = LevelFilter::Warn;

/// Where the log file goes for `paths`.
pub fn log_path(paths: &Paths) -> PathBuf {
    paths.state_dir().join("textweaver.log")
}

/// Parses a level name (`off`, `error`, `warn`, `info`, `debug`, `trace`;
/// any case). `None` for anything else.
pub fn parse_level(s: &str) -> Option<LevelFilter> {
    s.trim().parse().ok()
}

/// The level to log at: `flag` (from `--log`) if given, else the
/// `TEXTWEAVER_LOG` variable, else [`DEFAULT_LEVEL`]. An unknown name is
/// an error that names the choices.
pub fn choose_level(flag: Option<&str>, env: Option<&str>) -> Result<LevelFilter, String> {
    match flag.or(env).map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(DEFAULT_LEVEL),
        Some(s) => parse_level(s).ok_or_else(|| {
            format!("Unknown log level {s}. Use off, error, warn, info, debug, or trace.")
        }),
    }
}

/// A `log` backend writing one line per record to a rotating file.
#[derive(Debug)]
pub struct FileLogger {
    level: LevelFilter,
    inner: Mutex<Inner>,
}

#[derive(Debug)]
struct Inner {
    path: PathBuf,
    file: Option<File>,
    written: u64,
    max_bytes: u64,
}

impl FileLogger {
    /// A logger writing records at `level` or more severe to `path`,
    /// rotating at `max_bytes`. The file is created on the first record.
    pub fn new(path: PathBuf, level: LevelFilter, max_bytes: u64) -> Self {
        FileLogger {
            level,
            inner: Mutex::new(Inner {
                path,
                file: None,
                written: 0,
                max_bytes: max_bytes.max(1),
            }),
        }
    }

    /// The level this logger writes.
    pub fn level(&self) -> LevelFilter {
        self.level
    }
}

/// Renames `path` to `path.1`, `path.1` to `path.2`, and so on, keeping
/// [`KEEP`] old files.
fn rotate(path: &Path) {
    let numbered = |n: usize| {
        let mut p = path.as_os_str().to_owned();
        p.push(format!(".{n}"));
        PathBuf::from(p)
    };
    let _ = std::fs::remove_file(numbered(KEEP));
    for n in (1..KEEP).rev() {
        let _ = std::fs::rename(numbered(n), numbered(n + 1));
    }
    let _ = std::fs::rename(path, numbered(1));
}

impl Inner {
    fn open(&mut self) -> Option<&mut File> {
        if self.file.is_none() {
            if let Some(dir) = self.path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let size = std::fs::metadata(&self.path).map_or(0, |m| m.len());
            if size >= self.max_bytes {
                rotate(&self.path);
            }
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .ok()?;
            self.written = std::fs::metadata(&self.path).map_or(0, |m| m.len());
            self.file = Some(file);
        }
        self.file.as_mut()
    }

    fn write_line(&mut self, line: &str) {
        if self.written >= self.max_bytes {
            self.file = None;
            rotate(&self.path);
        }
        let Some(file) = self.open() else {
            return;
        };
        if file.write_all(line.as_bytes()).is_ok() {
            self.written += line.len() as u64;
        }
    }
}

/// `YYYY-MM-DD hh:mm:ss` in UTC for a Unix time in seconds.
pub fn utc_timestamp(secs: u64) -> String {
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let rem = secs % 86_400;
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let line = format!(
            "{} UTC {:<5} {}: {}\n",
            utc_timestamp(secs),
            record.level(),
            record.target(),
            record.args()
        );
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.write_line(&line);
    }

    fn flush(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(f) = inner.file.as_mut() {
            let _ = f.flush();
        }
    }
}

/// Installs the file logger for `paths` at `level` (nothing at
/// [`LevelFilter::Off`]). Returns the log file's path, or `None` when
/// logging is off or a logger was already installed.
pub fn init(paths: &Paths, level: LevelFilter) -> Option<PathBuf> {
    if level == LevelFilter::Off {
        return None;
    }
    let path = log_path(paths);
    let logger = FileLogger::new(path.clone(), level, MAX_BYTES);
    // One logger for the life of the process.
    let logger: &'static FileLogger = Box::leak(Box::new(logger));
    log::set_logger(logger).ok()?;
    log::set_max_level(level);
    log::info!(
        "textweaver {} started (log level {level})",
        env!("CARGO_PKG_VERSION")
    );
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(logger: &FileLogger, level: log::Level, msg: &str) {
        logger.log(
            &Record::builder()
                .level(level)
                .target("test")
                .args(format_args!("{msg}"))
                .build(),
        );
        logger.flush();
    }

    #[test]
    fn levels_filter_and_lines_are_written() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state").join("textweaver.log");
        let logger = FileLogger::new(path.clone(), LevelFilter::Warn, MAX_BYTES);
        record(&logger, log::Level::Info, "not written");
        assert!(!path.exists(), "nothing logged, no file");
        record(&logger, log::Level::Warn, "cannot save position: disk full");
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("WARN  test: cannot save position: disk full"));
        assert!(!text.contains("not written"));
        assert_eq!(text.lines().count(), 1);
    }

    #[test]
    fn the_file_rotates_and_keeps_three_old_ones() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("textweaver.log");
        let logger = FileLogger::new(path.clone(), LevelFilter::Trace, 200);
        for i in 0..40 {
            record(
                &logger,
                log::Level::Error,
                &format!("message number {i:03}"),
            );
        }
        let size = std::fs::metadata(&path).unwrap().len();
        assert!(size < 400, "{size}");
        for n in 1..=KEEP {
            assert!(dir.path().join(format!("textweaver.log.{n}")).exists());
        }
        assert!(
            !dir.path()
                .join(format!("textweaver.log.{}", KEEP + 1))
                .exists()
        );
        let newest = std::fs::read_to_string(&path).unwrap();
        assert!(newest.contains("message number 039"));
        // A new logger over a full file rotates it first.
        drop(logger);
        std::fs::write(&path, "x".repeat(300)).unwrap();
        let logger = FileLogger::new(path.clone(), LevelFilter::Trace, 200);
        record(&logger, log::Level::Error, "fresh");
        assert!(std::fs::read_to_string(&path).unwrap().starts_with("20"));
    }

    #[test]
    fn levels_come_from_the_flag_then_the_environment() {
        assert_eq!(choose_level(None, None), Ok(DEFAULT_LEVEL));
        assert_eq!(
            choose_level(Some("debug"), Some("off")),
            Ok(LevelFilter::Debug)
        );
        assert_eq!(choose_level(None, Some("OFF")), Ok(LevelFilter::Off));
        assert_eq!(choose_level(None, Some("")), Ok(DEFAULT_LEVEL));
        assert!(
            choose_level(Some("loud"), None)
                .unwrap_err()
                .contains("Unknown log level loud")
        );
    }

    #[test]
    fn timestamps_are_utc_dates() {
        assert_eq!(utc_timestamp(0), "1970-01-01 00:00:00");
        // 2026-09-25 12:34:56 UTC.
        assert_eq!(utc_timestamp(1_790_339_696), "2026-09-25 12:34:56");
        assert_eq!(utc_timestamp(951_782_400), "2000-02-29 00:00:00");
    }
}
