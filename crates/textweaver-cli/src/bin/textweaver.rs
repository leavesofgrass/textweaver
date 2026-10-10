//! `textweaver`: a second name for `tw` (B1-o1).
//!
//! On Linux and macOS the packages make `textweaver` a link to `tw`. On
//! Windows a link needs special rights, and a copy of `tw.exe` would undo
//! the size the single program saves, so this small launcher runs `tw`
//! beside it with the same arguments, standard input, output and error,
//! and ends with `tw`'s exit status. It uses only the standard library,
//! so it stays small. Where it runs on Unix (a cargo build), it replaces
//! itself with `tw` instead of waiting for it.

use std::path::PathBuf;
use std::process::{Command, ExitCode};

/// `tw`'s file name on this system.
const TW: &str = if cfg!(windows) { "tw.exe" } else { "tw" };

/// Where `tw` should be: beside this program.
fn tw_path() -> Option<PathBuf> {
    let me = std::env::current_exe().ok()?;
    Some(me.parent()?.join(TW))
}

fn main() -> ExitCode {
    let Some(tw) = tw_path().filter(|p| p.is_file()) else {
        eprintln!(
            "Error: tw was not found beside textweaver. Reinstall textweaver, or run tw itself."
        );
        return ExitCode::FAILURE;
    };
    let mut cmd = Command::new(&tw);
    cmd.args(std::env::args_os().skip(1));
    run(cmd, &tw)
}

/// Unix: becomes `tw`, so signals and the terminal go straight to it.
#[cfg(unix)]
fn run(mut cmd: Command, tw: &std::path::Path) -> ExitCode {
    use std::os::unix::process::CommandExt as _;
    let err = cmd.exec();
    eprintln!("Error: could not start {}: {err}.", tw.display());
    ExitCode::FAILURE
}

/// Elsewhere: runs `tw`, waits, and passes its exit status on.
// shortcut: Ctrl+C reaches both processes in the console, so a stopped
// command ends the launcher too and the shell sees the stop status rather
// than tw's; ignoring the signal here would need unsafe code
// (SetConsoleCtrlHandler). Upgrade if a script needs tw's own status after
// Ctrl+C.
#[cfg(not(unix))]
fn run(mut cmd: Command, tw: &std::path::Path) -> ExitCode {
    match cmd.status() {
        Ok(status) => match status.code() {
            Some(0) => ExitCode::SUCCESS,
            Some(code) => std::process::exit(code),
            None => ExitCode::FAILURE,
        },
        Err(err) => {
            eprintln!("Error: could not start {}: {err}.", tw.display());
            ExitCode::FAILURE
        }
    }
}
