//! Finding the user's DECtalk library and the host that can run it.
//!
//! textweaver ships no DECtalk and never downloads one: it uses a DECtalk
//! the user installed and licensed (ADR-0021). Candidates, in order; the
//! first existing file that exports DECtalk's API wins:
//!
//! 1. `TEXTWEAVER_DECTALK_LIBRARY`;
//! 2. [`DectalkConfig::library`](crate::DectalkConfig::library), the
//!    backend option (the application maps a `[speech.dectalk] library`
//!    setting onto it);
//! 3. the usual install folders. Windows: `DECtalk\DECtalk.dll`,
//!    `Fonix\DECtalk\DECtalk.dll`, and `Fonix DECtalk\DECtalk.dll` under
//!    `%ProgramFiles%` and `%ProgramFiles(x86)%`, then `dectalk.dll` in
//!    `%SystemRoot%\System32` and `%SystemRoot%\SysWOW64` (where DECtalk
//!    Software's runtime installs it). Linux: `libtts.so` in `/usr/lib`,
//!    `/usr/local/lib`, and `/opt/dectalk/lib`, then `libdectalk.so` in
//!    `/usr/lib` and `/usr/local/lib`.
//!
//! A file counts as DECtalk only if it contains the name of DECtalk's
//! start-up function (`TextToSpeechStartup`), so an unrelated `libtts.so`
//! is passed over. No other location is searched.
//!
//! **Architecture.** The library's own header decides which host runs it
//! ([`library_arch`]: PE `Machine` on Windows, ELF class on Linux): a
//! 32-bit DLL (the usual DECtalk for Windows) runs in
//! `textweaver-dectalk-host-x86.exe` (built for `i686-pc-windows-msvc` by
//! `cargo xtask hosts`), a 64-bit one in the native
//! `textweaver-dectalk-host[.exe]`. On Linux a 32-bit `libtts.so` needs a
//! `textweaver-dectalk-host-x86` built for `i686-unknown-linux-gnu`, which
//! the xtask does not build.

use std::path::{Path, PathBuf};

use crate::DectalkConfig;

/// The function name a DECtalk library must export.
pub const SIGNATURE: &[u8] = b"TextToSpeechStartup";
/// Largest file examined for [`SIGNATURE`] (64 MiB).
const MAX_SCAN: u64 = 64 * 1024 * 1024;

/// Why a candidate is on the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// `TEXTWEAVER_DECTALK_LIBRARY`.
    Environment,
    /// The backend option (a setting).
    Option,
    /// A usual install location.
    Default,
}

/// One place the library might be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryCandidate {
    /// The file.
    pub path: PathBuf,
    /// Why it is listed.
    pub source: Source,
    /// Whether the file exists.
    pub exists: bool,
}

/// A library's machine architecture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arch {
    /// 32-bit x86.
    X86,
    /// x86-64.
    X64,
    /// 64-bit ARM.
    Arm64,
    /// Something else.
    Other,
}

impl Arch {
    /// Words for messages read aloud.
    pub fn describe(self) -> &'static str {
        match self {
            Arch::X86 => "32-bit x86",
            Arch::X64 => "x86-64",
            Arch::Arm64 => "ARM64",
            Arch::Other => "an unsupported architecture",
        }
    }
}

/// Where the usual install locations are (injectable for tests).
#[derive(Clone, Debug, Default)]
pub struct Places {
    /// `TEXTWEAVER_DECTALK_LIBRARY`.
    pub env_library: Option<PathBuf>,
    /// `%ProgramFiles%` (the 64-bit one).
    pub program_files: Option<PathBuf>,
    /// `%ProgramFiles(x86)%`.
    pub program_files_x86: Option<PathBuf>,
    /// `%SystemRoot%`.
    pub system_root: Option<PathBuf>,
    /// Whether to list the Windows locations (else the Linux ones).
    pub windows: bool,
}

impl Places {
    /// This machine's.
    pub fn current() -> Places {
        let var = |k: &str| {
            std::env::var_os(k)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        Places {
            env_library: var(crate::LIBRARY_ENV),
            program_files: var("ProgramW6432").or_else(|| var("ProgramFiles")),
            program_files_x86: var("ProgramFiles(x86)"),
            system_root: var("SystemRoot"),
            windows: cfg!(windows),
        }
    }
}

fn join(base: &Path, parts: &[&str]) -> PathBuf {
    parts.iter().fold(base.to_path_buf(), |p, c| p.join(c))
}

/// Every candidate, in order, existing or not.
pub fn library_candidates(option: Option<&Path>, places: &Places) -> Vec<LibraryCandidate> {
    let mut paths: Vec<(PathBuf, Source)> = Vec::new();
    if let Some(p) = &places.env_library {
        paths.push((p.clone(), Source::Environment));
    }
    if let Some(p) = option {
        paths.push((p.to_path_buf(), Source::Option));
    }
    if places.windows {
        for pf in [&places.program_files, &places.program_files_x86]
            .into_iter()
            .flatten()
        {
            for dir in [&["DECtalk"][..], &["Fonix", "DECtalk"], &["Fonix DECtalk"]] {
                paths.push((join(&join(pf, dir), &["DECtalk.dll"]), Source::Default));
            }
        }
        if let Some(root) = &places.system_root {
            for sys in ["System32", "SysWOW64"] {
                paths.push((join(root, &[sys, "dectalk.dll"]), Source::Default));
            }
        }
    } else {
        for p in [
            "/usr/lib/libtts.so",
            "/usr/local/lib/libtts.so",
            "/opt/dectalk/lib/libtts.so",
            "/usr/lib/libdectalk.so",
            "/usr/local/lib/libdectalk.so",
        ] {
            paths.push((PathBuf::from(p), Source::Default));
        }
    }
    let mut seen: Vec<PathBuf> = Vec::new();
    paths
        .into_iter()
        .filter(|(p, _)| {
            let new = !seen.contains(p);
            if new {
                seen.push(p.clone());
            }
            new
        })
        .map(|(path, source)| LibraryCandidate {
            exists: path.is_file(),
            path,
            source,
        })
        .collect()
}

/// The architecture in a PE (Windows) or ELF (Linux) header, from the
/// file's first bytes.
pub fn arch_from_header(bytes: &[u8]) -> Option<Arch> {
    if bytes.starts_with(b"MZ") {
        let at = u32::from_le_bytes(bytes.get(0x3C..0x40)?.try_into().ok()?) as usize;
        if bytes.get(at..at.checked_add(4)?)? != b"PE\0\0" {
            return None;
        }
        let machine = u16::from_le_bytes(bytes.get(at + 4..at + 6)?.try_into().ok()?);
        return Some(match machine {
            0x014C => Arch::X86,
            0x8664 => Arch::X64,
            0xAA64 => Arch::Arm64,
            _ => Arch::Other,
        });
    }
    if bytes.starts_with(b"\x7fELF") {
        let little = *bytes.get(5)? == 1;
        let raw = bytes.get(18..20)?;
        let machine = if little {
            u16::from_le_bytes([raw[0], raw[1]])
        } else {
            u16::from_be_bytes([raw[0], raw[1]])
        };
        return Some(match (bytes.get(4)?, machine) {
            (1, 3) => Arch::X86,
            (2, 62) => Arch::X64,
            (2, 183) => Arch::Arm64,
            _ => Arch::Other,
        });
    }
    None
}

/// The architecture of the library at `path` (reads its header).
pub fn library_arch(path: &Path) -> Option<Arch> {
    use std::io::Read;
    let mut head = Vec::with_capacity(4096);
    std::fs::File::open(path)
        .ok()?
        .take(4096)
        .read_to_end(&mut head)
        .ok()?;
    if let Some(a) = arch_from_header(&head) {
        return Some(a);
    }
    // A PE header beyond the first 4 KiB: read up to it.
    if head.starts_with(b"MZ") && head.len() >= 0x40 {
        let at = u32::from_le_bytes(head[0x3C..0x40].try_into().ok()?) as u64;
        if at < MAX_SCAN {
            let mut all = Vec::new();
            std::fs::File::open(path)
                .ok()?
                .take(at + 6)
                .read_to_end(&mut all)
                .ok()?;
            return arch_from_header(&all);
        }
    }
    None
}

/// Whether `bytes` contain DECtalk's start-up function name.
pub fn has_signature(bytes: &[u8]) -> bool {
    bytes.windows(SIGNATURE.len()).any(|w| w == SIGNATURE)
}

/// Whether the file at `path` looks like a DECtalk library (see the
/// module docs).
pub fn looks_like_dectalk(path: &Path) -> bool {
    match std::fs::metadata(path) {
        Ok(m) if m.len() <= MAX_SCAN => std::fs::read(path).is_ok_and(|b| has_signature(&b)),
        _ => false,
    }
}

/// The library textweaver will use, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryChoice {
    /// The chosen candidate.
    pub candidate: LibraryCandidate,
    /// Its architecture, when the header could be read.
    pub arch: Option<Arch>,
    /// A sentence explaining the choice, for diagnostics.
    pub reason: String,
}

/// Picks the first existing candidate that looks like DECtalk. A library
/// named explicitly (environment or option) that is not DECtalk is an
/// error rather than being passed over, so a typo is reported.
pub fn choose_library(candidates: &[LibraryCandidate]) -> Result<LibraryChoice, String> {
    choose_with(candidates, looks_like_dectalk, library_arch)
}

fn choose_with(
    candidates: &[LibraryCandidate],
    is_dectalk: impl Fn(&Path) -> bool,
    arch_of: impl Fn(&Path) -> Option<Arch>,
) -> Result<LibraryChoice, String> {
    for c in candidates.iter().filter(|c| c.exists) {
        if !is_dectalk(&c.path) {
            if c.source == Source::Default {
                continue;
            }
            return Err(format!(
                "{} does not look like a DECtalk library (it has no TextToSpeechStartup)",
                c.path.display()
            ));
        }
        let arch = arch_of(&c.path);
        let why = match c.source {
            Source::Environment => format!("named by {}", crate::LIBRARY_ENV),
            Source::Option => "named in the settings".to_string(),
            Source::Default => "found at a usual install location".to_string(),
        };
        let arch_text = arch.map_or("unknown architecture", Arch::describe);
        return Ok(LibraryChoice {
            reason: format!("DECtalk at {}, {why} ({arch_text})", c.path.display()),
            candidate: c.clone(),
            arch,
        });
    }
    let tried: Vec<String> = candidates
        .iter()
        .map(|c| c.path.display().to_string())
        .collect();
    Err(format!(
        "no DECtalk found (looked for {}); install a licensed DECtalk, or set {} to its library",
        if tried.is_empty() {
            "nothing".to_string()
        } else {
            tried.join(", ")
        },
        crate::LIBRARY_ENV
    ))
}

/// The native host's file name.
pub const HOST_NAME: &str = if cfg!(windows) {
    "textweaver-dectalk-host.exe"
} else {
    "textweaver-dectalk-host"
};

/// The 32-bit host's file name (`cargo xtask hosts` installs it next to
/// the binaries on Windows).
pub const HOST_NAME_X86: &str = if cfg!(windows) {
    "textweaver-dectalk-host-x86.exe"
} else {
    "textweaver-dectalk-host-x86"
};

/// Host executables that can run a library of architecture `arch` (`None`:
/// any), in order: [`DectalkConfig::host`], `TEXTWEAVER_DECTALK_HOST`,
/// beside the running executable (and its parent directory, for test
/// binaries in `deps/`), and on Windows a cargo `i686-pc-windows-msvc`
/// build under the target directory. Only existing files are returned.
pub fn host_candidates(config: &DectalkConfig, arch: Option<Arch>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    if let Some(h) = &config.host {
        out.push(h.clone());
    }
    if let Some(h) = std::env::var_os(crate::HOST_ENV).filter(|v| !v.is_empty()) {
        out.push(PathBuf::from(h));
    }
    let want_x86 = !config.fake_engine && arch != Some(Arch::X64);
    let want_native = config.fake_engine || arch != Some(Arch::X86);
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(PathBuf::from))
    {
        for d in dir.ancestors().take(2) {
            if want_x86 {
                out.push(d.join(HOST_NAME_X86));
            }
            if want_native {
                out.push(d.join(HOST_NAME));
            }
        }
        if want_x86 && cfg!(windows) {
            for d in dir.ancestors().take(4) {
                for profile in ["release", "debug"] {
                    out.push(d.join("i686-pc-windows-msvc").join(profile).join(HOST_NAME));
                }
            }
        }
    }
    let mut seen = Vec::new();
    out.retain(|p| {
        let keep = p.is_file() && !seen.contains(p);
        if keep {
            seen.push(p.clone());
        }
        keep
    });
    out
}

/// Everything discovery found, for `tw backends` and diagnostics.
#[derive(Clone, Debug)]
pub struct Diagnosis {
    /// Every library candidate, in order.
    pub candidates: Vec<LibraryCandidate>,
    /// The chosen library, or why none can be used.
    pub library: Result<LibraryChoice, String>,
    /// Host executables that could run it.
    pub hosts: Vec<PathBuf>,
}

impl Diagnosis {
    /// A readable, multi-line summary.
    pub fn summary(&self) -> String {
        let mut s = String::new();
        match &self.library {
            Ok(c) => s.push_str(&format!("Using {}.\n", c.reason)),
            Err(e) => s.push_str(&format!("Not available: {e}.\n")),
        }
        for c in &self.candidates {
            s.push_str(&format!(
                "  {} {} ({:?})\n",
                if c.exists { "found  " } else { "missing" },
                c.path.display(),
                c.source
            ));
        }
        match self.hosts.first() {
            Some(h) => s.push_str(&format!("Host: {}\n", h.display())),
            None => s.push_str("Host: not found (run `cargo xtask hosts`)\n"),
        }
        s
    }
}

/// Runs discovery for `config` on this machine.
pub fn diagnose(config: &DectalkConfig) -> Diagnosis {
    let candidates = library_candidates(config.library.as_deref(), &Places::current());
    let library = choose_library(&candidates);
    let arch = library.as_ref().ok().and_then(|c| c.arch);
    Diagnosis {
        hosts: host_candidates(config, arch),
        candidates,
        library,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pe(machine: u16) -> Vec<u8> {
        let mut b = vec![0u8; 0x100];
        b[..2].copy_from_slice(b"MZ");
        b[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        b[0x80..0x84].copy_from_slice(b"PE\0\0");
        b[0x84..0x86].copy_from_slice(&machine.to_le_bytes());
        b
    }

    fn elf(class: u8, machine: u16) -> Vec<u8> {
        let mut b = vec![0u8; 64];
        b[..4].copy_from_slice(b"\x7fELF");
        b[4] = class;
        b[5] = 1;
        b[18..20].copy_from_slice(&machine.to_le_bytes());
        b
    }

    #[test]
    fn reads_pe_and_elf_architectures() {
        assert_eq!(arch_from_header(&pe(0x014C)), Some(Arch::X86));
        assert_eq!(arch_from_header(&pe(0x8664)), Some(Arch::X64));
        assert_eq!(arch_from_header(&pe(0xAA64)), Some(Arch::Arm64));
        assert_eq!(arch_from_header(&pe(0x1234)), Some(Arch::Other));
        assert_eq!(arch_from_header(&elf(1, 3)), Some(Arch::X86));
        assert_eq!(arch_from_header(&elf(2, 62)), Some(Arch::X64));
        assert_eq!(arch_from_header(b"not a library"), None);
        assert_eq!(arch_from_header(b"MZ"), None);
        let mut bad = pe(0x014C);
        bad[0x3C..0x40].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(arch_from_header(&bad), None);
    }

    #[test]
    fn library_files_are_read_for_their_architecture_and_signature() {
        let dir = tempfile::tempdir().unwrap();
        let dll = dir.path().join("DECtalk.dll");
        let mut bytes = pe(0x014C);
        bytes.extend_from_slice(b"\0TextToSpeechStartupEx\0");
        std::fs::write(&dll, &bytes).unwrap();
        assert_eq!(library_arch(&dll), Some(Arch::X86));
        assert!(looks_like_dectalk(&dll));
        let other = dir.path().join("libtts.so");
        std::fs::write(&other, elf(2, 62)).unwrap();
        assert_eq!(library_arch(&other), Some(Arch::X64));
        assert!(!looks_like_dectalk(&other));
        assert!(!looks_like_dectalk(&dir.path().join("missing.dll")));
    }

    fn windows_places() -> Places {
        Places {
            env_library: Some(PathBuf::from(r"E:\mine\dectalk.dll")),
            program_files: Some(PathBuf::from(r"C:\Program Files")),
            program_files_x86: Some(PathBuf::from(r"C:\Program Files (x86)")),
            system_root: Some(PathBuf::from(r"C:\Windows")),
            windows: true,
        }
    }

    #[test]
    fn windows_order_is_env_option_program_files_then_system() {
        let c = library_candidates(Some(Path::new(r"D:\opt\DECtalk.dll")), &windows_places());
        let paths: Vec<PathBuf> = c.iter().map(|c| c.path.clone()).collect();
        let pf = Path::new(r"C:\Program Files");
        let pf86 = Path::new(r"C:\Program Files (x86)");
        let win = Path::new(r"C:\Windows");
        assert_eq!(
            paths,
            [
                PathBuf::from(r"E:\mine\dectalk.dll"),
                PathBuf::from(r"D:\opt\DECtalk.dll"),
                pf.join("DECtalk").join("DECtalk.dll"),
                pf.join("Fonix").join("DECtalk").join("DECtalk.dll"),
                pf.join("Fonix DECtalk").join("DECtalk.dll"),
                pf86.join("DECtalk").join("DECtalk.dll"),
                pf86.join("Fonix").join("DECtalk").join("DECtalk.dll"),
                pf86.join("Fonix DECtalk").join("DECtalk.dll"),
                win.join("System32").join("dectalk.dll"),
                win.join("SysWOW64").join("dectalk.dll"),
            ]
        );
        assert_eq!(c[0].source, Source::Environment);
        assert_eq!(c[1].source, Source::Option);
        assert!(c[2..].iter().all(|c| c.source == Source::Default));
    }

    #[test]
    fn linux_order_is_env_option_then_libtts() {
        let places = Places {
            env_library: None,
            ..Places::default()
        };
        let c = library_candidates(None, &places);
        assert_eq!(c[0].path, PathBuf::from("/usr/lib/libtts.so"));
        assert_eq!(c.len(), 5);
        let with_env = Places {
            env_library: Some(PathBuf::from("/usr/lib/libtts.so")),
            ..Places::default()
        };
        // A default the user also named is listed once, as theirs.
        let c = library_candidates(None, &with_env);
        assert_eq!(c.len(), 5);
        assert_eq!(c[0].source, Source::Environment);
    }

    fn cand(path: &str, source: Source) -> LibraryCandidate {
        LibraryCandidate {
            path: PathBuf::from(path),
            source,
            exists: true,
        }
    }

    #[test]
    fn the_first_dectalk_wins_and_other_libraries_are_passed_over() {
        let list = [
            LibraryCandidate {
                exists: false,
                ..cand("/gone/libtts.so", Source::Environment)
            },
            cand("/usr/lib/libtts.so", Source::Default),
            cand("/opt/dectalk/lib/libtts.so", Source::Default),
        ];
        let c = choose_with(&list, |p| p.starts_with("/opt"), |_| Some(Arch::X64)).unwrap();
        assert_eq!(
            c.candidate.path,
            PathBuf::from("/opt/dectalk/lib/libtts.so")
        );
        assert_eq!(c.arch, Some(Arch::X64));
        assert!(c.reason.contains("usual install location"), "{}", c.reason);
        assert!(c.reason.contains("x86-64"));
    }

    #[test]
    fn a_named_library_that_is_not_dectalk_is_an_error() {
        let list = [
            cand("/home/me/notdectalk.so", Source::Option),
            cand("/usr/lib/libtts.so", Source::Default),
        ];
        let e = choose_with(&list, |p| p.starts_with("/usr"), |_| None).unwrap_err();
        assert!(e.contains("does not look like a DECtalk library"), "{e}");
    }

    #[test]
    fn nothing_found_explains_where_it_looked_and_what_to_do() {
        let list = [LibraryCandidate {
            exists: false,
            ..cand("/usr/lib/libtts.so", Source::Default)
        }];
        let e = choose_with(&list, |_| true, |_| None).unwrap_err();
        assert!(e.contains("/usr/lib/libtts.so"), "{e}");
        assert!(e.contains(crate::LIBRARY_ENV), "{e}");
        let e = choose_with(&[], |_| true, |_| None).unwrap_err();
        assert!(e.contains("looked for nothing"), "{e}");
    }

    #[test]
    fn host_search_honours_the_option_and_skips_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        let host = dir.path().join("my-host");
        std::fs::write(&host, b"").unwrap();
        let cfg = DectalkConfig {
            host: Some(host.clone()),
            ..DectalkConfig::default()
        };
        let found = host_candidates(&cfg, Some(Arch::X64));
        assert_eq!(found.first(), Some(&host));
        let cfg = DectalkConfig {
            host: Some(dir.path().join("absent")),
            ..DectalkConfig::default()
        };
        assert!(host_candidates(&cfg, None).iter().all(|p| p.is_file()));
    }
}
