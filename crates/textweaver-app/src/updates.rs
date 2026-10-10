//! Updates (B1-u1): textweaver checks GitHub for a newer release, asks
//! before downloading it, checks it against the release's checksums, and
//! installs it, saying each step in words.
//!
//! - **Asked once.** The first-run list of optional components has a row,
//!   "Check for updates automatically", so the first run keeps its three
//!   steps; a copy that started before this existed asks the same question
//!   once, when nothing else is open. `[updates] check` (Settings, with
//!   F1 help) changes it later. Until the answer is yes nothing is read.
//! - **Once a day, at start.** With `[updates] check` on, the first tick
//!   after a frontend starts reads the public release list, at most once a
//!   day (`[updates] last_check`). Help, Check for updates reads it any
//!   time. Only the public list is read, with the neutral User-Agent;
//!   nothing about the person or the computer is sent.
//! - **Nothing said without news.** An automatic check that finds nothing,
//!   or fails, says nothing (the log keeps why). Help, Check for updates
//!   says the result either way.
//! - **Asked before downloading,** with the version and size; the question
//!   waits until reading stops. A no is kept (`[updates] declined`) and
//!   that release is not offered again by the automatic check; a newer one
//!   is.
//! - **Checked, then installed.** The package is pinned by its listed size
//!   and the SHA-256 in `SHA256SUMS.txt`; a mismatch stops with a sentence
//!   and changes nothing. Linux and macOS replace the files at once (the
//!   running program keeps its copy) and say to restart; Windows swaps them
//!   when textweaver closes ([`App::shutdown`]), and the window starts
//!   again. Settings, notes, the library, state, and components are never
//!   touched: they are not in the install folder.
//! - **Progress** at most every ten seconds, in words.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub use textweaver_components::{swap, update};

use swap::{Installed, Place};
use textweaver_components::{Fetcher, StandardFetcher, can_download};
use textweaver_keymap::{ActionId, Frontend};
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;
use update::{Offer, Target, UpdateError};

use crate::app::App;
use crate::command::{Confirm, Effect};
use crate::playback::Playback;

/// How long between automatic checks.
const DAY: u64 = 24 * 60 * 60;

/// The most often download progress is said.
const PROGRESS_EVERY: Duration = Duration::from_secs(10);

/// The version running now.
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// A yes-or-no question about updates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Question {
    /// Download this release?
    Download(Offer),
    /// Check for updates automatically? (A copy that started before the
    /// first-run row existed.)
    Automatic,
}

/// What a helper thread finished with.
enum Done {
    Checked(Result<Option<Offer>, UpdateError>),
    Installed(Result<Installed, UpdateError>),
}

/// Work on a helper thread.
struct Job {
    rx: Receiver<Done>,
    /// Help, Check for updates (says the result either way).
    manual: bool,
    /// Bytes downloaded so far, and the package's size (0 while checking).
    done: Arc<AtomicU64>,
    total: u64,
    /// When progress was last said.
    said_at: Instant,
    said: u64,
}

/// The app's update state.
#[derive(Default)]
pub(crate) struct UpdatesState {
    /// Tests: a fake release server, its list's address, and a fake
    /// install folder.
    source: Option<(Arc<dyn Fetcher>, String, Option<PathBuf>)>,
    /// A frontend started (the GUI or the terminal reader): automatic
    /// checks may run. `tw` and tests never set it.
    started: bool,
    /// Ask the automatic-check question when nothing else is open.
    ask_automatic: bool,
    job: Option<Job>,
    /// A newer release waiting to be offered when reading stops.
    offer: Option<Offer>,
    /// The question waiting for y or n.
    pub(crate) question: Option<Question>,
    /// Windows: unpacked, swapped in when textweaver closes.
    staged: Option<(PathBuf, PathBuf)>,
}

impl std::fmt::Debug for UpdatesState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdatesState")
            .field("started", &self.started)
            .field("offer", &self.offer)
            .field("question", &self.question)
            .field("staged", &self.staged)
            .finish_non_exhaustive()
    }
}

/// Gives the command its handler.
pub(crate) fn register(app: &mut App) {
    app.register_handler(ActionId::CheckForUpdates, |app| app.check_for_updates());
}

/// The words for an update error, meaning first.
pub fn error_text(c: &Catalog, e: &UpdateError) -> String {
    let reason = e.to_string();
    match e {
        UpdateError::Fetch(_) | UpdateError::Unreadable => {
            c.fmt("update-error-check", &args!["reason" => reason])
        }
        UpdateError::NoPackage(v) => {
            c.fmt("update-error-no-package", &args!["version" => v.as_str()])
        }
        UpdateError::NoChecksum(_) => c.tr("update-error-no-checksum"),
        UpdateError::Mismatch(_) => c.tr("update-error-mismatch"),
        UpdateError::Cancelled => c.tr("update-error-cancelled"),
        UpdateError::Download(_) => c.fmt("update-error-download", &args!["reason" => reason]),
        UpdateError::NotPackage(_) => c.tr("update-error-not-package"),
        UpdateError::Unpack(_) | UpdateError::Install { .. } => {
            c.fmt("update-error-install", &args!["reason" => reason])
        }
    }
}

/// Seconds since 1970, now.
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl App {
    // ----- Setup -------------------------------------------------------

    /// Tests: reads releases from `releases` through `fetcher` (a fake
    /// release server) and installs into `install` (a fake install folder)
    /// instead of where this program came from.
    pub fn set_update_source(
        &mut self,
        fetcher: Arc<dyn Fetcher>,
        releases: &str,
        install: Option<PathBuf>,
    ) {
        self.updates.source = Some((fetcher, releases.to_owned(), install));
    }

    /// A frontend started: automatic checks may run from the next tick.
    /// On a later start, a copy never asked about updates asks once.
    pub(crate) fn updates_started(&mut self, first_run: bool) {
        self.updates.started = true;
        if !first_run && !self.settings.updates.asked && self.paths.is_some() {
            self.updates.ask_automatic = true;
        }
    }

    /// The fetcher and the release list's address, when this build can
    /// download.
    fn update_fetcher(&self) -> Option<(Arc<dyn Fetcher>, String)> {
        if let Some((f, a, _)) = &self.updates.source {
            return Some((Arc::clone(f), a.clone()));
        }
        can_download().then(|| {
            (
                Arc::new(StandardFetcher) as Arc<dyn Fetcher>,
                update::RELEASES.to_owned(),
            )
        })
    }

    /// Where an update would go.
    fn update_place(&self, target: &Target) -> Result<Place, UpdateError> {
        match &self.updates.source {
            Some((_, _, Some(dir))) => Ok(Place::Folder(dir.clone())),
            _ => Place::find(target),
        }
    }

    // ----- Checking ----------------------------------------------------

    /// Help, Check for updates: reads the release list now and says what
    /// it found, update or not.
    pub(crate) fn check_for_updates(&mut self) -> Vec<Effect> {
        if self.updates.job.is_some() {
            let msg = self.msg("update-busy");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        if self.update_fetcher().is_none() || Target::this().is_none() {
            let msg = self.msg("update-not-in-build");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let msg = self.msg("update-checking");
        self.say_result(&msg);
        self.start_update_check(true);
        vec![Effect::Redraw]
    }

    /// True when the automatic check is due: on, started by a frontend,
    /// a day since the last, and this build can download.
    fn automatic_check_due(&self) -> bool {
        let u = &self.settings.updates;
        self.updates.started
            && u.check
            && self.paths.is_some()
            && self.updates.job.is_none()
            && self.updates.offer.is_none()
            && self.updates.question.is_none()
            && self.updates.staged.is_none()
            && now_secs().saturating_sub(u.last_check) >= DAY
    }

    fn start_update_check(&mut self, manual: bool) {
        let (Some((fetcher, address)), Some(target)) = (self.update_fetcher(), Target::this())
        else {
            return;
        };
        if !manual {
            let now = now_secs();
            let _ = self.update_settings(|s| s.updates.last_check = now);
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let wake = self.waker_slot();
        let spawned = std::thread::Builder::new()
            .name("textweaver-update".into())
            .spawn(move || {
                let r = update::check(&*fetcher, &address, CURRENT, &target);
                let _ = tx.send(Done::Checked(r));
                wake.wake();
            });
        if spawned.is_err() {
            log::warn!("cannot start the update check");
            return;
        }
        self.updates.job = Some(Job {
            rx,
            manual,
            done: Arc::new(AtomicU64::new(0)),
            total: 0,
            said_at: Instant::now(),
            said: 0,
        });
    }

    fn update_checked(&mut self, manual: bool, result: Result<Option<Offer>, UpdateError>) {
        let offer = match result {
            Ok(Some(offer)) => offer,
            Ok(None) => {
                if manual {
                    let msg = self.msg_args("update-none", &args!["version" => CURRENT]);
                    self.say_result(&msg);
                }
                return;
            }
            Err(e) => {
                log::info!("update check: {e}");
                if manual {
                    let msg = error_text(self.cat(), &e);
                    self.error(&msg);
                }
                return;
            }
        };
        if !manual && !update::is_newer(&offer.version, &self.settings.updates.declined) {
            log::info!("update {} was declined", offer.version);
            return;
        }
        let installable = Target::this().map(|t| self.update_place(&t));
        if let Some(Err(e)) = installable {
            log::info!("update {} found, not installable here: {e}", offer.version);
            if manual {
                let msg = self.msg_args(
                    "update-available-elsewhere",
                    &args!["version" => offer.version.as_str()],
                );
                self.say_result(&msg);
            }
            return;
        }
        // Asked when reading stops (updates_tick).
        self.updates.offer = Some(offer);
    }

    fn update_question_text(&self, q: &Question) -> String {
        match q {
            Question::Download(offer) => self.msg_args(
                "update-found",
                &args!["version" => offer.version.as_str(), "size" => offer.size_text()],
            ),
            Question::Automatic => self.msg("update-automatic-question"),
        }
    }

    /// True when a question may be asked: nothing open, not reading.
    fn update_may_ask(&self) -> bool {
        self.list.is_none()
            && !self.mode.is_prompt()
            && !self.confirmation_pending()
            && self.playback != Playback::Reading
    }

    fn ask_update(&mut self, q: Question) {
        let text = self.update_question_text(&q);
        self.updates.question = Some(q);
        self.ask(&text);
    }

    /// The answer to an update question.
    pub(crate) fn confirm_update(&mut self, answer: Confirm) -> Vec<Effect> {
        let Some(q) = self.updates.question.take() else {
            return vec![Effect::Redraw];
        };
        match (answer, q) {
            (Confirm::Repeat, q) => self.ask_update(q),
            (Confirm::Yes, Question::Download(offer)) => self.start_update_download(offer),
            (Confirm::No, Question::Download(offer)) => {
                let _ = self.update_settings(|s| s.updates.declined = offer.version.clone());
                let msg = self.msg("update-declined");
                self.tell(&msg);
            }
            (answer, Question::Automatic) => self.set_update_check(answer == Confirm::Yes),
        }
        vec![Effect::Redraw]
    }

    /// Turns the automatic check on or off, marks the question asked, and
    /// says which, in words.
    pub(crate) fn set_update_check(&mut self, on: bool) {
        let _ = self.update_settings(|s| {
            s.updates.check = on;
            s.updates.asked = true;
        });
        let msg = self.msg(if on {
            "update-automatic-on"
        } else {
            "update-automatic-off"
        });
        self.say_result(&msg);
    }

    // ----- Downloading and installing ----------------------------------

    fn start_update_download(&mut self, offer: Offer) {
        let (Some((fetcher, _)), Some(target), Some(paths)) =
            (self.update_fetcher(), Target::this(), self.paths.clone())
        else {
            let msg = self.msg("update-not-in-build");
            self.tell(&msg);
            return;
        };
        let place = match self.update_place(&target) {
            Ok(p) => p,
            Err(e) => {
                let msg = error_text(self.cat(), &e);
                self.error(&msg);
                return;
            }
        };
        let root = paths.cache_dir.join("updates");
        let dir = root.join(&offer.version);
        let (tx, rx) = std::sync::mpsc::channel();
        let done = Arc::new(AtomicU64::new(0));
        let wake = self.waker_slot();
        let (done2, offer2) = (Arc::clone(&done), offer.clone());
        let spawned = std::thread::Builder::new()
            .name("textweaver-update".into())
            .spawn(move || {
                clear_older(&root, &offer2.version);
                let cancel = AtomicBool::new(false);
                let r = update::download_update(
                    &offer2,
                    &*fetcher,
                    &dir,
                    &mut |p| {
                        done2.store(p.done, Ordering::Relaxed);
                        wake.wake();
                    },
                    &cancel,
                )
                .and_then(|package| swap::install(&package, &target, &place, &dir));
                let _ = tx.send(Done::Installed(r));
                wake.wake();
            });
        if spawned.is_err() {
            log::warn!("cannot start the update download");
            return;
        }
        let msg = self.msg_args("update-downloading", &args!["size" => offer.size_text()]);
        self.tell(&msg);
        self.updates.job = Some(Job {
            rx,
            manual: true,
            done,
            total: offer.package.size,
            said_at: Instant::now(),
            said: 0,
        });
    }

    fn update_installed(&mut self, result: Result<Installed, UpdateError>) {
        match result {
            Ok(Installed::Done) => {
                let msg = self.msg("update-installed");
                self.tell(&msg);
            }
            Ok(Installed::OnClose { new_root, install }) => {
                self.updates.staged = Some((new_root, install));
                let msg = self.msg("update-on-close");
                self.tell(&msg);
            }
            Err(e) => {
                log::warn!("update: {e}");
                let msg = error_text(self.cat(), &e);
                self.error(&msg);
            }
        }
    }

    /// From [`App::tick`]: the automatic check when due, a finished check
    /// or download, progress at most every ten seconds, and a waiting
    /// question once reading stops.
    pub(crate) fn updates_tick(&mut self) -> Vec<Effect> {
        if self.automatic_check_due() {
            self.start_update_check(false);
        }
        if let Some(mut job) = self.updates.job.take() {
            let done = job.done.load(Ordering::Relaxed);
            if job.total > 0
                && done > job.said
                && done < job.total
                && job.said_at.elapsed() >= PROGRESS_EVERY
            {
                let percent = done * 100 / job.total;
                let msg = self.msg_args("update-progress", &args!["percent" => percent]);
                self.say_progress(&msg);
                job.said_at = Instant::now();
                job.said = done;
            }
            match job.rx.try_recv() {
                Err(TryRecvError::Empty) => self.updates.job = Some(job),
                Err(TryRecvError::Disconnected) => log::warn!("the update job stopped"),
                Ok(Done::Checked(r)) => self.update_checked(job.manual, r),
                Ok(Done::Installed(r)) => self.update_installed(r),
            }
        }
        if self.updates.question.is_none() && self.update_may_ask() {
            if let Some(offer) = self.updates.offer.take() {
                self.ask_update(Question::Download(offer));
                return vec![Effect::Redraw];
            }
            if self.updates.ask_automatic {
                self.updates.ask_automatic = false;
                if !self.settings.updates.asked {
                    self.ask_update(Question::Automatic);
                    return vec![Effect::Redraw];
                }
            }
        }
        Vec::new()
    }

    /// From [`App::shutdown`]: on Windows, a downloaded and checked update
    /// is swapped in by the new `tw` once this program has closed, and the
    /// window starts again.
    pub(crate) fn finish_update_on_close(&mut self) {
        let Some((new_root, install)) = self.updates.staged.take() else {
            return;
        };
        let Ok(exe) = std::env::current_exe() else {
            return;
        };
        let restart = (self.keymap.frontend() == Frontend::Gui).then_some(exe.as_path());
        if let Err(e) = swap::start_finisher(&new_root, &install, &exe, restart) {
            log::warn!("update not finished: {e}");
        }
    }

    /// True while an update is checked for or downloaded.
    pub fn update_job_running(&self) -> bool {
        self.updates.job.is_some()
    }

    /// Waits up to `limit` for update work to finish (tests).
    pub fn wait_for_updates(&mut self, limit: Duration) -> bool {
        let start = Instant::now();
        while self.updates.job.is_some() {
            if start.elapsed() > limit {
                return false;
            }
            self.updates_tick();
            std::thread::sleep(Duration::from_millis(5));
        }
        true
    }
}

/// Removes downloads of other versions under `root` (the update folder
/// in the cache, which only textweaver writes), so old packages do not
/// pile up. Best effort.
fn clear_older(root: &std::path::Path, keep: &str) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let staging = format!("{keep}.");
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name == keep || name.starts_with(&staging) {
            continue;
        }
        let path = e.path();
        let r = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        if let Err(err) = r {
            log::info!("old update {}: {err}", path.display());
        }
    }
}

#[cfg(test)]
mod tests;
