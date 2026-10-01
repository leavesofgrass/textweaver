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
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;
use textweaver_text::Document;

use crate::app::App;
use crate::command::Effect;
use crate::disk::FileStamp;

/// Files at least this large open in the background (512 KiB).
pub const BACKGROUND_OPEN_BYTES: u64 = 512 * 1024;

/// How often a slow open says it is still going.
pub const PROGRESS_EVERY: Duration = Duration::from_secs(3);

/// What the loading thread sends back: the document, the file's stamp,
/// and the text's stamp (computed there, off the input thread).
type Loaded = Result<(Document, Option<FileStamp>, textweaver_store::TextStamp), LoadError>;

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

/// Why `path` could not be opened, in plain words for the status line or
/// the terminal: a missing file, a folder, or a file that cannot be read
/// gets a sentence, and any other failure the loader's own message. The
/// operating system's error code is left out. Shared by the reader and
/// `tw`, so both say the same thing.
pub fn open_failure_reason(path: &Path, err: &LoadError) -> String {
    open_failure_reason_in(&Catalog::english(), path, err)
}

/// [`open_failure_reason`] in the catalog's language.
pub fn open_failure_reason_in(c: &Catalog, path: &Path, err: &LoadError) -> String {
    let name = file_name(path);
    if path.is_dir() {
        return c.fmt("opening-is-folder", &args!["name" => name]);
    }
    match err {
        LoadError::Io(_, io) => match io.kind() {
            std::io::ErrorKind::NotFound => {
                let folder = path
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .map(|p| p.display().to_string());
                match folder {
                    Some(f) => c.fmt("opening-no-file-in", &args!["name" => name, "folder" => f]),
                    None => c.fmt("opening-no-file-here", &args!["name" => name]),
                }
            }
            std::io::ErrorKind::PermissionDenied => c.tr("opening-no-permission"),
            _ => {
                let text = io.to_string();
                // "The system cannot find the path specified. (os error 3)":
                // the code adds nothing when read aloud.
                let plain = text.split(" (os error").next().unwrap_or(&text).trim();
                format!("{plain}.")
            }
        },
        LoadError::Parse(detail) if matches!(extension(path).as_str(), "rtf") => {
            log::warn!("{name}: {detail}");
            c.tr("opening-damaged-rtf")
        }
        LoadError::Parse(detail) if matches!(extension(path).as_str(), "odt" | "ott" | "fodt") => {
            log::warn!("{name}: {detail}");
            c.tr("opening-damaged-odt")
        }
        LoadError::Parse(detail) if matches!(extension(path).as_str(), "tex" | "latex" | "ltx") => {
            log::warn!("{name}: {detail}");
            c.tr("opening-damaged-latex")
        }
        LoadError::Parse(detail) if extension(path) == "eml" => {
            log::warn!("{name}: {detail}");
            c.tr("opening-damaged-email")
        }
        LoadError::Parse(detail) if matches!(extension(path).as_str(), "mhtml" | "mht") => {
            log::warn!("{name}: {detail}");
            c.tr("opening-damaged-mhtml")
        }
        // W6o: the formats of ADR-0044.
        LoadError::Parse(detail) => match extension(path).as_str() {
            "json" | "jsonl" | "ndjson" | "geojson" | "webmanifest" => {
                log::warn!("{name}: {detail}");
                c.tr("opening-damaged-json")
            }
            "ipynb" => {
                log::warn!("{name}: {detail}");
                c.tr("opening-damaged-notebook")
            }
            "svg" => {
                log::warn!("{name}: {detail}");
                c.tr("opening-damaged-svg")
            }
            "mml" | "mathml" => {
                log::warn!("{name}: {detail}");
                c.tr("opening-damaged-mathml")
            }
            _ => err.to_string(),
        },
        other => other.to_string(),
    }
}

/// The lowercase extension of `path`, or nothing.
fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// "Could not open NAME: REASON", for announcements.
pub fn open_failure_message(path: &Path, err: &LoadError) -> String {
    open_failure_message_in(&Catalog::english(), path, err)
}

/// [`open_failure_message`] in the catalog's language.
pub fn open_failure_message_in(c: &Catalog, path: &Path, err: &LoadError) -> String {
    c.fmt(
        "opening-failed",
        &args![
            "name" => file_name(path),
            "reason" => open_failure_reason_in(c, path, err)
        ],
    )
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
            Err(crate::app::AppError::Load(e)) => {
                let msg = open_failure_message_in(self.cat(), path, &e);
                self.error(&msg);
                vec![Effect::Redraw]
            }
            Err(e) => {
                let msg = self.msg_args(
                    "opening-failed",
                    &args!["name" => file_name(path), "reason" => e.to_string()],
                );
                self.error(&msg);
                vec![Effect::Redraw]
            }
        }
    }

    /// Starts loading `path` on a helper thread.
    fn begin_opening(&mut self, path: &Path) -> Vec<Effect> {
        if let Some(o) = self.opening.take() {
            // A newer open replaces one still loading.
            o.progress.cancel();
            let msg = self.msg_args("opening-stopped", &args!["name" => o.name.as_str()]);
            self.note(&msg);
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
                    .map(|doc| {
                        let text = crate::relocate::text_stamp(&doc);
                        (doc, stamp, text)
                    });
                let _ = tx.send(result);
                wake.wake();
            });
        if let Err(e) = spawned {
            log::warn!("cannot open in the background ({e}); opening at once");
            return self.open_now(path);
        }
        let now = Instant::now();
        let name = file_name(path);
        let msg = self.msg_args("opening-started", &args!["name" => name.as_str()]);
        self.tell(&msg);
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
            let msg = self.msg_args("opening-stopped", &args!["name" => o.name.as_str()]);
            self.tell(&msg);
        }
        vec![Effect::Redraw]
    }

    /// From [`App::tick`]: makes a loaded document current, or says how
    /// the loading is getting on.
    pub(crate) fn opening_tick(&mut self, now: Instant) -> Vec<Effect> {
        let cat = self.catalog();
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
                            cat.fmt(
                                "opening-still-step",
                                &args!["name" => o.name.as_str(), "step" => lower],
                            )
                        }
                        None => cat.fmt(
                            "opening-still",
                            &args!["name" => o.name.as_str(), "secs" => secs],
                        ),
                    };
                    self.show(&msg);
                }
                return Vec::new();
            }
            Err(TryRecvError::Disconnected) => {
                let name = o.name.clone();
                self.opening = None;
                let msg = self.msg_args("opening-stopped-unexpectedly", &args!["name" => name]);
                self.error(&msg);
                return vec![Effect::Redraw];
            }
        };
        let Some(o) = self.opening.take() else {
            return Vec::new();
        };
        match result {
            Ok((doc, stamp, text)) => self.adopt_loaded(&o.path, doc, stamp, Some(text)),
            Err(e) => {
                let msg = open_failure_message_in(self.cat(), &o.path, &e);
                self.error(&msg);
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

    /// Opening never waits for the writer on the input thread (Wave 6,
    /// W6f, from the performance lessons): with the disk stalled, a
    /// document opened again finds its position from the queued save, at
    /// once. Run with `--nocapture` for the numbers.
    ///
    /// The disk stays stalled until the test lets it go, so the check is
    /// the order of events, not a time: the open finished while the stall
    /// still held. (Before, it asserted the open took under 1.2 s, which
    /// failed on a loaded Windows runner on Wednesday, September 30, 2026.)
    #[test]
    fn opening_does_not_wait_for_a_slow_disk() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use textweaver_core::CharPos;
        use textweaver_store::{DocKey, Paths};

        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::under(&tmp.path().join("home"));
        let mut app = App::new(crate::AppConfig {
            paths: Some(paths),
            ..crate::AppConfig::for_tests()
        });
        let text = "Word after word, sentence after sentence. ".repeat(250_000);
        let doc = || textweaver_text::Document::from_plain_text(&text);
        let big = doc();
        let t = Instant::now();
        let _ = crate::relocate::text_stamp(&big);
        let stamp_ms = t.elapsed().as_secs_f64() * 1000.0;
        let key = DocKey("big".into());
        app.open_document(big, key.clone(), "Big".into());
        app.set_cursor(CharPos(12_345));
        // The disk stalls until released; leaving the document queues its
        // position behind the stall.
        let (release, held) = channel::<()>();
        let stall_over = Arc::new(AtomicBool::new(false));
        app.writer.send(crate::writer::Job::Hold {
            release: held,
            done: Arc::clone(&stall_over),
        });
        app.open_document(
            textweaver_text::Document::from_plain_text("Other."),
            DocKey("other".into()),
            "Other".into(),
        );
        let t = Instant::now();
        app.open_document(doc(), key, "Big".into());
        let open_ms = t.elapsed().as_secs_f64() * 1000.0;
        // The open is done, and the disk is still stalled: nothing waited
        // for the writer.
        assert!(
            !stall_over.load(Ordering::Acquire),
            "the open waited for the stalled disk"
        );
        let at = app.session().unwrap().cursor;
        assert!(at >= CharPos(12_300) && at <= CharPos(12_345), "{at:?}");
        let _ = release.send(());
        let t = Instant::now();
        // A hang check, not a speed check.
        assert!(app.writer.flush(Duration::from_secs(60)));
        let rest_ms = t.elapsed().as_secs_f64() * 1000.0;
        assert!(stall_over.load(Ordering::Acquire));
        println!(
            "text stamp on {} MB: {stamp_ms:.1} ms; open with the disk stalled: \
             {open_ms:.1} ms; the queued saves after the stall: {rest_ms:.1} ms",
            text.len() / 1_000_000
        );
    }

    #[test]
    fn scans_archives_and_web_pages_may_be_slow() {
        assert!(may_be_slow(Path::new("scan.PDF")));
        assert!(may_be_slow(Path::new("photo.jpg")));
        assert!(may_be_slow(Path::new("https://example.org/page")));
        assert!(may_be_slow(Path::new("no-such.zip!notes.md")));
        assert!(!may_be_slow(Path::new("notes.md")));
    }

    #[test]
    fn damaged_rtf_and_odt_files_are_named_plainly() {
        let parse = || LoadError::Parse("XML: unexpected end of stream".into());
        assert_eq!(
            open_failure_message(Path::new("handout.RTF"), &parse()),
            "Could not open handout.RTF: it is not a readable RTF file; it may be damaged."
        );
        assert_eq!(
            open_failure_message(Path::new("notes.odt"), &parse()),
            "Could not open notes.odt: it is not a readable OpenDocument text file; it may be damaged."
        );
    }

    #[test]
    fn damaged_latex_email_and_web_archives_are_named_plainly() {
        let parse = || LoadError::Parse("it has more than 10000 parts".into());
        assert_eq!(
            open_failure_message(Path::new("notes.tex"), &parse()),
            "Could not open notes.tex: it is not a readable LaTeX file; it may be damaged or too large."
        );
        assert_eq!(
            open_failure_message(Path::new("message.eml"), &parse()),
            "Could not open message.eml: it is not a readable email message; it may be damaged or too large."
        );
        assert_eq!(
            open_failure_message(Path::new("page.MHT"), &parse()),
            "Could not open page.MHT: it is not a readable web archive; it may be damaged or too large."
        );
    }

    #[test]
    fn damaged_json_notebooks_drawings_and_formulas_are_named_plainly() {
        let parse = || LoadError::Parse("XML: unexpected end of stream".into());
        for (file, words) in [
            ("data.json", "a readable JSON file; it may be too large."),
            (
                "growth.ipynb",
                "a readable Jupyter notebook; it may be damaged or too large.",
            ),
            (
                "chart.SVG",
                "a readable SVG drawing; it may be damaged or too large.",
            ),
            (
                "f.mml",
                "a readable MathML formula; it may be damaged or too large.",
            ),
        ] {
            assert_eq!(
                open_failure_message(Path::new(file), &parse()),
                format!("Could not open {file}: it is not {words}")
            );
        }
        // Other formats keep the loader's words.
        assert_eq!(
            open_failure_message(Path::new("x.weird"), &parse()),
            "Could not open x.weird: parse error: XML: unexpected end of stream"
        );
    }
}
