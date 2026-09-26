//! Citations in `tw convert` (ADR-0019): Pandoc citations in Markdown
//! (`[@doe2020]`, `[see @doe2020, p. 12; @roe2021]`, `@doe2020 says`) are
//! formatted with `textweaver-cite` in a CSL style, and a References section
//! of the cited works is appended, before the Markdown is rendered or
//! loaded. So HTML, PDF, DOCX, EPUB, braille, and text output all carry
//! formatted citations and a bibliography, without Pandoc.
//!
//! **Libraries**, first match wins: the file given with `--bibliography`
//! (CSL-JSON, BibTeX, BibLaTeX, or RIS), else the one the document's front
//! matter names (`bibliography: refs.bib`, relative to the document); then
//! the folder library (`references.json` beside the document); then the
//! user's library (`references.json` in textweaver's data folder).
//!
//! **Which citations.** Code spans, code blocks, math, raw HTML, and front
//! matter are never searched. With the Pandoc flavor every citation is
//! formatted (an unknown key reads "missing reference KEY", with a
//! warning), as Pandoc does. With the other flavors, where `@name` is more
//! often a mention than a citation, a bracketed citation is formatted when
//! one of its keys is in a library, and a bare `@key` only when its key is.
//!
//! **HTML** gets citations linked to their entry (`#ref-KEY`) and entries
//! with those ids, so the links resolve (the renderer alone, with no
//! references, links nothing). Note styles (Chicago notes) put citations in
//! footnotes.

use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use pulldown_cmark::{Event, Options, Parser, Tag};
use textweaver_cite::pandoc::find_citations;
use textweaver_cite::{
    Citation, CitationStyle, Formatter, Layered, Library, OutputFormat as CiteFormat,
};

/// Citation settings for a conversion run.
#[derive(Clone, Debug)]
pub struct CitationOptions {
    /// Format citations at all.
    pub enabled: bool,
    /// A bibliography file used before the folder and user libraries
    /// (CSL-JSON, BibTeX, BibLaTeX, or RIS).
    pub bibliography: Option<PathBuf>,
    /// The CSL style: a short name (`apa`, `mla`, `chicago`, `ieee`, ...)
    /// or a `.csl` file.
    pub style: String,
    /// The user's library (`references.json` in the data folder), when
    /// known.
    pub user_library: Option<PathBuf>,
}

impl Default for CitationOptions {
    fn default() -> Self {
        CitationOptions {
            enabled: true,
            bibliography: None,
            style: DEFAULT_STYLE.to_owned(),
            user_library: None,
        }
    }
}

/// The style used when none is given (as `tw cite format`).
pub const DEFAULT_STYLE: &str = "apa";

/// The heading of the appended bibliography.
pub const REFERENCES_HEADING: &str = "References";

/// Libraries and the style, loaded once per run and shared by the workers.
pub(crate) struct Citations {
    options: CitationOptions,
    style: CitationStyle,
    user: OnceLock<Option<Arc<Library>>>,
    /// Loaded libraries by path (bibliography files and folder libraries).
    files: Mutex<HashMap<PathBuf, Option<Arc<Library>>>>,
}

impl std::fmt::Debug for Citations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Citations")
            .field("options", &self.options)
            .finish()
    }
}

/// Markdown with its citations formatted, and what to tell the user.
#[derive(Debug, Default)]
pub(crate) struct Cited {
    /// The Markdown to render or load instead of the source.
    pub markdown: Option<String>,
    /// Warnings, each a sentence.
    pub warnings: Vec<String>,
}

impl Citations {
    /// Resolves the style; an unknown style is an error before any file is
    /// converted.
    pub fn new(options: CitationOptions) -> Result<Self, String> {
        let style = CitationStyle::resolve(&options.style).map_err(|e| {
            format!(
                "Cannot use the citation style {}: {e}. Run tw cite styles for the list.",
                options.style
            )
        })?;
        Ok(Citations {
            options,
            style,
            user: OnceLock::new(),
            files: Mutex::new(HashMap::new()),
        })
    }

    fn load(&self, path: &Path, warnings: &mut Vec<String>) -> Option<Arc<Library>> {
        if let Ok(files) = self.files.lock()
            && let Some(lib) = files.get(path)
        {
            return lib.clone();
        }
        let is_json = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("json"));
        let loaded = if !path.exists() {
            None
        } else if is_json {
            Library::load(path)
                .map(Arc::new)
                .map_err(|e| e.to_string())
                .ok()
        } else {
            textweaver_cite::formats::read_file(path)
                .map(|refs| Arc::new(Library::from_references(refs)))
                .map_err(|e| e.to_string())
                .ok()
        };
        if loaded.is_none() && path.exists() {
            warnings.push(format!(
                "The references in {} could not be read, so they were not used.",
                path.display()
            ));
        }
        if let Ok(mut files) = self.files.lock() {
            files.insert(path.to_owned(), loaded.clone());
        }
        loaded
    }

    /// The Markdown `text` of `source` with citations formatted for
    /// `html` output or not; `pandoc` is the Pandoc flavor. `None` when there
    /// is nothing to format.
    pub fn apply(&self, source: &Path, text: &str, html: bool, pandoc: bool) -> Cited {
        let mut cited = Cited::default();
        if !self.options.enabled || !text.contains('@') {
            return cited;
        }
        let (meta, body) = textweaver_render::frontmatter::split(text, pandoc);
        let body_start = text.len() - body.len();
        let skip = unsearched(body);
        let found: Vec<Citation> = find_citations(body)
            .into_iter()
            .filter(|c| !skip.iter().any(|r| overlaps(r, &c.range)))
            .collect();
        if found.is_empty() {
            return cited;
        }

        // Libraries, first match wins.
        let dir = source.parent().unwrap_or(Path::new("."));
        let mut layers: Vec<Arc<Library>> = Vec::new();
        let named = self.options.bibliography.clone().or_else(|| {
            meta.get("bibliography")
                .map(textweaver_render::frontmatter::value_text)
                .map(|b| b.trim().to_owned())
                .filter(|b| !b.is_empty())
                .map(|b| dir.join(b))
        });
        if let Some(b) = &named {
            match self.load(b, &mut cited.warnings) {
                Some(lib) => layers.push(lib),
                None if !b.exists() => cited
                    .warnings
                    .push(format!("The bibliography {} does not exist.", b.display())),
                None => {}
            }
        }
        if let Some(lib) = self.load(
            &textweaver_cite::folder_library_path(dir),
            &mut cited.warnings,
        ) {
            layers.push(lib);
        }
        let user = self.user.get_or_init(|| {
            let path = self.options.user_library.as_ref()?;
            Library::load(path).ok().map(Arc::new)
        });
        if let Some(lib) = user {
            layers.push(lib.clone());
        }
        let refs: Vec<&Library> = layers.iter().map(|l| l.as_ref()).collect();
        let source_libs = Layered { layers: &refs };
        let known = |key: &str| refs.iter().any(|l| l.contains(key));

        let cites: Vec<Citation> = found
            .into_iter()
            .filter(|c| {
                pandoc
                    || if c.narrative {
                        c.items.iter().all(|i| known(&i.key))
                    } else {
                        c.items.iter().any(|i| known(&i.key))
                    }
            })
            .collect();
        if cites.is_empty() {
            return cited;
        }
        let format = if html {
            CiteFormat::Html
        } else {
            CiteFormat::Markdown
        };
        let rendered = match Formatter::new(&self.style, format).document(&cites, &source_libs) {
            Ok(r) => r,
            Err(e) => {
                cited
                    .warnings
                    .push(format!("The citations could not be formatted: {e}."));
                return cited;
            }
        };
        for key in &rendered.missing {
            cited.warnings.push(format!(
                "The citation key {key} is not in any library, so it reads as missing reference {key}."
            ));
        }

        // Replace the citations, back to front so ranges stay valid.
        let mut out = body.to_owned();
        let mut notes: Vec<String> = Vec::new();
        for (n, (c, formatted)) in cites.iter().zip(&rendered.citations).enumerate().rev() {
            let replacement = if rendered.note_style {
                notes.push(format!(
                    "[^cite-{}]: {}",
                    n + 1,
                    inline_markdown(formatted, html)
                ));
                format!("[^cite-{}]", n + 1)
            } else if html {
                let first = c.items.first().map(|i| i.key.as_str()).unwrap_or("");
                let keys: Vec<&str> = c.items.iter().map(|i| i.key.as_str()).collect();
                if known(first) {
                    format!(
                        "<span class=\"citation\" data-cites=\"{}\"><a href=\"#{}\">{}</a></span>",
                        attr(&keys.join(" ")),
                        attr(&ref_id(first)),
                        formatted
                    )
                } else {
                    format!(
                        "<span class=\"citation\" data-cites=\"{}\">{}</span>",
                        attr(&keys.join(" ")),
                        formatted
                    )
                }
            } else {
                formatted.clone()
            };
            out.replace_range(c.range.clone(), &replacement);
        }
        notes.reverse();
        if !out.ends_with('\n') {
            out.push('\n');
        }
        if !notes.is_empty() {
            out.push('\n');
            for note in &notes {
                out.push_str(note);
                out.push_str("\n\n");
            }
        }
        if !rendered.bibliography.is_empty() {
            let has_heading = last_heading_is_references(&out);
            if !has_heading {
                out.push_str(&format!("\n## {REFERENCES_HEADING}\n\n"));
            } else {
                out.push('\n');
            }
            if html {
                out.push_str("<div id=\"refs\" class=\"references\" role=\"list\">\n");
                for e in &rendered.bibliography {
                    out.push_str(&format!(
                        "<p id=\"{}\" class=\"reference\" role=\"listitem\">{}</p>\n",
                        attr(&ref_id(&e.key)),
                        e.text
                    ));
                }
                out.push_str("</div>\n");
            } else {
                for e in &rendered.bibliography {
                    out.push_str(&e.text);
                    out.push_str("\n\n");
                }
            }
        }
        let mut full = text[..body_start].to_owned();
        full.push_str(&out);
        cited.markdown = Some(full);
        cited
    }
}

/// The id of a bibliography entry, as Pandoc names it.
fn ref_id(key: &str) -> String {
    format!("ref-{key}")
}

fn attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// A footnote's text on one line (a formatted note may hold line breaks).
fn inline_markdown(s: &str, _html: bool) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn overlaps(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start < b.end && b.start < a.end
}

/// Byte ranges of `markdown` no citation is looked for in: code, math, and
/// raw HTML.
fn unsearched(markdown: &str) -> Vec<Range<usize>> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_MATH;
    let mut out = Vec::new();
    for (event, range) in Parser::new_ext(markdown, options).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_) | Tag::HtmlBlock)
            | Event::Code(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::InlineHtml(_)
            | Event::Html(_) => out.push(range),
            _ => {}
        }
    }
    out
}

/// True when the document already ends with a References (or
/// Bibliography) heading, under which the entries go.
fn last_heading_is_references(markdown: &str) -> bool {
    let last = markdown
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty());
    last.is_some_and(|l| {
        let t = l.trim_start_matches('#').trim().to_lowercase();
        l.starts_with('#') && matches!(t.as_str(), "references" | "bibliography" | "works cited")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_math_and_html_are_not_searched() {
        let md = "Text [@a].\n\n`[@b]` and $[@c]$ <span data-x=\"@d\">x</span>\n\n```\n[@e]\n```\n";
        let skip = unsearched(md);
        let found: Vec<String> = find_citations(md)
            .into_iter()
            .filter(|c| !skip.iter().any(|r| overlaps(r, &c.range)))
            .flat_map(|c| c.items.into_iter().map(|i| i.key))
            .collect();
        assert_eq!(found, ["a"]);
    }

    #[test]
    fn references_heading_is_found() {
        assert!(last_heading_is_references(
            "# Paper\n\ntext\n\n## References\n\n"
        ));
        assert!(!last_heading_is_references(
            "# Paper\n\n## References\n\nMore.\n"
        ));
    }
}
