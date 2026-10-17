//! `tw`'s launcher (installed as `tw.exe` in the Windows package).
//!
//! textweaver is one program, `textweaver.exe`. It is a GUI-subsystem
//! program, so starting the app from the Start menu opens no console
//! window; the price is that a shell does not wait for it. This small
//! console program is what terminals run as `tw`: it finds
//! `textweaver.exe` beside itself, runs it as `textweaver.exe --tw
//! ARGUMENTS` with the same standard input, output, and error, waits for
//! it, and ends with its exit status. `textweaver.exe` attaches to the
//! console this launcher holds, so the terminal reader, its keys, and the
//! screen reader's and Braille display's view of it work as in any
//! console program.
//!
//! Ctrl+C reaches every program attached to a console. The launcher
//! ignores it while it waits, so the status the shell sees is the one
//! `textweaver.exe` ends with; a command stopped with Ctrl+C reports its
//! own status. The handler is not inherited: `textweaver.exe` handles
//! Ctrl+C as it always has.
//!
//! It uses only the standard library. On Linux and macOS `tw` is a link to
//! `textweaver`, so the packages do not use it there; built there, it
//! replaces itself with `textweaver --tw`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// The program this launcher runs: `textweaver.exe` on Windows.
const TARGET: &str = if cfg!(windows) {
    "textweaver.exe"
} else {
    "textweaver"
};

/// The first argument that puts `textweaver` in terminal mode.
const TERMINAL_FLAG: &str = "--tw";

/// Where `textweaver` should be: beside the launcher at `me`.
fn target_beside(me: &Path) -> Option<PathBuf> {
    Some(me.parent()?.join(TARGET))
}

/// The arguments `textweaver` gets: the terminal flag, then the launcher's
/// own arguments, unchanged.
fn target_args(args: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    std::iter::once(OsString::from(TERMINAL_FLAG))
        .chain(args)
        .collect()
}

fn main() -> ExitCode {
    let target = std::env::current_exe()
        .ok()
        .and_then(|me| target_beside(&me))
        .filter(|p| p.is_file());
    let Some(target) = target else {
        eprintln!(
            "Error: textweaver was not found beside tw: {TARGET} should be in the same folder. Reinstall textweaver."
        );
        return ExitCode::FAILURE;
    };
    let mut cmd = Command::new(&target);
    cmd.args(target_args(std::env::args_os().skip(1)));
    run(cmd, &target)
}

/// Unix: becomes `textweaver`, so signals and the terminal go straight to
/// it.
#[cfg(unix)]
fn run(mut cmd: Command, target: &Path) -> ExitCode {
    use std::os::unix::process::CommandExt as _;
    let err = cmd.exec();
    eprintln!("Error: could not start {}: {err}.", target.display());
    ExitCode::FAILURE
}

/// Windows: runs `textweaver.exe`, waits, and passes its exit status on.
#[cfg(not(unix))]
fn run(mut cmd: Command, target: &Path) -> ExitCode {
    console::ignore_ctrl_c();
    match cmd.status() {
        Ok(status) => match status.code() {
            Some(0) => ExitCode::SUCCESS,
            Some(code) => std::process::exit(code),
            None => ExitCode::FAILURE,
        },
        Err(err) => {
            eprintln!("Error: could not start {}: {err}.", target.display());
            ExitCode::FAILURE
        }
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod console {
    //! One Win32 call, declared here so the launcher needs no crate.

    /// `CTRL_C_EVENT` and `CTRL_BREAK_EVENT`.
    const CTRL_C: u32 = 0;
    const CTRL_BREAK: u32 = 1;

    type Handler = unsafe extern "system" fn(u32) -> i32;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetConsoleCtrlHandler(handler: Option<Handler>, add: i32) -> i32;
    }

    /// Says Ctrl+C and Ctrl+Break are handled, so this process keeps
    /// waiting; closing the console still ends it.
    unsafe extern "system" fn handled(event: u32) -> i32 {
        i32::from(event == CTRL_C || event == CTRL_BREAK)
    }

    /// Keeps the launcher alive through Ctrl+C. A handler routine, unlike
    /// `SetConsoleCtrlHandler(NULL, TRUE)`, is not inherited, so the
    /// program it starts still receives Ctrl+C.
    pub(super) fn ignore_ctrl_c() {
        // SAFETY: `handled` is a plain function that lives for the whole
        // process and touches no state; registering it has no other effect.
        unsafe {
            SetConsoleCtrlHandler(Some(handled), 1);
        }
    }
}

#[cfg(all(not(unix), not(windows)))]
mod console {
    pub(super) fn ignore_ctrl_c() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_target_is_textweaver_beside_the_launcher() {
        let me = Path::new("install").join(if cfg!(windows) { "tw.exe" } else { "tw" });
        assert_eq!(target_beside(&me), Some(Path::new("install").join(TARGET)));
    }

    #[test]
    fn the_arguments_follow_the_terminal_flag_unchanged() {
        let args = target_args(["convert", "a b.md", "--to", "html"].map(OsString::from));
        assert_eq!(args, ["--tw", "convert", "a b.md", "--to", "html"]);
        assert_eq!(target_args(Vec::new()), ["--tw"]);
    }
}
