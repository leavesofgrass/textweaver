//! Pandoc loader (feature `pandoc`): the long tail of formats (OpenDocument,
//! RTF, reStructuredText, Org, LaTeX, DocBook, and more) through a `pandoc`
//! found on `PATH`.
//!
//! The source is converted with `pandoc --from FORMAT --to html5
//! --standalone --wrap=none --sandbox` and the HTML read by the HTML loader's
//! rules, so the result has the same canonical shape as every other format.
//! Unlike star, which asked Pandoc for Markdown and decoded its output with
//! the Windows code page (corrupting non-ASCII text, and speaking simple
//! table dashes), the output is read as UTF-8 bytes and never re-parsed as
//! Markdown. `--sandbox` keeps a document from making Pandoc read other
//! files (LaTeX `\input`, for example); Pandoc 2.19 or later is required.
//!
//! The loader's priority is below the native loaders', so it never takes a
//! format textweaver reads itself; it reports itself unavailable when
//! `pandoc --version` does not run.

use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ropey::Rope;
use textweaver_core::MarkerKind;
use textweaver_text::Document;

use crate::{LoadError, LoadOptions, Loader, Source, meta_for, title_from_path};

/// Priority of the Pandoc loader: below the native loaders.
pub const PANDOC_PRIORITY: i32 = 5;

/// Extensions and the Pandoc reader for each.
const FORMATS: &[(&str, &str)] = &[
    ("odt", "odt"),
    ("rtf", "rtf"),
    ("rst", "rst"),
    ("rest", "rst"),
    ("org", "org"),
    ("tex", "latex"),
    ("latex", "latex"),
    ("ltx", "latex"),
    ("dbk", "docbook"),
    ("docbook", "docbook"),
    ("textile", "textile"),
    ("mediawiki", "mediawiki"),
    ("wiki", "mediawiki"),
    ("fb2", "fb2"),
    ("opml", "opml"),
    ("ipynb", "ipynb"),
    ("t2t", "t2t"),
    ("muse", "muse"),
    ("creole", "creole"),
    ("jats", "jats"),
    ("man", "man"),
    ("typ", "typst"),
    ("bib", "bibtex"),
];

/// Extensions the Pandoc loader claims.
pub const EXTENSIONS: &[&str] = &[
    "odt",
    "rtf",
    "rst",
    "rest",
    "org",
    "tex",
    "latex",
    "ltx",
    "dbk",
    "docbook",
    "textile",
    "mediawiki",
    "wiki",
    "fb2",
    "opml",
    "ipynb",
    "t2t",
    "muse",
    "creole",
    "jats",
    "man",
    "typ",
    "bib",
];

/// How long Pandoc may run on one document before it is stopped, unless
/// `TEXTWEAVER_PANDOC_TIMEOUT` (seconds) or [`PandocLoader::with_timeout`]
/// says otherwise.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

/// Loads documents through Pandoc.
#[derive(Clone, Copy, Debug)]
pub struct PandocLoader {
    timeout: Duration,
}

impl Default for PandocLoader {
    /// The timeout from `TEXTWEAVER_PANDOC_TIMEOUT` (whole seconds), else
    /// [`DEFAULT_TIMEOUT`].
    fn default() -> Self {
        PandocLoader {
            timeout: timeout_from_env(),
        }
    }
}

impl PandocLoader {
    /// A loader that stops Pandoc after `timeout`.
    pub fn with_timeout(timeout: Duration) -> Self {
        PandocLoader { timeout }
    }

    /// How long Pandoc may run on one document.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

fn timeout_from_env() -> Duration {
    std::env::var("TEXTWEAVER_PANDOC_TIMEOUT")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|&s| s > 0)
        .map_or(DEFAULT_TIMEOUT, Duration::from_secs)
}

/// The program to run: the copy in textweaver's components folder, then
/// `TEXTWEAVER_PANDOC`, then `pandoc` on `PATH`
/// (`textweaver_store::find_helper`).
pub fn program() -> OsString {
    textweaver_store::find_helper("pandoc", Some("TEXTWEAVER_PANDOC"))
        .map(OsString::from)
        .or_else(|| std::env::var_os("TEXTWEAVER_PANDOC").filter(|p| !p.is_empty()))
        .unwrap_or_else(|| "pandoc".into())
}

fn command(program: &OsStr) -> Command {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut c = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: no console window flashes from a GUI.
        c.creation_flags(0x0800_0000);
    }
    c
}

/// True when Pandoc runs. A yes is remembered for the process; a no is
/// checked again next time, so Pandoc installed meanwhile (textweaver can
/// fetch it) is found without a restart.
pub fn pandoc_available() -> bool {
    static AVAILABLE: AtomicBool = AtomicBool::new(false);
    if AVAILABLE.load(Ordering::Relaxed) {
        return true;
    }
    let runs = command(&program())
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    AVAILABLE.store(runs, Ordering::Relaxed);
    runs
}

/// What a finished run produced.
#[derive(Debug)]
pub(crate) struct RunOutput {
    pub(crate) success: bool,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

/// Runs `program` with `args`, writing `input` to its stdin and reading its
/// stdout and stderr each on their own thread, so a program that fills one
/// pipe while textweaver waits on another cannot deadlock. After `timeout`
/// the process is killed and the result is an error saying so.
pub(crate) fn run(
    program: &OsStr,
    args: &[&str],
    input: Vec<u8>,
    timeout: Duration,
) -> Result<RunOutput, LoadError> {
    let name = Path::new(program)
        .file_stem()
        .map_or_else(|| "pandoc".into(), |s| s.to_string_lossy().into_owned());
    let mut child = command(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| LoadError::Unsupported(format!("cannot run {name}: {e}")))?;
    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let writer = std::thread::spawn(move || {
        if let Some(mut stdin) = stdin {
            // A program that exits without reading its input closes the
            // pipe; that is its business, not an error here.
            let _ = stdin.write_all(&input);
        }
    });
    let drain = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut p) = pipe {
                let _ = p.read_to_end(&mut buf);
            }
            buf
        })
    };
    let out_reader = drain(stdout.map(|p| Box::new(p) as Box<dyn Read + Send>));
    let err_reader = drain(stderr.map(|p| Box::new(p) as Box<dyn Read + Send>));
    let deadline = Instant::now() + timeout;
    let mut pause = Duration::from_millis(1);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Ok(None) => {
                std::thread::sleep(pause);
                pause = (pause * 2).min(Duration::from_millis(50));
            }
            Err(e) => {
                let _ = child.kill();
                return Err(LoadError::Unsupported(format!("{name} failed: {e}")));
            }
        }
    };
    let Some(status) = status else {
        // Not joined: a process the killed one started may still hold the
        // pipes open, and the threads end when it does.
        return Err(LoadError::Parse(format!(
            "{name} took longer than {} and was stopped",
            spoken_duration(timeout)
        )));
    };
    let _ = writer.join();
    let stdout = out_reader.join().unwrap_or_default();
    let stderr = err_reader.join().unwrap_or_default();
    Ok(RunOutput {
        success: status.success(),
        stdout,
        stderr,
    })
}

/// "2 minutes", "90 seconds", "1 second".
fn spoken_duration(d: Duration) -> String {
    let s = d.as_secs();
    if s >= 60 && s.is_multiple_of(60) {
        let m = s / 60;
        format!("{m} minute{}", if m == 1 { "" } else { "s" })
    } else if s >= 1 {
        format!("{s} second{}", if s == 1 { "" } else { "s" })
    } else {
        format!("{} milliseconds", d.as_millis())
    }
}

/// Pandoc's readers that take a binary package, not text.
const BINARY_READERS: &[&str] = &["odt"];

/// The input as Pandoc wants it: Pandoc reads text as UTF-8 only, on every
/// system, so a text source in another encoding (a Windows-1252 `.rst`
/// from an older editor, UTF-16 with a byte order mark, XML that declares
/// Latin-1) is converted to UTF-8 first, as the native loaders decode
/// (see [`crate::encoding`]). The output side needs nothing: Pandoc
/// writes UTF-8, and it is read as UTF-8 bytes, never with the Windows
/// code page (star's bug).
fn utf8_input(from: &str, bytes: Vec<u8>) -> Vec<u8> {
    if BINARY_READERS.contains(&from) || std::str::from_utf8(&bytes).is_ok() {
        return bytes;
    }
    let declared = crate::encoding::sniff_xml_encoding(&bytes);
    crate::decode_bytes(&bytes, declared.as_deref())
        .text
        .into_bytes()
}

/// Converts `input` from Pandoc reader `from` to standalone HTML.
fn to_html(from: &str, input: Vec<u8>, timeout: Duration) -> Result<Vec<u8>, LoadError> {
    let out = run(
        &program(),
        &[
            "--from",
            from,
            "--to",
            "html5",
            "--standalone",
            "--wrap=none",
            "--sandbox",
        ],
        input,
        timeout,
    )?;
    if !out.success {
        let msg = String::from_utf8_lossy(&out.stderr);
        let first = msg.lines().next().unwrap_or("").trim();
        return Err(LoadError::Parse(format!(
            "pandoc could not read it: {first}"
        )));
    }
    Ok(out.stdout)
}

impl Loader for PandocLoader {
    fn id(&self) -> &'static str {
        "pandoc"
    }

    fn extensions(&self) -> &'static [&'static str] {
        EXTENSIONS
    }

    fn available(&self) -> bool {
        pandoc_available()
    }

    fn priority(&self) -> i32 {
        PANDOC_PRIORITY
    }

    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        let hint = source.hint().unwrap_or_default();
        let from = FORMATS
            .iter()
            .find(|(e, _)| *e == hint)
            .map(|(_, f)| *f)
            .ok_or_else(|| LoadError::Unsupported(format!("pandoc reader for .{hint}")))?;
        let html = to_html(from, utf8_input(from, source.read()?), self.timeout)?;
        let html = String::from_utf8_lossy(&html);
        let mut meta = meta_for(source, self.id());
        meta.properties
            .insert("pandoc.from".into(), from.to_owned());
        let (text, markers) = crate::html::convert(&html, options, &mut meta);
        // Pandoc titles untitled documents after the input ("-"); prefer
        // the first level-1 heading, then the file name.
        if meta
            .title
            .as_deref()
            .is_none_or(|t| t.trim() == "-" || t.trim().is_empty())
        {
            meta.title = markers
                .iter()
                .find(|m| m.kind == MarkerKind::Heading && m.level == 1)
                .map(|m| {
                    text.chars()
                        .skip(m.range.start.0)
                        .take(m.range.len())
                        .collect()
                })
                .or_else(|| title_from_path(source));
        }
        Ok(Document::new(meta, Rope::from_str(&text), markers))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Registry;

    #[test]
    fn text_input_reaches_pandoc_as_utf8() {
        // Windows-1252 and UTF-16 text become UTF-8; UTF-8 and packages
        // pass through untouched.
        assert_eq!(utf8_input("rst", b"caf\xe9".to_vec()), "café".as_bytes());
        assert_eq!(
            utf8_input("org", b"\xff\xfeh\x00\xe9\x00".to_vec()),
            "hé".as_bytes()
        );
        let latin1 = b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><p>\xe0</p>".to_vec();
        assert!(String::from_utf8(utf8_input("docbook", latin1)).is_ok_and(|s| s.contains('à')));
        assert_eq!(utf8_input("rst", "ünïcode".into()), "ünïcode".as_bytes());
        assert_eq!(
            utf8_input("odt", b"PK\x03\x04\xe9".to_vec()),
            b"PK\x03\x04\xe9"
        );
    }

    /// A shell program and the arguments that run `script` in it.
    #[cfg(windows)]
    fn shell(script: &str) -> (OsString, Vec<String>) {
        (
            "powershell".into(),
            vec![
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-Command".into(),
                script.into(),
            ],
        )
    }

    #[cfg(not(windows))]
    fn shell(script: &str) -> (OsString, Vec<String>) {
        ("sh".into(), vec!["-c".into(), script.into()])
    }

    fn run_script(script: &str, timeout: Duration) -> Result<RunOutput, LoadError> {
        let (program, args) = shell(script);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        run(&program, &args, b"input".to_vec(), timeout)
    }

    #[test]
    fn a_full_stderr_pipe_does_not_deadlock() {
        // Two megabytes on stderr before anything on stdout: with stdout
        // read to the end before stderr, the program blocks on a full
        // stderr pipe and the reader waits for ever.
        #[cfg(windows)]
        let script = "[Console]::Error.Write('x' * 2000000); [Console]::Out.Write('done')";
        #[cfg(not(windows))]
        let script = "head -c 2000000 /dev/zero | tr '\\0' x >&2; printf done";
        let out = run_script(script, Duration::from_secs(60)).unwrap();
        assert!(out.success);
        assert_eq!(out.stdout, b"done");
        assert!(out.stderr.len() >= 2_000_000, "{}", out.stderr.len());
    }

    #[test]
    fn a_hung_program_is_stopped_at_the_timeout() {
        #[cfg(windows)]
        let script = "Start-Sleep -Seconds 60";
        #[cfg(not(windows))]
        let script = "exec sleep 60";
        let started = Instant::now();
        let err = run_script(script, Duration::from_secs(2)).unwrap_err();
        let took = started.elapsed();
        assert!(took < Duration::from_secs(30), "took {took:?}");
        assert!(
            err.to_string().contains("took longer than 2 seconds"),
            "{err}"
        );
    }

    #[test]
    fn timeouts_read_well_and_come_from_the_environment() {
        assert_eq!(spoken_duration(Duration::from_secs(120)), "2 minutes");
        assert_eq!(spoken_duration(Duration::from_secs(60)), "1 minute");
        assert_eq!(spoken_duration(Duration::from_secs(90)), "90 seconds");
        assert_eq!(spoken_duration(Duration::from_secs(1)), "1 second");
        assert_eq!(
            PandocLoader::with_timeout(Duration::from_secs(5)).timeout(),
            Duration::from_secs(5)
        );
        if std::env::var_os("TEXTWEAVER_PANDOC_TIMEOUT").is_none() {
            assert_eq!(PandocLoader::default().timeout(), DEFAULT_TIMEOUT);
        }
    }

    #[test]
    fn a_missing_program_is_a_clear_error() {
        let err = run(
            OsStr::new("textweaver-no-such-pandoc"),
            &["--version"],
            Vec::new(),
            Duration::from_secs(5),
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("cannot run textweaver-no-such-pandoc"),
            "{err}"
        );
    }

    #[test]
    fn only_registered_on_request() {
        assert!(!Registry::with_builtins().ids().contains(&"pandoc"));
        assert!(Registry::with_pandoc(None).ids().contains(&"pandoc"));
    }

    #[test]
    fn claims_the_long_tail_below_native_loaders() {
        let l = PandocLoader::default();
        for (e, _) in FORMATS {
            assert!(l.extensions().contains(e), "{e}");
        }
        assert!(!l.extensions().contains(&"docx"));
        assert!(l.priority() < crate::NATIVE_PRIORITY);
    }

    #[test]
    fn converts_restructured_text_when_pandoc_is_installed() {
        if !pandoc_available() {
            eprintln!("pandoc not installed; skipped");
            return;
        }
        let src = "=====\nTitle\n=====\n\nSome *emphasis* and caf\u{e9}.\n\n- one\n- two\n\n=====  =====\nA      B\n=====  =====\n1      2\n=====  =====\n";
        // The loader itself: with the `carta` feature, carta outranks Pandoc
        // for `.rst` in the registry.
        let doc = PandocLoader::default()
            .load(
                &Source::Bytes {
                    data: src.as_bytes().to_vec(),
                    hint: "rst".into(),
                },
                &LoadOptions::default(),
            )
            .unwrap();
        assert_eq!(doc.meta.format, "pandoc");
        let text = doc.text().to_string();
        assert!(text.contains("Some emphasis and caf\u{e9}."), "{text}");
        assert!(text.contains("one\ntwo"), "{text}");
        assert!(text.contains("A | B\n1 | 2"), "{text}");
        assert_eq!(doc.meta.title.as_deref(), Some("Title"));
        assert_eq!(doc.marker_index().count(MarkerKind::ListItem, None), 2);
    }
}
