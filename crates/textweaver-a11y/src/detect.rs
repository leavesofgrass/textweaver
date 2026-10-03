//! Is a screen reader running? Checked once at startup, so textweaver can
//! offer hybrid mode on its first run with one.
//!
//! - **Windows**: the system screen reader flag
//!   (`SystemParametersInfo(SPI_GETSCREENREADER)`, which NVDA, JAWS,
//!   Narrator, and most others set), and the process list for `nvda.exe`,
//!   `jfw.exe` (JAWS), `narrator.exe`, and ZoomText or Fusion, which also
//!   names the screen reader.
//! - **macOS**: `defaults read com.apple.universalaccess voiceOverOnOffKey`.
//! - **Linux**: GNOME's `screen-reader-enabled` setting (`gsettings`), the
//!   AT-SPI bus's `ScreenReaderEnabled` property (`busctl`), and an `orca`
//!   process.
//!
//! Every check is read-only and gives up quickly; a check that fails counts
//! as "no". `TEXTWEAVER_SCREEN_READER` overrides the result: `0` or `off`
//! for none, `1` or `on` for an unnamed one, any other text as its name.

use std::time::Duration;

/// A running screen reader, as far as textweaver can tell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Detected {
    /// Its name ("NVDA", "JAWS", "VoiceOver", "Orca"), when known.
    pub name: Option<String>,
}

impl Detected {
    /// "NVDA" or "A screen reader", for the question at startup.
    pub fn spoken_name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| "A screen reader".to_owned())
    }
}

/// The screen reader a process belongs to, from its executable name.
pub fn screen_reader_for_process(exe: &str) -> Option<&'static str> {
    let exe = exe.trim().to_ascii_lowercase();
    let stem = exe.strip_suffix(".exe").unwrap_or(&exe);
    match stem {
        "nvda" => Some("NVDA"),
        "jfw" => Some("JAWS"),
        "narrator" => Some("Narrator"),
        "zoomtext" | "zt" | "fusion" => Some("ZoomText"),
        "orca" => Some("Orca"),
        _ => None,
    }
}

/// The first screen reader among process names.
pub fn from_processes<'a>(names: impl IntoIterator<Item = &'a str>) -> Option<&'static str> {
    names.into_iter().find_map(screen_reader_for_process)
}

/// `defaults read com.apple.universalaccess voiceOverOnOffKey` prints `1`
/// while VoiceOver is on.
pub fn parse_macos_voiceover(output: &str) -> bool {
    output.trim() == "1"
}

/// `gsettings get org.gnome.desktop.a11y.applications
/// screen-reader-enabled` prints `true` or `false`; `busctl --user
/// get-property org.a11y.Bus /org/a11y/bus org.a11y.Status
/// ScreenReaderEnabled` prints `b true` or `b false`.
pub fn parse_linux_flag(output: &str) -> bool {
    let t = output.trim();
    t == "true" || t == "b true"
}

/// What `TEXTWEAVER_SCREEN_READER` says: `Some(None)` for "no screen
/// reader", `Some(Some(d))` for one, `None` when unset or empty.
pub fn from_env_value(value: Option<&str>) -> Option<Option<Detected>> {
    let v = value?.trim();
    if v.is_empty() {
        return None;
    }
    Some(match v.to_ascii_lowercase().as_str() {
        "0" | "off" | "no" | "false" | "none" => None,
        "1" | "on" | "yes" | "true" => Some(Detected { name: None }),
        _ => Some(Detected {
            name: Some(v.to_owned()),
        }),
    })
}

/// Checks whether a screen reader is running. Takes a few milliseconds on
/// Windows and up to about half a second elsewhere (it may start one or two
/// small programs); call it once at startup.
pub fn detect() -> Option<Detected> {
    if let Some(forced) = from_env_value(std::env::var("TEXTWEAVER_SCREEN_READER").ok().as_deref())
    {
        return forced;
    }
    platform::detect()
}

/// Runs a command with a time limit and no window; its stdout on success
/// ([`textweaver_core::process::run_with_timeout`]).
#[cfg(not(windows))]
fn run(program: &str, args: &[&str], limit: Duration) -> Option<String> {
    textweaver_core::process::run_with_timeout(std::path::Path::new(program), args, limit)
}

/// How long one probe program may take.
pub const PROBE_LIMIT: Duration = Duration::from_millis(400);

#[cfg(windows)]
#[allow(unsafe_code)]
mod platform {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        SPI_GETSCREENREADER, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
    };
    use windows::core::BOOL;

    use super::Detected;

    /// The system-wide "a screen reader is running" flag.
    fn system_flag() -> bool {
        let mut on = BOOL(0);
        // SAFETY: SPI_GETSCREENREADER writes one BOOL to the pointer, which
        // points at a live local of that type.
        let ok = unsafe {
            SystemParametersInfoW(
                SPI_GETSCREENREADER,
                0,
                Some(std::ptr::from_mut(&mut on).cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )
        };
        ok.is_ok() && on.as_bool()
    }

    /// Closes the snapshot handle however the walk ends.
    struct Snapshot(HANDLE);

    impl Drop for Snapshot {
        fn drop(&mut self) {
            // SAFETY: the handle came from CreateToolhelp32Snapshot and is
            // closed exactly once, here.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }

    /// The executable names of running processes (read-only; a snapshot of
    /// the process list, nothing is opened).
    fn process_names() -> Vec<String> {
        // SAFETY: a process snapshot takes no pointers.
        let Ok(handle) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
            return Vec::new();
        };
        let snap = Snapshot(handle);
        let mut entry = PROCESSENTRY32W {
            dwSize: u32::try_from(std::mem::size_of::<PROCESSENTRY32W>()).unwrap_or(0),
            ..Default::default()
        };
        let mut names = Vec::new();
        // SAFETY: `entry` is a live PROCESSENTRY32W with dwSize set, as the
        // walk functions require; the snapshot handle is open.
        let mut more = unsafe { Process32FirstW(snap.0, &mut entry) }.is_ok();
        while more {
            let len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            names.push(String::from_utf16_lossy(&entry.szExeFile[..len]));
            // SAFETY: as above.
            more = unsafe { Process32NextW(snap.0, &mut entry) }.is_ok();
        }
        names
    }

    pub(super) fn detect() -> Option<Detected> {
        let names = process_names();
        let named = super::from_processes(names.iter().map(String::as_str));
        if named.is_some() || system_flag() {
            Some(Detected {
                name: named.map(str::to_owned),
            })
        } else {
            None
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{Detected, PROBE_LIMIT, parse_macos_voiceover, run};

    pub(super) fn detect() -> Option<Detected> {
        let out = run(
            "defaults",
            &["read", "com.apple.universalaccess", "voiceOverOnOffKey"],
            PROBE_LIMIT,
        )?;
        parse_macos_voiceover(&out).then(|| Detected {
            name: Some("VoiceOver".to_owned()),
        })
    }
}

#[cfg(all(not(windows), not(target_os = "macos")))]
mod platform {
    use super::{Detected, PROBE_LIMIT, parse_linux_flag, run};

    /// An `orca` process, from `/proc/*/comm` (Linux only; elsewhere empty).
    fn orca_running() -> bool {
        let Ok(dir) = std::fs::read_dir("/proc") else {
            return false;
        };
        dir.filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .bytes()
                    .all(|b| b.is_ascii_digit())
            })
            .any(|e| {
                std::fs::read_to_string(e.path().join("comm")).is_ok_and(|c| c.trim() == "orca")
            })
    }

    pub(super) fn detect() -> Option<Detected> {
        let orca = || {
            Some(Detected {
                name: Some("Orca".to_owned()),
            })
        };
        if orca_running() {
            return orca();
        }
        let gnome = run(
            "gsettings",
            &[
                "get",
                "org.gnome.desktop.a11y.applications",
                "screen-reader-enabled",
            ],
            PROBE_LIMIT,
        );
        if gnome.as_deref().is_some_and(parse_linux_flag) {
            return orca();
        }
        let atspi = run(
            "busctl",
            &[
                "--user",
                "get-property",
                "org.a11y.Bus",
                "/org/a11y/bus",
                "org.a11y.Status",
                "ScreenReaderEnabled",
            ],
            PROBE_LIMIT,
        );
        atspi
            .as_deref()
            .is_some_and(parse_linux_flag)
            .then_some(Detected { name: None })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_names_map_to_screen_readers() {
        assert_eq!(screen_reader_for_process("nvda.exe"), Some("NVDA"));
        assert_eq!(screen_reader_for_process("JFW.EXE"), Some("JAWS"));
        assert_eq!(screen_reader_for_process("Narrator.exe"), Some("Narrator"));
        assert_eq!(screen_reader_for_process("orca"), Some("Orca"));
        assert_eq!(screen_reader_for_process("nvda_slave.exe"), None);
        assert_eq!(screen_reader_for_process("explorer.exe"), None);
        assert_eq!(
            from_processes(["System", "explorer.exe", "jfw.exe", "nvda.exe"]),
            Some("JAWS")
        );
        assert_eq!(from_processes(["System"]), None);
    }

    #[test]
    fn probe_output_is_parsed() {
        assert!(parse_macos_voiceover("1\n"));
        assert!(!parse_macos_voiceover("0\n"));
        assert!(!parse_macos_voiceover(""));
        assert!(parse_linux_flag("true\n"));
        assert!(parse_linux_flag("b true\n"));
        assert!(!parse_linux_flag("b false"));
        assert!(!parse_linux_flag("false"));
    }

    #[test]
    fn the_environment_can_override() {
        assert_eq!(from_env_value(None), None);
        assert_eq!(from_env_value(Some(" ")), None);
        assert_eq!(from_env_value(Some("0")), Some(None));
        assert_eq!(from_env_value(Some("off")), Some(None));
        assert_eq!(
            from_env_value(Some("1")),
            Some(Some(Detected { name: None }))
        );
        let named = from_env_value(Some("JAWS")).unwrap().unwrap();
        assert_eq!(named.spoken_name(), "JAWS");
        assert_eq!(Detected { name: None }.spoken_name(), "A screen reader");
    }

    /// The real probe is read-only and never panics; its answer depends on
    /// the machine, so it is not checked.
    #[test]
    fn detection_never_panics() {
        let _ = platform::detect();
    }
}
