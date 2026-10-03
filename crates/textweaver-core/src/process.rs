//! Running other programs: one rule for finding them, starting them and
//! reading what they print.
//!
//! - [`find_program`] looks a program up on `PATH` with one Windows rule:
//!   the `PATHEXT` extensions first (`.exe`, `.cmd`, ...), then the bare
//!   name, because a bare `whisper` beside `whisper.exe` is usually a shell
//!   script Windows cannot start. Elsewhere the bare name only.
//! - [`command`] makes a [`Command`] that never flashes a console window
//!   when started from the window (Windows `CREATE_NO_WINDOW`); a screen
//!   reader announces such a window as a focus change. [`hide_window`] does
//!   the same to a command built elsewhere.
//! - [`run_with_timeout`] runs a short probe and gives up after a limit.
//! - [`decode_output`] turns a child's output into text with one explicit
//!   rule (see its docs), instead of an implicit `from_utf8_lossy` at each
//!   call.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Windows' `CREATE_NO_WINDOW` process creation flag.
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// The extensions Windows tries when `PATHEXT` is not set.
const DEFAULT_PATHEXT: &str = ".COM;.EXE;.BAT;.CMD";

/// `name` on the `PATH` of this process (see [`find_program_in`]).
pub fn find_program(name: &str) -> Option<PathBuf> {
    find_program_in(name, &std::env::var_os("PATH")?)
}

/// `name` in the folders of `path_var` (a `PATH`-style list), in order.
/// On Windows each folder is tried with the extensions in `PATHEXT` first,
/// in lower case, then with the bare name; a name that already ends in one
/// of those extensions is tried as it is first. Elsewhere only the bare
/// name is tried.
pub fn find_program_in(name: &str, path_var: &OsStr) -> Option<PathBuf> {
    let pathext = std::env::var("PATHEXT").ok();
    let candidates = candidate_names(name, cfg!(windows), pathext.as_deref());
    std::env::split_paths(path_var)
        .filter(|dir| !dir.as_os_str().is_empty())
        .flat_map(|dir| candidates.iter().map(move |n| dir.join(n)))
        .find(|p| p.is_file())
}

/// The file names [`find_program_in`] tries in each folder, in order.
fn candidate_names(name: &str, windows: bool, pathext: Option<&str>) -> Vec<OsString> {
    if !windows {
        return vec![name.into()];
    }
    let exts: Vec<String> = pathext
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(DEFAULT_PATHEXT)
        .split(';')
        .map(str::trim)
        .filter(|e| e.starts_with('.') && e.len() > 1)
        .map(str::to_ascii_lowercase)
        .collect();
    let lower = name.to_ascii_lowercase();
    let mut v: Vec<OsString> = Vec::with_capacity(exts.len() + 1);
    if exts.iter().any(|e| lower.ends_with(e.as_str())) {
        v.push(name.into());
    }
    v.extend(exts.iter().map(|e| OsString::from(format!("{name}{e}"))));
    if !v.iter().any(|n| n == name) {
        v.push(name.into());
    }
    v
}

/// A [`Command`] for `program` that never opens a console window on
/// Windows ([`hide_window`]). Standard input, output and error are left
/// as [`Command`] sets them; callers choose.
pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut c = Command::new(program);
    hide_window(&mut c);
    c
}

/// Sets `CREATE_NO_WINDOW` on `cmd` on Windows, so starting a console
/// program from the window does not flash a console window. Does nothing
/// elsewhere.
pub fn hide_window(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Runs `program` with `args` and no window, waiting at most `limit`;
/// its standard output, decoded with [`decode_output`], when it exits
/// successfully in time. A program still running at the limit is killed.
pub fn run_with_timeout(program: &Path, args: &[&str], limit: Duration) -> Option<String> {
    let mut child = command(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let out = child.wait_with_output().ok()?;
                return status.success().then(|| decode_output(&out.stdout));
            }
            Ok(None) if start.elapsed() < limit => std::thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

/// Text from a child program's output, by one rule:
///
/// - output that is valid UTF-8 is read as UTF-8 (every program textweaver
///   starts writes UTF-8 or plain ASCII when it can: ffmpeg, whisper.cpp,
///   liblouis, Tesseract, pandoc, `reg`, and textweaver's own hosts);
/// - otherwise, on Windows, it is read as Windows-1252, the ANSI code page
///   of Western-language Windows, which programs built on the ANSI C
///   runtime write to a pipe (an accented user name in a path, say);
/// - otherwise, elsewhere, as UTF-8 with each invalid sequence replaced by
///   U+FFFD.
///
/// Files textweaver writes and reads back are always UTF-8; this rule is
/// for pipes only. Windows systems with another ANSI code page (Japanese,
/// Cyrillic) get Windows-1252 letters for those bytes, which is still
/// readable as an error message, never a panic or a lost message.
pub fn decode_output(bytes: &[u8]) -> String {
    decode_output_for(bytes, cfg!(windows))
}

fn decode_output_for(bytes: &[u8], windows: bool) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_owned(),
        Err(_) if windows => bytes.iter().map(|&b| windows_1252(b)).collect(),
        Err(_) => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// One Windows-1252 byte as a char (the five unassigned bytes as the C1
/// control of the same value, as Windows itself maps them).
fn windows_1252(b: u8) -> char {
    const HIGH: [char; 32] = [
        '\u{20AC}', '\u{81}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}',
        '\u{2021}', '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{8D}',
        '\u{017D}', '\u{8F}', '\u{90}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}',
        '\u{2013}', '\u{2014}', '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}',
        '\u{9D}', '\u{017E}', '\u{0178}',
    ];
    match b {
        0x80..=0x9F => HIGH[usize::from(b - 0x80)],
        _ => char::from(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: Vec<OsString>) -> Vec<String> {
        v.into_iter()
            .map(|n| n.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn windows_tries_pathext_first_then_the_bare_name() {
        assert_eq!(
            names(candidate_names("ffmpeg", true, Some(".EXE;.CMD"))),
            ["ffmpeg.exe", "ffmpeg.cmd", "ffmpeg"]
        );
        assert_eq!(
            names(candidate_names("tool", true, None)),
            ["tool.com", "tool.exe", "tool.bat", "tool.cmd", "tool"]
        );
        assert_eq!(
            names(candidate_names("tesseract.exe", true, Some(".EXE"))),
            ["tesseract.exe", "tesseract.exe.exe"]
        );
        assert_eq!(
            names(candidate_names("piper", false, Some(".EXE"))),
            ["piper"]
        );
    }

    #[test]
    fn finds_a_program_in_the_first_folder_that_has_it() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let file = if cfg!(windows) { "tool.exe" } else { "tool" };
        std::fs::write(b.path().join(file), b"").unwrap();
        let path = std::env::join_paths([a.path(), b.path()]).unwrap();
        assert_eq!(find_program_in("tool", &path), Some(b.path().join(file)));
        assert_eq!(find_program_in("other", &path), None);
        assert_eq!(find_program_in("tool", OsStr::new("")), None);
    }

    #[test]
    fn output_is_utf8_when_it_can_be() {
        let s = "Ada Example\u{e9} \u{2014} ok";
        assert_eq!(decode_output_for(s.as_bytes(), true), s);
        assert_eq!(decode_output_for(s.as_bytes(), false), s);
    }

    #[test]
    fn other_output_is_windows_1252_on_windows() {
        // "C:\Users\Ren\xe9e\x80" from a program writing the ANSI code page.
        let bytes = b"C:\\Users\\Ren\xe9e \x80 \x93q\x94";
        assert_eq!(
            decode_output_for(bytes, true),
            "C:\\Users\\Ren\u{e9}e \u{20ac} \u{201c}q\u{201d}"
        );
        assert_eq!(
            decode_output_for(bytes, false),
            "C:\\Users\\Ren\u{fffd}e \u{fffd} \u{fffd}q\u{fffd}"
        );
    }

    #[test]
    fn a_missing_program_gives_nothing() {
        let missing = Path::new("textweaver-no-such-program");
        assert_eq!(
            run_with_timeout(missing, &[], Duration::from_millis(50)),
            None
        );
    }

    #[test]
    fn command_keeps_the_program() {
        let c = command("echo");
        assert_eq!(c.get_program(), OsStr::new("echo"));
    }
}
