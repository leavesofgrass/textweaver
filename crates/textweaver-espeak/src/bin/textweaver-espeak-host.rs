//! `textweaver-espeak-host`: runs eSpeak NG in its own process.
//!
//! ```text
//! textweaver-espeak-host [--library PATH] [--engine espeak|fake]
//! ```
//!
//! Speaks the framed protocol of `textweaver_espeak::protocol` on stdin
//! and stdout; logs go to stderr. `--library` names the libespeak-ng to
//! load (textweaver passes the one it chose for this host's
//! architecture); without it the host searches as the in-process backend
//! does: the components folder, `TEXTWEAVER_ESPEAK_LIBRARY`, then the
//! usual places. If eSpeak NG cannot start, the host still sends one
//! `Error` frame (token 0) saying why, then exits with status 1.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use textweaver_enginehost::serve::{AtEnd, log_line};
use textweaver_espeak::host::{self, fake, real};
use textweaver_espeak::protocol::{self, Message, Reply};

const USAGE: &str = "usage: textweaver-espeak-host [--library PATH] [--engine espeak|fake]";

struct Args {
    library: Option<PathBuf>,
    fake: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        library: None,
        fake: false,
    };
    let mut it = std::env::args_os().skip(1);
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
    log_line(&format!("textweaver-espeak-host: {message}"));
    let mut out = std::io::stdout().lock();
    let _ = protocol::write_frame(&mut out, &Reply::Error { token: 0, message }.encode());
    let _ = out.flush();
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => return fail(e),
    };
    let stdin = std::io::stdin();
    let mut stdout = std::io::BufWriter::with_capacity(64 * 1024, std::io::stdout().lock());
    let result = if args.fake {
        host::run(
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
        log_line("textweaver-espeak-host: eSpeak NG started");
        host::run(&mut engine, stdin, &mut stdout, AtEnd::host())
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        // textweaver closed the pipe; nothing is listening any more.
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(e) => {
            log_line(&format!("textweaver-espeak-host: {e}"));
            ExitCode::FAILURE
        }
    }
}
