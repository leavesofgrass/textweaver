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
//!
//! MathCAT holds one expression at a time. Speech, braille, and navigation
//! share it: the thread remembers which MathML is current and parses a new
//! one only when it changes. Navigation keeps its expression and the moves
//! made in it, so after another formula was spoken or brailled it parses
//! the expression again and retraces the moves (setting MathML resets
//! MathCAT's place, and its generated ids change).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use crate::{BrailleOptions, Error, NavMove, NavStep, Options, mathcat_verbosity};

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

/// The most moves kept for retracing a navigation. Later moves are not
/// recorded, so a retrace after that many moves (and another expression
/// spoken in between) lands where the record ends.
const MAX_MOVES: usize = 256;

/// What the thread answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    /// Words or braille.
    Text(String),
    /// A navigation step.
    Step(NavStep),
}

type Reply = Sender<Result<Answer, Error>>;

pub(crate) enum Job {
    /// Load the rules now.
    WarmUp,
    /// Speak one MathML expression.
    Speak {
        mathml: String,
        options: Options,
        reply: Reply,
    },
    /// Braille for one MathML expression.
    Braille {
        mathml: String,
        braille: BrailleOptions,
        reply: Reply,
    },
    /// Start navigating an expression, or take a step in the current one.
    Navigate {
        /// The expression to start on; `None` steps in the current one.
        start: Option<String>,
        step: NavMove,
        options: Options,
        braille: BrailleOptions,
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
    text(wait(&answer)?)
}

pub(crate) fn braille(mathml: &str, braille: &BrailleOptions) -> Result<String, Error> {
    let (reply, answer) = mpsc::channel();
    send(Job::Braille {
        mathml: mathml.to_owned(),
        braille: *braille,
        reply,
    })?;
    text(wait(&answer)?)
}

pub(crate) fn navigate(
    start: Option<&str>,
    step: NavMove,
    options: &Options,
    braille: &BrailleOptions,
) -> Result<NavStep, Error> {
    let (reply, answer) = mpsc::channel();
    send(Job::Navigate {
        start: start.map(str::to_owned),
        step,
        options: options.clone(),
        braille: *braille,
        reply,
    })?;
    match wait(&answer)? {
        Answer::Step(step) => Ok(step),
        Answer::Text(_) => Err(Error::Unavailable),
    }
}

fn text(answer: Answer) -> Result<String, Error> {
    match answer {
        Answer::Text(t) => Ok(t),
        Answer::Step(_) => Err(Error::Unavailable),
    }
}

#[cfg(test)]
pub(crate) fn panic_for_test() -> Result<String, Error> {
    let (reply, answer) = mpsc::channel();
    send(Job::Panic { reply })?;
    text(wait(&answer)?)
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

fn wait(answer: &Receiver<Result<Answer, Error>>) -> Result<Answer, Error> {
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
            } => (
                guarded(|| state.speak(&mathml, &options)).map(Answer::Text),
                reply,
            ),
            Job::Braille {
                mathml,
                braille,
                reply,
            } => (
                guarded(|| state.braille(&mathml, &braille)).map(Answer::Text),
                reply,
            ),
            Job::Navigate {
                start,
                step,
                options,
                braille,
                reply,
            } => (
                guarded(|| state.navigate(start, step, &options, &braille)).map(Answer::Step),
                reply,
            ),
            #[cfg(test)]
            Job::Panic { reply } => (
                guarded(|| -> Result<Answer, Error> { panic!("a test panic in the engine") }),
                reply,
            ),
        };
        if result == Err(Error::Crashed) {
            // Preferences are set again, and the expression parsed again,
            // before the next request.
            state.prefs = None;
            state.braille_prefs = None;
            state.current = None;
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
    /// The speech preferences last set.
    prefs: Option<Options>,
    /// The braille preferences last set.
    braille_prefs: Option<BrailleOptions>,
    /// The MathML MathCAT holds now (its current expression).
    current: Option<String>,
    /// The expression being navigated and the moves made in it.
    nav: Option<Navigation>,
}

/// An expression being navigated.
struct Navigation {
    mathml: String,
    /// MathCAT's commands since the start, in order.
    moves: Vec<&'static str>,
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

    fn apply_braille(&mut self, braille: &BrailleOptions) -> Result<(), Error> {
        if self.braille_prefs.as_ref() == Some(braille) {
            return Ok(());
        }
        self.braille_prefs = None;
        set("BrailleCode", braille.code.mathcat_name())?;
        // No dots 7 and 8: braille files and the status line are six-dot,
        // and a navigation step shows only the part it reached.
        set("BrailleNavHighlight", "Off")?;
        // UEB's grade 1 indicators depend on the text around the math.
        set(
            "UEB_START_MODE",
            if braille.grade2 { "Grade2" } else { "Grade1" },
        )?;
        self.braille_prefs = Some(*braille);
        Ok(())
    }

    /// Makes `mathml` MathCAT's current expression, parsing it only when
    /// another one is current.
    fn set_current(&mut self, mathml: &str) -> Result<(), Error> {
        if self.current.as_deref() == Some(mathml) {
            return Ok(());
        }
        self.current = None;
        libmathcat::set_mathml(mathml).map_err(|e| classify(&e))?;
        self.current = Some(mathml.to_owned());
        Ok(())
    }

    fn speak(&mut self, mathml: &str, options: &Options) -> Result<String, Error> {
        self.ready()?;
        self.apply(options)?;
        self.set_current(mathml)?;
        let words = libmathcat::get_spoken_text().map_err(|e| classify(&e))?;
        Ok(clean_words(&words, options))
    }

    fn braille(&mut self, mathml: &str, braille: &BrailleOptions) -> Result<String, Error> {
        self.ready()?;
        self.apply_braille(braille)?;
        self.set_current(mathml)?;
        let cells = libmathcat::get_braille("").map_err(|e| classify(&e))?;
        Ok(clean_braille(&cells))
    }

    fn navigate(
        &mut self,
        start: Option<String>,
        step: NavMove,
        options: &Options,
        braille: &BrailleOptions,
    ) -> Result<NavStep, Error> {
        self.ready()?;
        self.apply(options)?;
        self.apply_braille(braille)?;
        if let Some(mathml) = start {
            self.nav = None;
            // Always parsed again: setting MathML is what resets the place.
            self.current = None;
            self.set_current(&mathml)?;
            self.nav = Some(Navigation {
                mathml,
                moves: Vec::new(),
            });
        }
        let Some(nav) = self.nav.as_ref() else {
            return Err(Error::Rejected("no expression is being navigated".into()));
        };
        if self.current.as_deref() != Some(nav.mathml.as_str()) {
            // Another expression was spoken since: parse this one again and
            // retrace the moves.
            let mathml = nav.mathml.clone();
            let moves = nav.moves.clone();
            self.set_current(&mathml)?;
            for m in moves {
                libmathcat::do_navigate_command(m).map_err(|e| classify(&e))?;
            }
        }
        let before = libmathcat::get_navigation_mathml_id().map_err(|e| classify(&e))?;
        let command = step.mathcat_command();
        let words = libmathcat::do_navigate_command(command).map_err(|e| classify(&e))?;
        let after = libmathcat::get_navigation_mathml_id().map_err(|e| classify(&e))?;
        let moved = before != after;
        if moved
            && step.moves()
            && let Some(nav) = self.nav.as_mut()
            && nav.moves.len() < MAX_MOVES
        {
            nav.moves.push(command);
        }
        let cells = libmathcat::get_navigation_braille().map_err(|e| classify(&e))?;
        Ok(NavStep {
            speech: clean_words(&words, options),
            braille: clean_braille(&cells),
            moved,
        })
    }
}

/// MathCAT's words, cleaned: no control characters (MathCAT passes an
/// escape sequence in the source through, and it must not reach the
/// engine or the terminal), pauses removed when asked, single spaces.
fn clean_words(words: &str, options: &Options) -> String {
    let mut words = words.to_owned();
    words.retain(|c| !c.is_control() || c.is_whitespace());
    if !options.pauses {
        words = without_pauses(&words);
    }
    words.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// MathCAT's braille as six-dot Unicode braille: dots 7 and 8 dropped, a
/// space as the blank cell, anything that is not a braille cell left out,
/// and no blank cells at either end.
pub(crate) fn clean_braille(cells: &str) -> String {
    cells
        .chars()
        .filter_map(|c| {
            let v = u32::from(c);
            if (0x2800..=0x28FF).contains(&v) {
                char::from_u32(0x2800 + ((v - 0x2800) & 0x3F))
            } else if c == ' ' {
                Some(crate::BLANK_CELL)
            } else {
                None
            }
        })
        .collect::<String>()
        .trim_matches(crate::BLANK_CELL)
        .to_owned()
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
