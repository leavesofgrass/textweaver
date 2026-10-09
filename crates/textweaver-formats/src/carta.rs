//! carta loader (feature `carta`): Org mode, reStructuredText, MediaWiki,
//! DokuWiki, and Jira read in process, in pure Rust, by
//! [carta](https://github.com/mfkrause/carta), a young reimplementation of
//! Pandoc (MIT or Apache-2.0); and Markdown written out as AsciiDoc, Typst,
//! LaTeX, MediaWiki, or Org ([`write_markdown`]).
//!
//! carta converts the source to an HTML fragment, and the HTML loader's
//! rules read it, exactly as the Pandoc loader does, so offsets, markers,
//! highlighting, Braille, and navigation stay textweaver's own. The loader
//! ranks below the native loaders and above Pandoc: it never takes a format
//! textweaver reads itself, and where both are registered (as in
//! `tw convert`) the in-process reader wins over the subprocess.
//!
//! The formats claimed are the ones the comparison with Pandoc found
//! usable (B1-k1). DokuWiki pages and Jira markup have no extension of
//! their own (a DokuWiki page is a `.txt` file), so they are read when
//! named: `tw convert --from dokuwiki`, or a hint of `dokuwiki` or `jira`.
//! Typst and LaTeX are not read here: carta's Typst reader reads any file
//! a document names, with no way to turn that off, and textweaver's own
//! LaTeX loader keeps tables and the title that carta's loses.
//!
//! carta's reStructuredText reader follows `.. include::` to any path on
//! the disk, relative to the working folder, with no sandbox (Pandoc's
//! `--sandbox` forbids it). The loader turns each include into a comment
//! before carta sees it, so a document can never pull another file in.

use std::borrow::Cow;

use ropey::Rope;
use textweaver_core::MarkerKind;
use textweaver_text::Document;

use crate::{LoadError, LoadOptions, Loader, Source, meta_for, title_from_path};

/// Priority of the carta loader: below the native loaders, above Pandoc.
pub const CARTA_PRIORITY: i32 = 7;

/// Extensions and the carta reader for each.
const FORMATS: &[(&str, &str)] = &[
    ("org", "org"),
    ("rst", "rst"),
    ("rest", "rst"),
    ("mediawiki", "mediawiki"),
    ("wiki", "mediawiki"),
    ("dokuwiki", "dokuwiki"),
    ("jira", "jira"),
];

/// Extensions (and format names) the carta loader claims.
pub const EXTENSIONS: &[&str] = &["org", "rst", "rest", "mediawiki", "wiki", "dokuwiki", "jira"];

/// The formats [`write_markdown`] writes: carta's writer names, for the
/// formats textweaver does not write itself.
pub const WRITERS: &[&str] = &["asciidoc", "typst", "latex", "mediawiki", "org"];

/// Markdown (GitHub's flavor, with dollar math, footnotes, and YAML front
/// matter) written as a whole document in `to`, one of [`WRITERS`]: an
/// AsciiDoc or Org file with its title, a LaTeX file with its preamble, a
/// Typst file with its page setup, or MediaWiki markup.
///
/// One direction only: Markdown in, the format out, through carta's
/// document model and its built-in templates; no file is read.
///
/// shortcut: a document textweaver loaded from another format reaches carta
/// as Markdown (`to_markdown`), so what Markdown cannot say (a table cell
/// spanning columns, a page number) is lost on the way; upgrade to a
/// direct mapping from the canonical document to carta's model when
/// carta's API settles.
pub fn write_markdown(markdown: &str, to: &str) -> Result<String, LoadError> {
    if !WRITERS.contains(&to) {
        return Err(LoadError::Unsupported(format!("carta writer for {to}")));
    }
    let failed = |e: carta::Error| LoadError::Parse(format!("carta could not write {to}: {e}"));
    let (doc, media) =
        carta::read_document("gfm", markdown.as_bytes(), &carta::ReaderOptions::default())
            .map_err(failed)?;
    let mut writer = carta::WriterOptions::default();
    writer.standalone = true;
    match carta::render_document(to, doc, media, &writer).map_err(failed)? {
        carta::Output::Text(text) => Ok(text),
        carta::Output::Bytes(_) => Err(LoadError::Parse(format!("carta wrote bytes, not {to}"))),
    }
}

/// Loads documents through carta.
#[derive(Clone, Copy, Debug, Default)]
pub struct CartaLoader;

/// The source with every reStructuredText `.. include::` directive turned
/// into a comment (its indented options become the comment's body), so
/// carta never reads another file.
///
/// shortcut: line based, so an include written inside a literal block (an
/// example in a code listing) is commented out too; upgrade when carta
/// offers a reader option that turns file access off.
fn without_includes(text: &str) -> Cow<'_, str> {
    let is_include = |line: &str| {
        line.trim_start()
            .strip_prefix("..")
            .map(str::trim_start)
            .and_then(|rest| rest.split_once("::"))
            .is_some_and(|(name, _)| name.eq_ignore_ascii_case("include"))
    };
    if !text.lines().any(is_include) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        if is_include(line) {
            let indent = &line[..line.len() - line.trim_start().len()];
            out.push_str(indent);
            out.push_str(".. textweaver skipped an include\n");
        } else {
            out.push_str(line);
        }
    }
    Cow::Owned(out)
}

/// The warning a document gets when an include was left out.
pub const INCLUDE_WARNING: &str =
    "An include directive was left out, so the file it names is not read.";

/// Converts `text` from carta reader `from` to an HTML fragment, and
/// whether an include was left out. A title in the metadata (Org's
/// `#+TITLE`, for one) opens the fragment as a level-1 heading, as
/// Pandoc's standalone title block does.
fn to_html(from: &str, text: &str) -> Result<(String, bool), LoadError> {
    use carta::ast::MetaValue;
    let text = if from == "rst" {
        without_includes(text)
    } else {
        Cow::Borrowed(text)
    };
    let skipped = matches!(text, Cow::Owned(_));
    let failed = |e: carta::Error| LoadError::Parse(format!("carta could not read it: {e}"));
    let (doc, media) =
        carta::read_document(from, text.as_bytes(), &carta::ReaderOptions::default())
            .map_err(failed)?;
    let title = match doc.meta.get("title") {
        Some(MetaValue::MetaString(t)) => t.to_string(),
        Some(MetaValue::MetaInlines(i)) => carta::ast::to_plain_text(i),
        _ => String::new(),
    };
    // MathML, which the HTML loader reads and speaks, not TeX source.
    let mut writer = carta::WriterOptions::default();
    writer.math_method = carta::MathMethod::Mathml;
    let body = match carta::render_document("html", doc, media, &writer).map_err(failed)? {
        carta::Output::Text(html) => html,
        carta::Output::Bytes(_) => {
            return Err(LoadError::Parse("carta wrote bytes, not HTML".into()));
        }
    };
    if title.trim().is_empty() {
        return Ok((body, skipped));
    }
    let title = title
        .trim()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    Ok((format!("<h1 class=\"title\">{title}</h1>\n{body}"), skipped))
}

impl Loader for CartaLoader {
    fn id(&self) -> &'static str {
        "carta"
    }

    fn extensions(&self) -> &'static [&'static str] {
        EXTENSIONS
    }

    fn priority(&self) -> i32 {
        CARTA_PRIORITY
    }

    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        let hint = source.hint().unwrap_or_default();
        let from = FORMATS
            .iter()
            .find(|(e, _)| *e == hint)
            .map(|(_, f)| *f)
            .ok_or_else(|| LoadError::Unsupported(format!("carta reader for .{hint}")))?;
        // Decoded as the native loaders decode (Windows-1252, UTF-16 with a
        // byte order mark), since carta reads UTF-8 only.
        let decoded = crate::decode_bytes(&source.read()?, None).text;
        let (html, skipped) = to_html(from, &decoded)?;
        let mut meta = meta_for(source, self.id());
        meta.properties.insert("carta.from".into(), from.to_owned());
        if skipped {
            crate::add_warning(&mut meta, INCLUDE_WARNING);
        }
        let (text, markers) = crate::html::convert(&html, options, &mut meta);
        // The fragment has no title element: take the first level-1
        // heading (the metadata title when there is one), then the file
        // name.
        if meta.title.as_deref().is_none_or(|t| t.trim().is_empty()) {
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

    fn load(src: &str, hint: &str) -> Document {
        Registry::with_builtins()
            .load(
                &Source::Bytes {
                    data: src.as_bytes().to_vec(),
                    hint: hint.into(),
                },
                &LoadOptions::default(),
            )
            .unwrap()
    }

    #[test]
    fn claims_its_formats_below_native_loaders() {
        let l = CartaLoader;
        for (e, _) in FORMATS {
            assert!(l.extensions().contains(e), "{e}");
        }
        assert!(!l.extensions().contains(&"tex"));
        assert!(l.priority() < crate::NATIVE_PRIORITY);
        assert!(Registry::with_builtins().ids().contains(&"carta"));
    }

    #[test]
    fn reads_org_into_the_canonical_shape() {
        let doc = load(
            "#+TITLE: Field <Notes>\n* Crows\nThey are /clever/ and caf\u{e9}.\n\n- one\n- two\n",
            "org",
        );
        assert_eq!(doc.meta.format, "carta");
        let text = doc.text().to_string();
        assert!(text.contains("They are clever and caf\u{e9}."), "{text}");
        assert_eq!(doc.meta.title.as_deref(), Some("Field <Notes>"));
        assert!(text.starts_with("Field <Notes>\n"), "{text}");
        assert_eq!(doc.marker_index().count(MarkerKind::Heading, None), 2);
        assert_eq!(doc.marker_index().count(MarkerKind::ListItem, None), 2);
    }

    #[test]
    fn reads_restructured_text_and_mediawiki() {
        let rst = load("Title\n=====\n\nSome *emphasis*.\n\n- one\n- two\n", "rst");
        assert!(rst.text().to_string().contains("Some emphasis."));
        assert_eq!(rst.marker_index().count(MarkerKind::ListItem, None), 2);
        let wiki = load("== Crows ==\nThey are ''clever''.\n* one\n* two\n", "wiki");
        assert!(wiki.text().to_string().contains("They are clever."));
        assert_eq!(wiki.marker_index().count(MarkerKind::Heading, None), 1);
    }

    #[test]
    fn reads_dokuwiki_and_jira_by_name() {
        let doku = load("====== Crows ======\nThey are //clever//.\n", "dokuwiki");
        assert!(doku.text().to_string().contains("They are clever."));
        assert_eq!(doku.marker_index().count(MarkerKind::Heading, None), 1);
        let jira = load("h1. Crows\nThey are _clever_.\n* one\n* two\n", "jira");
        assert!(jira.text().to_string().contains("They are clever."));
        assert_eq!(jira.marker_index().count(MarkerKind::ListItem, None), 2);
    }

    #[test]
    fn writes_markdown_as_each_format() {
        let md = "---\ntitle: Field Notes\n---\n\n# Crows\n\nThey are *clever* and caf\u{e9}.\n\n- one\n- two\n";
        let expect = [
            ("asciidoc", "= Field Notes"),
            ("typst", "Crows"),
            ("latex", r"\documentclass"),
            ("mediawiki", "= Crows ="),
            ("org", "#+title: Field Notes"),
        ];
        for (to, needle) in expect {
            let out = write_markdown(md, to).unwrap();
            assert!(out.contains(needle), "{to}: {out}");
            assert!(out.contains("caf\u{e9}"), "{to}: {out}");
        }
        assert!(write_markdown(md, "typst-or-not").is_err());
        // The Org output reads back through the loader with its heading.
        let back = load(&write_markdown(md, "org").unwrap(), "org");
        assert!(back.text().to_string().contains("They are clever"));
    }

    #[test]
    fn a_restructured_text_include_never_reads_another_file() {
        let dir = tempfile::tempdir().unwrap();
        let secret = dir.path().join("secret.rst");
        std::fs::write(&secret, "The secret word is pelican.\n").unwrap();
        let src = format!(
            "Before.\n\n.. include:: {}\n\n  .. INCLUDE:: {}\n\nAfter.\n",
            secret.display(),
            secret.display()
        );
        let doc = load(&src, "rst");
        assert_eq!(crate::warnings(&doc.meta), [INCLUDE_WARNING]);
        let text = doc.text().to_string();
        assert!(!text.contains("pelican"), "{text}");
        assert!(
            text.contains("Before.") && text.contains("After."),
            "{text}"
        );
        assert_eq!(without_includes("No directives.\n"), "No directives.\n");
    }
}
