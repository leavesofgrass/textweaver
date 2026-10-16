//! Host mode: the programs that run engine hosts (Friday, October 16,
//! 2026; see the addendum to ADR-0007).
//!
//! textweaver used to ship one helper program per engine and architecture
//! (`textweaver-eci-host.exe`, `textweaver-sapi-host-x86.exe`, and so on).
//! Antivirus heuristics flag small unsigned helpers like these, so the
//! helpers are now combined. Engines still run in their own process; only
//! the number of executables changed:
//!
//! - **Native architecture.** The program that needs an engine (`tw`, the
//!   textweaver app) starts a copy of itself in a hidden host mode,
//!   `<program> --engine-host <engine> [host arguments]`. It checks for
//!   [`FLAG`] before any other argument parsing or start-up, and calls
//!   [`enable_self`] otherwise, which makes [`self_host`] offer it as the
//!   native host.
//! - **32-bit engines on Windows.** One program, [`X86_HOST_NAME`], runs
//!   every engine's 32-bit library: `textweaver-host-x86.exe <engine>
//!   [host arguments]` (it also accepts the [`FLAG`] form).
//!
//! A host path named by a setting or an environment variable keeps
//! working. [`engine_args`] decides from the file name what goes before
//! the host's own arguments: nothing for a separate per-engine host from
//! an earlier release (or a development build of one), the flag and the
//! engine for any other program.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// The first argument that puts a program in host mode; the engine's name
/// follows it.
pub const FLAG: &str = "--engine-host";

/// The combined 32-bit host's file name (Windows; `cargo xtask hosts`
/// installs it beside the programs).
pub const X86_HOST_NAME: &str = if cfg!(windows) {
    "textweaver-host-x86.exe"
} else {
    "textweaver-host-x86"
};

/// The combined host's cargo binary name: what a development build of it
/// is called before `cargo xtask hosts` installs it as [`X86_HOST_NAME`].
pub const DEV_BIN: &str = "textweaver-host";

/// The 32-bit Windows target triple.
const X86_TARGET: &str = "i686-pc-windows-msvc";

/// The engines a host runs, by the name host mode takes.
pub const ENGINES: [&str; 4] = ["eci", "espeak", "dectalk", "sapi"];

/// File stems (lowercase, without `.exe`) of programs that take [`FLAG`].
const COMBINED: [&str; 6] = [
    "tw",
    "textweaver",
    "textweaver-gui",
    "textweaver-xilem",
    DEV_BIN,
    "textweaver-host-x86",
];

static SELF_HOST: AtomicBool = AtomicBool::new(false);

/// Records that this program runs engine hosts itself (it checks for
/// [`FLAG`] first thing). Called by the programs' `main`, never by tests:
/// a test binary cannot be a host.
pub fn enable_self() {
    SELF_HOST.store(true, Ordering::SeqCst);
}

/// This program, when it runs engine hosts itself ([`enable_self`]): the
/// native host for every engine.
pub fn self_host() -> Option<PathBuf> {
    if !SELF_HOST.load(Ordering::SeqCst) {
        return None;
    }
    std::env::current_exe().ok()
}

/// Where the combined 32-bit host may be, in order: beside the running
/// program and in its parent folder (test binaries sit in `deps/`), then,
/// on Windows, a cargo `i686-pc-windows-msvc` build under the target
/// folder. The paths are not checked.
pub fn x86_hosts() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(PathBuf::from))
    else {
        return out;
    };
    for d in dir.ancestors().take(2) {
        out.push(d.join(X86_HOST_NAME));
    }
    if !cfg!(windows) {
        return out;
    }
    for d in dir.ancestors().take(4) {
        for profile in ["release", "debug"] {
            out.push(
                d.join(X86_TARGET)
                    .join(profile)
                    .join(format!("{DEV_BIN}.exe")),
            );
        }
    }
    out
}

/// True when the program at `path` takes [`FLAG`] and an engine name
/// before the host's own arguments: this program in host mode, the
/// combined 32-bit host, or another textweaver program. A separate
/// per-engine host (`textweaver-eci-host.exe`) does not.
pub fn takes_flag(path: &Path) -> bool {
    if self_host().is_some_and(|me| same_file(&me, path)) {
        return true;
    }
    let Some(stem) = path.file_name().and_then(OsStr::to_str) else {
        return false;
    };
    let stem = stem.to_ascii_lowercase();
    let stem = stem.strip_suffix(".exe").unwrap_or(&stem);
    COMBINED.contains(&stem)
}

fn same_file(a: &Path, b: &Path) -> bool {
    a == b
        || matches!(
            (a.canonicalize(), b.canonicalize()),
            (Ok(x), Ok(y)) if x == y
        )
}

/// The arguments that go before the host's own when starting `path` as
/// `engine`'s host: `--engine-host <engine>` for a program that
/// [`takes_flag`], none for a separate per-engine host.
pub fn engine_args(path: &Path, engine: &str) -> Vec<OsString> {
    if takes_flag(path) {
        vec![FLAG.into(), engine.into()]
    } else {
        Vec::new()
    }
}

/// A host-mode request read from a command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    /// The engine to run (one of [`ENGINES`], unchecked).
    pub engine: String,
    /// The host's own arguments.
    pub args: Vec<OsString>,
}

/// Reads host mode from `args`, the arguments after the program's name:
/// `None` unless the first is [`FLAG`]; then the engine that follows it,
/// or an error in words when it is missing.
pub fn parse(args: &[OsString]) -> Option<Result<Request, String>> {
    let (first, rest) = args.split_first()?;
    if first != FLAG {
        return None;
    }
    Some(match rest.split_first() {
        Some((engine, args)) => match engine.to_str() {
            Some(e) => Ok(Request {
                engine: e.to_owned(),
                args: args.to_vec(),
            }),
            None => Err(format!(
                "the engine name {} is not valid text",
                engine.to_string_lossy()
            )),
        },
        None => Err(format!(
            "{FLAG} needs an engine name: one of {}",
            ENGINES.join(", ")
        )),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(v: &[&str]) -> Vec<OsString> {
        v.iter().map(OsString::from).collect()
    }

    #[test]
    fn host_mode_is_chosen_by_the_first_argument_only() {
        assert_eq!(parse(&[]), None);
        assert_eq!(parse(&os(&["book.md"])), None);
        assert_eq!(parse(&os(&["book.md", FLAG, "eci"])), None);
        assert_eq!(
            parse(&os(&[FLAG, "eci", "--engine", "fake"])),
            Some(Ok(Request {
                engine: "eci".into(),
                args: os(&["--engine", "fake"]),
            }))
        );
        let missing = parse(&os(&[FLAG])).unwrap().unwrap_err();
        assert!(missing.contains("eci, espeak, dectalk, sapi"), "{missing}");
    }

    #[test]
    fn combined_hosts_take_the_flag_and_separate_hosts_do_not() {
        for p in [
            "tw.exe",
            "/usr/bin/tw",
            "TEXTWEAVER-GUI.EXE",
            "textweaver-xilem",
            "textweaver-host-x86.exe",
            "target/i686-pc-windows-msvc/debug/textweaver-host.exe",
        ] {
            assert!(takes_flag(Path::new(p)), "{p}");
            assert_eq!(engine_args(Path::new(p), "sapi"), os(&[FLAG, "sapi"]));
        }
        for p in [
            "textweaver-eci-host.exe",
            "textweaver-sapi-host-x86.exe",
            "/opt/hosts/textweaver-dectalk-host",
            "my-host",
        ] {
            assert!(!takes_flag(Path::new(p)), "{p}");
            assert!(engine_args(Path::new(p), "eci").is_empty());
        }
    }

    #[test]
    fn the_x86_host_is_looked_for_beside_the_program() {
        let found = x86_hosts();
        assert!(found[0].ends_with(X86_HOST_NAME));
        assert_eq!(
            found.iter().any(|p| p.ends_with("textweaver-host.exe")),
            cfg!(windows)
        );
    }
}
