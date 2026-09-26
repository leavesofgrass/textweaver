//! `textweaver-eci-host`: runs the ECI engine in its own process (ADR-0007).
//!
//! ```text
//! textweaver-eci-host [--library PATH] [--sample-rate HZ] [--engine eci|fake]
//! ```
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

struct Args {
    library: Option<PathBuf>,
    sample_rate: Option<u32>,
    fake: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        library: None,
        sample_rate: None,
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
            "--engine" => match it.next().as_deref() {
                Some("eci") => args.fake = false,
                Some("fake") => args.fake = true,
                other => return Err(format!("unknown engine {other:?}")),
            },
            "--help" | "-h" => {
                return Err(
                    "usage: textweaver-eci-host [--library PATH] [--sample-rate HZ] [--engine eci|fake]"
                        .into(),
                );
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(args)
}

fn fail(message: String) -> ExitCode {
    eprintln!("textweaver-eci-host: {message}");
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
        let mut engine = fake::FakeEngine::new(fake::FakeConfig::default());
        host::run(&mut engine, stdin, &mut stdout)
    } else {
        let Some(path) = args.library.or_else(textweaver_eci::library_path) else {
            return fail(
                "no ECI library found (install Eloquence, or set TEXTWEAVER_ECI_LIBRARY)".into(),
            );
        };
        let mut engine = match ffi::EciEngine::load(&path, args.sample_rate) {
            Ok(e) => e,
            Err(e) => return fail(e),
        };
        host::run(&mut engine, stdin, &mut stdout)
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        // The backend closed the pipe; nothing is listening any more.
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("textweaver-eci-host: {e}");
            ExitCode::FAILURE
        }
    }
}
