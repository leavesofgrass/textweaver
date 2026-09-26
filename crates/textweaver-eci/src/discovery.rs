//! Finding the ECI library and the matching host executable.
//!
//! textweaver ships no engine: it uses an ECI library the user installed.
//! Candidates, in order (the first existing file wins):
//!
//! | Where | Product |
//! |---|---|
//! | `TEXTWEAVER_ECI_LIBRARY` | detected from the path |
//! | [`EciConfig::library`](crate::EciConfig::library) (the backend option) | detected from the path |
//! | Windows: `%ProgramFiles%\OpenEVV\lib\x86_64\eci.dll` | OpenEVV (SAPI5 install) |
//! | Windows: `%APPDATA%\nvda\addons\openevv\synthDrivers\_openevv\lib\x86_64\eci.dll` | OpenEVV (NVDA add-on) |
//! | Windows: `C:\Program Files (x86)\Code Factory\Eloquence for Windows\eci.dll` | Code Factory Eloquence |
//! | Linux: `/opt/oralux/voxin/lib/libibmeci.so`, `/usr/lib/libibmeci.so`, `/opt/IBM/ibmtts/lib/libibmeci.so` | Voxin |
//!
//! No other location is searched (in particular, not other NVDA add-ons).
//!
//! **Architecture.** The library's own header decides which host runs it
//! ([`library_arch`]: PE `Machine` on Windows, ELF class on Linux): a
//! 32-bit x86 DLL runs in `textweaver-eci-host-x86.exe` (built for
//! `i686-pc-windows-msvc`), a 64-bit one in the native
//! `textweaver-eci-host[.exe]`.
//!
//! **OpenEVV** (a reimplementation of IBM's ECI API) is recognised by its
//! install path (an `openevv` path component), or by an `eciVersion`
//! string mentioning OpenEVV once the host reports it. Its 32-bit `eci.dll`
//! exports `cdecl` functions, not IBM's `stdcall`, so textweaver supports
//! OpenEVV only through its x86_64 library (as OpenEVV's own NVDA add-on
//! does) and refuses a 32-bit OpenEVV library with an explanation rather
//! than calling it with the wrong convention.

use std::path::{Path, PathBuf};

use crate::EciConfig;

/// Which product an ECI library belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Product {
    /// OpenEVV (github.com/Mudb0y/openevv), user-installed.
    OpenEvv,
    /// Code Factory "Eloquence for Windows".
    CodeFactory,
    /// Voxin (ETI-Eloquence for Linux).
    Voxin,
    /// A library the user named that textweaver does not recognise.
    Unknown,
}

impl Product {
    /// Human-readable name.
    pub fn name(self) -> &'static str {
        match self {
            Product::OpenEvv => "OpenEVV",
            Product::CodeFactory => "Code Factory Eloquence",
            Product::Voxin => "Voxin",
            Product::Unknown => "ECI library",
        }
    }

    /// The product a library path belongs to, judged by the path.
    pub fn from_path(path: &Path) -> Product {
        let s = path
            .to_string_lossy()
            .to_ascii_lowercase()
            .replace('\\', "/");
        if s.split('/').any(|c| c == "openevv" || c == "_openevv") {
            Product::OpenEvv
        } else if s.contains("code factory") {
            Product::CodeFactory
        } else if s.contains("voxin") || s.ends_with("libibmeci.so") {
            Product::Voxin
        } else {
            Product::Unknown
        }
    }

    /// Refines a path-based guess with the version string the engine
    /// reported.
    pub fn with_version(self, version: &str) -> Product {
        if version.to_ascii_lowercase().contains("openevv") {
            Product::OpenEvv
        } else {
            self
        }
    }
}

/// Why a candidate is on the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// `TEXTWEAVER_ECI_LIBRARY`.
    Environment,
    /// The backend option.
    Option,
    /// A standard install location.
    Default,
}

/// One place the library might be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryCandidate {
    /// The file.
    pub path: PathBuf,
    /// The product it would be.
    pub product: Product,
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

/// Where the standard install locations live (injectable for tests).
#[derive(Clone, Debug, Default)]
pub struct Places {
    /// `TEXTWEAVER_ECI_LIBRARY`.
    pub env_library: Option<PathBuf>,
    /// `%ProgramFiles%`.
    pub program_files: Option<PathBuf>,
    /// `%APPDATA%`.
    pub appdata: Option<PathBuf>,
    /// Whether to list the Windows locations (else the Linux ones).
    pub windows: bool,
    /// Whether to search Code Factory's default location. Off unless
    /// `TEXTWEAVER_ECI_CODE_FACTORY=1`: its presence on a machine does not
    /// mean it is licensed there, so it is loaded only when the user says
    /// so (or names it in `TEXTWEAVER_ECI_LIBRARY` or the backend option).
    pub code_factory: bool,
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
            appdata: var("APPDATA"),
            windows: cfg!(windows),
            code_factory: var(crate::CODE_FACTORY_ENV).is_some_and(|v| v.as_os_str() == "1"),
        }
    }
}

fn join(base: &Path, parts: &[&str]) -> PathBuf {
    parts.iter().fold(base.to_path_buf(), |p, c| p.join(c))
}

/// Every candidate, in order, existing or not.
pub fn library_candidates(option: Option<&Path>, places: &Places) -> Vec<LibraryCandidate> {
    let mut out = Vec::new();
    let mut push = |path: PathBuf, source: Source, product: Option<Product>| {
        let product = product.unwrap_or_else(|| Product::from_path(&path));
        let exists = path.is_file();
        out.push(LibraryCandidate {
            path,
            product,
            source,
            exists,
        });
    };
    if let Some(p) = &places.env_library {
        push(p.clone(), Source::Environment, None);
    }
    if let Some(p) = option {
        push(p.to_path_buf(), Source::Option, None);
    }
    if places.windows {
        if let Some(pf) = &places.program_files {
            push(
                join(pf, &["OpenEVV", "lib", "x86_64", "eci.dll"]),
                Source::Default,
                Some(Product::OpenEvv),
            );
        }
        if let Some(ad) = &places.appdata {
            push(
                join(
                    ad,
                    &[
                        "nvda",
                        "addons",
                        "openevv",
                        "synthDrivers",
                        "_openevv",
                        "lib",
                        "x86_64",
                        "eci.dll",
                    ],
                ),
                Source::Default,
                Some(Product::OpenEvv),
            );
        }
        if places.code_factory {
            push(
                PathBuf::from(r"C:\Program Files (x86)\Code Factory\Eloquence for Windows\eci.dll"),
                Source::Default,
                Some(Product::CodeFactory),
            );
        }
    } else {
        for p in [
            "/opt/oralux/voxin/lib/libibmeci.so",
            "/usr/lib/libibmeci.so",
            "/opt/IBM/ibmtts/lib/libibmeci.so",
        ] {
            push(PathBuf::from(p), Source::Default, Some(Product::Voxin));
        }
    }
    out
}

/// The architecture in a PE (Windows) or ELF (Linux) header, from the
/// file's first bytes.
pub fn arch_from_header(bytes: &[u8]) -> Option<Arch> {
    if bytes.starts_with(b"MZ") {
        let at = u32::from_le_bytes(bytes.get(0x3C..0x40)?.try_into().ok()?) as usize;
        if bytes.get(at..at + 4)? != b"PE\0\0" {
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
    let mut f = std::fs::File::open(path).ok()?;
    let mut head = vec![0u8; 4096];
    let n = f.read(&mut head).ok()?;
    head.truncate(n);
    if let Some(a) = arch_from_header(&head) {
        return Some(a);
    }
    // A PE header beyond the first 4 KiB: read up to it.
    if head.starts_with(b"MZ") && head.len() >= 0x40 {
        let at = u32::from_le_bytes(head[0x3C..0x40].try_into().ok()?) as usize;
        if at < 16 * 1024 * 1024 {
            let bytes = std::fs::read(path).ok()?;
            return arch_from_header(bytes.get(..at + 6)?);
        }
    }
    None
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

/// Picks the first existing candidate; refuses a 32-bit OpenEVV library.
pub fn choose_library(candidates: &[LibraryCandidate]) -> Result<LibraryChoice, String> {
    let Some(c) = candidates.iter().find(|c| c.exists) else {
        let tried: Vec<String> = candidates
            .iter()
            .map(|c| c.path.display().to_string())
            .collect();
        return Err(format!(
            "no ECI library found (tried {}); see `tw eloquence` for how to get Eloquence, or set {}",
            tried.join(", "),
            crate::LIBRARY_ENV
        ));
    };
    let arch = library_arch(&c.path);
    if c.product == Product::OpenEvv && arch == Some(Arch::X86) {
        return Err(format!(
            "{} is OpenEVV's 32-bit eci.dll, which uses the cdecl calling convention instead of \
             IBM's stdcall; textweaver supports OpenEVV only through its x86_64 library \
             (lib\\x86_64\\eci.dll)",
            c.path.display()
        ));
    }
    let why = match c.source {
        Source::Environment => format!("named by {}", crate::LIBRARY_ENV),
        Source::Option => "named in the backend settings".to_string(),
        Source::Default => "found at its standard install location".to_string(),
    };
    let arch_text = match arch {
        Some(Arch::X86) => "32-bit x86",
        Some(Arch::X64) => "x86-64",
        Some(Arch::Arm64) => "ARM64",
        Some(Arch::Other) => "an unsupported architecture",
        None => "unknown architecture",
    };
    Ok(LibraryChoice {
        reason: format!(
            "{} at {}, {why} ({arch_text})",
            c.product.name(),
            c.path.display()
        ),
        candidate: c.clone(),
        arch,
    })
}

/// The native host's file name.
pub const HOST_NAME: &str = if cfg!(windows) {
    "textweaver-eci-host.exe"
} else {
    "textweaver-eci-host"
};

/// The 32-bit Windows host's file name (`cargo xtask hosts` installs it
/// next to the binaries).
pub const HOST_NAME_X86: &str = "textweaver-eci-host-x86.exe";

/// Host executables that can run a library of architecture `arch` (`None`:
/// any), in order: [`EciConfig::host`], `TEXTWEAVER_ECI_HOST`, beside the
/// running executable (and its parent directory, for test binaries in
/// `deps/`), and on Windows a cargo `i686-pc-windows-msvc` build under the
/// target directory. Only existing files are returned.
pub fn host_candidates(config: &EciConfig, arch: Option<Arch>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    if let Some(h) = &config.host {
        out.push(h.clone());
    }
    if let Some(h) = std::env::var_os(crate::HOST_ENV).filter(|v| !v.is_empty()) {
        out.push(PathBuf::from(h));
    }
    let want_x86 = cfg!(windows) && !config.fake_engine && arch != Some(Arch::X64);
    let want_native = config.fake_engine || !cfg!(windows) || arch != Some(Arch::X86);
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
        if want_x86 {
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
                "  {} {} ({}, {:?})\n",
                if c.exists { "found  " } else { "missing" },
                c.path.display(),
                c.product.name(),
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
pub fn diagnose(config: &EciConfig) -> Diagnosis {
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

    /// A minimal PE file: DOS header pointing at a PE header with `machine`.
    fn pe(machine: u16) -> Vec<u8> {
        let mut b = vec![0u8; 0x80 + 24];
        b[0] = b'M';
        b[1] = b'Z';
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
        assert_eq!(arch_from_header(&elf(2, 62)), Some(Arch::X64));
        assert_eq!(arch_from_header(&elf(1, 3)), Some(Arch::X86));
        assert_eq!(arch_from_header(b"MZ"), None);
        assert_eq!(arch_from_header(b"not a library"), None);
        let mut bad = pe(0x8664);
        bad[0x80] = b'X';
        assert_eq!(arch_from_header(&bad), None);
    }

    #[test]
    fn products_are_recognised_by_path() {
        assert_eq!(
            Product::from_path(Path::new(r"C:\Program Files\OpenEVV\lib\x86_64\eci.dll")),
            Product::OpenEvv
        );
        assert_eq!(
            Product::from_path(Path::new(
                r"C:\Users\j\AppData\Roaming\nvda\addons\openevv\synthDrivers\_openevv\lib\x86_64\eci.dll"
            )),
            Product::OpenEvv
        );
        assert_eq!(
            Product::from_path(Path::new(
                r"C:\Program Files (x86)\Code Factory\Eloquence for Windows\eci.dll"
            )),
            Product::CodeFactory
        );
        assert_eq!(
            Product::from_path(Path::new("/opt/oralux/voxin/lib/libibmeci.so")),
            Product::Voxin
        );
        // An unrelated NVDA add-on named Eloquence is not OpenEVV.
        assert_eq!(
            Product::from_path(Path::new(
                r"C:\Users\j\AppData\Roaming\nvda\addons\Eloquence\synthDrivers\eci.dll"
            )),
            Product::Unknown
        );
        assert_eq!(
            Product::Unknown.with_version("OpenEVV 0.3"),
            Product::OpenEvv
        );
        assert_eq!(Product::Voxin.with_version("1.5.8"), Product::Voxin);
    }

    #[test]
    fn windows_order_is_env_option_openevv_then_code_factory() {
        let dir = tempfile::tempdir().unwrap();
        let places = Places {
            env_library: Some(dir.path().join("env.dll")),
            program_files: Some(dir.path().join("pf")),
            appdata: Some(dir.path().join("ad")),
            windows: true,
            code_factory: true,
        };
        let opt = dir.path().join("opt.dll");
        let c = library_candidates(Some(&opt), &places);
        let sources: Vec<Source> = c.iter().map(|c| c.source).collect();
        assert_eq!(
            sources,
            [
                Source::Environment,
                Source::Option,
                Source::Default,
                Source::Default,
                Source::Default
            ]
        );
        assert!(
            c[2].path
                .ends_with(join(Path::new("OpenEVV"), &["lib", "x86_64", "eci.dll"]))
        );
        assert_eq!(c[2].product, Product::OpenEvv);
        assert!(
            c[3].path
                .ends_with(join(Path::new("_openevv"), &["lib", "x86_64", "eci.dll"]))
        );
        assert_eq!(c[4].product, Product::CodeFactory);
        assert!(
            !c.iter()
                .any(|c| c.path.to_string_lossy().contains("Eloquence\\synth"))
        );
    }

    #[test]
    fn code_factory_is_searched_only_when_enabled() {
        let dir = tempfile::tempdir().unwrap();
        let places = Places {
            env_library: None,
            program_files: Some(dir.path().join("pf")),
            appdata: Some(dir.path().join("ad")),
            windows: true,
            code_factory: false,
        };
        let c = library_candidates(None, &places);
        assert!(c.iter().all(|c| c.product != Product::CodeFactory));
        let c = library_candidates(
            None,
            &Places {
                code_factory: true,
                ..places
            },
        );
        assert!(c.iter().any(|c| c.product == Product::CodeFactory));
    }

    #[test]
    fn linux_order_is_env_option_then_voxin() {
        let places = Places {
            env_library: Some("/x/libibmeci.so".into()),
            windows: false,
            code_factory: true,
            ..Places::default()
        };
        let c = library_candidates(None, &places);
        let paths: Vec<String> = c.iter().map(|c| c.path.display().to_string()).collect();
        assert_eq!(
            paths,
            [
                "/x/libibmeci.so",
                "/opt/oralux/voxin/lib/libibmeci.so",
                "/usr/lib/libibmeci.so",
                "/opt/IBM/ibmtts/lib/libibmeci.so"
            ]
        );
    }

    #[test]
    fn the_first_existing_candidate_wins_with_its_architecture() {
        let dir = tempfile::tempdir().unwrap();
        let pf = dir.path().join("pf");
        let evv = join(&pf, &["OpenEVV", "lib", "x86_64"]);
        std::fs::create_dir_all(&evv).unwrap();
        std::fs::write(evv.join("eci.dll"), pe(0x8664)).unwrap();
        let places = Places {
            env_library: Some(dir.path().join("missing.dll")),
            program_files: Some(pf),
            appdata: None,
            windows: true,
            code_factory: true,
        };
        let c = library_candidates(None, &places);
        let choice = choose_library(&c).unwrap();
        assert_eq!(choice.candidate.product, Product::OpenEvv);
        assert_eq!(choice.arch, Some(Arch::X64));
        assert!(
            choice.reason.starts_with("OpenEVV at "),
            "{}",
            choice.reason
        );
        assert!(choice.reason.contains("x86-64"));
    }

    #[test]
    fn a_32_bit_openevv_library_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path().join("openevv").join("eci.dll");
        std::fs::create_dir_all(lib.parent().unwrap()).unwrap();
        std::fs::write(&lib, pe(0x014C)).unwrap();
        let places = Places {
            env_library: Some(lib),
            windows: true,
            code_factory: true,
            ..Places::default()
        };
        let err = choose_library(&library_candidates(None, &places)).unwrap_err();
        assert!(err.contains("cdecl"), "{err}");
        // A 32-bit library from another product is fine (it gets the x86 host).
        let cf = dir.path().join("Code Factory").join("eci.dll");
        std::fs::create_dir_all(cf.parent().unwrap()).unwrap();
        std::fs::write(&cf, pe(0x014C)).unwrap();
        let places = Places {
            env_library: Some(cf),
            windows: true,
            code_factory: true,
            ..Places::default()
        };
        let ok = choose_library(&library_candidates(None, &places)).unwrap();
        assert_eq!(ok.arch, Some(Arch::X86));
    }

    #[test]
    fn nothing_found_explains_where_it_looked() {
        let missing = LibraryCandidate {
            path: "/nope/libibmeci.so".into(),
            product: Product::Voxin,
            source: Source::Environment,
            exists: false,
        };
        let err = choose_library(&[missing]).unwrap_err();
        assert!(err.contains("/nope/libibmeci.so"), "{err}");
    }
}
