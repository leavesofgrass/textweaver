//! Pandoc loader (feature `pandoc`): the long tail of formats (OpenDocument,
//! RTF, reStructuredText, Org, LaTeX, DocBook, and more) through a `pandoc`
//! found on `PATH`.
//!
//! The source is converted with `pandoc --from FORMAT --to html5
//! --standalone --wrap=none --sandbox` and the HTML read by the HTML loader's
//! rules, so the result has the same canonical shape as every other format.
//! Unlike Star, which asked Pandoc for Markdown and decoded its output with
//! the Windows code page (corrupting non-ASCII text, and speaking simple
//! table dashes), the output is read as UTF-8 bytes and never re-parsed as
//! Markdown. `--sandbox` keeps a document from making Pandoc read other
//! files (LaTeX `\input`, for example); Pandoc 2.19 or later is required.
//!
//! The loader's priority is below the native loaders', so it never takes a
//! format textweaver reads itself; it reports itself unavailable when
//! `pandoc --version` does not run.

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

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

const EXTENSIONS: &[&str] = &[
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

/// Loads documents through Pandoc.
#[derive(Clone, Copy, Debug, Default)]
pub struct PandocLoader;

/// The program to run (`TEXTWEAVER_PANDOC`, else `pandoc` on `PATH`).
fn program() -> String {
    std::env::var("TEXTWEAVER_PANDOC").unwrap_or_else(|_| "pandoc".to_owned())
}

fn command() -> Command {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut c = Command::new(program());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: no console window flashes from a GUI.
        c.creation_flags(0x0800_0000);
    }
    c
}

/// True when Pandoc runs (checked once per process).
pub fn pandoc_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        command()
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    })
}

/// Converts `input` from Pandoc reader `from` to standalone HTML.
fn to_html(from: &str, input: Vec<u8>) -> Result<Vec<u8>, LoadError> {
    let mut child = command()
        .args([
            "--from",
            from,
            "--to",
            "html5",
            "--standalone",
            "--wrap=none",
            "--sandbox",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| LoadError::Unsupported(format!("cannot run pandoc: {e}")))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| LoadError::Unsupported("pandoc has no input pipe".into()))?;
    // Feed the input from another thread so a large output cannot deadlock.
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let mut out = Vec::new();
    let mut err = Vec::new();
    if let Some(mut o) = child.stdout.take() {
        let _ = o.read_to_end(&mut out);
    }
    if let Some(mut e) = child.stderr.take() {
        let _ = e.read_to_end(&mut err);
    }
    let _ = writer.join();
    let status = child
        .wait()
        .map_err(|e| LoadError::Unsupported(format!("pandoc failed: {e}")))?;
    if !status.success() {
        let msg = String::from_utf8_lossy(&err);
        let first = msg.lines().next().unwrap_or("").trim();
        return Err(LoadError::Parse(format!(
            "pandoc could not read it: {first}"
        )));
    }
    Ok(out)
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
        let html = to_html(from, source.read()?)?;
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
    fn claims_the_long_tail_below_native_loaders() {
        let l = PandocLoader;
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
        let doc = Registry::with_builtins()
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
