//! `textweaver-eci-host`: runs the ECI engine in its own process (ADR-0007).
//!
//! ```text
//! textweaver-eci-host [--library PATH] [--sample-rate HZ] [--dictionaries DIR]
//!                     [--skip-dictionary-volume N]... [--engine eci|fake]
//!                     [--start-delay-ms MS] [--dictionary-delay-ms MS]
//! ```
//!
//! `--start-delay-ms` (fake engine only, for tests) waits before starting,
//! like a cold engine. `--dictionary-delay-ms` (fake engine only) waits
//! that long for each dictionary file, like an engine that loads a large
//! dictionary slowly (OpenEVV 0.3.0 takes about a minute for the English
//! root dictionary).
//!
//! `--dictionaries` loads the pronunciation dictionaries in `DIR` (see
//! `textweaver_eci::dictionaries`); without it none are loaded.
//! `--skip-dictionary-volume` leaves out one volume (0 main, 1 root,
//! 2 abbreviations); it may be given more than once.
//!
//! Speaks the framed protocol of `textweaver_eci::protocol` on stdin and
//! stdout; logs go to stderr. The library path defaults to
//! `TEXTWEAVER_ECI_LIBRARY`, then the platform's standard install location.
//! If the engine cannot start, the host still sends one `Error` frame (token
//! 0) describing why, then exits with status 1.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use textweaver_eci::host::{self, fake, ffi};
use textweaver_eci::protocol::{self, Reply};
use textweaver_enginehost::serve::{AtEnd, log_line};

struct Args {
    library: Option<PathBuf>,
    sample_rate: Option<u32>,
    dictionaries: Option<PathBuf>,
    skip_volumes: Vec<u8>,
    fake: bool,
    start_delay_ms: u64,
    dictionary_delay_ms: u64,
}

const USAGE: &str = "usage: textweaver-eci-host [--library PATH] [--sample-rate HZ] \
     [--dictionaries DIR] [--skip-dictionary-volume N]... [--engine eci|fake] \
     [--start-delay-ms MS] [--dictionary-delay-ms MS]";

fn number<T: std::str::FromStr>(
    it: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<T, String> {
    let v = it.next().ok_or_else(|| format!("{flag} needs a number"))?;
    v.parse().map_err(|_| format!("bad number {v} for {flag}"))
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        library: None,
        sample_rate: None,
        dictionaries: None,
        skip_volumes: Vec::new(),
        fake: false,
        start_delay_ms: 0,
        dictionary_delay_ms: 0,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--library" => {
                args.library = Some(it.next().ok_or("--library needs a path")?.into());
            }
            "--sample-rate" => args.sample_rate = Some(number(&mut it, "--sample-rate")?),
            "--dictionaries" => {
                args.dictionaries =
                    Some(it.next().ok_or("--dictionaries needs a directory")?.into());
            }
            "--skip-dictionary-volume" => {
                args.skip_volumes
                    .push(number(&mut it, "--skip-dictionary-volume")?);
            }
            "--start-delay-ms" => args.start_delay_ms = number(&mut it, "--start-delay-ms")?,
            "--dictionary-delay-ms" => {
                args.dictionary_delay_ms = number(&mut it, "--dictionary-delay-ms")?;
            }
            "--engine" => match it.next().as_deref() {
                Some("eci") => args.fake = false,
                Some("fake") => args.fake = true,
                other => return Err(format!("unknown engine {other:?}")),
            },
            "--help" | "-h" => return Err(USAGE.into()),
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
        std::thread::sleep(Duration::from_millis(args.start_delay_ms));
        let mut engine = fake::FakeEngine::new(fake::FakeConfig {
            dictionaries: args.dictionaries,
            skip_volumes: args.skip_volumes,
            dictionary_delay: Duration::from_millis(args.dictionary_delay_ms),
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
        engine.skip_dictionary_volumes(&args.skip_volumes);
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
