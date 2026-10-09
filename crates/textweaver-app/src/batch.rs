//! Batch conversion from the menus (File, Batch convert; ADR-0016).
//!
//! Three short questions, each with its default first, then a yes or no:
//!
//! 1. **The folder** to convert, in the file browser
//!    ([`App::choose_folder`]).
//! 2. **The format**, Markdown first, then PDF and the other outputs
//!    `textweaver-convert` writes.
//!    HTML then asks "Theme for the HTML page?", the reading theme first
//!    (or this session's last answer); Escape cancels.
//! 3. **Where**: a `converted` folder inside the one chosen (the default),
//!    beside each source file, or another folder chosen in the browser.
//!
//! Then "Convert 48 files to PDF into D:\Notes\converted? y or n". The
//! work runs on another thread with the converter's own progress and
//! cancel ([`Converter::run_plan_with`]), so the reader stays usable.
//! Progress is said in tens of percent, never more often than every ten
//! seconds ([`Importance::Progress`], which the interface setting may
//! quiet). Escape while it runs asks "Stop converting?"; a file being
//! written is finished whole, and no file is left half-written.
//!
//! At the end the counts are said in words ([`Importance::Answer`], never
//! silenced), and when files failed they are listed, each "report.docx:
//! the reason", name first for a 40-cell Braille line; Enter on one opens
//! its source. A conversion report (each source's name and SHA-256, the
//! version and date, and what could not be made accessible, with where it
//! is) is saved as `conversion-report.md` in the output folder
//! ([`textweaver_convert::REPORT_FILE`]), and the end of the run says where
//! it is, and how many files have items not made accessible.
//!
//! Without the `publish` feature there is no converter, and the command
//! stays hidden, like every pending command ([`crate::menu::PENDING`]).

use std::path::PathBuf;

/// What a batch list is for, so [`App`](crate::app::App)'s Enter can act.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(feature = "publish"), allow(dead_code))]
pub(crate) enum BatchList {
    /// The output formats, in the order shown.
    Format,
    /// Where the files go: a `converted` folder, beside the sources, or
    /// another folder.
    Where,
    /// The files that failed, in the order listed.
    Failures(Vec<PathBuf>),
}

#[cfg(feature = "publish")]
pub(crate) use run::BatchState;

#[cfg(not(feature = "publish"))]
#[derive(Default)]
pub(crate) struct BatchState(());

#[cfg(not(feature = "publish"))]
impl crate::app::App {
    pub(crate) fn batch_running(&self) -> bool {
        false
    }
    pub(crate) fn batch_question(&self) -> bool {
        false
    }
    pub(crate) fn confirm_batch(
        &mut self,
        _: crate::command::Confirm,
    ) -> Vec<crate::command::Effect> {
        vec![crate::command::Effect::Redraw]
    }
    pub(crate) fn ask_stop_batch(&mut self) -> Vec<crate::command::Effect> {
        vec![crate::command::Effect::Redraw]
    }
    pub(crate) fn batch_tick(&mut self, _: std::time::Instant) -> Vec<crate::command::Effect> {
        Vec::new()
    }
    pub(crate) fn choose_batch(&mut self, _: BatchList, _: usize) -> Vec<crate::command::Effect> {
        vec![crate::command::Effect::Redraw]
    }
}

/// Gives the Batch convert command its handler (with `publish`).
pub(crate) fn register(app: &mut crate::app::App) {
    #[cfg(feature = "publish")]
    app.register_handler(textweaver_keymap::ActionId::BatchConvert, |app| {
        app.batch_convert()
    });
    #[cfg(not(feature = "publish"))]
    let _ = app;
}

#[cfg(feature = "publish")]
mod run {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::mpsc::{Receiver, TryRecvError};
    use std::time::{Duration, Instant};

    use textweaver_a11y::{Importance, Priority};
    use textweaver_convert::{
        ConvertOptions, Converter, OutputFormat, Plan, ReportFormat, Status, Summary,
    };
    use textweaver_lexicon::args;

    use super::BatchList;
    use crate::app::{App, ListKind};
    use crate::command::{Confirm, Effect};

    /// How often progress may be said, at most.
    pub(crate) const PROGRESS_EVERY: Duration = Duration::from_secs(10);

    /// The formats offered, in order: Markdown first (the owner's most
    /// used), then PDF, then the rest, and last the ones carta writes
    /// (AsciiDoc, Typst, LaTeX, MediaWiki, Org) when this build has it.
    pub(crate) fn formats() -> Vec<OutputFormat> {
        let mut formats = vec![
            OutputFormat::Markdown,
            OutputFormat::Pdf,
            OutputFormat::Html,
            OutputFormat::Text,
            OutputFormat::Epub,
            OutputFormat::Docx,
            OutputFormat::Brf,
        ];
        formats.extend(
            OutputFormat::ALL
                .into_iter()
                .filter(|f| f.carta_writer().is_some() && f.available()),
        );
        formats
    }

    /// The batch being set up, the question open, and the run.
    #[derive(Default)]
    pub(crate) struct BatchState {
        source: Option<PathBuf>,
        format: Option<OutputFormat>,
        /// The output folder; `None` for beside each source.
        out: Option<PathBuf>,
        question: Option<Question>,
        run: Option<Run>,
    }

    enum Question {
        /// "Convert 48 files to PDF into ...? y or n", with the plan.
        Start(Box<Converter>, Plan),
        /// "Stop converting? y or n".
        Stop,
    }

    /// What the batch thread sends at the end: the summary and where the
    /// report went, or why the batch could not run.
    type Finished = Result<(Summary, Result<PathBuf, String>), String>;

    struct Run {
        format: OutputFormat,
        total: usize,
        done: Arc<AtomicUsize>,
        cancel: Arc<AtomicBool>,
        rx: Receiver<Finished>,
        /// When progress was last said, and the tens of percent it said.
        said_at: Instant,
        said_tens: usize,
    }

    impl App {
        /// File, Batch convert: the folder first.
        pub(crate) fn batch_convert(&mut self) -> Vec<Effect> {
            if let Some(run) = &self.batch.run {
                let msg = self.msg_args(
                    "batch-busy",
                    &args![
                        "done" => run.done.load(Ordering::Relaxed),
                        "total" => run.total
                    ],
                );
                self.tell(&msg);
                return vec![Effect::Redraw];
            }
            self.batch.question = None;
            let purpose = self.msg("batch-choose-source");
            self.choose_folder(&purpose, |app, folder| app.batch_source_chosen(folder))
        }

        /// The folder to convert was chosen: the formats.
        pub(crate) fn batch_source_chosen(&mut self, folder: PathBuf) -> Vec<Effect> {
            let name = display_name(&folder);
            self.batch.source = Some(folder);
            self.batch.format = None;
            self.batch.out = None;
            let items: Vec<String> = formats().iter().map(|f| f.label().to_owned()).collect();
            let msg = self.msg_args(
                "batch-format-intro",
                &args!["name" => name, "n" => items.len()],
            );
            self.list = Some(ListKind::Batch(BatchList::Format));
            self.tell(&msg);
            vec![Effect::ShowList {
                title: self.msg("batch-format-title"),
                items,
            }]
        }

        /// The default output folder: `converted` inside the source.
        fn default_out(&self) -> Option<PathBuf> {
            self.batch.source.as_ref().map(|s| s.join("converted"))
        }

        /// Enter in a batch list.
        pub(crate) fn choose_batch(&mut self, list: BatchList, n: usize) -> Vec<Effect> {
            match list {
                BatchList::Format => match formats().get(n) {
                    Some(&f) => {
                        self.batch.format = Some(f);
                        if f == OutputFormat::Html {
                            return self.ask_html_theme(crate::authoring_state::ThemeFor::Batch);
                        }
                        self.batch_where()
                    }
                    None => vec![Effect::Redraw],
                },
                BatchList::Where => match n {
                    0 => {
                        self.batch.out = self.default_out();
                        self.batch_plan()
                    }
                    1 => {
                        self.batch.out = None;
                        self.batch_plan()
                    }
                    _ => {
                        let purpose = self.msg("batch-choose-output");
                        self.choose_folder(&purpose, |app, folder| {
                            app.batch.out = Some(folder);
                            app.batch_plan()
                        })
                    }
                },
                BatchList::Failures(paths) => match paths.get(n) {
                    Some(p) => self.open_command(p.clone()),
                    None => vec![Effect::Redraw],
                },
            }
        }

        /// The theme for the HTML pages was chosen: the where list.
        pub(crate) fn batch_theme_chosen(&mut self) -> Vec<Effect> {
            self.batch_where()
        }

        /// The where list: the `converted` folder, beside the sources, or
        /// another folder.
        fn batch_where(&mut self) -> Vec<Effect> {
            let converted = self
                .default_out()
                .map_or_else(String::new, |p| p.display().to_string());
            let items = vec![
                self.msg_args("batch-where-converted", &args!["path" => converted]),
                self.msg("batch-where-beside"),
                self.msg("batch-where-choose"),
            ];
            self.list = Some(ListKind::Batch(BatchList::Where));
            let msg = self.msg("batch-where-intro");
            self.tell(&msg);
            vec![Effect::ShowList {
                title: self.msg("batch-where-title"),
                items,
            }]
        }

        /// The conversion settings for a batch to `to`.
        fn batch_options(&self, to: OutputFormat) -> ConvertOptions {
            // One core is left for the reader, so it stays quick.
            let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
            let mut o = ConvertOptions {
                to,
                out_dir: self.batch.out.clone(),
                jobs: Some(cores.saturating_sub(1).max(1)),
                pandoc: false,
                load: self.load_options(),
                ..ConvertOptions::default()
            };
            o.write.braille.math_code = match self.settings.braille.math_code {
                textweaver_store::MathBrailleCode::Nemeth => textweaver_convert::MathCode::Nemeth,
                textweaver_store::MathBrailleCode::Ueb => textweaver_convert::MathCode::Ueb,
            };
            o.write.braille.table_format = crate::publish::braille_tables(&self.settings);
            if to == OutputFormat::Html {
                o.theme_css = Some(self.html_theme_css());
            }
            o.citations.user_library = self
                .paths
                .as_ref()
                .map(|p| textweaver_cite::user_library_path(&p.data_dir));
            o
        }

        /// Plans the batch and asks to start it.
        fn batch_plan(&mut self) -> Vec<Effect> {
            let (Some(source), Some(to)) = (self.batch.source.clone(), self.batch.format) else {
                return vec![Effect::Redraw];
            };
            let planned = Converter::new(self.batch_options(to))
                .map_err(|e| e.to_string())
                .and_then(|c| {
                    let plan = c
                        .plan(std::slice::from_ref(&source))
                        .map_err(|e| e.to_string())?;
                    Ok((c, plan))
                });
            let (conv, plan) = match planned {
                Ok(x) => x,
                Err(e) => {
                    let msg = self.msg_args("batch-start-failed", &args!["error" => e]);
                    self.error(&msg);
                    return vec![Effect::Redraw];
                }
            };
            if plan.is_empty() {
                let msg = self.msg_args(
                    "batch-nothing",
                    &args!["path" => source.display().to_string()],
                );
                self.tell(&msg);
                return vec![Effect::Redraw];
            }
            let question = self.batch_start_question(&plan, to);
            self.batch.question = Some(Question::Start(Box::new(conv), plan));
            self.ask(&question);
            vec![Effect::Redraw]
        }

        fn batch_start_question(&self, plan: &Plan, to: OutputFormat) -> String {
            let n = plan.len();
            match &self.batch.out {
                Some(out) => self.msg_args(
                    "batch-confirm",
                    &args![
                        "n" => n,
                        "format" => to.label(),
                        "path" => out.display().to_string()
                    ],
                ),
                None => self.msg_args(
                    "batch-confirm-beside",
                    &args!["n" => n, "format" => to.label()],
                ),
            }
        }

        /// True while a batch runs.
        pub(crate) fn batch_running(&self) -> bool {
            self.batch.run.is_some()
        }

        /// True while a batch question waits for y or n.
        pub(crate) fn batch_question(&self) -> bool {
            self.batch.question.is_some()
        }

        /// Escape while a batch runs: asks before stopping it.
        pub(crate) fn ask_stop_batch(&mut self) -> Vec<Effect> {
            self.batch.question = Some(Question::Stop);
            let q = self.msg("batch-stop-question");
            self.ask(&q);
            vec![Effect::Redraw]
        }

        /// Answers a batch question.
        pub(crate) fn confirm_batch(&mut self, answer: Confirm) -> Vec<Effect> {
            let Some(question) = self.batch.question.take() else {
                return vec![Effect::Redraw];
            };
            match (question, answer) {
                (Question::Start(conv, plan), Confirm::Yes) => self.batch_start(*conv, plan),
                (Question::Stop, Confirm::Yes) => {
                    if let Some(run) = &self.batch.run {
                        run.cancel.store(true, Ordering::SeqCst);
                    }
                    let msg = self.msg("batch-stopping");
                    self.tell(&msg);
                    vec![Effect::Redraw]
                }
                (Question::Start(..), Confirm::No) => {
                    let msg = self.msg("common-cancelled");
                    self.tell(&msg);
                    vec![Effect::Redraw]
                }
                (Question::Stop, Confirm::No) => {
                    let msg = self.msg("batch-still-converting");
                    self.tell(&msg);
                    vec![Effect::Redraw]
                }
                (q, Confirm::Repeat) => {
                    let text = match &q {
                        Question::Start(conv, plan) => {
                            self.batch_start_question(plan, conv.options().to)
                        }
                        Question::Stop => self.msg("batch-stop-question"),
                    };
                    self.batch.question = Some(q);
                    self.ask(&text);
                    vec![Effect::Redraw]
                }
            }
        }

        /// Starts the batch on its own thread.
        fn batch_start(&mut self, conv: Converter, plan: Plan) -> Vec<Effect> {
            let to = conv.options().to;
            let total = plan.len();
            let report_dir = self.batch.out.clone().or_else(|| self.batch.source.clone());
            let done = Arc::new(AtomicUsize::new(0));
            let cancel = Arc::new(AtomicBool::new(false));
            let (tx, rx) = std::sync::mpsc::channel();
            let wake = self.waker_slot();
            let (d, c) = (Arc::clone(&done), Arc::clone(&cancel));
            let spawned = std::thread::Builder::new()
                .name("tw-batch".into())
                .spawn(move || {
                    let result = conv
                        .run_plan_with(
                            plan,
                            |_| {
                                d.fetch_add(1, Ordering::Relaxed);
                            },
                            &c,
                        )
                        .map_err(|e| e.to_string())
                        .map(|summary| {
                            let report = match report_dir {
                                Some(dir) => summary
                                    .write_report(&dir, ReportFormat::Markdown)
                                    .map_err(|e| e.to_string()),
                                None => Err(String::new()),
                            };
                            (summary, report)
                        });
                    let _ = tx.send(result);
                    wake.wake();
                });
            if let Err(e) = spawned {
                let msg = self.msg_args("batch-start-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
                return vec![Effect::Redraw];
            }
            let now = Instant::now();
            self.batch.run = Some(Run {
                format: to,
                total,
                done,
                cancel,
                rx,
                said_at: now,
                said_tens: 0,
            });
            let msg = self.msg_args(
                "batch-started",
                &args!["n" => total, "format" => to.label()],
            );
            self.tell(&msg);
            vec![Effect::Redraw]
        }

        /// From [`App::tick`]: progress, and the end of a run.
        pub(crate) fn batch_tick(&mut self, now: Instant) -> Vec<Effect> {
            let Some(run) = self.batch.run.as_mut() else {
                return Vec::new();
            };
            match run.rx.try_recv() {
                Ok(Ok((summary, report))) => {
                    let run = self.batch.run.take();
                    let format = run.map_or(OutputFormat::Markdown, |r| r.format);
                    self.batch_finished(format, &summary, report)
                }
                Ok(Err(e)) => {
                    self.batch.run = None;
                    let msg = self.msg_args("batch-start-failed", &args!["error" => e]);
                    self.error(&msg);
                    vec![Effect::Redraw]
                }
                Err(TryRecvError::Empty) => {
                    let done = run.done.load(Ordering::Relaxed);
                    let tens = (done * 10 / run.total.max(1)).min(9);
                    if tens > run.said_tens && now.duration_since(run.said_at) >= PROGRESS_EVERY {
                        run.said_tens = tens;
                        run.said_at = now;
                        let total = run.total;
                        let msg = self.msg_args(
                            "batch-progress",
                            &args!["percent" => tens * 10, "done" => done, "total" => total],
                        );
                        self.announce_as(&msg, Priority::Polite, Importance::Progress);
                    }
                    Vec::new()
                }
                Err(TryRecvError::Disconnected) => {
                    self.batch.run = None;
                    let msg = self.msg("batch-thread-stopped");
                    self.error(&msg);
                    vec![Effect::Redraw]
                }
            }
        }

        /// Says how the batch ended, and lists the failures.
        fn batch_finished(
            &mut self,
            format: OutputFormat,
            s: &Summary,
            report: Result<PathBuf, String>,
        ) -> Vec<Effect> {
            let failures: Vec<(PathBuf, String)> = s
                .failures()
                .map(|f| {
                    let reason = match &f.status {
                        Status::Failed(r) => r.clone(),
                        _ => String::new(),
                    };
                    (f.source.clone(), reason)
                })
                .collect();
            let id = if s.canceled > 0 {
                "batch-stopped"
            } else {
                "batch-done"
            };
            let mut text = self.msg_args(
                id,
                &args![
                    "converted" => s.converted,
                    "skipped" => s.skipped,
                    "failed" => s.failed,
                    "left" => s.canceled,
                    "format" => format.label()
                ],
            );
            if s.with_issues > 0 {
                text.push(' ');
                text.push_str(&self.msg_args("batch-inaccessible", &args!["n" => s.with_issues]));
            }
            // Where the report is, always, in words.
            match &report {
                Ok(path) => {
                    text.push(' ');
                    text.push_str(
                        &self
                            .msg_args("batch-report", &args!["path" => path.display().to_string()]),
                    );
                }
                Err(e) if !e.is_empty() => {
                    text.push(' ');
                    text.push_str(
                        &self.msg_args("batch-report-failed", &args!["error" => e.clone()]),
                    );
                }
                _ => {}
            }
            if failures.is_empty() {
                self.announce_as(&text, Priority::Polite, Importance::Answer);
                return vec![Effect::Redraw];
            }
            if s.converted == 0 && s.skipped == 0 {
                self.speech.earcon(textweaver_speech::Earcon::Error);
            }
            // The failures as a list, unless the user is in another list
            // or typing: then the report file holds them.
            if self.list.is_some() || self.mode.is_prompt() || self.confirmation_pending() {
                self.announce_as(&text, Priority::Assertive, Importance::Error);
                return vec![Effect::Redraw];
            }
            let items: Vec<String> = failures
                .iter()
                .map(|(path, reason)| {
                    self.msg_args(
                        "batch-failure-item",
                        &args!["name" => display_name(path), "reason" => reason.clone()],
                    )
                })
                .collect();
            let paths = failures.into_iter().map(|(p, _)| p).collect();
            self.list = Some(ListKind::Batch(BatchList::Failures(paths)));
            self.announce_as(&text, Priority::Assertive, Importance::Error);
            vec![Effect::ShowList {
                title: self.msg_args("batch-failures-title", &args!["n" => items.len()]),
                items,
            }]
        }
    }

    /// A folder's or file's own name, or its whole path.
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

        /// An app with a run under way of `total` files; the sender keeps
        /// the run from ending.
        fn running(total: usize, t0: Instant) -> (App, std::sync::mpsc::Sender<Finished>) {
            let mut app = App::new(crate::AppConfig::for_tests());
            let (tx, rx) = std::sync::mpsc::channel();
            app.batch.run = Some(Run {
                format: OutputFormat::Pdf,
                total,
                done: Arc::new(AtomicUsize::new(0)),
                cancel: Arc::new(AtomicBool::new(false)),
                rx,
                said_at: t0,
                said_tens: 0,
            });
            (app, tx)
        }

        fn set_done(app: &App, n: usize) {
            if let Some(r) = &app.batch.run {
                r.done.store(n, Ordering::Relaxed);
            }
        }

        #[test]
        fn progress_is_in_tens_and_at_most_every_ten_seconds() {
            let t0 = Instant::now();
            let (mut app, _tx) = running(48, t0);
            app.tell("Started.");
            // 25 percent after 3 seconds: too soon.
            set_done(&app, 12);
            app.batch_tick(t0 + Duration::from_secs(3));
            assert_eq!(app.status_text(), "Started.");
            // At 10 seconds, the tens reached so far.
            app.batch_tick(t0 + Duration::from_secs(10));
            assert_eq!(app.status_text(), "20 percent converted, 12 of 48 files.");
            // No new ten: quiet, however long it takes.
            app.tell("Quiet.");
            app.batch_tick(t0 + Duration::from_secs(40));
            assert_eq!(app.status_text(), "Quiet.");
            // A new ten, but only 5 seconds after the last message.
            set_done(&app, 20);
            app.batch_tick(t0 + Duration::from_secs(15));
            assert_eq!(app.status_text(), "Quiet.");
            app.batch_tick(t0 + Duration::from_secs(20));
            assert_eq!(app.status_text(), "40 percent converted, 20 of 48 files.");
            // The key fact in the first 40 cells of a Braille line.
            assert!(app.status_text().chars().count() <= 40);
        }

        #[test]
        fn escape_asks_then_stops_the_run() {
            let t0 = Instant::now();
            let (mut app, _tx) = running(10, t0);
            app.dispatch(Command::Cancel);
            assert_eq!(
                app.status_text(),
                "Stop converting? Files already done are kept. y or n"
            );
            assert!(app.confirmation_pending());
            app.dispatch(Command::Confirm(Confirm::No));
            assert_eq!(app.status_text(), "Still converting.");
            let cancel = app.batch.run.as_ref().map(|r| Arc::clone(&r.cancel));
            assert!(!cancel.as_ref().is_some_and(|c| c.load(Ordering::SeqCst)));
            app.dispatch(Command::Cancel);
            app.dispatch(Command::Confirm(Confirm::Yes));
            assert_eq!(app.status_text(), "Stopping after the files being written.");
            assert!(cancel.is_some_and(|c| c.load(Ordering::SeqCst)));
            // The command says the run is busy instead of starting another.
            app.dispatch(Command::Action(textweaver_keymap::ActionId::BatchConvert));
            assert_eq!(
                app.status_text(),
                "Already converting, 0 of 10 files. Escape stops."
            );
        }

        #[test]
        fn a_stopped_run_says_so_first() {
            let t0 = Instant::now();
            let (mut app, tx) = running(10, t0);
            let summary = Summary {
                format: Some(OutputFormat::Pdf),
                converted: 3,
                canceled: 7,
                ..Summary::default()
            };
            tx.send(Ok((summary, Err(String::new()))))
                .unwrap_or_default();
            app.batch_tick(t0);
            assert_eq!(
                app.status_text(),
                "Stopped. Converted 3 files; 7 not converted; 0 failed."
            );
            assert!(!app.batch_running());
        }
    }
}
