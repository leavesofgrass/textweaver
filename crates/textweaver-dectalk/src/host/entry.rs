//! The host's entry point: what `<program> --engine-host dectalk` runs (and
//! `textweaver-host-x86.exe dectalk`, the combined 32-bit host). Its arguments:
//! process (ADR-0021).
//!
//! ```text
//! <program> --engine-host dectalk [--library PATH] [--convention cdecl|stdcall] [--engine dectalk|fake]
//!                         [--start-delay-ms MS]
//! ```
//!
//! `--start-delay-ms` (fake engine only, for tests) waits before starting.
//!
//! Speaks the framed protocol of `textweaver_dectalk::protocol` on stdin
//! and stdout; logs go to stderr. The library defaults to
//! `TEXTWEAVER_DECTALK_LIBRARY`, then the usual install folders.
//! `--convention` forces the calling convention of a 32-bit Windows
//! library (normally detected). If DECtalk cannot start, the host still
//! sends one `Error` frame (token 0) saying why, then exits with status 1.

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use super::{fake, ffi};
use crate::discovery;
use crate::protocol::{self, Reply};
use textweaver_enginehost::serve::{AtEnd, log_line};

struct Args {
    library: Option<PathBuf>,
    convention: Option<ffi::Convention>,
    fake: bool,
    start_delay_ms: u64,
}

const USAGE: &str = "usage: <program> --engine-host dectalk [--library PATH] \
                     [--convention cdecl|stdcall] [--engine dectalk|fake] \
                     [--start-delay-ms MS]";

fn parse_args(raw: Vec<OsString>) -> Result<Args, String> {
    let mut args = Args {
        library: None,
        convention: None,
        fake: false,
        start_delay_ms: 0,
    };
    let mut it = raw.into_iter().map(|a| a.to_string_lossy().into_owned());
    while let Some(a) = it.next() {
        match a.as_str() {
            "--library" => {
                args.library = Some(it.next().ok_or("--library needs a path")?.into());
            }
            "--convention" => {
                let v = it.next().ok_or("--convention needs cdecl or stdcall")?;
                args.convention = Some(
                    ffi::Convention::parse(&v).ok_or_else(|| format!("unknown convention {v}"))?,
                );
            }
            "--start-delay-ms" => {
                let v = it.next().ok_or("--start-delay-ms needs a number")?;
                args.start_delay_ms = v.parse().map_err(|_| format!("bad delay {v}"))?;
            }
            "--engine" => match it.next().as_deref() {
                Some("dectalk") => args.fake = false,
                Some("fake") => args.fake = true,
                other => return Err(format!("unknown engine {other:?}")),
            },
            "--help" | "-h" => return Err(USAGE.into()),
            other => return Err(format!("unknown argument {other}; {USAGE}")),
        }
    }
    Ok(args)
}

fn fail(message: String) -> ExitCode {
    log_line(&message);
    let mut out = std::io::stdout().lock();
    let _ = protocol::write_frame(&mut out, &Reply::Error { token: 0, message }.encode());
    let _ = out.flush();
    ExitCode::FAILURE
}

/// Runs the dectalk host with `args` (the host's own arguments, after the
/// program name and host mode's) until its input ends or it is told to
/// quit; the exit status says how it ended.
pub fn main(args: Vec<OsString>) -> ExitCode {
    let args = match parse_args(args) {
        Ok(a) => a,
        Err(e) => return fail(e),
    };
    let stdin = std::io::stdin();
    let mut stdout = std::io::BufWriter::with_capacity(64 * 1024, std::io::stdout().lock());
    let result = if args.fake {
        std::thread::sleep(std::time::Duration::from_millis(args.start_delay_ms));
        super::run(&mut fake::FakeEngine, stdin, &mut stdout, AtEnd::host())
    } else {
        let path = match args.library {
            // A library named on the command line is used as given (the
            // backend has already checked it).
            Some(p) => p,
            None => {
                let candidates = discovery::library_candidates(None, &discovery::Places::current());
                match discovery::choose_library(&candidates) {
                    Ok(c) => {
                        log_line(&format!("using {}", c.reason));
                        c.candidate.path
                    }
                    Err(e) => return fail(e),
                }
            }
        };
        let mut engine = match ffi::DectalkEngine::load(&path, args.convention) {
            Ok(e) => e,
            Err(e) => return fail(e),
        };
        log_line(&format!(
            "DECtalk started ({} convention, {})",
            engine.convention().name(),
            if engine.has_callback() {
                "buffer callback"
            } else {
                "single buffer"
            }
        ));
        super::run(&mut engine, stdin, &mut stdout, AtEnd::host())
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        // The backend closed the pipe; nothing is listening any more.
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(e) => {
            log_line(&format!("{e}"));
            ExitCode::FAILURE
        }
    }
}
