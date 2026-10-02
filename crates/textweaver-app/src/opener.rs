//! Opening a link or a file in the system's default program, safely
//! (W8a, the R4 review's first finding).
//!
//! Before, Windows opened every target through `cmd /C start`, so a link
//! whose address held `&` (or another `cmd` metacharacter) could run a
//! command after one "y", and any URL scheme was passed on, including
//! `ms-msdt:`. Now:
//!
//! - Only `http:`, `https:` and `mailto:` addresses and files that exist
//!   (a path, or a `file:` address) are opened ([`classify`]). Anything
//!   else is refused, and the reader hears why in words.
//! - No shell ever reads the target. Windows calls `ShellExecuteW`; macOS
//!   runs `open` and Linux `xdg-open` with the target as one argument
//!   ([`system_open`]). A file is passed as an absolute path, so it can
//!   never be read as an option.

use std::path::{Path, PathBuf};

/// What a target is, once it is allowed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenTarget {
    /// An `http:` or `https:` address, as written.
    Web(String),
    /// A `mailto:` address, as written.
    Mail(String),
    /// A file or folder that exists, as an absolute path.
    File(PathBuf),
}

impl OpenTarget {
    /// The text handed to the system: the address, or the file's path.
    pub fn as_text(&self) -> String {
        match self {
            OpenTarget::Web(s) | OpenTarget::Mail(s) => s.clone(),
            OpenTarget::File(p) => p.display().to_string(),
        }
    }
}

/// Why a target was not opened.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Refused {
    /// An address whose scheme is not allowed (`ms-msdt`, `javascript`).
    #[error("{0} links are not opened")]
    Scheme(String),
    /// A file that is not there.
    #[error("the file was not found")]
    Missing,
    /// Empty, or with control characters in it.
    #[error("not a valid address")]
    Invalid,
}

/// The scheme of `target` (`https`), lowercase, when it has one. A drive
/// letter (`C:\notes`) is not a scheme.
pub fn scheme_of(target: &str) -> Option<String> {
    let (scheme, _) = target.split_once(':')?;
    let ok = scheme.len() > 1
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c));
    ok.then(|| scheme.to_ascii_lowercase())
}

/// Decides whether `target` may be opened, and as what. A relative path
/// is taken from the current folder.
pub fn classify(target: &str) -> Result<OpenTarget, Refused> {
    let target = target.trim();
    if target.is_empty() || target.chars().any(char::is_control) {
        return Err(Refused::Invalid);
    }
    match scheme_of(target).as_deref() {
        Some("http" | "https") => Ok(OpenTarget::Web(target.to_owned())),
        Some("mailto") => Ok(OpenTarget::Mail(target.to_owned())),
        Some("file") => {
            let path = target
                .trim_start_matches("file://")
                .trim_start_matches("file:");
            existing(Path::new(path))
        }
        Some(other) => Err(Refused::Scheme(other.to_owned())),
        None => existing(Path::new(target)),
    }
}

/// `path` as an absolute path, when it exists.
fn existing(path: &Path) -> Result<OpenTarget, Refused> {
    if !path.exists() {
        return Err(Refused::Missing);
    }
    let absolute = std::path::absolute(path).map_err(|_| Refused::Missing)?;
    Ok(OpenTarget::File(absolute))
}

/// How the system opens a target: what [`open_with_system`] runs. No
/// shell is ever part of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SystemOpen {
    /// Windows: `ShellExecuteW` with the verb "open" on this text.
    ShellExecute(String),
    /// macOS and Linux: this program, with these arguments.
    Program {
        /// `open` or `xdg-open`.
        program: &'static str,
        /// The target, as one argument.
        args: Vec<String>,
    },
}

/// What this system runs to open `target`.
pub fn system_open(target: &OpenTarget) -> SystemOpen {
    let text = target.as_text();
    if cfg!(target_os = "windows") {
        SystemOpen::ShellExecute(text)
    } else if cfg!(target_os = "macos") {
        SystemOpen::Program {
            program: "open",
            args: vec![text],
        }
    } else {
        SystemOpen::Program {
            program: "xdg-open",
            args: vec![text],
        }
    }
}

/// Opens `target` (a file path or an address) with the default program,
/// after [`classify`] allows it. A refused target is an error of kind
/// `InvalidInput` that wraps the [`Refused`] reason.
pub fn open_with_system(target: &str) -> std::io::Result<()> {
    let allowed =
        classify(target).map_err(|r| std::io::Error::new(std::io::ErrorKind::InvalidInput, r))?;
    run(&system_open(&allowed))
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn run(how: &SystemOpen) -> std::io::Result<()> {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::{HSTRING, PCWSTR, w};
    let SystemOpen::ShellExecute(text) = how else {
        return Err(std::io::Error::other("not a Windows opener"));
    };
    let file = HSTRING::from(text.as_str());
    // SAFETY: every pointer is a valid, NUL-terminated wide string that
    // outlives the call, or null; no window handle is passed.
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            &file,
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // Values above 32 mean success.
    if result.0 as isize > 32 {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "the system could not open it (code {})",
            result.0 as isize
        )))
    }
}

#[cfg(not(windows))]
fn run(how: &SystemOpen) -> std::io::Result<()> {
    use std::process::{Command, Stdio};
    let SystemOpen::Program { program, args } = how else {
        return Err(std::io::Error::other("not a program opener"));
    };
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_and_mail_addresses_are_allowed_as_written() {
        let url = "https://example.com/a?b=1&calc";
        assert_eq!(classify(url), Ok(OpenTarget::Web(url.to_owned())));
        assert_eq!(
            classify("HTTP://example.com"),
            Ok(OpenTarget::Web("HTTP://example.com".into()))
        );
        assert_eq!(
            classify("mailto:ada@example.com"),
            Ok(OpenTarget::Mail("mailto:ada@example.com".into()))
        );
    }

    #[test]
    fn other_schemes_are_refused() {
        for (t, s) in [
            ("ms-msdt:/id PCWDiagnostic", "ms-msdt"),
            ("javascript:alert(1)", "javascript"),
            ("search-ms:query=x", "search-ms"),
            ("MS-SETTINGS:", "ms-settings"),
        ] {
            assert_eq!(classify(t), Err(Refused::Scheme(s.into())), "{t}");
        }
    }

    #[test]
    fn a_shell_metacharacter_is_not_a_file() {
        assert_eq!(classify("notes.md&calc"), Err(Refused::Missing));
        assert_eq!(classify("x & calc.exe"), Err(Refused::Missing));
        assert_eq!(classify("a\nb"), Err(Refused::Invalid));
        assert_eq!(classify("   "), Err(Refused::Invalid));
    }

    #[test]
    fn files_that_exist_open_by_absolute_path() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("report.html");
        std::fs::write(&file, "x").unwrap();
        let t = file.display().to_string();
        assert_eq!(classify(&t), Ok(OpenTarget::File(file.clone())));
        let missing = dir.path().join("gone.html");
        assert_eq!(
            classify(&missing.display().to_string()),
            Err(Refused::Missing)
        );
        // A drive letter is not a scheme.
        assert_eq!(scheme_of(r"C:\notes"), None);
        assert_eq!(scheme_of("https://x"), Some("https".into()));
    }

    #[test]
    fn the_system_opener_never_uses_a_shell() {
        let t = OpenTarget::Web("https://example.com/?a=1&calc".into());
        match system_open(&t) {
            SystemOpen::ShellExecute(text) => {
                assert!(cfg!(windows));
                assert_eq!(text, "https://example.com/?a=1&calc");
            }
            SystemOpen::Program { program, args } => {
                assert!(!cfg!(windows));
                assert!(program == "open" || program == "xdg-open");
                assert_eq!(args, ["https://example.com/?a=1&calc"]);
            }
        }
    }

    #[test]
    fn a_refused_target_is_never_run() {
        let e = open_with_system("ms-msdt:/id x").unwrap_err();
        assert_eq!(e.kind(), std::io::ErrorKind::InvalidInput);
        let e = open_with_system("nothing-here.md&calc").unwrap_err();
        assert_eq!(e.kind(), std::io::ErrorKind::InvalidInput);
    }
}
