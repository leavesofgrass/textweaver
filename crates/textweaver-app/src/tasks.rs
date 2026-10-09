//! Background work and questions for the authoring features: the tick that
//! re-parses the structure while editing and collects exports and
//! reference lookups finished on other threads, and the "Open it? y or n"
//! question that follows an export.

use std::time::{Duration, Instant};

use textweaver_lexicon::args;

use crate::app::App;
use crate::authoring_state::{Job, Launcher, Question};
use crate::command::{Confirm, Effect};

impl App {
    /// Housekeeping for the authoring features, from [`App::tick`].
    pub(crate) fn authoring_tick(&mut self, now: Instant) -> Vec<Effect> {
        let mut effects = Vec::new();
        if self.structure_tick(now) {
            effects.push(Effect::Redraw);
        }
        if self.poll_jobs(now) {
            effects.push(Effect::Redraw);
        }
        self.live_preview_tick(now);
        effects
    }

    /// Applies every finished background job, and says how a long export
    /// is getting on. True when one finished.
    fn poll_jobs(&mut self, now: Instant) -> bool {
        let jobs = std::mem::take(&mut self.authoring.jobs);
        let mut done = false;
        for job in jobs {
            match job {
                Job::Export {
                    what,
                    kind,
                    rx,
                    mut progress,
                } => match rx.try_recv() {
                    Ok(result) => {
                        done = true;
                        self.export_finished(&what, kind, result);
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        if let Some(secs) = progress.due(now) {
                            self.export_progress(kind, &what, secs);
                        }
                        self.authoring.jobs.push(Job::Export {
                            what,
                            kind,
                            rx,
                            progress,
                        });
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        done = true;
                        let msg = self.msg_args("tasks-stopped", &args!["what" => what]);
                        self.error(&msg);
                    }
                },
                Job::Lookup { input, rx } => match rx.try_recv() {
                    Ok(result) => {
                        done = true;
                        self.lookup_finished(&input, result);
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        self.authoring.jobs.push(Job::Lookup { input, rx });
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        done = true;
                        let msg = self.msg_args("tasks-lookup-stopped", &args!["input" => input]);
                        self.error(&msg);
                    }
                },
            }
        }
        done
    }

    /// Waits until every background export, lookup, and misspelling count
    /// has finished (or `timeout` passes) and applies the results; for
    /// tests. True when none is left.
    pub fn wait_for_background(&mut self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            self.poll_jobs(Instant::now());
            let _ = self.spell_count_tick();
            if self.authoring.jobs.is_empty() && self.spell_count.is_none() {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Sets how files and web addresses are opened. By default they open
    /// in the system's default program, but only in a session that keeps
    /// files (with `AppConfig::paths` set), so tests never start other
    /// programs; tests record the requests with a launcher of their own.
    pub fn set_launcher(&mut self, launcher: Launcher) {
        self.authoring.launcher = Some(launcher);
    }

    /// Asks "Open it? y or n" about `target` (a file or an address).
    pub(crate) fn offer_open(&mut self, target: String, question: &str) {
        self.authoring.question = Some(Question::Open(target, question.to_owned()));
        self.ask(question);
    }

    /// Opens `target` with the default program, announcing failures.
    pub(crate) fn launch(&mut self, target: &str) {
        // Only web and mail addresses and files that exist, never through
        // a shell (W8a): anything else is refused here, in words, before
        // any opener (the system's or a test's) sees it.
        if let Err(refused) = crate::opener::classify(target) {
            let msg = self.refused_message(&refused);
            self.error(&msg);
            return;
        }
        let result = match (self.authoring.launcher.clone(), &self.paths) {
            (Some(launcher), _) => launcher(target),
            (None, Some(_)) => crate::opener::open_with_system(target),
            (None, None) => Err(std::io::Error::other(self.msg("tasks-launch-off"))),
        };
        match result {
            Ok(()) => {
                let msg = self.msg("tasks-opening");
                self.note(&msg);
            }
            Err(e) => {
                let msg = self.msg_args(
                    "tasks-could-not-open",
                    &args!["target" => target, "error" => e.to_string()],
                );
                self.error(&msg);
            }
        }
    }

    /// Why a link or file was not opened, in words (40 cells or fewer).
    pub(crate) fn refused_message(&self, refused: &crate::opener::Refused) -> String {
        use crate::opener::Refused;
        match refused {
            Refused::Scheme(scheme) => {
                self.msg_args("open-refused-scheme", &args!["scheme" => scheme.as_str()])
            }
            Refused::Missing => self.msg("open-refused-missing"),
            Refused::Invalid => self.msg("open-refused-invalid"),
        }
    }

    /// The answer to a question asked here.
    pub(crate) fn confirm_authoring(&mut self, answer: Confirm) -> Vec<Effect> {
        let Some(q) = self.authoring.question.clone() else {
            return vec![Effect::Redraw];
        };
        match (q, answer) {
            (Question::Open(target, _), Confirm::Yes) => {
                self.authoring.question = None;
                self.launch(&target);
            }
            (Question::Open(..), Confirm::No) => {
                self.authoring.question = None;
                let msg = self.msg("tasks-not-opened");
                self.note(&msg);
            }
            (Question::Open(target, question), Confirm::Repeat) => {
                // The whole question again, so "it" is still named.
                self.authoring.question = Some(Question::Open(target, question.clone()));
                self.ask(&question);
            }
            (Question::ReplaceAll(question), answer) => {
                return self.confirm_replace_all(question, answer);
            }
        }
        vec![Effect::Redraw]
    }
}
