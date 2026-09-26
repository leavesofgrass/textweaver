//! Hot-folder watching, with Star's `watch_*` semantics:
//!
//! - Files already in the folder are converted at start (they may have
//!   arrived while nothing was watching), then new ones as they appear.
//!   Only the folder itself is watched, not subfolders.
//! - A file is converted only once its size has stayed the same, and above
//!   zero, for `stable` (Star's `watch_stable_seconds`, 2 seconds), checked
//!   every `poll` (`watch_poll_interval`, half a second), and once it can
//!   be opened, so half-copied files are never read.
//! - After a successful conversion the source moves to `processed/` when
//!   `move_processed` is on (`watch_move_processed`, on by default); after
//!   a failure it always moves to `failed/`, so it is not retried forever.
//!   A name already taken there gets a timestamp, never overwriting.
//! - Every attempt is logged with a UTC timestamp to
//!   `<output>/textweaver-watch.log`.
//!
//! Filesystem events come from `notify`; the folder is also rescanned
//! every few seconds, so a missed event (network drives) only delays a file.
//! Deliberate difference from Star: an output with the same name is
//! replaced (atomically), since a file dropped again is usually a newer
//! version, where Star wrote `name (2).md`.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};

use notify::{RecursiveMode, Watcher};

use crate::{ConvertError, Converter, FileResult, Job, Status, extension};

/// Watch settings (Star's `watch_*` settings).
#[derive(Clone, Debug)]
pub struct WatchOptions {
    /// How long a file's size must hold still before it is converted.
    pub stable: Duration,
    /// How often sizes are checked.
    pub poll: Duration,
    /// Move converted sources to `processed/`.
    pub move_processed: bool,
    /// Folder name for converted sources.
    pub processed_dir: String,
    /// Folder name for sources that failed.
    pub failed_dir: String,
    /// Rescan the folder this often, in case an event was missed.
    pub rescan: Duration,
}

impl Default for WatchOptions {
    fn default() -> Self {
        WatchOptions {
            stable: Duration::from_secs(2),
            poll: Duration::from_millis(500),
            move_processed: true,
            processed_dir: "processed".to_owned(),
            failed_dir: "failed".to_owned(),
            rescan: Duration::from_secs(5),
        }
    }
}

/// Something the watcher did, for the caller to announce or print.
#[derive(Clone, Debug)]
pub enum WatchEvent {
    /// Watching began.
    Started {
        /// The watched folder.
        input: PathBuf,
        /// Where outputs go.
        output: PathBuf,
        /// True when filesystem events are available (else only rescans).
        events: bool,
    },
    /// A file was converted or failed.
    File(FileResult),
    /// A source was moved to `processed/` or `failed/`.
    Moved {
        /// Where it was.
        from: PathBuf,
        /// Where it is now.
        to: PathBuf,
    },
    /// A file was ignored (no reader for its type).
    Ignored(PathBuf),
    /// A problem that does not stop watching.
    Error(String),
    /// Watching stopped.
    Stopped,
}

impl WatchEvent {
    /// A line that reads well aloud.
    pub fn sentence(&self) -> String {
        let name = |p: &Path| {
            p.file_name().map_or_else(
                || p.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            )
        };
        match self {
            WatchEvent::Started { input, output, .. } => format!(
                "Watching {} and writing to {}. Press Control C to stop.",
                input.display(),
                output.display()
            ),
            WatchEvent::File(r) => match &r.status {
                Status::Converted if r.warnings.is_empty() => {
                    format!("Converted {}.", name(&r.source))
                }
                Status::Converted => format!(
                    "Converted {}. Warning: {}",
                    name(&r.source),
                    r.warnings.join(" ")
                ),
                Status::Skipped => format!("{} is up to date.", name(&r.source)),
                Status::Failed(reason) => {
                    format!("Could not convert {}: {reason}", name(&r.source))
                }
            },
            WatchEvent::Moved { from, to } => format!(
                "Moved {} to {}.",
                name(from),
                to.parent().map_or_else(String::new, &name)
            ),
            WatchEvent::Ignored(p) => {
                format!("Ignored {}: not a document type textweaver reads.", name(p))
            }
            WatchEvent::Error(e) => format!("Problem: {e}"),
            WatchEvent::Stopped => "Stopped watching.".to_owned(),
        }
    }
}

/// A file waiting to settle.
struct Pending {
    size: u64,
    since: Instant,
}

/// Watches `input` until `stop` is set, converting files into `output`
/// with `conv`, reporting through `on_event`.
pub fn watch(
    conv: &Converter,
    input: &Path,
    output: &Path,
    opts: &WatchOptions,
    stop: &AtomicBool,
    on_event: &mut dyn FnMut(&WatchEvent),
) -> Result<(), ConvertError> {
    if !input.is_dir() {
        return Err(ConvertError::Watch(
            input.to_owned(),
            "not a folder".to_owned(),
        ));
    }
    std::fs::create_dir_all(output).map_err(|e| ConvertError::Io(output.to_owned(), e))?;
    let log_path = output.join("textweaver-watch.log");
    let mut emit = |e: WatchEvent| {
        log_line(&log_path, &e);
        on_event(&e);
    };

    let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
    let mut watcher = notify::recommended_watcher(tx).ok();
    let events = match watcher.as_mut() {
        Some(w) => w.watch(input, RecursiveMode::NonRecursive).is_ok(),
        None => false,
    };
    emit(WatchEvent::Started {
        input: input.to_owned(),
        output: output.to_owned(),
        events,
    });

    let exts = conv.source_extensions();
    let out_ext = conv.options().to.extension();
    let mut pending: HashMap<PathBuf, Pending> = HashMap::new();
    let mut ignored: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    let mut last_scan: Option<Instant> = None;
    let input_canon = input.canonicalize().ok();

    while !stop.load(Ordering::SeqCst) {
        // New candidates: events, and a periodic rescan.
        let mut seen: Vec<PathBuf> = Vec::new();
        match rx.recv_timeout(opts.poll) {
            Ok(Ok(ev)) => seen.extend(ev.paths),
            Ok(Err(e)) => emit(WatchEvent::Error(e.to_string())),
            Err(_) => {}
        }
        while let Ok(ev) = rx.try_recv() {
            if let Ok(ev) = ev {
                seen.extend(ev.paths);
            }
        }
        if last_scan.is_none_or(|t| t.elapsed() >= opts.rescan) || !events {
            last_scan = Some(Instant::now());
            if let Ok(dir) = std::fs::read_dir(input) {
                seen.extend(dir.flatten().map(|e| e.path()));
            }
        }
        for p in seen {
            // Only files directly in the folder (not processed/ or failed/).
            let parent_ok = p.parent() == Some(input)
                || p.parent().and_then(|d| d.canonicalize().ok()) == input_canon;
            if !parent_ok {
                continue;
            }
            if !p.is_file() || pending.contains_key(&p) || is_temporary(&p) {
                continue;
            }
            let ext = extension(&p);
            if !exts.contains(&ext.as_str()) || (ext == out_ext && output == input) {
                if ignored.insert(p.clone()) {
                    emit(WatchEvent::Ignored(p));
                }
                continue;
            }
            pending.insert(
                p,
                Pending {
                    size: u64::MAX,
                    since: Instant::now(),
                },
            );
        }

        // Convert files that have settled.
        let mut ready: Vec<PathBuf> = Vec::new();
        pending.retain(|p, st| {
            let Ok(meta) = std::fs::metadata(p) else {
                return false; // gone
            };
            let size = meta.len();
            if size != st.size || size == 0 {
                st.size = size;
                st.since = Instant::now();
                return true;
            }
            if st.since.elapsed() >= opts.stable && std::fs::File::open(p).is_ok() {
                ready.push(p.clone());
                return false;
            }
            true
        });
        ready.sort();
        for source in ready {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            let name = source
                .file_name()
                .map_or_else(|| PathBuf::from("output"), PathBuf::from);
            let job = Job {
                output: output.join(name.with_extension(out_ext)),
                source: source.clone(),
                root: input.to_owned(),
            };
            let result = conv.convert_job(&job);
            let failed = matches!(result.status, Status::Failed(_));
            emit(WatchEvent::File(result));
            let dest = if failed {
                Some(&opts.failed_dir)
            } else if opts.move_processed {
                Some(&opts.processed_dir)
            } else {
                None
            };
            if let Some(dir) = dest {
                match move_aside(&source, &input.join(dir)) {
                    Ok(to) => emit(WatchEvent::Moved { from: source, to }),
                    Err(e) => emit(WatchEvent::Error(format!(
                        "could not move {}: {e}",
                        source.display()
                    ))),
                }
            }
        }
    }
    drop(watcher);
    emit(WatchEvent::Stopped);
    Ok(())
}

/// Partial downloads and editor lock files.
fn is_temporary(p: &Path) -> bool {
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    name.starts_with('.')
        || name.starts_with("~$")
        || [".tmp", ".part", ".crdownload", ".download", ".partial", "~"]
            .iter()
            .any(|s| name.ends_with(s))
}

/// Moves `src` into `dir`, adding a timestamp (and a counter) to the name
/// when it is taken.
fn move_aside(src: &Path, dir: &Path) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let name = src.file_name().unwrap_or_default();
    let mut dest = dir.join(name);
    if dest.exists() {
        let stem = src.file_stem().unwrap_or_default().to_string_lossy();
        let ext = src
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        let stamp = compact_utc(SystemTime::now());
        dest = dir.join(format!("{stem}.{stamp}{ext}"));
        let mut n = 2;
        while dest.exists() {
            dest = dir.join(format!("{stem}.{stamp} ({n}){ext}"));
            n += 1;
        }
    }
    match std::fs::rename(src, &dest) {
        Ok(()) => Ok(dest),
        Err(_) => {
            // Across volumes: copy, then remove.
            std::fs::copy(src, &dest)?;
            std::fs::remove_file(src)?;
            Ok(dest)
        }
    }
}

fn log_line(path: &Path, e: &WatchEvent) {
    let line = format!("{}  {}\n", iso_utc(SystemTime::now()), e.sentence());
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = f.write_all(line.as_bytes());
    }
}

/// Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn utc_parts(t: SystemTime) -> (i64, u32, u32, u64, u64, u64) {
    let secs = t
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (y, mo, d) = civil((secs / 86_400) as i64);
    let rem = secs % 86_400;
    (y, mo, d, rem / 3600, rem % 3600 / 60, rem % 60)
}

/// `2026-09-25T14:03:07Z`.
pub fn iso_utc(t: SystemTime) -> String {
    let (y, mo, d, h, mi, s) = utc_parts(t);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// `20260925-140307`, for file names.
fn compact_utc(t: SystemTime) -> String {
    let (y, mo, d, h, mi, s) = utc_parts(t);
    format!("{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_formatting() {
        // 2026-09-25 00:00:00 UTC is 20,721 days after the epoch.
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(20_721 * 86_400 + 3_723);
        assert_eq!(iso_utc(t), "2026-09-25T01:02:03Z");
        assert_eq!(compact_utc(t), "20260925-010203");
        assert_eq!(iso_utc(SystemTime::UNIX_EPOCH), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn temporary_names() {
        assert!(is_temporary(Path::new("a/~$report.docx")));
        assert!(is_temporary(Path::new("a/file.md.part")));
        assert!(is_temporary(Path::new("a/.hidden.md")));
        assert!(!is_temporary(Path::new("a/notes.md")));
    }

    #[test]
    fn move_aside_never_overwrites() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dest = dir.path().join("processed");
        for _ in 0..3 {
            let src = dir.path().join("a.md");
            std::fs::write(&src, "x").expect("write");
            let to = move_aside(&src, &dest).expect("move");
            assert!(to.exists());
            assert!(!src.exists());
        }
        assert_eq!(std::fs::read_dir(&dest).expect("read").count(), 3);
    }
}
