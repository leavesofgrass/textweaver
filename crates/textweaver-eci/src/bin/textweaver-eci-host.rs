//! `textweaver-eci-host`: runs the ECI engine in its own process (ADR-0007).
//!
//! ```text
//! textweaver-eci-host [--library PATH] [--sample-rate HZ] [--dictionaries DIR]
//!                     [--engine eci|fake]
//! ```
//!
//! `--dictionaries` loads the pronunciation dictionaries in `DIR` (see
//! `textweaver_eci::dictionaries`); without it none are loaded.
//!
//! Speaks the framed protocol of `textweaver_eci::protocol` on stdin and
//! stdout; logs go to stderr. The library path defaults to
//! `TEXTWEAVER_ECI_LIBRARY`, then the platform's standard install location.
//! If the engine cannot start, the host still sends one `Error` frame (token
//! 0) describing why, then exits with status 1.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use textweaver_eci::host::{self, fake, ffi};
use textweaver_eci::protocol::{self, Reply};
use textweaver_enginehost::serve::{AtEnd, log_line};

struct Args {
    library: Option<PathBuf>,
    sample_rate: Option<u32>,
    dictionaries: Option<PathBuf>,
    fake: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        library: None,
        sample_rate: None,
        dictionaries: None,
        fake: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--library" => {
                args.library = Some(it.next().ok_or("--library needs a path")?.into());
            }
            "--sample-rate" => {
                let v = it.next().ok_or("--sample-rate needs a number")?;
                args.sample_rate = Some(v.parse().map_err(|_| format!("bad sample rate {v}"))?);
            }
            "--dictionaries" => {
                args.dictionaries =
                    Some(it.next().ok_or("--dictionaries needs a directory")?.into());
            }
            "--engine" => match it.next().as_deref() {
                Some("eci") => args.fake = false,
                Some("fake") => args.fake = true,
                other => return Err(format!("unknown engine {other:?}")),
            },
            "--help" | "-h" => {
                return Err(
                    "usage: textweaver-eci-host [--library PATH] [--sample-rate HZ] \
                     [--dictionaries DIR] [--engine eci|fake]"
                        .into(),
                );
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(args)
}

fn fail(message: String) -> ExitCode {
    log_line(&format!("textweaver-eci-host: {message}"));
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
        let mut engine = fake::FakeEngine::new(fake::FakeConfig {
            dictionaries: args.dictionaries,
            ..fake::FakeConfig::default()
        });
        host::run(&mut engine, stdin, &mut stdout, AtEnd::host())
    } else {
        use textweaver_eci::discovery;
        let candidates =
            discovery::library_candidates(args.library.as_deref(), &discovery::Places::current());
        let path = match discovery::choose_library(&candidates) {
            Ok(c) => {
                log_line(&format!("textweaver-eci-host: using {}", c.reason));
                c.candidate.path
            }
            Err(e) => return fail(e),
        };
        let mut engine = match ffi::EciEngine::load(&path, args.sample_rate, args.dictionaries) {
            Ok(e) => e,
            Err(e) => return fail(e),
        };
        host::run(&mut engine, stdin, &mut stdout, AtEnd::host())
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        // The backend closed the pipe; nothing is listening any more.
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(e) => {
            log_line(&format!("textweaver-eci-host: {e}"));
            ExitCode::FAILURE
        }
    }
}
