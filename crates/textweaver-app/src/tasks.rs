//! Background work and questions for the authoring features: the tick that
//! re-parses the structure while editing and collects exports and
//! reference lookups finished on other threads, and the "Open it? y or n"
//! question that follows an export.

use std::time::{Duration, Instant};

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
                        self.error(&format!("{what} stopped unexpectedly."));
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
                        self.error(&format!("Looking up {input} stopped unexpectedly."));
                    }
                },
            }
        }
        done
    }

    /// Waits until every background export and lookup has finished (or
    /// `timeout` passes) and applies the results; for tests. True when none
    /// is left.
    pub fn wait_for_background(&mut self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            self.poll_jobs(Instant::now());
            if self.authoring.jobs.is_empty() {
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
        self.authoring.question = Some(Question::Open(target));
        self.tell(question);
    }

    /// Opens `target` with the default program, announcing failures.
    pub(crate) fn launch(&mut self, target: &str) {
        let result = match (self.authoring.launcher.clone(), &self.paths) {
            (Some(launcher), _) => launcher(target),
            (None, Some(_)) => crate::authoring_state::open_with_system(target),
            (None, None) => Err(std::io::Error::other(
                "opening other programs is off in a session that keeps no files",
            )),
        };
        match result {
            Ok(()) => self.note("Opening."),
            Err(e) => self.error(&format!("Could not open {target}: {e}")),
        }
    }

    /// The answer to a question asked here.
    pub(crate) fn confirm_authoring(&mut self, answer: Confirm) -> Vec<Effect> {
        let Some(q) = self.authoring.question.clone() else {
            return vec![Effect::Redraw];
        };
        match (q, answer) {
            (Question::Open(target), Confirm::Yes) => {
                self.authoring.question = None;
                self.launch(&target);
            }
            (Question::Open(_), Confirm::No) => {
                self.authoring.question = None;
                self.note("Not opened.");
            }
            (Question::Open(_), Confirm::Repeat) => self.tell("Open it? y or n."),
        }
        vec![Effect::Redraw]
    }
}
