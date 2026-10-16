//! Host mode: how textweaver's programs run speech-engine hosts without a
//! separate helper program per engine (ADR-0007 addendum, Friday,
//! October 16, 2026).
//!
//! Engines that cannot run inside textweaver (Eloquence, SAPI 5 voices,
//! DECtalk, and eSpeak NG on Windows) still run in a process of their own,
//! for crash isolation and exact word timing. That process is now a copy
//! of the program that needs it, started as
//! `<program> --engine-host <engine> [host arguments]`: each program calls
//! [`host_mode`] first thing in `main`, before any other argument parsing
//! or start-up. For 32-bit engine libraries on Windows, one combined
//! program, `textweaver-host-x86.exe` (this crate's binary built for
//! `i686-pc-windows-msvc`), runs every engine ([`combined_main`]).
//!
//! The engines are `eci`, `espeak`, `dectalk` and `sapi`; each one's
//! arguments and protocol are its crate's `host` module.

use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

use textweaver_enginehost::hostmode::{self, Request};
pub use textweaver_enginehost::hostmode::{ENGINES, FLAG, X86_HOST_NAME};

/// Runs host mode when this program was started in it, and returns its
/// exit status; otherwise records that this program can run engine hosts
/// ([`hostmode::enable_self`]) and returns `None`, and `main` goes on.
///
/// Call it first thing in `main`:
///
/// ```no_run
/// fn main() -> std::process::ExitCode {
///     if let Some(status) = textweaver_host::host_mode() {
///         return status;
///     }
///     // The program itself.
///     std::process::ExitCode::SUCCESS
/// }
/// ```
pub fn host_mode() -> Option<ExitCode> {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let status = host_mode_with(&args);
    if status.is_none() {
        hostmode::enable_self();
    }
    status
}

/// [`host_mode`] for `args` (the arguments after the program's name),
/// without recording anything: `None` unless the first is [`FLAG`].
pub fn host_mode_with(args: &[OsString]) -> Option<ExitCode> {
    Some(match hostmode::parse(args)? {
        Ok(Request { engine, args }) => run_engine(&engine, args),
        Err(e) => fail(&e),
    })
}

/// The combined host's `main`: `textweaver-host-x86.exe <engine> [host
/// arguments]`, or the [`FLAG`] form the programs take.
pub fn combined_main(args: Vec<OsString>) -> ExitCode {
    if let Some(status) = host_mode_with(&args) {
        return status;
    }
    let mut it = args.into_iter();
    match it.next() {
        Some(engine) => match engine.into_string() {
            Ok(engine) => run_engine(&engine, it.collect()),
            Err(e) => fail(&format!(
                "the engine name {} is not valid text",
                e.to_string_lossy()
            )),
        },
        None => fail(&usage()),
    }
}

/// Runs `engine`'s host with `args` until its input ends or it is told to
/// quit. An engine textweaver does not know fails in words on stderr.
pub fn run_engine(engine: &str, args: Vec<OsString>) -> ExitCode {
    match engine {
        "eci" => textweaver_eci::host::main(args),
        "espeak" => textweaver_espeak::host::main(args),
        "dectalk" => textweaver_dectalk::host::main(args),
        "sapi" => textweaver_sapi::host::main(args),
        other => fail(&format!("unknown engine {other}; {}", usage())),
    }
}

fn usage() -> String {
    format!(
        "usage: textweaver-host-x86 ENGINE [ARGUMENTS], where ENGINE is one of {}",
        ENGINES.join(", ")
    )
}

/// Writes `message` to stderr (never failing) and returns failure. The
/// engine protocols differ, so nothing goes to stdout: textweaver sees
/// the host end and logs this line.
fn fail(message: &str) -> ExitCode {
    let mut err = std::io::stderr().lock();
    let _ = writeln!(err, "engine host: {message}");
    let _ = err.flush();
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(v: &[&str]) -> Vec<OsString> {
        v.iter().map(OsString::from).collect()
    }

    #[test]
    fn ordinary_arguments_leave_host_mode_off() {
        assert!(host_mode_with(&[]).is_none());
        assert!(host_mode_with(&os(&["book.md"])).is_none());
        assert!(host_mode_with(&os(&["speak", FLAG, "eci"])).is_none());
    }

    #[test]
    fn host_mode_without_a_known_engine_fails() {
        assert_eq!(host_mode_with(&os(&[FLAG])), Some(ExitCode::FAILURE));
        assert_eq!(
            host_mode_with(&os(&[FLAG, "votrax"])),
            Some(ExitCode::FAILURE)
        );
        assert_eq!(combined_main(Vec::new()), ExitCode::FAILURE);
        assert_eq!(combined_main(os(&["votrax"])), ExitCode::FAILURE);
    }
}
