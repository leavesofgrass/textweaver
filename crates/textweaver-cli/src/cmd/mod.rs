//! Subcommands. Each file is owned by one agent (the agent briefs).

pub mod backends;
pub mod cite;
pub mod components;
pub mod convert;
pub mod convert_layout;
pub mod define;
pub mod dictate;
pub mod eloquence;
pub mod export_audio;
pub mod info;
pub mod library;
pub mod lint;
pub mod marks;
pub mod migrate;
pub mod ocr;
pub mod open;
pub mod profiles;
pub mod search;
pub mod serve;
pub mod settings;
pub mod speak;
pub mod stats;
pub mod summarize;
pub mod sync;
pub mod text;
pub mod vault;
pub mod voices;

/// The data folders: under `home` when `--home DIR` was given, else the
/// usual place (which `TEXTWEAVER_HOME` moves).
pub(crate) fn paths(
    home: Option<&std::path::Path>,
) -> anyhow::Result<textweaver_app::store::Paths> {
    match home {
        Some(h) => Ok(textweaver_app::store::Paths::under(h)),
        None => textweaver_app::store::Paths::platform().map_err(|e| {
            anyhow::Error::new(Plain(format!(
                "Could not find textweaver's data folder: {e}. Give one with --home DIR, or set TEXTWEAVER_HOME."
            )))
        }),
    }
}

/// Set once standard output's reader has gone (`tw search x | head`):
/// every later write is dropped quietly.
static PIPE_CLOSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `tw`'s one standard output writer. Every command writes its results
/// through it, directly or through [`out!`] and [`outln!`]. A closed pipe
/// ends the output quietly: the write succeeds, and the rest of the
/// output is dropped, so the command still finishes its work (a save, a
/// download) and exits with its own status. Any other failure is
/// returned.
pub(crate) struct Stdout;

impl std::io::Write for Stdout {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        use std::sync::atomic::Ordering;
        if PIPE_CLOSED.load(Ordering::Relaxed) {
            return Ok(buf.len());
        }
        match std::io::stdout().lock().write(buf) {
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                PIPE_CLOSED.store(true, Ordering::Relaxed);
                Ok(buf.len())
            }
            other => other,
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        use std::sync::atomic::Ordering;
        if PIPE_CLOSED.load(Ordering::Relaxed) {
            return Ok(());
        }
        match std::io::stdout().lock().flush() {
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                PIPE_CLOSED.store(true, Ordering::Relaxed);
                Ok(())
            }
            other => other,
        }
    }
}

/// Writes formatted text to [`Stdout`], for [`out!`] and [`outln!`]. A
/// failure other than a closed pipe (a full disk under a redirect) ends
/// `tw` with its one error line and status 1, as `println!` would have
/// panicked.
pub(crate) fn write_out(args: std::fmt::Arguments<'_>) {
    use std::io::Write as _;
    if let Err(e) = Stdout.write_fmt(args) {
        eprintln!(
            "{}",
            one_line(&anyhow::Error::new(e).context("Could not write the output"))
        );
        std::process::exit(1);
    }
}

/// `print!` through `tw`'s one standard output writer, [`Stdout`].
macro_rules! out {
    ($($arg:tt)*) => {
        $crate::cmd::write_out(format_args!($($arg)*))
    };
}

/// `println!` through `tw`'s one standard output writer, [`Stdout`].
macro_rules! outln {
    () => {
        $crate::cmd::write_out(format_args!("\n"))
    };
    ($($arg:tt)*) => {
        $crate::cmd::write_out(format_args!("{}\n", format_args!($($arg)*)))
    };
}

pub(crate) use {out, outln};

/// Writes `text` to standard output through [`Stdout`]. A closed pipe
/// (`tw text big.pdf | head`) ends the output quietly instead of a panic
/// about "failed printing to stdout"; any other failure is reported.
pub(crate) fn print_all(text: &str) -> anyhow::Result<()> {
    use std::io::Write as _;
    let mut out = Stdout;
    out.write_all(text.as_bytes())?;
    out.flush()?;
    Ok(())
}

/// An error that ends `tw` with exit status 1 without printing anything
/// more: the command has already said what it found (or did not find).
/// Search with no match, define with no definition, lint with problems,
/// and `tw cite check` with missing keys end this way (the "Command line"
/// page, exit codes).
#[derive(Debug)]
pub(crate) struct NothingFound;

impl std::fmt::Display for NothingFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("nothing found")
    }
}

impl std::error::Error for NothingFound {}

/// An error whose message already reads as a whole line for the user:
/// what failed, why, and what to do. [`one_line`] adds no next step to it.
#[derive(Debug)]
pub(crate) struct Plain(pub String);

impl std::fmt::Display for Plain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Plain {}

/// The one line `tw` prints for an error, on standard error: "Error: what
/// failed: why. What to do." The causes are joined with colons on the
/// same line, the first letter is a capital, and a next step is added when
/// the cause is one the user can act on (a missing file, a permission).
pub(crate) fn one_line(err: &anyhow::Error) -> String {
    let mut parts: Vec<String> = Vec::new();
    for cause in err.chain() {
        let text = cause.to_string();
        // One line: newlines inside a message become spaces.
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        // "(os error 2)" says nothing a listener can use.
        let text = match text.rfind(" (os error ") {
            Some(i) if text.ends_with(')') => text[..i].to_owned(),
            _ => text,
        };
        let text = text.trim_end_matches('.').trim().to_owned();
        // Some wrappers repeat their source; skip a part already said.
        if text.is_empty() || parts.last().is_some_and(|p| p.ends_with(&text)) {
            continue;
        }
        parts.push(text);
    }
    let mut line = parts.join(": ");
    if line.is_empty() {
        line.push_str("Something went wrong");
    }
    if let Some(first) = line.chars().next() {
        let upper: String = first.to_uppercase().collect();
        line.replace_range(..first.len_utf8(), &upper);
    }
    if !line.ends_with(['.', '?', '!']) {
        line.push('.');
    }
    let has_step = err.chain().any(|c| c.downcast_ref::<Plain>().is_some());
    if !has_step && let Some(step) = next_step(err) {
        line.push(' ');
        line.push_str(step);
    }
    format!("Error: {line}")
}

/// What to do about the error's deepest input or output cause, when there
/// is a plain answer.
fn next_step(err: &anyhow::Error) -> Option<&'static str> {
    use std::io::ErrorKind;
    let io = err
        .chain()
        .filter_map(|c| c.downcast_ref::<std::io::Error>())
        .last()?;
    Some(match io.kind() {
        ErrorKind::NotFound => "Check the name and the folder, then try again.",
        ErrorKind::PermissionDenied => {
            "Check that you may read or write it, or choose another place."
        }
        ErrorKind::AlreadyExists => "Choose another name, or remove the old one first.",
        ErrorKind::StorageFull => "Free some disk space, then try again.",
        ErrorKind::TimedOut => "Check the connection, then try again.",
        _ => return None,
    })
}

/// Asks a yes or no question on standard error and reads the answer from
/// `input`; "y" or "yes" in any case is yes. `terminal` says whether
/// standard input is a terminal: without one, `tw` never waits for an
/// answer and fails with a line that names `yes_flag`, so a script stops
/// instead of hanging (the "Command line" page, questions).
pub(crate) fn confirm(
    question: &str,
    input: &mut dyn std::io::BufRead,
    terminal: bool,
    yes_flag: &str,
) -> anyhow::Result<bool> {
    use std::io::Write as _;
    if !terminal {
        return Err(Plain(format!(
            "Stopped before asking: standard input is not a terminal. Add {yes_flag} to go ahead without a question."
        ))
        .into());
    }
    {
        let mut err = std::io::stderr().lock();
        write!(err, "{question} ")?;
        err.flush()?;
    }
    let mut answer = String::new();
    input.read_line(&mut answer)?;
    Ok(is_yes(&answer))
}

/// Whether `answer` is "y" or "yes", in any case, around spaces.
pub(crate) fn is_yes(answer: &str) -> bool {
    matches!(answer.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Whether standard input is a terminal, for [`confirm`].
pub(crate) fn stdin_is_terminal() -> bool {
    use std::io::IsTerminal as _;
    std::io::stdin().is_terminal()
}

#[cfg(test)]
mod tests {
    use anyhow::Context as _;

    #[test]
    fn print_all_writes_and_reports_nothing_on_success() {
        assert!(super::print_all("").is_ok());
    }

    #[test]
    fn errors_print_on_one_line_with_a_capital_and_a_next_step() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = Err::<(), _>(io)
            .context("could not open notes.md")
            .unwrap_err();
        let line = super::one_line(&err);
        assert_eq!(
            line,
            "Error: Could not open notes.md: file not found. Check the name and the folder, then try again."
        );
        assert!(!line.contains('\n'));
    }

    #[test]
    fn a_plain_error_keeps_its_own_next_step() {
        let err = anyhow::Error::new(super::Plain("Stopped. Add --yes.".into()));
        assert_eq!(super::one_line(&err), "Error: Stopped. Add --yes.");
    }

    #[test]
    fn answers_are_y_or_yes_in_any_case() {
        for yes in ["y", "Y", "yes", " YES \n"] {
            assert!(super::is_yes(yes), "{yes}");
        }
        for no in ["", "n", "yeah", "no"] {
            assert!(!super::is_yes(no), "{no}");
        }
    }

    #[test]
    fn no_question_without_a_terminal() {
        let mut input: &[u8] = b"y\n";
        let err = super::confirm("Remove it?", &mut input, false, "--yes").unwrap_err();
        assert!(super::one_line(&err).contains("Add --yes"));
    }
}
