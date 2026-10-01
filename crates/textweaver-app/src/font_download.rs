//! Lexend on first choice (W7l): when the reader chooses a reading font
//! that is not bundled and not installed, the app asks before downloading
//! it ("Download the Lexend font, 206 KB, SIL Open Font License? y or
//! n"), naming its size and license, as the voice manager does for Piper
//! voices. On a yes, a helper thread fetches the pinned files, checks
//! each by size and SHA-256, and keeps them with the license in the data
//! folder's `fonts/lexend/` (`textweaver_fonts::downloaded`). The GUI
//! then registers them, and the PDF and EPUB writers find Lexend by name.
//!
//! The question comes after the change is said: [`App::set_setting`]
//! marks the font setting, and the question is asked right after the
//! setting's own message ([`App::set_setting_command`]) or on the next
//! [`App::tick`] (the GUI's settings dialog). The GUI's font list asks
//! through [`App::offer_font_download`]. A no keeps the choice; another
//! reading font is used meanwhile, and the next choice asks again.
//!
//! Without the `publish` feature the lean reader links no HTTP client:
//! there is nothing to ask, and choosing Lexend says downloads are not in
//! this build.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};

use textweaver_fonts::downloaded::{self, DownloadError, DownloadableFont, Fetcher};
use textweaver_lexicon::args;

use crate::app::App;
use crate::command::{Confirm, Effect};

/// Says whether a family is installed on this computer (by family name,
/// ignoring case).
pub type InstalledCheck = Arc<dyn Fn(&str) -> bool + Send + Sync>;

/// Font downloads: the question, the download, and how to fetch.
#[derive(Default)]
pub(crate) struct FontDownloads {
    /// The font the reader is asked about.
    pub(crate) question: Option<&'static DownloadableFont>,
    /// The font setting changed; ask when nothing else is being asked.
    pub(crate) offer_pending: bool,
    /// A download on a helper thread.
    job: Option<(
        &'static DownloadableFont,
        Receiver<Result<PathBuf, DownloadError>>,
    )>,
    /// Set by tests and frontends; the network otherwise.
    fetcher: Option<Arc<dyn Fetcher>>,
    /// Set by tests; the installed fonts otherwise.
    installed: Option<InstalledCheck>,
    /// Moves on when a font finished downloading.
    generation: u64,
}

impl std::fmt::Debug for FontDownloads {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontDownloads")
            .field("question", &self.question.map(|f| f.key))
            .field("offer_pending", &self.offer_pending)
            .field("downloading", &self.job.as_ref().map(|j| j.0.key))
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

/// The data folder's `fonts`, where downloaded fonts are kept.
pub fn fonts_folder(paths: &textweaver_store::Paths) -> PathBuf {
    paths.data_dir.join("fonts")
}

/// Tells the font code (the PDF and EPUB writers' lookups) where this
/// session's downloaded fonts are. `tw` calls it before converting; the
/// app calls it when it starts with a data folder.
pub fn use_downloaded_fonts(paths: Option<&textweaver_store::Paths>) {
    downloaded::set_folder(paths.map(fonts_folder));
}

/// The fetcher downloads use when none was set: HTTPS with the `publish`
/// feature, none in the lean reader.
fn default_fetcher() -> Option<Arc<dyn Fetcher>> {
    #[cfg(feature = "publish")]
    {
        Some(Arc::new(downloaded::HttpFetcher))
    }
    #[cfg(not(feature = "publish"))]
    {
        None
    }
}

impl App {
    /// Where this session keeps downloaded fonts (`fonts` in the data
    /// folder); `None` in a session that keeps no files.
    pub fn fonts_folder(&self) -> Option<PathBuf> {
        self.paths.as_ref().map(fonts_folder)
    }

    /// Fetches font downloads through `fetcher` from now on (tests pass a
    /// fake one that hands out fixtures; the default goes to the network).
    pub fn set_font_fetcher(&mut self, fetcher: Arc<dyn Fetcher>) {
        self.fonts.fetcher = Some(fetcher);
    }

    /// Decides with `check` whether a family is installed on this computer
    /// (tests; the default reads the installed fonts once).
    pub fn set_installed_fonts_check(&mut self, check: InstalledCheck) {
        self.fonts.installed = Some(check);
    }

    /// Moves on each time a font finishes downloading, so a GUI knows to
    /// register the new files ([`textweaver_fonts::downloaded`]).
    pub fn font_downloads(&self) -> u64 {
        self.fonts.generation
    }

    /// True when `font` can be used without a download: it is in the data
    /// folder, or installed on this computer under its name or an alias.
    fn font_available(&self, font: &DownloadableFont) -> bool {
        if self
            .fonts_folder()
            .is_some_and(|d| font.is_installed_in(&d))
        {
            return true;
        }
        let mut names = std::iter::once(font.name).chain(font.aliases.iter().copied());
        match &self.fonts.installed {
            Some(check) => names.any(|n| check(n)),
            None => {
                let faces = textweaver_fonts::system::installed();
                names.any(|n| textweaver_fonts::system::find_family(faces, n).is_some())
            }
        }
    }

    /// The question: "Download the Lexend font, 206 KB, SIL Open Font
    /// License? y or n".
    fn font_question(&self, font: &DownloadableFont) -> String {
        self.msg_args(
            "font-download-question",
            &args![
                "font" => font.name,
                "kb" => font.kilobytes(),
                "licence" => font.license_name
            ],
        )
    }

    /// Asks before downloading the reading font now chosen, when it is a
    /// font textweaver downloads (Lexend) and it is neither downloaded nor
    /// installed. Says why when it cannot be downloaded here. Nothing
    /// happens for any other font.
    pub fn offer_font_download(&mut self) -> Vec<Effect> {
        self.fonts.offer_pending = false;
        let family = self.settings.reading_aids.font.family.clone();
        let Some(font) = downloaded::downloadable(&family) else {
            return Vec::new();
        };
        if self.font_available(font) {
            return Vec::new();
        }
        if self.fonts.job.is_some() {
            let msg = self.msg_args("font-download-busy", &args!["font" => font.name]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        if self.fonts_folder().is_none() {
            let msg = self.msg_args("font-download-no-folder", &args!["font" => font.name]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        if self.fonts.fetcher.is_none() && default_fetcher().is_none() {
            let msg = self.msg_args("font-download-not-in-build", &args!["font" => font.name]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        self.fonts.question = Some(font);
        let question = self.font_question(font);
        self.ask(&question);
        vec![Effect::Redraw]
    }

    /// Answers the font question.
    pub(crate) fn confirm_font(&mut self, answer: Confirm) -> Vec<Effect> {
        let Some(font) = self.fonts.question.take() else {
            return vec![Effect::Redraw];
        };
        match answer {
            Confirm::Repeat => {
                self.fonts.question = Some(font);
                let question = self.font_question(font);
                self.ask(&question);
            }
            Confirm::No => {
                let msg = self.msg_args("font-download-declined", &args!["font" => font.name]);
                self.tell(&msg);
            }
            Confirm::Yes => self.start_font_download(font),
        }
        vec![Effect::Redraw]
    }

    fn start_font_download(&mut self, font: &'static DownloadableFont) {
        let (Some(dir), Some(fetcher)) = (
            self.fonts_folder(),
            self.fonts.fetcher.clone().or_else(default_fetcher),
        ) else {
            let msg = self.msg_args("font-download-no-folder", &args!["font" => font.name]);
            self.error(&msg);
            return;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let wake = self.waker_slot();
        let spawned = std::thread::Builder::new()
            .name("textweaver-font-download".into())
            .spawn(move || {
                let _ = tx.send(font.install(&dir, &*fetcher));
                wake.wake();
            });
        if spawned.is_err() {
            let msg = self.msg_args(
                "font-download-failed",
                &args!["font" => font.name, "error" => "no helper thread"],
            );
            self.error(&msg);
            return;
        }
        self.fonts.job = Some((font, rx));
        let msg = self.msg_args("font-downloading", &args!["font" => font.name]);
        self.tell(&msg);
    }

    /// From [`App::tick`]: asks a pending question when nothing else is
    /// asked, and collects a finished download.
    pub(crate) fn font_download_tick(&mut self) -> Vec<Effect> {
        let mut effects = Vec::new();
        if self.fonts.offer_pending && !self.confirmation_pending() {
            effects.extend(self.offer_font_download());
        }
        let Some((font, rx)) = self.fonts.job.take() else {
            return effects;
        };
        match rx.try_recv() {
            Ok(Ok(_)) => {
                self.fonts.generation += 1;
                let msg = self.msg_args("font-downloaded", &args!["font" => font.name]);
                self.tell(&msg);
            }
            Ok(Err(e)) => {
                let msg = self.msg_args(
                    "font-download-failed",
                    &args!["font" => font.name, "error" => e.to_string()],
                );
                self.error(&msg);
            }
            Err(TryRecvError::Empty) => {
                self.fonts.job = Some((font, rx));
                return effects;
            }
            Err(TryRecvError::Disconnected) => {
                let msg = self.msg_args(
                    "font-download-failed",
                    &args!["font" => font.name, "error" => "the download stopped"],
                );
                self.error(&msg);
            }
        }
        effects.push(Effect::Redraw);
        effects
    }

    /// Waits up to `limit` for a font download to finish (tests).
    pub fn wait_for_font_download(&mut self, limit: std::time::Duration) -> bool {
        let start = std::time::Instant::now();
        while self.fonts.job.is_some() {
            if start.elapsed() > limit {
                return false;
            }
            self.font_download_tick();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        true
    }
}
