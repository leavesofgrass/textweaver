//! Audio export from the menus (File, Export audio; ADR-0011).
//!
//! Two short questions, each with its default first, then a yes or no:
//!
//! 1. **The format**: FLAC first (lossless, half WAV's size, written in
//!    process), then WAV; MP3 and M4B only when ffmpeg is found, and the
//!    first question says in words when it is not.
//! 2. **Where**: beside the document (the default, `essay.flac`), or
//!    another folder chosen in the file browser ([`App::choose_folder`]).
//!
//! Then "Export essay.flac with Microsoft David at 200 words a minute?
//! y or n", naming the voice and speed the export uses: the reader's own
//! engine when it can write audio files, else the best one that can
//! (engines that only play, such as Omnivox, are never offered). The work
//! runs on its own thread with its own engine, through
//! [`textweaver_export::export`], so the reader stays usable. Progress is
//! said in tens of percent, never more often than every ten seconds
//! ([`Importance::Progress`], which the interface setting may quiet).
//! Escape while it runs asks "Stop the export?"; the export stops between
//! sentences and leaves no half-written file.
//!
//! At the end: "Wrote essay.flac: 42 minutes, 12 chapters. Open it? y or
//! n." ([`Importance::Question`], never silenced), or without the question
//! when another list or question is open.
//!
//! Without the `audio-export` feature the command stays hidden, like every
//! pending command ([`crate::menu::PENDING`]).
//!
//! [`Importance::Progress`]: textweaver_a11y::Importance::Progress
//! [`Importance::Question`]: textweaver_a11y::Importance::Question

/// What an audio export list is for, so [`App`](crate::app::App)'s Enter
/// can act.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(feature = "audio-export"), allow(dead_code))]
pub(crate) enum AudioList {
    /// The formats, in the order shown.
    Format,
    /// Where the file goes: beside the document, or another folder.
    Where,
}

#[cfg(feature = "audio-export")]
pub(crate) use run::AudioState;

#[cfg(not(feature = "audio-export"))]
#[derive(Default)]
pub(crate) struct AudioState(());

#[cfg(not(feature = "audio-export"))]
impl crate::app::App {
    pub(crate) fn audio_running(&self) -> bool {
        false
    }
    pub(crate) fn audio_question(&self) -> bool {
        false
    }
    pub(crate) fn confirm_audio(
        &mut self,
        _: crate::command::Confirm,
    ) -> Vec<crate::command::Effect> {
        vec![crate::command::Effect::Redraw]
    }
    pub(crate) fn ask_stop_audio(&mut self) -> Vec<crate::command::Effect> {
        vec![crate::command::Effect::Redraw]
    }
    pub(crate) fn audio_tick(&mut self, _: std::time::Instant) -> Vec<crate::command::Effect> {
        Vec::new()
    }
    pub(crate) fn choose_audio(&mut self, _: AudioList, _: usize) -> Vec<crate::command::Effect> {
        vec![crate::command::Effect::Redraw]
    }
}

/// Gives the Export audio command its handler (with `audio-export`).
pub(crate) fn register(app: &mut crate::app::App) {
    #[cfg(feature = "audio-export")]
    app.register_handler(textweaver_keymap::ActionId::ExportAudio, |app| {
        app.export_audio()
    });
    #[cfg(not(feature = "audio-export"))]
    let _ = app;
}

#[cfg(feature = "audio-export")]
mod run {
    use std::ops::ControlFlow;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::mpsc::{Receiver, TryRecvError};
    use std::time::{Duration, Instant};

    use textweaver_a11y::{Importance, Priority};
    use textweaver_export::{
        AudioFormat, CueOptions, ExportOptions, ExportReport, SubtitleRequest, ffmpeg,
    };
    use textweaver_lexicon::args;
    use textweaver_speech::{BackendFactory, BackendInfo, BackendRegistry, Caps, VoiceParams};
    use textweaver_text::Document;

    use super::AudioList;
    use crate::app::{App, ListKind};
    use crate::command::{Confirm, Effect};

    /// How often progress may be said, at most.
    pub(crate) const PROGRESS_EVERY: Duration = Duration::from_secs(10);

    /// The export being set up, the question open, and the run.
    #[derive(Default)]
    pub(crate) struct AudioState {
        /// The formats offered this time, in order.
        formats: Vec<AudioFormat>,
        format: Option<AudioFormat>,
        /// The ffmpeg found when the export began.
        ffmpeg: Option<PathBuf>,
        question: Option<Question>,
        run: Option<Run>,
        /// Tests: the engines to choose from instead of the settings'.
        pub(crate) registry: Option<BackendRegistry>,
        /// Tests: the ffmpeg to use (`Some(None)`: none) instead of the
        /// one on `PATH`.
        pub(crate) ffmpeg_override: Option<Option<PathBuf>>,
    }

    enum Question {
        /// "Export essay.flac with ...? y or n".
        Start(Box<Plan>),
        /// "Stop the export? y or n".
        Stop,
    }

    /// Everything the export thread needs.
    struct Plan {
        doc: Document,
        out: PathBuf,
        format: AudioFormat,
        backend: BackendInfo,
        factory: BackendFactory,
        params: VoiceParams,
        voice: String,
        wpm: u16,
        ffmpeg: Option<PathBuf>,
        options: ExportOptions,
        subtitles: Option<SubtitleRequest>,
    }

    /// What the export thread sends at the end.
    type Finished = Result<ExportReport, Ended>;

    /// How an export ended without a file.
    enum Ended {
        /// Escape, then yes.
        Cancelled,
        /// The engine or the file failed; the reason.
        Failed(String),
    }

    struct Run {
        out: PathBuf,
        done: Arc<AtomicUsize>,
        total: Arc<AtomicUsize>,
        cancel: Arc<AtomicBool>,
        rx: Receiver<Finished>,
        /// When progress was last said, and the tens of percent it said.
        said_at: Instant,
        said_tens: usize,
    }

    /// The engine an export uses: the reader's own when it can write audio
    /// files, else the one in `[speech] backend` when it can, else the
    /// highest-priority available engine that can (engines used only when
    /// named, such as the test double, are passed over then).
    pub(crate) fn choose_backend(
        list: &[BackendInfo],
        running: &str,
        configured: &str,
    ) -> Option<BackendInfo> {
        let writes = |b: &&BackendInfo| b.available && b.caps.contains(Caps::SYNTH_TO_FILE);
        let named = |id: &str| list.iter().filter(writes).find(|b| b.id == id).cloned();
        named(running)
            .or_else(|| named(configured))
            .or_else(|| list.iter().filter(writes).find(|b| !b.opt_in).cloned())
    }

    /// The formats offered: FLAC and WAV always, MP3 and M4B with ffmpeg.
    pub(crate) fn formats(ffmpeg: bool) -> Vec<AudioFormat> {
        let mut f = vec![AudioFormat::Flac, AudioFormat::Wav];
        if ffmpeg {
            f.extend([AudioFormat::Mp3, AudioFormat::M4b]);
        }
        f.retain(|f| ffmpeg || !f.needs_ffmpeg());
        f
    }

    fn extension(f: AudioFormat) -> &'static str {
        match f {
            AudioFormat::Wav => "wav",
            AudioFormat::Flac => "flac",
            AudioFormat::Mp3 => "mp3",
            AudioFormat::M4b => "m4b",
        }
    }

    impl App {
        /// The document being read (as last opened or reloaded), its
        /// folder, and its base name.
        fn audio_source(&self) -> Option<(Document, PathBuf, String)> {
            let doc = self.session.as_ref()?.doc.clone();
            let path = doc.meta.path.clone();
            let folder = path
                .as_ref()
                .and_then(|p| p.parent().map(Path::to_owned))
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| self.loose_folder());
            let stem = path
                .as_ref()
                .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
                .or_else(|| self.session.as_ref().map(|s| s.title.clone()))
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "untitled".to_owned());
            Some((doc, folder, stem))
        }

        /// File, Export audio: the format first.
        pub(crate) fn export_audio(&mut self) -> Vec<Effect> {
            if let Some(run) = &self.audio.run {
                let msg = self.msg_args("audio-busy", &args!["name" => display_name(&run.out)]);
                self.tell(&msg);
                return vec![Effect::Redraw];
            }
            self.audio.question = None;
            let Some((_, _, stem)) = self.audio_source() else {
                let open = self.key(textweaver_keymap::ActionId::Open);
                let msg = self.msg_args("app-no-document-open", &args!["key" => open]);
                self.tell(&msg);
                return vec![Effect::Redraw];
            };
            let ffmpeg = match &self.audio.ffmpeg_override {
                Some(f) => f.clone(),
                None => ffmpeg::find(),
            };
            self.audio.formats = formats(ffmpeg.is_some());
            self.audio.ffmpeg = ffmpeg;
            self.audio.format = None;
            let items: Vec<String> = self
                .audio
                .formats
                .clone()
                .into_iter()
                .map(|f| self.format_item(f))
                .collect();
            let mut msg = self.msg_args(
                "audio-format-intro",
                &args!["name" => stem, "n" => items.len()],
            );
            if self.audio.ffmpeg.is_none() {
                msg.push(' ');
                msg.push_str(&self.msg("audio-no-ffmpeg"));
            }
            self.list = Some(ListKind::Audio(AudioList::Format));
            self.tell(&msg);
            vec![Effect::ShowList {
                title: self.msg("audio-format-title"),
                items,
            }]
        }

        fn format_item(&self, f: AudioFormat) -> String {
            self.msg(match f {
                AudioFormat::Flac => "audio-format-flac",
                AudioFormat::Wav => "audio-format-wav",
                AudioFormat::Mp3 => "audio-format-mp3",
                AudioFormat::M4b => "audio-format-m4b",
            })
        }

        /// The file beside the document in the chosen format.
        fn default_audio_out(&self) -> Option<PathBuf> {
            let (_, folder, stem) = self.audio_source()?;
            let f = self.audio.format?;
            Some(folder.join(format!("{stem}.{}", extension(f))))
        }

        /// Enter in an audio export list.
        pub(crate) fn choose_audio(&mut self, list: AudioList, n: usize) -> Vec<Effect> {
            match list {
                AudioList::Format => match self.audio.formats.get(n) {
                    Some(&f) => {
                        self.audio.format = Some(f);
                        self.audio_where()
                    }
                    None => vec![Effect::Redraw],
                },
                AudioList::Where => match n {
                    0 => match self.default_audio_out() {
                        Some(out) => self.audio_plan(out),
                        None => vec![Effect::Redraw],
                    },
                    _ => {
                        let purpose = self.msg("audio-choose-folder");
                        self.choose_folder(&purpose, |app, folder| {
                            let name = app
                                .default_audio_out()
                                .and_then(|p| p.file_name().map(ToOwned::to_owned));
                            match name {
                                Some(name) => app.audio_plan(folder.join(name)),
                                None => vec![Effect::Redraw],
                            }
                        })
                    }
                },
            }
        }

        /// The where list: beside the document, or another folder.
        fn audio_where(&mut self) -> Vec<Effect> {
            let beside = self
                .default_audio_out()
                .map_or_else(String::new, |p| p.display().to_string());
            let items = vec![
                self.msg_args("audio-where-beside", &args!["path" => beside]),
                self.msg("audio-where-choose"),
            ];
            self.list = Some(ListKind::Audio(AudioList::Where));
            let msg = self.msg("audio-where-intro");
            self.tell(&msg);
            vec![Effect::ShowList {
                title: self.msg("audio-where-title"),
                items,
            }]
        }

        /// Chooses the engine and voice, and asks to start.
        fn audio_plan(&mut self, out: PathBuf) -> Vec<Effect> {
            let (Some(format), Some((doc, _, _))) = (self.audio.format, self.audio_source()) else {
                return vec![Effect::Redraw];
            };
            let registry = self
                .audio
                .registry
                .clone()
                .unwrap_or_else(|| textweaver_engines::speech_registry_for(&self.settings));
            let running = self.speech.backend_id().to_owned();
            let Some(backend) =
                choose_backend(&registry.list(), &running, &self.settings.speech.backend)
            else {
                let msg = self.msg("audio-no-engine");
                self.error(&msg);
                return vec![Effect::Redraw];
            };
            let Some(factory) = registry.factory(backend.id) else {
                let msg = self.msg("audio-no-engine");
                self.error(&msg);
                return vec![Effect::Redraw];
            };
            let config = textweaver_engines::service_config(&self.settings);
            // A configured voice belongs to the engine it was chosen in.
            let same = backend.id == self.settings.speech.backend || backend.id == running;
            let params = VoiceParams {
                voice: if same {
                    self.settings.speech.voice.clone()
                } else {
                    None
                },
                ..config.params.clone()
            };
            let voice = self.voice_label(&backend, &running, params.voice.as_deref());
            let wpm = self.settings.speech.rate.wpm();
            let plan = crate::export::subtitle_plan(&self.settings, &out, None, false);
            let subtitles = plan.path.map(|path| SubtitleRequest {
                path,
                cues: CueOptions {
                    word_level: plan.word_level,
                    ..CueOptions::default()
                },
            });
            let options = ExportOptions {
                narration: crate::playback::narration_policy(&self.settings),
                normalize: config.normalize,
                punctuation: config.punctuation,
                split_caps: config.split_caps,
                ..ExportOptions::default()
            };
            let plan = Plan {
                doc,
                out,
                format,
                backend,
                factory,
                params,
                voice,
                wpm,
                ffmpeg: self.audio.ffmpeg.clone(),
                options,
                subtitles,
            };
            let question = self.audio_start_question(&plan);
            self.audio.question = Some(Question::Start(Box::new(plan)));
            self.ask(&question);
            vec![Effect::Redraw]
        }

        /// The voice as it is said: its name when the reader's engine
        /// lists it, the engine's name for its default voice.
        fn voice_label(&self, backend: &BackendInfo, running: &str, voice: Option<&str>) -> String {
            let Some(id) = voice else {
                return backend.name.to_owned();
            };
            if backend.id == running
                && let textweaver_speech::VoiceList::Ready(list) = self.speech.voice_list()
                && let Some(v) = list.iter().find(|v| v.id == id)
            {
                return v.name.clone();
            }
            id.to_owned()
        }

        fn audio_start_question(&self, plan: &Plan) -> String {
            self.msg_args(
                "audio-confirm",
                &args![
                    "name" => display_name(&plan.out),
                    "voice" => plan.voice.clone(),
                    "wpm" => plan.wpm,
                    "path" => plan
                        .out
                        .parent()
                        .map_or_else(String::new, |p| p.display().to_string())
                ],
            )
        }

        /// True while an export runs.
        pub(crate) fn audio_running(&self) -> bool {
            self.audio.run.is_some()
        }

        /// True while an export question waits for y or n.
        pub(crate) fn audio_question(&self) -> bool {
            self.audio.question.is_some()
        }

        /// Escape while an export runs: asks before stopping it.
        pub(crate) fn ask_stop_audio(&mut self) -> Vec<Effect> {
            self.audio.question = Some(Question::Stop);
            let q = self.msg("audio-stop-question");
            self.ask(&q);
            vec![Effect::Redraw]
        }

        /// Answers an export question.
        pub(crate) fn confirm_audio(&mut self, answer: Confirm) -> Vec<Effect> {
            let Some(question) = self.audio.question.take() else {
                return vec![Effect::Redraw];
            };
            match (question, answer) {
                (Question::Start(plan), Confirm::Yes) => self.audio_start(*plan),
                (Question::Stop, Confirm::Yes) => {
                    if let Some(run) = &self.audio.run {
                        run.cancel.store(true, Ordering::SeqCst);
                    }
                    let msg = self.msg("audio-stopping");
                    self.tell(&msg);
                    vec![Effect::Redraw]
                }
                (Question::Start(_), Confirm::No) => {
                    let msg = self.msg("common-cancelled");
                    self.tell(&msg);
                    vec![Effect::Redraw]
                }
                (Question::Stop, Confirm::No) => {
                    let msg = self.msg("audio-still-exporting");
                    self.tell(&msg);
                    vec![Effect::Redraw]
                }
                (q, Confirm::Repeat) => {
                    let text = match &q {
                        Question::Start(plan) => self.audio_start_question(plan),
                        Question::Stop => self.msg("audio-stop-question"),
                    };
                    self.audio.question = Some(q);
                    self.ask(&text);
                    vec![Effect::Redraw]
                }
            }
        }

        /// Starts the export on its own thread, with its own engine.
        fn audio_start(&mut self, plan: Plan) -> Vec<Effect> {
            let out = plan.out.clone();
            let format = plan.format;
            let done = Arc::new(AtomicUsize::new(0));
            let total = Arc::new(AtomicUsize::new(0));
            let cancel = Arc::new(AtomicBool::new(false));
            let (tx, rx) = std::sync::mpsc::channel();
            let wake = self.waker_slot();
            let (d, t, c) = (Arc::clone(&done), Arc::clone(&total), Arc::clone(&cancel));
            let spawned = std::thread::Builder::new()
                .name("tw-audio-export".into())
                .spawn(move || {
                    let result = run_export(plan, &d, &t, &c);
                    let _ = tx.send(result);
                    wake.wake();
                });
            if let Err(e) = spawned {
                let msg = self.msg_args("audio-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
                return vec![Effect::Redraw];
            }
            self.audio.run = Some(Run {
                out: out.clone(),
                done,
                total,
                cancel,
                rx,
                said_at: Instant::now(),
                said_tens: 0,
            });
            let msg = self.msg_args(
                "audio-started",
                &args!["name" => display_name(&out), "format" => format.name()],
            );
            self.tell(&msg);
            vec![Effect::Redraw]
        }

        /// From [`App::tick`]: progress, and the end of a run.
        pub(crate) fn audio_tick(&mut self, now: Instant) -> Vec<Effect> {
            let Some(run) = self.audio.run.as_mut() else {
                return Vec::new();
            };
            match run.rx.try_recv() {
                Ok(result) => {
                    let out = self.audio.run.take().map(|r| r.out).unwrap_or_default();
                    self.audio_finished(&out, result)
                }
                Err(TryRecvError::Empty) => {
                    let done = run.done.load(Ordering::Relaxed);
                    let total = run.total.load(Ordering::Relaxed);
                    let tens = (done * 10).checked_div(total).unwrap_or(0).min(9);
                    if tens > run.said_tens && now.duration_since(run.said_at) >= PROGRESS_EVERY {
                        run.said_tens = tens;
                        run.said_at = now;
                        let msg = self.msg_args("audio-progress", &args!["percent" => tens * 10]);
                        self.announce_as(&msg, Priority::Polite, Importance::Progress);
                    }
                    Vec::new()
                }
                Err(TryRecvError::Disconnected) => {
                    self.audio.run = None;
                    let msg = self.msg("audio-thread-stopped");
                    self.error(&msg);
                    vec![Effect::Redraw]
                }
            }
        }

        /// Says how the export ended: the file, its length, and its
        /// chapters first, then offers to open it.
        fn audio_finished(&mut self, out: &Path, result: Finished) -> Vec<Effect> {
            let report = match result {
                Ok(r) => r,
                Err(Ended::Cancelled) => {
                    let msg = self.msg("audio-stopped");
                    self.tell(&msg);
                    return vec![Effect::Redraw];
                }
                Err(Ended::Failed(e)) => {
                    let msg = self.msg_args("audio-failed", &args!["error" => e]);
                    self.error(&msg);
                    return vec![Effect::Redraw];
                }
            };
            let t = &report.timeline;
            let length =
                textweaver_lexicon::i18n::duration(self.cat(), t.duration_ms as f64 / 1000.0);
            let mut text = self.msg_args(
                "audio-done",
                &args![
                    "name" => display_name(out),
                    "length" => length,
                    "chapters" => t.chapters.len()
                ],
            );
            if let Some(sub) = &report.subtitles {
                text.push(' ');
                text.push_str(
                    &self.msg_args("audio-subtitles", &args!["name" => display_name(sub)]),
                );
            }
            // With another list or question open, the result alone.
            if self.list.is_some() || self.mode.is_prompt() || self.confirmation_pending() {
                self.announce_as(&text, Priority::Polite, Importance::Answer);
                return vec![Effect::Redraw];
            }
            text.push(' ');
            text.push_str(&self.msg("tasks-open-it-question"));
            self.offer_open(out.display().to_string(), &text);
            vec![Effect::Redraw]
        }
    }

    /// The export itself, on its own thread.
    fn run_export(
        plan: Plan,
        done: &AtomicUsize,
        total: &AtomicUsize,
        cancel: &AtomicBool,
    ) -> Finished {
        let fail = |e: String| Ended::Failed(e);
        let mut backend = (plan.factory)().map_err(|e| fail(e.to_string()))?;
        backend
            .set_params(&plan.params)
            .map_err(|e| fail(e.to_string()))?;
        if !backend.capabilities().contains(Caps::SYNTH_TO_FILE) {
            return Err(fail(format!(
                "{} cannot write audio files",
                plan.backend.name
            )));
        }
        let result = textweaver_export::export(
            &plan.doc,
            backend.as_mut(),
            &plan.out,
            plan.subtitles.as_ref(),
            plan.ffmpeg.as_deref(),
            &plan.options,
            &mut |p| {
                total.store(p.total, Ordering::Relaxed);
                done.store(p.done, Ordering::Relaxed);
                if cancel.load(Ordering::SeqCst) {
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            },
        );
        match result {
            Ok(r) => Ok(r),
            Err(textweaver_export::ExportError::Cancelled) => Err(Ended::Cancelled),
            Err(e) => Err(fail(e.to_string())),
        }
    }

    /// A file's own name, or its whole path.
    fn display_name(p: &Path) -> String {
        p.file_name().map_or_else(
            || p.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        )
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::command::Command;
        use textweaver_keymap::ActionId;

        /// An app reading a two-heading document from `dir`, exporting
        /// with the recording double and no ffmpeg; launches are recorded.
        fn app_with_doc(dir: &Path) -> (App, Arc<std::sync::Mutex<Vec<String>>>) {
            let file = dir.join("essay.md");
            std::fs::write(
                &file,
                "# Light\n\nPlants use light.\n\n# Water\n\nRoots drink.\n",
            )
            .unwrap();
            let mut config = crate::AppConfig::for_tests();
            config.settings.speech.backend = "recording".into();
            let mut app = App::new(config);
            app.audio.registry = Some(BackendRegistry::with_builtins());
            app.audio.ffmpeg_override = Some(None);
            let opened = Arc::new(std::sync::Mutex::new(Vec::new()));
            let o = Arc::clone(&opened);
            app.set_launcher(Arc::new(move |t: &str| {
                o.lock().unwrap().push(t.to_owned());
                Ok(())
            }));
            app.dispatch(Command::Open(file));
            (app, opened)
        }

        fn wait_until_done(app: &mut App) {
            let deadline = Instant::now() + Duration::from_secs(60);
            while app.audio_running() && Instant::now() < deadline {
                let _ = app.tick(Instant::now());
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(!app.audio_running(), "the export did not finish in time");
        }

        #[test]
        fn engines_that_write_files_are_chosen_in_order() {
            let info = |id: &'static str, prio, opt_in, files: bool| BackendInfo {
                id,
                name: id,
                priority: prio,
                opt_in,
                available: true,
                caps: if files {
                    Caps::SYNTH_TO_FILE
                } else {
                    Caps::empty()
                },
            };
            let list = [
                info("omnivox", 900, false, false),
                info("sapi", 500, false, true),
                info("espeak", 100, false, true),
                info("recording", 0, true, true),
            ];
            let id = |b: Option<BackendInfo>| b.map(|b| b.id);
            // The reader's engine when it writes files.
            assert_eq!(id(choose_backend(&list, "espeak", "")), Some("espeak"));
            // An engine that only plays is passed over for the best.
            assert_eq!(id(choose_backend(&list, "omnivox", "")), Some("sapi"));
            // The configured engine, even one used only when named.
            assert_eq!(
                id(choose_backend(&list, "null", "recording")),
                Some("recording")
            );
            assert_eq!(id(choose_backend(&list[..1], "omnivox", "")), None);
        }

        #[test]
        fn mp3_and_m4b_only_with_ffmpeg() {
            assert_eq!(formats(false), [AudioFormat::Flac, AudioFormat::Wav]);
            assert_eq!(
                formats(true),
                [
                    AudioFormat::Flac,
                    AudioFormat::Wav,
                    AudioFormat::Mp3,
                    AudioFormat::M4b
                ]
            );
        }

        #[test]
        fn the_flow_writes_flac_beside_the_document_and_offers_to_open_it() {
            let dir = tempfile::tempdir().unwrap();
            let (mut app, opened) = app_with_doc(dir.path());
            let effects = app.dispatch(Command::Action(ActionId::ExportAudio));
            // FLAC first; no ffmpeg, said in words (then the first item).
            assert!(
                app.status_text().starts_with(
                    "Export essay as audio: choose a format, 2 choices. \
                     MP3 and M4B need ffmpeg, which was not found."
                ),
                "{}",
                app.status_text()
            );
            let items = effects.iter().find_map(|e| match e {
                Effect::ShowList { items, .. } => Some(items.clone()),
                _ => None,
            });
            assert_eq!(
                items.as_deref().map(|i| i[0].as_str()),
                Some("FLAC: lossless, about half the size of WAV")
            );
            app.dispatch(Command::Choose(0));
            assert!(
                app.status_text().starts_with("Where should the audio go?"),
                "{}",
                app.status_text()
            );
            app.dispatch(Command::Choose(0));
            let out = dir.path().join("essay.flac");
            let folder = dir.path().display().to_string();
            assert_eq!(
                app.status_text(),
                format!(
                    "Export essay.flac with Recording (test double) at {} words a minute, into {folder}? y or n",
                    app.settings.speech.rate.wpm()
                )
            );
            app.dispatch(Command::Confirm(Confirm::Yes));
            wait_until_done(&mut app);
            let text = app.status_text().to_owned();
            assert!(
                text.starts_with("Wrote essay.flac: ")
                    && text.ends_with("2 chapters. Open it? y or n."),
                "{text}"
            );
            assert_eq!(&std::fs::read(&out).unwrap()[..4], b"fLaC");
            app.dispatch(Command::Confirm(Confirm::Yes));
            assert_eq!(
                opened.lock().unwrap().as_slice(),
                [out.display().to_string()]
            );
        }

        #[test]
        fn escape_asks_then_stops_without_a_file() {
            let dir = tempfile::tempdir().unwrap();
            let (mut app, _) = app_with_doc(dir.path());
            let (_tx, rx) = std::sync::mpsc::channel();
            let out = dir.path().join("essay.wav");
            app.audio.run = Some(Run {
                out: out.clone(),
                done: Arc::new(AtomicUsize::new(0)),
                total: Arc::new(AtomicUsize::new(10)),
                cancel: Arc::new(AtomicBool::new(false)),
                rx,
                said_at: Instant::now(),
                said_tens: 0,
            });
            app.dispatch(Command::Cancel);
            assert_eq!(
                app.status_text(),
                "Stop the export? No file is kept. y or n"
            );
            app.dispatch(Command::Confirm(Confirm::No));
            assert_eq!(app.status_text(), "Still exporting audio.");
            let cancel = app.audio.run.as_ref().map(|r| Arc::clone(&r.cancel));
            app.dispatch(Command::Cancel);
            app.dispatch(Command::Confirm(Confirm::Yes));
            assert!(cancel.is_some_and(|c| c.load(Ordering::SeqCst)));
            // A second export waits for this one.
            app.dispatch(Command::Action(ActionId::ExportAudio));
            assert_eq!(
                app.status_text(),
                "Already exporting essay.wav. Escape stops."
            );
            // The thread ends cancelled: said, and no file.
            let (tx, rx) = std::sync::mpsc::channel();
            if let Some(r) = app.audio.run.as_mut() {
                r.rx = rx;
            }
            tx.send(Err(Ended::Cancelled)).unwrap_or_default();
            app.audio_tick(Instant::now());
            assert_eq!(
                app.status_text(),
                "Audio export stopped; no file was written."
            );
            assert!(!out.exists());
        }

        #[test]
        fn progress_is_in_tens_and_at_most_every_ten_seconds() {
            let dir = tempfile::tempdir().unwrap();
            let (mut app, _) = app_with_doc(dir.path());
            let (_tx, rx) = std::sync::mpsc::channel();
            let t0 = Instant::now();
            let done = Arc::new(AtomicUsize::new(0));
            app.audio.run = Some(Run {
                out: dir.path().join("essay.flac"),
                done: Arc::clone(&done),
                total: Arc::new(AtomicUsize::new(40)),
                cancel: Arc::new(AtomicBool::new(false)),
                rx,
                said_at: t0,
                said_tens: 0,
            });
            app.tell("Started.");
            done.store(13, Ordering::Relaxed);
            app.audio_tick(t0 + Duration::from_secs(4));
            assert_eq!(app.status_text(), "Started.");
            app.audio_tick(t0 + Duration::from_secs(10));
            assert_eq!(app.status_text(), "Exporting audio, 30 percent.");
            assert!(app.status_text().chars().count() <= 40);
            app.tell("Quiet.");
            app.audio_tick(t0 + Duration::from_secs(30));
            assert_eq!(app.status_text(), "Quiet.");
        }

        #[test]
        fn without_a_document_it_says_so() {
            let mut app = App::new(crate::AppConfig::for_tests());
            app.dispatch(Command::Action(ActionId::ExportAudio));
            assert!(
                app.status_text().starts_with("No document is open"),
                "{}",
                app.status_text()
            );
        }
    }
}
