//! The MathCAT thread.
//!
//! MathCAT's state is thread-local, so one thread, started on first use,
//! makes every call. Requests arrive on a channel with their own reply
//! channel; the caller waits up to [`TIMEOUT`]. Every request runs inside
//! `catch_unwind`: a panic is answered with [`Error::Crashed`], reported in
//! the log once, and the thread goes on to the next request.
//!
//! MathCAT installs a process-wide panic hook that stores the message and
//! prints nothing. That would silence every other thread's panics too, and
//! drop the hook textweaver's terminal reader installs to restore the
//! terminal. So the thread wraps it: panics on this thread go to MathCAT's
//! hook (quietly, since they are caught and reported here), and panics on
//! any other thread go to the hook that was there before.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use crate::{Error, Options, mathcat_verbosity};

/// The thread's name, which the panic hook checks.
pub(crate) const THREAD_NAME: &str = "textweaver-mathcat";

/// The thread's stack: MathCAT recurses over the expression tree.
const STACK_BYTES: usize = 16 * 1024 * 1024;

/// How long a caller waits for an answer. The first request also loads
/// the rules, which takes about a second in a debug build.
pub(crate) const TIMEOUT: Duration = Duration::from_secs(10);

/// The rules directory name inside MathCAT's embedded zip.
const RULES_DIR: &str = "Rules";

/// What MathCAT's own panic guard puts at the start of its error message.
const CRASH_PREFIX: &str = "MathCAT crash";

type Reply = Sender<Result<String, Error>>;

pub(crate) enum Job {
    /// Load the rules now.
    WarmUp,
    /// Speak one MathML expression.
    Speak {
        mathml: String,
        options: Options,
        reply: Reply,
    },
    /// Panic inside the request guard, as a MathCAT bug would.
    #[cfg(test)]
    Panic { reply: Reply },
}

/// The request channel, or `None` if the thread could not be started.
static ENGINE: OnceLock<Option<Sender<Job>>> = OnceLock::new();

/// Set when a caller gave up waiting; cleared when the thread answers
/// again. While set, requests fail at once instead of each waiting out
/// the timeout behind a stuck expression.
static STALLED: AtomicBool = AtomicBool::new(false);

/// Set once a crash has been reported in the log.
static CRASH_REPORTED: AtomicBool = AtomicBool::new(false);

fn engine() -> Option<&'static Sender<Job>> {
    ENGINE
        .get_or_init(|| {
            let (tx, rx) = mpsc::channel();
            match std::thread::Builder::new()
                .name(THREAD_NAME.into())
                .stack_size(STACK_BYTES)
                .spawn(move || run(rx))
            {
                Ok(_) => Some(tx),
                Err(e) => {
                    log::warn!("MathCAT: the math speech thread could not start: {e}");
                    None
                }
            }
        })
        .as_ref()
}

pub(crate) fn warm_up() {
    if let Some(tx) = engine() {
        let _ = tx.send(Job::WarmUp);
    }
}

pub(crate) fn speak(mathml: &str, options: &Options) -> Result<String, Error> {
    let (reply, answer) = mpsc::channel();
    send(Job::Speak {
        mathml: mathml.to_owned(),
        options: options.clone(),
        reply,
    })?;
    wait(&answer)
}

#[cfg(test)]
pub(crate) fn panic_for_test() -> Result<String, Error> {
    let (reply, answer) = mpsc::channel();
    send(Job::Panic { reply })?;
    wait(&answer)
}

fn send(job: Job) -> Result<(), Error> {
    if STALLED.load(Ordering::Acquire) {
        return Err(Error::Timeout);
    }
    engine()
        .ok_or(Error::Unavailable)?
        .send(job)
        .map_err(|_| Error::Unavailable)
}

fn wait(answer: &Receiver<Result<String, Error>>) -> Result<String, Error> {
    match answer.recv_timeout(TIMEOUT) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => {
            STALLED.store(true, Ordering::Release);
            log::warn!(
                "MathCAT: no answer in {} seconds; textweaver's own math speech is used until it answers",
                TIMEOUT.as_secs()
            );
            Err(Error::Timeout)
        }
        Err(RecvTimeoutError::Disconnected) => Err(Error::Unavailable),
    }
}

/// The thread's body.
fn run(jobs: Receiver<Job>) {
    install_panic_hook();
    let mut state = State::default();
    while let Ok(job) = jobs.recv() {
        let (result, reply) = match job {
            Job::WarmUp => {
                let r = guarded(|| state.ready());
                if let Err(e) = r {
                    log::warn!("MathCAT: {e}");
                }
                continue;
            }
            Job::Speak {
                mathml,
                options,
                reply,
            } => (guarded(|| state.speak(&mathml, &options)), reply),
            #[cfg(test)]
            Job::Panic { reply } => (
                guarded(|| -> Result<String, Error> { panic!("a test panic in the engine") }),
                reply,
            ),
        };
        if result == Err(Error::Crashed) {
            // Preferences are set again before the next expression.
            state.prefs = None;
        }
        STALLED.store(false, Ordering::Release);
        let _ = reply.send(result);
    }
}

/// Runs `f`, turning a panic (ours, or one MathCAT's own guard reports)
/// into [`Error::Crashed`], reported once.
fn guarded<T>(f: impl FnOnce() -> Result<T, Error>) -> Result<T, Error> {
    let result = catch_unwind(AssertUnwindSafe(f)).unwrap_or(Err(Error::Crashed));
    if result.as_ref().err() == Some(&Error::Crashed) {
        report_crash();
    }
    result
}

fn report_crash() {
    if !CRASH_REPORTED.swap(true, Ordering::AcqRel) {
        log::warn!(
            "MathCAT stopped on an internal error; textweaver's own math speech reads that formula. Later ones still go to MathCAT."
        );
    } else {
        log::debug!("MathCAT stopped on an internal error again");
    }
}

/// Wraps MathCAT's panic hook so it only hears this thread's panics.
fn install_panic_hook() {
    let previous = std::panic::take_hook();
    libmathcat::interface::init_panic_handler();
    let mathcat = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if std::thread::current().name() == Some(THREAD_NAME) {
            mathcat(info);
        } else {
            previous(info);
        }
    }));
}

/// MathCAT's error as ours: a crash its own guard caught, or a refusal.
fn classify(e: &libmathcat::errors::Error) -> Error {
    let message = libmathcat::errors_to_string(e);
    if message.trim_start().starts_with(CRASH_PREFIX) {
        Error::Crashed
    } else {
        Error::Rejected(first_line(&message))
    }
}

fn first_line(s: &str) -> String {
    s.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("unknown error")
        .chars()
        .take(200)
        .collect()
}

/// What the thread has set up so far.
#[derive(Default)]
struct State {
    /// The result of loading the rules, once tried.
    rules: Option<Result<(), Error>>,
    /// The preferences last set.
    prefs: Option<Options>,
}

impl State {
    fn ready(&mut self) -> Result<(), Error> {
        self.rules
            .get_or_insert_with(|| {
                libmathcat::set_rules_dir(RULES_DIR)
                    .map_err(|e| match classify(&e) {
                        Error::Rejected(m) => Error::Rules(m),
                        other => other,
                    })
                    // The rules are embedded and never change, so MathCAT
                    // need not check files for changes on every call.
                    .and_then(|()| set("CheckRuleFiles", "None"))
            })
            .clone()
    }

    fn apply(&mut self, options: &Options) -> Result<(), Error> {
        if self.prefs.as_ref() == Some(options) {
            return Ok(());
        }
        self.prefs = None;
        if set("Language", options.language).is_err() {
            set("Language", "en")?;
        }
        set("SpeechStyle", options.style.mathcat_name())?;
        set("Verbosity", mathcat_verbosity(options.verbosity))?;
        self.prefs = Some(options.clone());
        Ok(())
    }

    fn speak(&mut self, mathml: &str, options: &Options) -> Result<String, Error> {
        self.ready()?;
        self.apply(options)?;
        libmathcat::set_mathml(mathml).map_err(|e| classify(&e))?;
        let mut words = libmathcat::get_spoken_text().map_err(|e| classify(&e))?;
        // MathCAT passes control characters in the source through; an
        // escape sequence must not reach the engine or the terminal.
        words.retain(|c| !c.is_control() || c.is_whitespace());
        if !options.pauses {
            words = without_pauses(&words);
        }
        Ok(words.split_whitespace().collect::<Vec<_>>().join(" "))
    }
}

/// `words` without the commas and semicolons MathCAT adds for pauses: those
/// before a space or at the end. A comma inside a number ("3,5" in German)
/// stays.
pub(crate) fn without_pauses(words: &str) -> String {
    let mut out = String::with_capacity(words.len());
    let mut chars = words.chars().peekable();
    while let Some(c) = chars.next() {
        let pause = matches!(c, ',' | ';') && chars.peek().is_none_or(|n| n.is_whitespace());
        if !pause {
            out.push(c);
        }
    }
    out
}

fn set(name: &str, value: &str) -> Result<(), Error> {
    libmathcat::set_preference(name, value).map_err(|e| classify(&e))
}
