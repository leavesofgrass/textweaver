//! Rate, pitch, volume, speed presets, display toggles, and the voice
//! manager's wiring. Every change is announced (star showed rate changes
//! only visually) and marks the settings for saving on quit.
//!
//! The voice manager's model (rows, filters, labels, per-voice rate and
//! pitch) is `crate::voice_manager`; this file runs its commands: using a
//! voice (switching engine when needed), favourites, downloading and
//! removing Piper voices, and fetching the Piper catalogue, each download
//! only after a yes.
//!
//! **Preview** (the Say Status key in the list, the GUI's Preview button)
//! speaks a sample in the focused voice without choosing it. A voice of
//! the running engine is previewed in place (`SpeechService::preview`);
//! another engine's voice by starting that engine on a helper thread with
//! the frontend's own speech starter, saying the sample, and closing it.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};

use net::DownloadPlan;
use textweaver_a11y::{Announcement, Verbosity};
use textweaver_core::{Pitch, Rate, Volume};
use textweaver_engines::piper::{Catalog, InstalledVoice, PiperError};
use textweaver_lexicon::args;
use textweaver_speech::Earcon;

use crate::app::{App, ListKind};
use crate::command::{Confirm, Effect};
use crate::restart::SpeechStarter;
use crate::voice_manager::{
    PIPER, VoiceControl, VoiceControls, VoiceEntry, VoiceManager, VoiceRow, VoiceStatus,
    cached_catalog, catalog_path, engine_entries, params_key, piper_entries, remember_params,
    remembered_params,
};

/// The longest a preview on another engine may play before its engine is
/// closed.
const PREVIEW_LIMIT: std::time::Duration = std::time::Duration::from_secs(30);

/// Piper voice downloads: HTTP, so part of the app's `publish` feature.
#[cfg(feature = "publish")]
mod net {
    pub(crate) use textweaver_engines::piper::download::{
        DownloadPlan, download, fetch_catalog_json, plan,
    };
}

/// Without the `publish` feature the lean reader links no HTTP client:
/// every download says it is not in this build.
#[cfg(not(feature = "publish"))]
mod net {
    use textweaver_engines::piper::{
        Catalog, CatalogVoice, InstalledVoice, PiperError, VoiceStore,
    };

    /// Stands in for the download plan; never made.
    #[derive(Debug)]
    pub(crate) struct DownloadPlan {
        pub(crate) voice: CatalogVoice,
    }

    impl DownloadPlan {
        pub(crate) fn describe(&self) -> String {
            self.voice.describe()
        }
        pub(crate) fn total_bytes(&self) -> u64 {
            self.voice.size_bytes()
        }
    }

    fn missing() -> PiperError {
        PiperError::Unsupported("downloading voices is not in this build of textweaver".into())
    }

    pub(crate) fn plan(_: &CatalogVoice) -> Result<DownloadPlan, PiperError> {
        Err(missing())
    }

    pub(crate) fn fetch_catalog_json() -> Result<(String, Catalog), PiperError> {
        Err(missing())
    }

    pub(crate) fn download(
        _: &DownloadPlan,
        _: &VoiceStore,
        _: &mut dyn FnMut(u64, u64) -> bool,
    ) -> Result<InstalledVoice, PiperError> {
        Err(missing())
    }
}

/// A question the voice manager is waiting on (y or n).
#[derive(Debug)]
pub(crate) enum VoiceQuestion {
    /// Download this voice (its size and licence were said).
    Download(Box<DownloadPlan>),
    /// Remove this installed Piper voice: key and name.
    Remove(String, String),
    /// Fetch the Piper catalogue.
    FetchCatalog,
}

/// Work on a background thread.
pub(crate) enum VoiceJob {
    /// Asking Hugging Face for a voice's files and licence.
    Plan(Receiver<Result<DownloadPlan, PiperError>>),
    /// Downloading a voice: its name, bytes so far and in all, the
    /// quarter last announced, and a way to cancel.
    Download {
        name: String,
        rx: Receiver<Result<InstalledVoice, PiperError>>,
        done: Arc<AtomicU64>,
        total: u64,
        told: u64,
        cancel: Arc<AtomicBool>,
    },
    /// Fetching the catalogue.
    Catalog(Receiver<Result<Catalog, PiperError>>),
}

impl std::fmt::Debug for VoiceJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            VoiceJob::Plan(_) => "Plan",
            VoiceJob::Download { .. } => "Download",
            VoiceJob::Catalog(_) => "Catalog",
        })
    }
}

/// The voice manager's state in the app.
#[derive(Debug, Default)]
pub(crate) struct VoicesState {
    pub(crate) manager: VoiceManager,
    pub(crate) question: Option<VoiceQuestion>,
    pub(crate) job: Option<VoiceJob>,
    /// The other engines' voices, listed once per session in the
    /// background (each engine is started, asked, and closed).
    pub(crate) others: Option<Vec<VoiceEntry>>,
    pub(crate) others_rx: Option<Receiver<Vec<VoiceEntry>>>,
    /// A preview playing on another engine.
    pub(crate) preview: Option<OtherPreview>,
}

/// A preview on another engine, on its helper thread.
#[derive(Debug)]
pub(crate) struct OtherPreview {
    /// The voice's name, for messages.
    voice: String,
    /// The engine's name, for messages.
    engine: String,
    /// Set to stop the preview and close its engine.
    cancel: Arc<AtomicBool>,
    /// How it ended.
    rx: Receiver<PreviewOutcome>,
}

/// How a preview on another engine ended.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PreviewOutcome {
    /// It played (or was stopped).
    Done,
    /// The engine could not be started (another one started in its place).
    NotStarted,
    /// The engine failed: why.
    Failed(String),
}

/// Starts the engine `settings` name through the frontend's `starter`,
/// says `sample` in `voice`, and closes the engine once it has been said,
/// `cancel` is set, or [`PREVIEW_LIMIT`] has passed. Runs on a helper
/// thread.
fn run_other_preview(
    starter: &SpeechStarter,
    settings: &textweaver_store::Settings,
    voice: &str,
    sample: &str,
    cancel: &AtomicBool,
) -> PreviewOutcome {
    use std::sync::mpsc::RecvTimeoutError;
    use textweaver_speech::SpeechStatus;
    let (service, _, _) = starter(settings);
    if service.backend_id() != settings.speech.backend {
        return PreviewOutcome::NotStarted;
    }
    let sp = &settings.speech;
    service.set_voice(Some(voice.to_owned()));
    service.set_rate(sp.rate);
    service.set_pitch(sp.pitch);
    service.set_volume(sp.volume);
    service.read(vec![textweaver_core::Utterance::announcement(sample)]);
    let deadline = std::time::Instant::now() + PREVIEW_LIMIT;
    loop {
        if cancel.load(Ordering::Relaxed) || std::time::Instant::now() >= deadline {
            service.stop();
            return PreviewOutcome::Done;
        }
        match service
            .statuses()
            .recv_timeout(std::time::Duration::from_millis(50))
        {
            Ok(SpeechStatus::Finished { .. } | SpeechStatus::Stopped { .. }) => {
                return PreviewOutcome::Done;
            }
            Ok(SpeechStatus::BackendError(e)) => return PreviewOutcome::Failed(e),
            Ok(_) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return PreviewOutcome::Failed(service.failure().unwrap_or_default());
            }
        }
    }
}

/// Starts every other available engine on a helper thread, lists its
/// voices, and closes it. Engines that need the main thread (Apple's) are
/// skipped, as are Piper (read from its folder) and the silent ones.
fn list_other_engines(
    registry: textweaver_speech::BackendRegistry,
    running: String,
) -> Option<Receiver<Vec<VoiceEntry>>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("textweaver-voice-engines".into())
        .spawn(move || {
            let mut out = Vec::new();
            for info in registry.list() {
                if !info.available
                    || info.opt_in
                    || info.id == running
                    || info.id == PIPER
                    || info.id == "null"
                    || info
                        .caps
                        .contains(textweaver_speech::Caps::REQUIRES_MAIN_THREAD)
                {
                    continue;
                }
                let Some(make) = registry.factory(info.id) else {
                    continue;
                };
                match make().and_then(|b| b.voices()) {
                    Ok(v) => out.extend(engine_entries(info.id, info.name, &v)),
                    Err(e) => log::info!("voice manager: {}: {e}", info.id),
                }
            }
            let _ = tx.send(out);
        })
        .ok()?;
    Some(rx)
}

impl App {
    /// The Piper voices folder: `[speech.piper] voices` or
    /// `TEXTWEAVER_PIPER_VOICES` when set, else `piper/voices` in this
    /// session's data folder. `None` in a session that keeps no files.
    fn piper_store(&self) -> Option<textweaver_engines::piper::VoiceStore> {
        let mut config = textweaver_engines::piper_config(&self.settings);
        let overridden = std::env::var_os("TEXTWEAVER_PIPER_VOICES").is_some_and(|v| !v.is_empty())
            || self
                .settings
                .speech
                .extra
                .get("piper")
                .and_then(|t| t.get("voices"))
                .and_then(|v| v.as_str())
                .is_some_and(|v| !v.is_empty());
        if !overridden {
            config.voices_dir = self.paths.as_ref()?.data_dir.join("piper").join("voices");
        }
        Some(config.store())
    }

    /// The engine and voice in use.
    fn current_voice(&self) -> (String, Option<String>) {
        (
            self.speech.backend_id().to_owned(),
            self.settings.speech.voice.clone(),
        )
    }

    /// Every voice the manager lists: the running engine's, then Piper's
    /// installed and downloadable voices (when Piper is not the running
    /// engine, its installed voices are read from the data folder).
    fn gather_voices(&self, engine_voices: &[textweaver_speech::Voice]) -> Vec<VoiceEntry> {
        let engine = self.speech.backend_id();
        let mut entries = engine_entries(engine, &self.backend_name, engine_voices);
        if let Some(store) = self.piper_store() {
            let catalog = cached_catalog(store.dir());
            let piper = piper_entries(&store, catalog.as_ref());
            entries.extend(
                piper
                    .into_iter()
                    .filter(|p| !(engine == PIPER && p.status == VoiceStatus::Ready)),
            );
        }
        entries
    }

    /// Choose voice (Alt+V): the voice manager. Lists every voice (the
    /// engine's list was made once when it started and is read without
    /// waiting; while it is still being made, this says so and the list
    /// opens when it arrives, [`voices_tick`](Self::voices_tick)).
    pub(crate) fn choose_voice(&mut self) -> Vec<Effect> {
        let voices = match self.speech.voice_list() {
            textweaver_speech::VoiceList::Ready(v) => v,
            textweaver_speech::VoiceList::Loading => {
                self.voices_pending = true;
                // The frontend is woken when they arrive (crate::wake).
                let wake = self.waker_slot();
                self.speech.voice_cache().on_ready(move || wake.wake());
                let msg = self.msg("voice-still-loading");
                self.tell(&msg);
                return vec![Effect::Redraw];
            }
            textweaver_speech::VoiceList::Failed(e) => {
                let msg = self.msg_args("voice-list-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
                Vec::new()
            }
        };
        let mut entries = self.gather_voices(&voices);
        match &self.voices.others {
            Some(others) => entries.extend(others.iter().cloned()),
            None if self.voices.others_rx.is_none() && self.can_start_speech() => {
                let registry = textweaver_engines::speech_registry_for(&self.settings);
                self.voices.others_rx =
                    list_other_engines(registry, self.speech.backend_id().to_owned());
            }
            None => {}
        }
        let favourites = self.settings.speech.favorite_voices.clone();
        self.voices.manager.offer_catalog = self.piper_store().is_some();
        // A favorite from another computer (sync) that no engine here has
        // is listed as not on this computer, once every engine's voices
        // are in.
        self.voices.manager.all_engines_listed = self.voices.others.is_some();
        self.voices.manager.set_entries(entries, &favourites);
        let shown = self.voices.manager.shown_sentence_in(self.cat());
        let preview = self.list_key_name(textweaver_keymap::ActionId::SayStatus);
        let msg = self.msg_args(
            "voice-manager-intro",
            &args!["shown" => shown, "preview" => preview],
        );
        self.tell(&msg);
        // Focus the voice in use, else the first voice; the filter rows
        // are above it.
        let (engine, voice) = self.current_voice();
        let rows = self.voices.manager.rows().len();
        let focus = (0..rows)
            .find(|&n| {
                self.voices.manager.entry_at(n).is_some_and(|e| {
                    e.engine == engine
                        && voice.as_deref().is_some_and(|v| {
                            v == e.voice.id || v.eq_ignore_ascii_case(&e.voice.name)
                        })
                })
            })
            .or_else(|| (0..rows).find(|&n| self.voices.manager.entry_at(n).is_some()));
        self.pending_list_focus = focus;
        self.show_voice_list()
    }

    /// Shows the voice list again, as it is now.
    fn show_voice_list(&mut self) -> Vec<Effect> {
        let (engine, voice) = self.current_voice();
        let items = self.voices.manager.labels_in(
            self.cat(),
            &self.settings.speech.favorite_voices,
            (&engine, voice.as_deref()),
        );
        self.list = Some(crate::app::ListKind::Voices);
        vec![Effect::ShowList {
            title: self.msg("voice-list-title"),
            items,
        }]
    }

    /// Opens the voice list asked for while the voices were loading, once
    /// they arrive (from [`App::tick`]); if something else is open by
    /// then, says they are ready instead. Also collects the voice
    /// manager's background work.
    pub(crate) fn voices_tick(&mut self) -> Vec<Effect> {
        let mut effects = self.voice_job_tick();
        if let Some(rx) = &self.voices.others_rx
            && let Ok(others) = rx.try_recv()
        {
            self.voices.others_rx = None;
            let n = others.len();
            self.voices.others = Some(others);
            if self.list == Some(ListKind::Voices) {
                effects.extend(self.relist_voices(n));
            }
        }
        effects.extend(self.preview_tick());
        if !self.voices_pending || self.speech.voice_list().is_loading() {
            return effects;
        }
        self.voices_pending = false;
        if self.list.is_some() || self.mode.is_prompt() || self.confirmation_pending() {
            let keys = self.keys(textweaver_keymap::ActionId::ChooseVoice);
            let msg = self.msg_args("voice-ready", &args!["keys" => keys]);
            self.tell(&msg);
            effects.push(Effect::Redraw);
            return effects;
        }
        effects.extend(self.choose_voice());
        effects
    }

    /// Enter on row `n` of the voice list.
    pub(crate) fn choose_voice_row(&mut self, n: usize) -> Vec<Effect> {
        let favourites = self.settings.speech.favorite_voices.clone();
        match self.voices.manager.row(n).cloned() {
            Some(VoiceRow::LanguageFilter) => {
                let cat = self.catalog();
                let s = self.voices.manager.next_language_in(&cat, &favourites);
                self.tell(&s);
                self.pending_list_focus = Some(n);
                self.show_voice_list()
            }
            Some(VoiceRow::EngineFilter) => {
                let cat = self.catalog();
                let s = self.voices.manager.next_engine_in(&cat, &favourites);
                self.tell(&s);
                self.pending_list_focus = Some(n);
                self.show_voice_list()
            }
            Some(VoiceRow::FetchCatalog) => {
                self.voices.question = Some(VoiceQuestion::FetchCatalog);
                let question = self.msg("voice-fetch-catalog-question");
                self.ask(&question);
                vec![Effect::Redraw]
            }
            Some(VoiceRow::Missing(id)) => {
                let msg = self.msg_args("voices-missing", &args!["voice" => id.as_str()]);
                self.tell(&msg);
                self.pending_list_focus = Some(n);
                self.show_voice_list()
            }
            Some(VoiceRow::Voice(_)) => {
                let Some(e) = self.voices.manager.entry_at(n).cloned() else {
                    return vec![Effect::Redraw];
                };
                match e.status {
                    VoiceStatus::Ready => self.use_voice(&e),
                    VoiceStatus::Downloadable { .. } => self.plan_download(&e),
                }
            }
            None => vec![Effect::Redraw],
        }
    }

    /// Uses `e`: on the running engine at once with a sample; on another
    /// engine by restarting speech with it.
    fn use_voice(&mut self, e: &VoiceEntry) -> Vec<Effect> {
        self.cancel_other_preview();
        self.remember_voice_params();
        if e.engine == self.speech.backend_id() {
            self.select_voice(&e.voice.id, &e.voice.name);
            return vec![Effect::Redraw];
        }
        self.stop_speech();
        self.settings.speech.backend = e.engine.clone();
        self.settings.speech.voice = Some(e.voice.id.clone());
        self.settings_dirty = true;
        self.restore_voice_params(&e.key());
        textweaver_speech::forget_probes();
        let msg = self.msg_args(
            "voice-switching-engine",
            &args!["voice" => e.voice.name.as_str(), "engine" => e.engine_name.as_str()],
        );
        self.tell(&msg);
        self.restart_speech_command()
    }

    /// Enter on a voice to download: asks Hugging Face for its files and
    /// licence (a few kilobytes), then asks the user.
    fn plan_download(&mut self, e: &VoiceEntry) -> Vec<Effect> {
        if self.voices.job.is_some() {
            let msg = self.msg("voice-download-in-progress");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let Some(store) = self.piper_store() else {
            let msg = self.msg("voice-no-data-folder");
            self.error(&msg);
            return vec![Effect::Redraw];
        };
        let Some(voice) = cached_catalog(store.dir()).and_then(|c| c.get(&e.voice.id).cloned())
        else {
            let msg = self.msg("voice-not-in-list");
            self.error(&msg);
            return vec![Effect::Redraw];
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("textweaver-voice-plan".into())
            .spawn(move || {
                let _ = tx.send(net::plan(&voice));
            });
        if spawned.is_err() {
            let msg = self.msg("voice-download-start-failed");
            self.error(&msg);
            return vec![Effect::Redraw];
        }
        self.voices.job = Some(VoiceJob::Plan(rx));
        self.list = None;
        let msg = self.msg_args(
            "voice-reading-licence",
            &args!["voice" => e.voice.name.as_str()],
        );
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    /// True when the voice on row `n` of the voice list can be removed:
    /// only a downloaded Piper voice can (W8a). The GUI's Remove button
    /// is unavailable, with the reason, on any other row.
    pub fn voice_row_removable(&self, n: usize) -> bool {
        self.voices
            .manager
            .entry_at(n)
            .is_some_and(|e| e.engine == PIPER && e.status == VoiceStatus::Ready)
    }

    /// Delete on row `n`: removes a downloaded Piper voice, after a yes.
    pub(crate) fn remove_voice_row(&mut self, n: usize) -> Vec<Effect> {
        match self.voices.manager.entry_at(n).cloned() {
            Some(e) if self.voice_row_removable(n) => {
                self.list = None;
                self.voices.question = Some(VoiceQuestion::Remove(
                    e.voice.id.clone(),
                    e.voice.name.clone(),
                ));
                let question = self.msg_args(
                    "voice-remove-question",
                    &args!["voice" => e.voice.name.as_str()],
                );
                self.ask(&question);
            }
            _ => {
                let msg = self.msg("voice-only-piper-removable");
                self.tell(&msg);
                // The list stays open where it was (before, it closed).
                self.pending_list_focus = Some(n);
                return self.show_voice_list();
            }
        }
        vec![Effect::Redraw]
    }

    /// Answers the voice manager's question.
    pub(crate) fn confirm_voice(&mut self, answer: Confirm) -> Vec<Effect> {
        let Some(q) = self.voices.question.take() else {
            return vec![Effect::Redraw];
        };
        match (answer, q) {
            (Confirm::Repeat, q) => {
                let text = match &q {
                    VoiceQuestion::Download(p) => {
                        self.msg_args("voice-download-question", &args!["plan" => p.describe()])
                    }
                    VoiceQuestion::Remove(_, name) => {
                        self.msg_args("voice-remove-question", &args!["voice" => name.as_str()])
                    }
                    VoiceQuestion::FetchCatalog => self.msg("voice-fetch-catalog-question-short"),
                };
                self.voices.question = Some(q);
                self.ask(&text);
                vec![Effect::Redraw]
            }
            (Confirm::No, _) => {
                let msg = self.msg("common-cancelled");
                self.tell(&msg);
                vec![Effect::Redraw]
            }
            (Confirm::Yes, VoiceQuestion::Download(plan)) => self.start_download(*plan),
            (Confirm::Yes, VoiceQuestion::Remove(key, name)) => {
                if self.speech.backend_id() == PIPER
                    && self.settings.speech.voice.as_deref() == Some(key.as_str())
                {
                    let msg = self.msg_args("voice-in-use", &args!["voice" => name.as_str()]);
                    self.tell(&msg);
                    return vec![Effect::Redraw];
                }
                let removed = match self.piper_store() {
                    Some(store) => store.remove(&key),
                    None => Err(textweaver_engines::piper::PiperError::NotInstalled(
                        key.clone(),
                    )),
                };
                match removed {
                    Ok(()) => {
                        textweaver_speech::forget_probes();
                        let msg = self.msg_args("voice-removed", &args!["voice" => name.as_str()]);
                        self.tell(&msg);
                    }
                    Err(e) => {
                        let msg = self.msg_args(
                            "voice-remove-failed",
                            &args!["voice" => name.as_str(), "error" => e.to_string()],
                        );
                        self.error(&msg);
                    }
                }
                vec![Effect::Redraw]
            }
            (Confirm::Yes, VoiceQuestion::FetchCatalog) => {
                let Some(store) = self.piper_store() else {
                    let msg = self.msg("voice-no-data-folder");
                    self.error(&msg);
                    return vec![Effect::Redraw];
                };
                let (tx, rx) = std::sync::mpsc::channel();
                let dir = store.dir().to_owned();
                let spawned = std::thread::Builder::new()
                    .name("textweaver-voice-catalog".into())
                    .spawn(move || {
                        let r = net::fetch_catalog_json().and_then(|(json, catalog)| {
                            std::fs::create_dir_all(&dir)
                                .and_then(|()| std::fs::write(catalog_path(&dir), json))
                                .map_err(|e| PiperError::io(&dir, e))?;
                            Ok(catalog)
                        });
                        let _ = tx.send(r);
                    });
                if spawned.is_err() {
                    let msg = self.msg("voice-download-start-failed");
                    self.error(&msg);
                } else {
                    self.voices.job = Some(VoiceJob::Catalog(rx));
                    let msg = self.msg("voice-downloading-catalog");
                    self.tell(&msg);
                }
                vec![Effect::Redraw]
            }
        }
    }

    fn start_download(&mut self, plan: DownloadPlan) -> Vec<Effect> {
        let Some(store) = self.piper_store() else {
            let msg = self.msg("voice-no-data-folder");
            self.error(&msg);
            return vec![Effect::Redraw];
        };
        let name = plan.voice.describe();
        let short = textweaver_engines::piper::catalog::display_name(&plan.voice.name);
        let total = plan.total_bytes();
        let done = Arc::new(AtomicU64::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel();
        let (d, c) = (Arc::clone(&done), Arc::clone(&cancel));
        let spawned = std::thread::Builder::new()
            .name("textweaver-voice-download".into())
            .spawn(move || {
                let r = std::fs::create_dir_all(store.dir())
                    .map_err(|e| PiperError::io(store.dir(), e))
                    .and_then(|()| {
                        net::download(&plan, &store, &mut |n, _| {
                            d.store(n, Ordering::Relaxed);
                            !c.load(Ordering::Relaxed)
                        })
                    });
                let _ = tx.send(r);
            });
        if spawned.is_err() {
            let msg = self.msg("voice-download-start-failed");
            self.error(&msg);
            return vec![Effect::Redraw];
        }
        self.voices.job = Some(VoiceJob::Download {
            name: short,
            rx,
            done,
            total,
            told: 0,
            cancel,
        });
        let msg = self.msg_args("voice-downloading", &args!["voice" => name]);
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    /// Collects the voice manager's background work.
    fn voice_job_tick(&mut self) -> Vec<Effect> {
        let Some(job) = self.voices.job.take() else {
            return Vec::new();
        };
        match job {
            VoiceJob::Plan(rx) => match rx.try_recv() {
                Ok(Ok(plan)) => {
                    let question =
                        self.msg_args("voice-download-question", &args!["plan" => plan.describe()]);
                    self.ask(&question);
                    self.voices.question = Some(VoiceQuestion::Download(Box::new(plan)));
                }
                Ok(Err(e)) => {
                    let msg =
                        self.msg_args("voice-details-failed", &args!["error" => e.to_string()]);
                    self.error(&msg);
                }
                Err(TryRecvError::Empty) => self.voices.job = Some(VoiceJob::Plan(rx)),
                Err(TryRecvError::Disconnected) => {
                    let msg = self.msg("voice-download-stopped");
                    self.error(&msg);
                }
            },
            VoiceJob::Catalog(rx) => match rx.try_recv() {
                Ok(Ok(c)) => {
                    let msg = self.msg_args(
                        "voice-catalog-fetched",
                        &args!["voices" => c.voices.len(), "languages" => c.languages().len()],
                    );
                    self.tell(&msg);
                }
                Ok(Err(e)) => {
                    let msg =
                        self.msg_args("voice-catalog-failed", &args!["error" => e.to_string()]);
                    self.error(&msg);
                }
                Err(TryRecvError::Empty) => self.voices.job = Some(VoiceJob::Catalog(rx)),
                Err(TryRecvError::Disconnected) => {
                    let msg = self.msg("voice-download-stopped");
                    self.error(&msg);
                }
            },
            VoiceJob::Download {
                name,
                rx,
                done,
                total,
                told,
                cancel,
            } => match rx.try_recv() {
                Ok(Ok(v)) => {
                    textweaver_speech::forget_probes();
                    let msg = self.msg_args(
                        "voice-installed",
                        &args!["voice" => name, "licence" => v.licence.describe()],
                    );
                    self.tell(&msg);
                }
                Ok(Err(e)) => {
                    let msg = self.msg_args(
                        "voice-download-failed",
                        &args!["voice" => name, "error" => e.to_string()],
                    );
                    self.error(&msg);
                }
                Err(TryRecvError::Empty) => {
                    // Say how it is going at each quarter.
                    let quarter = (done.load(Ordering::Relaxed) * 4)
                        .checked_div(total)
                        .unwrap_or(0);
                    let mut told = told;
                    if quarter > told && quarter < 4 {
                        told = quarter;
                        let msg = self.msg_args(
                            "voice-downloading-percent",
                            &args!["voice" => name.as_str(), "pct" => quarter * 25],
                        );
                        self.show(&msg);
                    }
                    self.voices.job = Some(VoiceJob::Download {
                        name,
                        rx,
                        done,
                        total,
                        told,
                        cancel,
                    });
                    return Vec::new();
                }
                Err(TryRecvError::Disconnected) => {
                    let msg = self.msg("voice-download-stopped");
                    self.error(&msg);
                }
            },
        }
        vec![Effect::Redraw]
    }

    /// The other engines' voices arrived while the voice list is open: the
    /// list shows them at once, keeping the focus on the same voice, and
    /// says how many came (W7v). Before, the list had to be opened again.
    fn relist_voices(&mut self, n: usize) -> Vec<Effect> {
        let textweaver_speech::VoiceList::Ready(voices) = self.speech.voice_list() else {
            return Vec::new();
        };
        let selected = self.list_model.as_ref().map_or(0, |l| l.selected);
        let keep = self.voices.manager.entry_at(selected).map(VoiceEntry::key);
        let on_control = matches!(
            self.voices.manager.row(selected),
            Some(VoiceRow::LanguageFilter | VoiceRow::EngineFilter)
        );
        let mut entries = self.gather_voices(&voices);
        entries.extend(self.voices.others.iter().flatten().cloned());
        let favourites = self.settings.speech.favorite_voices.clone();
        self.voices.manager.all_engines_listed = true;
        self.voices.manager.set_entries(entries, &favourites);
        let m = &self.voices.manager;
        self.pending_list_focus = keep
            .and_then(|k| m.row_of(&k))
            .or(on_control.then_some(selected))
            .or_else(|| m.first_voice_row());
        if n > 0 {
            let msg = self.msg_args("voice-more-ready", &args!["n" => n]);
            self.tell(&msg);
        }
        self.show_voice_list()
    }

    /// Whether the voice list is shown.
    pub fn voice_list_open(&self) -> bool {
        self.list == Some(ListKind::Voices)
    }

    /// Whether the Say Status key acts on the focused row of the list
    /// shown ([`crate::ListKey::Details`]): a preview of the file in the
    /// file browser, a sample of the voice in the voice list; in a list
    /// of commands, whose F1 explains the focused row, the list's
    /// introduction. In other lists it repeats the list's introduction.
    pub fn list_has_details(&self) -> bool {
        self.browse_location().is_some() || self.voice_list_open() || self.command_list_shown()
    }

    /// Shows the voice manager's filters and its fetch row as controls of
    /// the frontend's own (`false`: the GUI's buttons, run with
    /// `Command::VoiceControl`) or as rows of the list (`true`, the
    /// default: the terminal).
    pub fn set_voice_controls_in_list(&mut self, in_list: bool) {
        self.voices.manager.separate_controls = !in_list;
    }

    /// The labels of the controls beside the voice list, while it is
    /// shown.
    pub fn voice_controls(&self) -> Option<VoiceControls> {
        if !self.voice_list_open() {
            return None;
        }
        let m = &self.voices.manager;
        let c = self.cat();
        Some(VoiceControls {
            language: m.language_row_in(c),
            engine: m.engine_row_in(c),
            fetch: m.offer_catalog.then(|| c.tr("voices-fetch-row")),
        })
    }

    /// A control beside the voice list (`Command::VoiceControl`).
    pub(crate) fn voice_control(&mut self, control: VoiceControl) -> Vec<Effect> {
        if !self.voice_list_open() {
            return vec![Effect::Redraw];
        }
        let n = self.list_model.as_ref().map_or(0, |l| l.selected);
        match control {
            VoiceControl::NextLanguage | VoiceControl::NextEngine => {
                let keep = self.voices.manager.entry_at(n).map(VoiceEntry::key);
                let favourites = self.settings.speech.favorite_voices.clone();
                let cat = self.catalog();
                let said = if control == VoiceControl::NextLanguage {
                    self.voices.manager.next_language_in(&cat, &favourites)
                } else {
                    self.voices.manager.next_engine_in(&cat, &favourites)
                };
                self.tell(&said);
                let m = &self.voices.manager;
                self.pending_list_focus = keep
                    .and_then(|k| m.row_of(&k))
                    .or_else(|| m.first_voice_row());
                self.show_voice_list()
            }
            VoiceControl::Preview => self.preview_voice_row(n),
            VoiceControl::FetchCatalog => {
                if !self.voices.manager.offer_catalog {
                    return vec![Effect::Redraw];
                }
                // As Enter on the fetch row: the list closes for the
                // question.
                self.list = None;
                self.list_model = None;
                self.voices.question = Some(VoiceQuestion::FetchCatalog);
                let question = self.msg("voice-fetch-catalog-question");
                self.ask(&question);
                vec![Effect::Redraw]
            }
        }
    }

    /// The Say Status key in the voice list (or the GUI's Preview button):
    /// a sample in the voice on row `n`, without choosing it. The sample
    /// starts with the voice's name, said in that voice, so no spoken
    /// message comes before it; the message is shown (and reaches a screen
    /// reader). On the filter and fetch rows, the list's introduction, as
    /// in other lists.
    pub(crate) fn preview_voice_row(&mut self, n: usize) -> Vec<Effect> {
        let e = match self.voices.manager.row(n).cloned() {
            Some(VoiceRow::Voice(_)) => self.voices.manager.entry_at(n).cloned(),
            Some(VoiceRow::Missing(id)) => {
                let msg = self.msg_args("voices-missing", &args!["voice" => id.as_str()]);
                self.tell(&msg);
                return vec![Effect::Redraw];
            }
            Some(VoiceRow::LanguageFilter | VoiceRow::EngineFilter | VoiceRow::FetchCatalog) => {
                return self.repeat_list_introduction();
            }
            None => None,
        };
        let Some(e) = e else {
            return vec![Effect::Redraw];
        };
        if let VoiceStatus::Downloadable { .. } = e.status {
            let msg = self.msg_args(
                "voice-preview-not-installed",
                &args!["voice" => e.voice.name.as_str()],
            );
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let sample = self.msg_args(
            "voice-preview-sample",
            &args!["voice" => e.voice.name.as_str()],
        );
        self.cancel_other_preview();
        if e.engine == self.speech.backend_id() {
            self.stop_speech();
            let msg = self.msg_args("voice-preview", &args!["voice" => e.voice.name.as_str()]);
            self.show(&msg);
            self.speech.preview(e.voice.id.clone(), sample);
            return vec![Effect::Redraw];
        }
        self.preview_on_other_engine(&e, sample)
    }

    /// Previews `e`, a voice of an engine that is not running: starts it on
    /// a helper thread with the frontend's speech starter
    /// ([`run_other_preview`]), with the voice's own rate and pitch.
    fn preview_on_other_engine(&mut self, e: &VoiceEntry, sample: String) -> Vec<Effect> {
        let Some(starter) = self.speech_starter() else {
            let msg = self.msg_args(
                "voice-preview-unavailable",
                &args!["engine" => e.engine_name.as_str()],
            );
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        self.stop_speech();
        let mut settings = self.settings.clone();
        settings.speech.backend = e.engine.clone();
        settings.speech.voice = Some(e.voice.id.clone());
        if let Some((rate, pitch)) = remembered_params(&settings, &e.key()) {
            settings.speech.rate = rate;
            settings.speech.pitch = pitch;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel();
        let (stop, voice) = (Arc::clone(&cancel), e.voice.id.clone());
        let wake = self.waker_slot();
        let spawned = std::thread::Builder::new()
            .name("textweaver-voice-preview".into())
            .spawn(move || {
                let outcome = run_other_preview(&starter, &settings, &voice, &sample, &stop);
                let _ = tx.send(outcome);
                wake.wake();
            });
        if spawned.is_err() {
            let msg = self.msg_args(
                "voice-preview-unavailable",
                &args!["engine" => e.engine_name.as_str()],
            );
            self.error(&msg);
            return vec![Effect::Redraw];
        }
        self.voices.preview = Some(OtherPreview {
            voice: e.voice.name.clone(),
            engine: e.engine_name.clone(),
            cancel,
            rx,
        });
        let msg = self.msg_args(
            "voice-preview-starting",
            &args!["voice" => e.voice.name.as_str(), "engine" => e.engine_name.as_str()],
        );
        self.show(&msg);
        vec![Effect::Redraw]
    }

    /// Stops a preview playing on another engine, if any.
    fn cancel_other_preview(&mut self) {
        if let Some(p) = self.voices.preview.take() {
            p.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Says how a preview on another engine ended, if it failed.
    fn preview_tick(&mut self) -> Vec<Effect> {
        let Some(p) = &self.voices.preview else {
            return Vec::new();
        };
        let outcome = match p.rx.try_recv() {
            Ok(o) => o,
            Err(TryRecvError::Empty) => return Vec::new(),
            Err(TryRecvError::Disconnected) => PreviewOutcome::Done,
        };
        let Some(p) = self.voices.preview.take() else {
            return Vec::new();
        };
        let msg = match outcome {
            PreviewOutcome::Done => return Vec::new(),
            PreviewOutcome::NotStarted => self.msg_args(
                "voice-preview-engine-failed",
                &args!["voice" => p.voice.as_str(), "engine" => p.engine.as_str()],
            ),
            PreviewOutcome::Failed(e) => self.msg_args(
                "voice-preview-failed",
                &args!["voice" => p.voice.as_str(), "error" => e],
            ),
        };
        self.error(&msg);
        vec![Effect::Redraw]
    }

    /// Space in the voice list: adds the voice on row `n` to
    /// `speech.favorite_voices`, or removes it, saved at once. The list
    /// stays open, in the same order, with the item relabelled.
    pub(crate) fn toggle_favourite_voice(&mut self, n: usize) -> Vec<Effect> {
        if let Some(VoiceRow::Missing(id)) = self.voices.manager.row(n).cloned() {
            // A favorite not on this computer: Space takes it off the list.
            self.settings.speech.favorite_voices.retain(|f| *f != id);
            self.settings_dirty = true;
            let favourites = self.settings.speech.favorite_voices.clone();
            self.voices.manager.refresh(&favourites);
            let msg = self.msg_args("voice-favourite-removed", &args!["voice" => id.as_str()]);
            self.tell(&msg);
            self.pending_list_focus =
                Some(n.min(self.voices.manager.rows().len().saturating_sub(1)));
            return self.show_voice_list();
        }
        let Some(e) = self.voices.manager.entry_at(n).cloned() else {
            let msg = self.msg("voice-only-voice-favourite");
            self.tell(&msg);
            return self.show_voice_list();
        };
        let v = &e.voice;
        let favs = &mut self.settings.speech.favorite_voices;
        let added = match favs
            .iter()
            .position(|f| *f == v.id || f.eq_ignore_ascii_case(&v.name))
        {
            Some(i) => {
                favs.remove(i);
                false
            }
            None => {
                favs.push(v.id.clone());
                true
            }
        };
        let msg = self.msg_args(
            if added {
                "voice-favourite-added"
            } else {
                "voice-favourite-removed"
            },
            &args!["voice" => v.name.as_str()],
        );
        self.settings_dirty = true;
        self.tell(&msg);
        self.show_voice_list()
    }

    /// Uses voice `id` of the running engine from now on (saved in the
    /// settings), with the rate and pitch it had last time, and speaks a
    /// sample with it.
    pub(crate) fn select_voice(&mut self, id: &str, name: &str) {
        self.stop_speech();
        self.remember_voice_params();
        self.settings.speech.voice = Some(id.to_owned());
        self.settings_dirty = true;
        self.speech.set_voice(Some(id.to_owned()));
        let restored = self.restore_voice_params(&params_key(self.speech.backend_id(), id));
        let msg = match restored {
            Some(wpm) => self.msg_args("voice-chosen-rate", &args!["voice" => name, "wpm" => wpm]),
            None => self.msg_args("voice-chosen", &args!["voice" => name]),
        };
        self.tell(&msg);
        let sample = self.msg("voice-sample");
        self.speech.say(sample, textweaver_speech::SayMode::Queue);
    }

    /// Saves the current rate and pitch as the current voice's own.
    fn remember_voice_params(&mut self) {
        let (engine, voice) = self.current_voice();
        let key = params_key(&engine, voice.as_deref().unwrap_or("default"));
        let (rate, pitch) = (self.settings.speech.rate, self.settings.speech.pitch);
        remember_params(&mut self.settings, &key, rate, pitch);
        self.settings_dirty = true;
    }

    /// Applies the rate and pitch remembered for `key`, if any; returns the
    /// rate in words per minute when it did.
    fn restore_voice_params(&mut self, key: &str) -> Option<u16> {
        let (rate, pitch) = remembered_params(&self.settings, key)?;
        self.settings.speech.rate = rate;
        self.settings.speech.pitch = pitch;
        self.settings_dirty = true;
        self.speech.set_rate(rate);
        self.speech.set_pitch(pitch);
        Some(rate.wpm())
    }

    /// Sends the voice settings to the speech service.
    pub(crate) fn apply_voice_settings(&mut self) {
        let sp = &self.settings.speech;
        self.speech.set_rate(sp.rate);
        self.speech.set_pitch(sp.pitch);
        self.speech.set_volume(sp.volume);
        if sp.voice.is_some() {
            self.speech.set_voice(sp.voice.clone());
        }
        self.speech.set_punctuation(sp.punctuation);
        self.speech.set_split_caps(sp.split_caps);
    }

    pub(crate) fn set_rate(&mut self, rate: Rate) {
        self.settings.speech.rate = rate;
        self.settings_dirty = true;
        self.speech.set_rate(rate);
        // Each voice keeps its own rate, as screen readers do.
        self.remember_voice_params();
    }

    pub(crate) fn change_rate(&mut self, delta: i32) {
        let old = self.settings.speech.rate;
        let new = old.step(delta);
        if new == old {
            self.speech.earcon(Earcon::Boundary);
            let msg = self.msg(if delta > 0 {
                "voice-fastest-rate"
            } else {
                "voice-slowest-rate"
            });
            self.tell(&msg);
            return;
        }
        self.set_rate(new);
        let msg = self.msg_args("voice-rate", &args!["wpm" => new.wpm()]);
        self.tell(&msg);
    }

    pub(crate) fn change_pitch(&mut self, delta: i8) {
        let old = self.settings.speech.pitch;
        let new = old.step(delta);
        if new == old {
            self.speech.earcon(Earcon::Boundary);
            let msg = self.msg(if delta > 0 {
                "voice-highest-pitch"
            } else {
                "voice-lowest-pitch"
            });
            self.tell(&msg);
            return;
        }
        self.settings.speech.pitch = new;
        self.settings_dirty = true;
        self.speech.set_pitch(new);
        self.remember_voice_params();
        let msg = pitch_words(self.cat(), new);
        self.tell(&msg);
    }

    pub(crate) fn change_volume(&mut self, delta: i16) {
        let old = self.settings.speech.volume;
        let new = old.step(delta);
        if new == old {
            self.speech.earcon(Earcon::Boundary);
            let msg = self.msg(if delta > 0 {
                "voice-full-volume"
            } else {
                "voice-volume-off"
            });
            self.tell(&msg);
            return;
        }
        self.settings.speech.volume = new;
        self.settings_dirty = true;
        self.speech.set_volume(new);
        let msg = volume_words(self.cat(), new);
        self.tell(&msg);
    }

    /// Cycles the speed presets from fastest to slowest (star's order: skim,
    /// normal, study, slow), starting after the preset matching the rate.
    pub(crate) fn cycle_speed_preset(&mut self) {
        let mut presets: Vec<(String, u16)> = self
            .settings
            .speech
            .speed_presets
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        if presets.is_empty() {
            let msg = self.msg("voice-no-speed-presets");
            self.tell(&msg);
            return;
        }
        presets.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let wpm = self.settings.speech.rate.wpm();
        let next = match presets.iter().position(|(_, w)| *w == wpm) {
            Some(i) => (i + 1) % presets.len(),
            None => presets.iter().position(|(_, w)| *w < wpm).unwrap_or(0),
        };
        let (name, w) = presets[next].clone();
        self.set_rate(Rate::Wpm(w).clamped());
        let msg = self.msg_args(
            "voice-speed-preset",
            &args!["name" => capitalize(&name), "wpm" => Rate::Wpm(w).clamped().wpm()],
        );
        self.tell(&msg);
    }

    pub(crate) fn toggle_line_numbers(&mut self) {
        let on = !self.settings.display.show_line_numbers;
        self.settings.display.show_line_numbers = on;
        self.settings_dirty = true;
        let msg = self.msg(if on {
            "voice-line-numbers-on"
        } else {
            "voice-line-numbers-off"
        });
        self.tell(&msg);
    }

    /// F9: single-key shortcuts on or off (`[keyboard] character_keys`), so
    /// dictation and typing never trigger commands. Saved at once.
    pub(crate) fn toggle_character_keys(&mut self) {
        let on = !self.settings.keyboard.character_keys;
        self.settings.keyboard.character_keys = on;
        self.keymap.set_character_keys(on);
        self.settings_dirty = true;
        let a = Announcement::CharacterKeys { on };
        let verbosity = self.settings.speech.verbosity;
        if a.text(verbosity).is_some() {
            let text = self.msg(if on {
                "voice-character-keys-on"
            } else {
                "voice-character-keys-off"
            });
            self.say_at(&text, Verbosity::Low, a.priority());
        }
    }
}

/// "Pitch plus 2", "Pitch minus 1", "Normal pitch".
fn pitch_words(c: &textweaver_lexicon::i18n::Catalog, p: Pitch) -> String {
    match p.semitones() {
        0 => c.tr("voice-pitch-normal"),
        s if s > 0 => c.fmt(
            "voice-pitch-plus",
            &args!["n" => u8::try_from(s).unwrap_or(0)],
        ),
        s => c.fmt("voice-pitch-minus", &args!["n" => s.unsigned_abs()]),
    }
}

fn volume_words(c: &textweaver_lexicon::i18n::Catalog, v: Volume) -> String {
    c.fmt("voice-volume", &args!["pct" => v.percent()])
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::AppConfig;
    use crate::command::Command;
    use crate::list_model::ListKey;
    use crate::testing::{SpeechLog, recording_service, recording_service_as};
    use std::sync::Mutex;
    use textweaver_keymap::ActionId;

    /// An app on the recording engine (`test-recording`, with "Test
    /// voice" and "Second voice"), and its speech log.
    fn app() -> (App, SpeechLog) {
        let (speech, log) = recording_service().unwrap();
        let app = App::new(AppConfig {
            speech,
            backend_name: "test-recording".into(),
            ..AppConfig::for_tests()
        });
        app.wait_for_speech_thread();
        (app, log)
    }

    /// A voice of a second engine, `test-other`.
    fn other(id: &str, name: &str) -> VoiceEntry {
        VoiceEntry {
            engine: "test-other".into(),
            engine_name: "Other engine".into(),
            voice: textweaver_speech::Voice {
                id: id.into(),
                name: name.into(),
                languages: vec!["de-DE".into()],
                ..textweaver_speech::Voice::default()
            },
            status: VoiceStatus::Ready,
        }
    }

    fn items(app: &App) -> Vec<String> {
        app.list_model()
            .map(|l| l.items.clone())
            .unwrap_or_default()
    }

    fn selected(app: &App) -> usize {
        app.list_model().map_or(usize::MAX, |l| l.selected)
    }

    /// Waits for a preview on another engine to end (at most 5 seconds).
    fn wait_for_preview(app: &mut App) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while app.voices.preview.is_some() && std::time::Instant::now() < deadline {
            let _ = app.tick(std::time::Instant::now());
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(app.voices.preview.is_none(), "the preview ended");
    }

    /// The GUI's voice manager: only voices in the list; the filters are
    /// controls that keep the focus on the same voice while it is shown.
    #[test]
    fn separate_controls_filter_and_keep_the_focused_voice() {
        let (mut app, _) = app();
        app.set_voice_controls_in_list(false);
        app.voices.others = Some(vec![other("hedda", "Hedda")]);
        assert_eq!(app.voice_controls(), None, "only while the list is open");
        app.dispatch(Command::Action(ActionId::ChooseVoice));
        assert!(app.voice_list_open());
        assert_eq!(items(&app).len(), 3, "{:?}", items(&app));
        assert!(items(&app)[2].starts_with("Hedda, de-DE, Other engine"));
        let c = app.voice_controls().unwrap();
        assert_eq!(c.language, "Language: all languages");
        assert_eq!(c.engine, "Engine: all engines");
        assert_eq!(c.fetch, None, "no data folder, nothing to fetch");
        app.dispatch(Command::ListKey(ListKey::Down));
        assert_eq!(selected(&app), 1);
        app.dispatch(Command::VoiceControl(VoiceControl::NextEngine));
        assert!(
            app.status_text()
                .starts_with("2 voices: all languages, test-recording."),
            "{}",
            app.status_text()
        );
        assert_eq!(
            app.voice_controls().unwrap().engine,
            "Engine: test-recording"
        );
        assert_eq!(selected(&app), 1, "still on Second voice");
        assert!(items(&app)[1].starts_with("Second voice"));
        app.dispatch(Command::VoiceControl(VoiceControl::NextEngine));
        assert_eq!(items(&app).len(), 1);
        assert_eq!(selected(&app), 0, "the first voice shown");
        app.dispatch(Command::VoiceControl(VoiceControl::NextLanguage));
        assert_eq!(app.voice_controls().unwrap().language, "Language: German");
        // Escape closes the list; a control then does nothing.
        app.dispatch(Command::ListKey(ListKey::Escape));
        assert!(!app.voice_list_open());
        app.dispatch(Command::VoiceControl(VoiceControl::NextEngine));
        assert!(app.list_model().is_none());
    }

    /// The Say Status key previews a voice of the running engine in its
    /// own voice, without choosing it; the list stays open.
    #[test]
    fn preview_speaks_a_sample_without_choosing_the_voice() {
        let (mut app, log) = app();
        app.dispatch(Command::Action(ActionId::ChooseVoice));
        // The introduction names the chord (Alt+End), never the single key
        // (z), which jumps by letter in a list.
        let intro = app.status_text().to_owned();
        assert!(intro.contains("+End previews a voice;"), "{intro}");
        // Terminal rows: the two filters, then the voices.
        app.dispatch(Command::ListFocus(3));
        app.dispatch(Command::ListKey(ListKey::Details));
        app.wait_for_speech_thread();
        assert_eq!(app.status_text(), "Preview: Second voice.");
        assert_eq!(
            log.texts().last().map(String::as_str),
            Some("Second voice. The quick brown fox jumps over the lazy dog.")
        );
        let voices: Vec<Option<String>> = log.all_params().into_iter().map(|p| p.voice).collect();
        assert!(voices.contains(&Some("second".into())), "{voices:?}");
        assert_ne!(
            log.params().and_then(|p| p.voice).as_deref(),
            Some("second")
        );
        assert_eq!(app.settings.speech.voice, None, "not chosen");
        assert!(app.voice_list_open());
        // On a filter row, the key repeats the list's introduction.
        app.dispatch(Command::ListFocus(0));
        app.dispatch(Command::ListKey(ListKey::Details));
        assert!(app.status_text().starts_with("Voice manager."));
    }

    /// Another engine's voice is previewed by starting that engine with
    /// the frontend's starter, then closing it; an engine that cannot
    /// start says so.
    #[test]
    fn preview_on_another_engine_starts_it_and_says_when_it_cannot() {
        let (mut app, main_log) = app();
        let other_log: Arc<Mutex<Option<SpeechLog>>> = Arc::default();
        let slot = Arc::clone(&other_log);
        let starter: SpeechStarter = Arc::new(move |s| {
            let id = if s.speech.backend == "test-other" {
                "test-other"
            } else {
                "test-recording"
            };
            let (svc, log) = recording_service_as(id).unwrap();
            *slot.lock().unwrap() = Some(log);
            (svc, "Other engine".into(), Vec::new())
        });
        app.set_speech_starter(starter);
        app.set_voice_controls_in_list(false);
        app.voices.others = Some(vec![other("hedda", "Hedda")]);
        app.dispatch(Command::Action(ActionId::ChooseVoice));
        app.dispatch(Command::ListFocus(2));
        app.dispatch(Command::VoiceControl(VoiceControl::Preview));
        assert_eq!(app.status_text(), "Preview: Hedda, starting Other engine.");
        wait_for_preview(&mut app);
        let log = other_log
            .lock()
            .unwrap()
            .clone()
            .expect("the engine started");
        assert_eq!(
            log.texts(),
            ["Hedda. The quick brown fox jumps over the lazy dog."]
        );
        assert_eq!(log.params().and_then(|p| p.voice).as_deref(), Some("hedda"));
        assert_eq!(app.settings.speech.backend, "auto", "nothing switched");
        assert!(
            !main_log.texts().iter().any(|t| t.starts_with("Hedda.")),
            "not on the running engine"
        );
        // The engine that starts is another one: the preview says so.
        app.voices.others = Some(vec![VoiceEntry {
            engine: "test-missing".into(),
            ..other("anna", "Anna")
        }]);
        app.dispatch(Command::ListKey(ListKey::Escape));
        app.dispatch(Command::Action(ActionId::ChooseVoice));
        app.dispatch(Command::ListFocus(2));
        app.dispatch(Command::ListKey(ListKey::Details));
        wait_for_preview(&mut app);
        assert_eq!(
            app.status_text(),
            "Error: Could not start Other engine to preview Anna."
        );
    }

    /// A Piper voice to download, and a favorite not on this computer,
    /// cannot be previewed, and say why.
    #[test]
    fn previews_of_voices_not_here_say_why() {
        let (mut app, _) = app();
        app.set_voice_controls_in_list(false);
        app.settings.speech.favorite_voices = vec!["eci:reed".into()];
        app.voices.others = Some(vec![VoiceEntry {
            engine: PIPER.into(),
            engine_name: "Piper neural voices".into(),
            status: VoiceStatus::Downloadable {
                bytes: 63_000_000,
                licence: textweaver_engines::piper::Licence::classify("CC0"),
            },
            ..other("en_US-amy-low", "Amy (low)")
        }]);
        app.dispatch(Command::Action(ActionId::ChooseVoice));
        let rows = items(&app);
        assert_eq!(
            rows.last().map(String::as_str),
            Some("eci:reed, favorite, not on this computer")
        );
        app.dispatch(Command::ListFocus(2));
        app.dispatch(Command::ListKey(ListKey::Details));
        assert_eq!(
            app.status_text(),
            "Amy (low) is not downloaded yet. Enter downloads it, after a question."
        );
        app.dispatch(Command::ListFocus(rows.len() - 1));
        app.dispatch(Command::ListKey(ListKey::Details));
        assert!(
            app.status_text()
                .starts_with("eci:reed is not on this computer.")
        );
    }

    /// The other engines' voices arriving while the list is open are
    /// listed at once, with the focus kept, and a favorite no engine has
    /// shows as not on this computer.
    #[test]
    fn other_engines_voices_join_the_open_list() {
        let (mut app, _) = app();
        app.settings.speech.favorite_voices = vec!["eci:reed".into()];
        let (tx, rx) = std::sync::mpsc::channel();
        app.voices.others_rx = Some(rx);
        app.dispatch(Command::Action(ActionId::ChooseVoice));
        assert_eq!(items(&app).len(), 4, "filters and two voices");
        app.dispatch(Command::ListFocus(3));
        tx.send(vec![other("hedda", "Hedda")]).unwrap();
        let _ = app.tick(std::time::Instant::now());
        let now = items(&app);
        assert_eq!(now.len(), 6, "{now:?}");
        assert!(now[4].starts_with("Hedda"));
        assert_eq!(now[5], "eci:reed, favorite, not on this computer");
        assert_eq!(selected(&app), 3, "still on Second voice");
        assert!(now[3].starts_with("Second voice"));
        assert!(
            app.status_text()
                .starts_with("One more voice from another engine is listed."),
            "{}",
            app.status_text()
        );
    }

    /// Delete on a voice that cannot be removed says so and keeps the list
    /// open where it was; Fetch closes it for its question.
    #[test]
    fn remove_keeps_the_list_and_fetch_asks() {
        let (mut app, _) = app();
        app.set_voice_controls_in_list(false);
        app.dispatch(Command::Action(ActionId::ChooseVoice));
        app.dispatch(Command::ListFocus(1));
        app.dispatch(Command::ListKey(ListKey::Delete));
        assert!(
            app.status_text()
                .starts_with("Only downloaded Piper voices can be removed."),
            "{}",
            app.status_text()
        );
        assert!(app.voice_list_open());
        assert_eq!(selected(&app), 1);
        app.voices.manager.offer_catalog = true;
        assert!(app.voice_controls().unwrap().fetch.is_some());
        app.dispatch(Command::VoiceControl(VoiceControl::FetchCatalog));
        assert!(!app.voice_list_open());
        assert!(app.list_model().is_none());
        assert!(app.confirmation_pending());
        assert!(
            app.status_text()
                .starts_with("Download the list of Piper voices")
        );
    }

    #[test]
    fn voice_words_read_well() {
        let c = textweaver_lexicon::i18n::Catalog::english();
        assert_eq!(pitch_words(&c, Pitch::Semitones(-2)), "Pitch minus 2.");
        assert_eq!(pitch_words(&c, Pitch::Semitones(0)), "Normal pitch.");
        assert_eq!(volume_words(&c, Volume::new(90)), "Volume 90 percent.");
    }
}
