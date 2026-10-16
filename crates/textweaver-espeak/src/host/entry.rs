//! The host's entry point: what `<program> --engine-host espeak` runs (and
//! `textweaver-host-x86.exe espeak`, the combined 32-bit host). Its arguments:
//!
//! ```text
//! <program> --engine-host espeak [--library PATH] [--engine espeak|fake]
//! ```
//!
//! Speaks the framed protocol of `textweaver_espeak::protocol` on stdin
//! and stdout; logs go to stderr. `--library` names the libespeak-ng to
//! load (textweaver passes the one it chose for this host's
//! architecture); without it the host searches as the in-process backend
//! does: the components folder, `TEXTWEAVER_ESPEAK_LIBRARY`, then the
//! usual places. If eSpeak NG cannot start, the host still sends one
//! `Error` frame (token 0) saying why, then exits with status 1.

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use super::{fake, real};
use crate::protocol::{self, Message, Reply};
use textweaver_enginehost::serve::{AtEnd, log_line};

const USAGE: &str = "usage: <program> --engine-host espeak [--library PATH] [--engine espeak|fake]";

struct Args {
    library: Option<PathBuf>,
    fake: bool,
}

fn parse_args(raw: Vec<OsString>) -> Result<Args, String> {
    let mut args = Args {
        library: None,
        fake: false,
    };
    let mut it = raw.into_iter();
    while let Some(a) = it.next() {
        match a.to_str() {
            Some("--library") => {
                args.library = Some(it.next().ok_or("--library needs a path")?.into());
            }
            Some("--engine") => match it.next().as_ref().and_then(|v| v.to_str()) {
                Some("espeak") => args.fake = false,
                Some("fake") => args.fake = true,
                other => return Err(format!("unknown engine {other:?}")),
            },
            Some("--help" | "-h") => return Err(USAGE.into()),
            _ => return Err(format!("unknown argument {}; {USAGE}", a.to_string_lossy())),
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

/// Runs the espeak host with `args` (the host's own arguments, after the
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
        super::run(
            &mut fake::FakeEngine::default(),
            stdin,
            &mut stdout,
            AtEnd::host(),
        )
    } else {
        let mut engine = match real::EspeakEngine::start(args.library) {
            Ok(e) => e,
            Err(e) => return fail(e),
        };
        log_line("eSpeak NG started");
        super::run(&mut engine, stdin, &mut stdout, AtEnd::host())
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        // textweaver closed the pipe; nothing is listening any more.
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(e) => {
            log_line(&format!("{e}"));
            ExitCode::FAILURE
        }
    }
}
