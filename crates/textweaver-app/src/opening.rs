//! Opening documents in the background (Wave 3, Agent W3a).
//!
//! A large PDF or EPUB takes seconds to load; the keyboard must not wait.
//! [`Command::Open`](crate::Command::Open), and choosing from the library,
//! load a file of [`BACKGROUND_OPEN_BYTES`] or more on a helper thread:
//!
//! - "Opening report.pdf. Escape cancels." is said at once;
//! - every [`PROGRESS_EVERY`] it says "Still opening report.pdf, 4
//!   seconds.";
//! - Escape ([`Command::Cancel`](crate::Command::Cancel)) cancels: "Stopped
//!   opening report.pdf." The loader cannot be interrupted, so its thread
//!   finishes on its own and the result is dropped;
//! - when it is loaded, [`App::tick`](crate::App::tick) makes it current as
//!   [`App::open`](crate::App::open) does, and the waker rings
//!   ([`crate::wake`]).
//!
//! Smaller files open at once, as before, except those that may be slow
//! whatever their size (Agent W3d): PDFs and pictures, whose pages may need
//! text recognition (OCR, ADR-0026), archives, and web addresses. While
//! pages are recognized, "Still opening" says which page ("Still opening
//! scan.pdf: recognizing text on page 3 (3 of 40)."), and Escape stops the
//! recognition before its next page as well. [`App::open`] itself always
//! opens at once (the command line, tests, and callers that want the
//! result).

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use textweaver_formats::{LoadError, Progress, Registry, Source};
use textweaver_text::Document;

use crate::app::App;
use crate::command::Effect;
use crate::disk::FileStamp;

/// Files at least this large open in the background (512 KiB).
pub const BACKGROUND_OPEN_BYTES: u64 = 512 * 1024;

/// How often a slow open says it is still going.
pub const PROGRESS_EVERY: Duration = Duration::from_secs(3);

/// What the loading thread sends back.
type Loaded = Result<(Document, Option<FileStamp>), LoadError>;

/// A document being opened in the background.
pub(crate) struct Opening {
    path: PathBuf,
    name: String,
    rx: Receiver<Loaded>,
    started: Instant,
    last_said: Instant,
    /// The loader's progress handle: cancelling it stops OCR.
    progress: Progress,
    /// The loader's latest report ("Recognizing text on page 3 (3 of
    /// 40).").
    report: Arc<Mutex<Option<String>>>,
}

impl std::fmt::Debug for Opening {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Opening")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

/// The file name of `path`, for announcements.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

impl App {
    /// True while a document is opening in the background.
    pub fn opening(&self) -> bool {
        self.opening.is_some()
    }

    /// The file opening in the background, and for how long it has been
    /// loading, for a frontend's progress display.
    pub fn opening_progress(&self, now: Instant) -> Option<(&Path, Duration)> {
        self.opening
            .as_ref()
            .map(|o| (o.path.as_path(), now.saturating_duration_since(o.started)))
    }

    /// Sets the size from which files open in the background (default
    /// [`BACKGROUND_OPEN_BYTES`]; 0 opens every file in the background,
    /// `u64::MAX` none).
    pub fn set_background_open_threshold(&mut self, bytes: u64) {
        self.background_open_bytes = bytes;
    }

    /// Opens `path` in the background when it is large or may be slow,
    /// else at once (announcing failures either way).
    pub(crate) fn open_maybe_in_background(&mut self, path: &Path) -> Vec<Effect> {
        let size = std::fs::metadata(path).map_or(0, |m| m.len());
        let slow = self.background_open_bytes != u64::MAX && may_be_slow(path);
        if size < self.background_open_bytes && !slow {
            return self.open_now(path);
        }
        self.begin_opening(path)
    }

    /// Opens `path` at once, announcing a failure.
    pub(crate) fn open_now(&mut self, path: &Path) -> Vec<Effect> {
        match self.open(path) {
            Ok(e) => e,
            Err(e) => {
                let name = path.display();
                self.error(&format!("Could not open {name}: {e}"));
                vec![Effect::Redraw]
            }
        }
    }

    /// Starts loading `path` on a helper thread.
    fn begin_opening(&mut self, path: &Path) -> Vec<Effect> {
        if let Some(o) = self.opening.take() {
            // A newer open replaces one still loading.
            o.progress.cancel();
            self.note(&format!("Stopped opening {}.", o.name));
        }
        let (tx, rx) = channel::<Loaded>();
        let mut options = self.load_options();
        let report: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let latest = Arc::clone(&report);
        let progress = Progress::new(move |r| {
            if let Ok(mut slot) = latest.lock() {
                *slot = Some(r.message.clone());
            }
        });
        options.progress = progress.clone();
        let wake = self.waker_slot();
        let owned = path.to_owned();
        let spawned = std::thread::Builder::new()
            .name("textweaver-open".into())
            .spawn(move || {
                // Taken before reading, so a change made while loading is
                // noticed.
                let stamp = FileStamp::of(&owned);
                let result = Registry::with_builtins()
                    .load(&Source::Path(owned), &options)
                    .map(|doc| (doc, stamp));
                let _ = tx.send(result);
                wake.wake();
            });
        if let Err(e) = spawned {
            log::warn!("cannot open in the background ({e}); opening at once");
            return self.open_now(path);
        }
        let now = Instant::now();
        let name = file_name(path);
        self.tell(&format!("Opening {name}. Escape cancels."));
        self.opening = Some(Opening {
            path: path.to_owned(),
            name,
            rx,
            started: now,
            last_said: now,
            progress,
            report,
        });
        vec![Effect::Redraw]
    }

    /// Escape while a document is opening: stops waiting for it.
    pub(crate) fn cancel_opening(&mut self) -> Vec<Effect> {
        if let Some(o) = self.opening.take() {
            o.progress.cancel();
            self.tell(&format!("Stopped opening {}.", o.name));
        }
        vec![Effect::Redraw]
    }

    /// From [`App::tick`]: makes a loaded document current, or says how
    /// the loading is getting on.
    pub(crate) fn opening_tick(&mut self, now: Instant) -> Vec<Effect> {
        let Some(o) = self.opening.as_mut() else {
            return Vec::new();
        };
        let result = match o.rx.try_recv() {
            Ok(r) => r,
            Err(TryRecvError::Empty) => {
                if now.saturating_duration_since(o.last_said) >= PROGRESS_EVERY {
                    o.last_said = now;
                    let secs = now.saturating_duration_since(o.started).as_secs();
                    let step = o.report.lock().ok().and_then(|r| r.clone());
                    let msg = match step {
                        // "Still opening scan.pdf: recognizing text on page 3 (3 of 40)."
                        Some(step) => {
                            let mut chars = step.chars();
                            let lower: String = chars
                                .next()
                                .map(|c| c.to_lowercase().chain(chars).collect())
                                .unwrap_or_default();
                            format!("Still opening {}: {lower}", o.name)
                        }
                        None => format!("Still opening {}, {secs} seconds.", o.name),
                    };
                    self.show(&msg);
                }
                return Vec::new();
            }
            Err(TryRecvError::Disconnected) => {
                let name = o.name.clone();
                self.opening = None;
                self.error(&format!(
                    "Could not open {name}: loading stopped unexpectedly."
                ));
                return vec![Effect::Redraw];
            }
        };
        let Some(o) = self.opening.take() else {
            return Vec::new();
        };
        match result {
            Ok((doc, stamp)) => self.adopt_loaded(&o.path, doc, stamp),
            Err(e) => {
                self.error(&format!("Could not open {}: {e}", o.path.display()));
                vec![Effect::Redraw]
            }
        }
    }

    /// Waits until a background open finishes (or `timeout` passes) and
    /// applies it; for tests. True when none is left.
    pub fn wait_for_open(&mut self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            let _ = self.tick(Instant::now());
            if self.opening.is_none() {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

/// Files that may be slow to open whatever their size: PDFs and pictures
/// (text recognition), archives (a member deep inside), and web addresses.
fn may_be_slow(path: &Path) -> bool {
    if Source::Path(path.to_owned()).url().is_some() {
        return true;
    }
    let s = path.to_string_lossy().to_ascii_lowercase();
    if s.contains('!') && !path.exists() {
        return true;
    }
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .is_some_and(|e| {
            matches!(
                e.as_str(),
                "pdf" | "png" | "jpg" | "jpeg" | "zip" | "tar" | "tgz" | "gz" | "7z"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_archives_and_web_pages_may_be_slow() {
        assert!(may_be_slow(Path::new("scan.PDF")));
        assert!(may_be_slow(Path::new("photo.jpg")));
        assert!(may_be_slow(Path::new("https://example.org/page")));
        assert!(may_be_slow(Path::new("no-such.zip!notes.md")));
        assert!(!may_be_slow(Path::new("notes.md")));
    }
}
