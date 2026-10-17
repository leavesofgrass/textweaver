//! Which part of the one program runs: the app or the terminal program
//! (the owner's decision, Friday, October 16, 2026).
//!
//! textweaver is one program, `textweaver` (`textweaver.exe` on Windows).
//! Most people want the app, so it is the default; the terminal reader and
//! the commands stay one flag away. `main` checks, in this order, before
//! any start-up of the app:
//!
//! 1. host mode, `--engine-host <engine>` (`textweaver_host::host_mode`,
//!    checked by `main` itself, before [`mode`]): a speech engine's host;
//! 2. `--tw` or `--cli` as the first argument: the terminal program, with
//!    the arguments after it, exactly as `tw` runs them;
//! 3. started under the name `tw` (a link on Linux and macOS): the
//!    terminal program with every argument;
//! 4. a `tw` command as the first argument (`textweaver convert ...`): the
//!    terminal program, so scripts written for the old `textweaver` keep
//!    working;
//! 5. anything else: the app, which opens a file named on the command line.

use std::ffi::{OsStr, OsString};
use std::path::Path;

/// The flags that run the terminal program: the same meaning, two names.
pub const TERMINAL_FLAGS: [&str; 2] = ["--tw", "--cli"];

/// The terminal program's name: a program started under it (`tw`, `tw.exe`,
/// `TW.EXE`) runs the terminal program.
pub const TERMINAL_NAME: &str = "tw";

/// What the program runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The terminal program, with these arguments (after the program's
    /// name, and without `--tw`).
    Terminal(Vec<OsString>),
    /// The app, which parses the arguments itself.
    App,
}

/// True when the program at `argv0` was started under the terminal
/// program's name: its file stem is `tw`, in any case.
pub fn started_as_tw(argv0: &OsStr) -> bool {
    Path::new(argv0)
        .file_stem()
        .and_then(OsStr::to_str)
        .is_some_and(|s| s.eq_ignore_ascii_case(TERMINAL_NAME))
}

/// Decides what runs, from the program's name `argv0`, its arguments
/// `args` (after the name), and `is_command`, which knows the terminal
/// program's commands (`textweaver_cli::is_command`). Host mode is checked
/// before this.
pub fn mode(argv0: &OsStr, args: &[OsString], is_command: impl Fn(&OsStr) -> bool) -> Mode {
    let first = args.first().map(OsString::as_os_str);
    if first.is_some_and(|a| TERMINAL_FLAGS.iter().any(|f| a == *f)) {
        return Mode::Terminal(args[1..].to_vec());
    }
    if started_as_tw(argv0) || first.is_some_and(is_command) {
        return Mode::Terminal(args.to_vec());
    }
    Mode::App
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    fn decide(argv0: &str, args: &[&str]) -> Mode {
        mode(OsStr::new(argv0), &os(args), textweaver_cli::is_command)
    }

    #[test]
    fn the_terminal_flags_run_the_terminal_program_with_the_rest() {
        for flag in TERMINAL_FLAGS {
            assert_eq!(decide("textweaver", &[flag]), Mode::Terminal(Vec::new()));
            assert_eq!(
                decide("textweaver", &[flag, "book.md", "--no-speech"]),
                Mode::Terminal(os(&["book.md", "--no-speech"]))
            );
            assert_eq!(
                decide("textweaver", &[flag, "--version"]),
                Mode::Terminal(os(&["--version"]))
            );
        }
        // Only as the first argument: later, it is the app's to refuse.
        assert_eq!(decide("textweaver", &["book.md", "--tw"]), Mode::App);
    }

    #[test]
    fn the_name_tw_runs_the_terminal_program_with_every_argument() {
        let mut names = vec!["tw", "tw.exe", "TW.EXE", "/usr/local/bin/tw"];
        // Backslashes separate folders only on Windows.
        if cfg!(windows) {
            names.push(r"C:\Programs\textweaver\Tw.exe");
        }
        for argv0 in names {
            assert!(started_as_tw(OsStr::new(argv0)), "{argv0}");
            assert_eq!(
                decide(argv0, &["book.md"]),
                Mode::Terminal(os(&["book.md"])),
                "{argv0}"
            );
            assert_eq!(decide(argv0, &[]), Mode::Terminal(Vec::new()), "{argv0}");
        }
        // `--tw` after the name tw is passed on, as tw would get it, and
        // removed only as the first argument.
        assert_eq!(decide("tw", &["--tw"]), Mode::Terminal(Vec::new()));
        for argv0 in [
            "textweaver",
            "textweaver.exe",
            "twine",
            "tw-launcher",
            "textweaver-gui",
        ] {
            assert!(!started_as_tw(OsStr::new(argv0)), "{argv0}");
        }
    }

    #[test]
    fn a_command_first_runs_the_command_line() {
        for args in [
            &["convert", "a.md", "--to", "html"][..],
            &["text", "book.md"],
            &["backends"],
            &["export-audio", "a.md", "--out", "a.mp3"],
            &["serve"],
        ] {
            assert_eq!(decide("textweaver", args), Mode::Terminal(os(args)));
        }
    }

    #[test]
    fn anything_else_opens_the_app() {
        for args in [
            &[][..],
            &["book.md"],
            &["--version"],
            &["--help"],
            &["--screenshot", "shot.png"],
            &["notes/convert.md"],
            &["--no-speech", "convert"],
        ] {
            assert_eq!(decide("textweaver", args), Mode::App, "{args:?}");
        }
        assert_eq!(decide("textweaver.exe", &["book.md"]), Mode::App);
    }
}
