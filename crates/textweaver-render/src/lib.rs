//! Markdown rendering (ADR-0016): Markdown to accessible HTML with a choice
//! of engines, flavors, LaTeX math as MathML, and MiniJinja templates.
//!
//! ```
//! use textweaver_render::{render, RenderOptions};
//!
//! let out = render("# Hello\n\nArea: $\\pi r^2$.\n", &RenderOptions::default());
//! assert!(out.html.contains("<h1 id=\"hello\">Hello</h1>"));
//! assert!(out.html.contains("<math"));
//! assert_eq!(out.title().as_deref(), Some("Hello"));
//! ```
//!
//! **Engines.** [`Engine::PulldownCmark`] (the default) streams events from
//! the source and is the fastest; [`Engine::Comrak`] builds a full AST and
//! implements the GFM spec completely (autolink literals, the tag filter).
//! Both feed the same event pipeline, so flavors, math, ids, footnotes, and
//! the HTML writer behave identically.
//!
//! **Flavors.** [`Flavor::CommonMark`] (the bare spec), [`Flavor::Gfm`]
//! (tables, task lists, strikethrough, autolinks, footnotes, alerts),
//! [`Flavor::Obsidian`] (GFM plus wikilinks, embeds, callouts, tags, block
//! references, highlights), and [`Flavor::Pandoc`] (definition lists, fenced
//! divs, bracketed spans, heading attributes, citations, sub- and
//! superscript, title blocks). YAML front matter is read in every flavor but
//! CommonMark and becomes template variables.
//!
//! **Accessibility.** Headings get ids for navigation and a table of
//! contents; math (LaTeX, and ASCIIMath in `asciimath` code fences or,
//! when asked, code spans) becomes MathML through `textweaver-math`, with
//! its source as `alttext` and as an annotation; footnotes
//! are collected into a labelled endnotes section with back links that say
//! where they go; callouts are `role="note"` (or `details` when foldable);
//! templates declare the language and landmarks.
//!
//! Owner: Agent L.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

mod engine;
pub mod frontmatter;
pub mod inline;
pub mod math;
mod pipeline;
mod preprocess;
pub mod sanitize;
pub mod slug;
pub mod template;

pub use pipeline::extract_section;
pub use template::{PageOptions, TemplateChoice, Templates};

/// The Markdown parser.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    /// pulldown-cmark: a streaming parser, the fastest (default).
    #[default]
    #[serde(rename = "pulldown")]
    PulldownCmark,
    /// comrak: a full AST, complete GitHub Flavored Markdown.
    Comrak,
}

impl Engine {
    /// Parses `pulldown`, `pulldown-cmark`, or `comrak`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "pulldown" | "pulldown-cmark" | "pulldowncmark" => Some(Engine::PulldownCmark),
            "comrak" => Some(Engine::Comrak),
            _ => None,
        }
    }

    /// The name used on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Engine::PulldownCmark => "pulldown",
            Engine::Comrak => "comrak",
        }
    }
}

/// The Markdown dialect.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Flavor {
    /// Plain CommonMark 0.31.2, no extensions.
    CommonMark,
    /// GitHub Flavored Markdown (default).
    #[default]
    Gfm,
    /// Obsidian: GFM plus wikilinks, embeds, callouts, tags, block ids.
    Obsidian,
    /// Pandoc Markdown: definition lists, fenced divs, spans, citations.
    Pandoc,
}

impl Flavor {
    /// Parses `commonmark`, `gfm`, `obsidian`, or `pandoc`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "commonmark" | "cm" | "strict" => Some(Flavor::CommonMark),
            "gfm" | "github" => Some(Flavor::Gfm),
            "obsidian" => Some(Flavor::Obsidian),
            "pandoc" => Some(Flavor::Pandoc),
            _ => None,
        }
    }

    /// The name used on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Flavor::CommonMark => "commonmark",
            Flavor::Gfm => "gfm",
            Flavor::Obsidian => "obsidian",
            Flavor::Pandoc => "pandoc",
        }
    }
}

/// What `![[Note]]` becomes when the note is not an image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbedMode {
    /// A link to the note (default).
    #[default]
    Link,
    /// The note's rendered text, when a [`Resolver`] finds it and the embed
    /// stands alone in its paragraph; otherwise a link.
    Inline,
}

/// Rendering settings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RenderOptions {
    /// Parser.
    pub engine: Engine,
    /// Dialect.
    pub flavor: Flavor,
    /// Math to MathML: LaTeX (`$…$`, `$$…$$`) and fenced code blocks marked
    /// `asciimath` (or `am`); off leaves the source.
    pub math: bool,
    /// Read inline code spans as ASCIIMath (`` `x^2` ``), as course
    /// material written for MathJax does. Off by default, because in
    /// Markdown a backtick marks code.
    pub asciimath: bool,
    /// Give every heading an id (needed for the table of contents).
    pub heading_ids: bool,
    /// Clean the HTML with ammonia (for untrusted Markdown).
    pub sanitize: bool,
    /// Curly quotes, dashes, and ellipses.
    pub smart_punctuation: bool,
    /// Read YAML front matter (and Pandoc title blocks in the Pandoc flavor).
    pub front_matter: bool,
    /// Obsidian embeds of notes.
    pub embeds: EmbedMode,
    /// Added to wikilink targets that have no extension.
    pub wikilink_extension: String,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            engine: Engine::default(),
            flavor: Flavor::default(),
            math: true,
            asciimath: false,
            heading_ids: true,
            sanitize: false,
            smart_punctuation: false,
            front_matter: true,
            embeds: EmbedMode::default(),
            wikilink_extension: ".html".to_owned(),
        }
    }
}

impl RenderOptions {
    /// Options that render exactly CommonMark: no extensions, no ids, no
    /// front matter, no math.
    pub fn commonmark() -> Self {
        RenderOptions {
            flavor: Flavor::CommonMark,
            math: false,
            heading_ids: false,
            front_matter: false,
            ..RenderOptions::default()
        }
    }
}

/// One table-of-contents entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TocEntry {
    /// Heading level, 1 to 6.
    pub level: u8,
    /// The heading's id.
    pub id: String,
    /// The heading's text.
    pub text: String,
}

/// A rendered document.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Rendered {
    /// The HTML body (a fragment, no `<html>` wrapper).
    pub html: String,
    /// Front matter.
    pub meta: Map<String, Value>,
    /// Headings with ids, in order.
    pub toc: Vec<TocEntry>,
    /// Obsidian `#tags`, in order of first appearance.
    pub tags: Vec<String>,
    /// True when the body contains MathML.
    pub has_math: bool,
    /// True when the body has a level-1 heading.
    pub has_h1: bool,
    /// Text of the first level-1 heading.
    pub first_h1: Option<String>,
}

impl Rendered {
    /// The title: front matter `title`, else the first level-1 heading.
    pub fn title(&self) -> Option<String> {
        self.meta
            .get("title")
            .map(frontmatter::value_text)
            .filter(|t| !t.trim().is_empty())
            .or_else(|| self.first_h1.clone())
    }

    /// The table of contents as nested ordered lists of links.
    pub fn toc_html(&self) -> String {
        toc_html(&self.toc)
    }
}

/// Nested `<ol>` lists for `entries`; deeper levels nest inside the item
/// before them, whatever the jump in level.
pub fn toc_html(entries: &[TocEntry]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut out = String::from("<ol>\n");
    // Levels of the open lists.
    let mut stack: Vec<u8> = vec![entries[0].level];
    for (i, e) in entries.iter().enumerate() {
        if i > 0 {
            let top = *stack.last().unwrap_or(&e.level);
            if e.level > top {
                out.push_str("\n<ol>\n");
                stack.push(e.level);
            } else {
                out.push_str("</li>\n");
                while stack.len() > 1 && e.level < *stack.last().unwrap_or(&0) {
                    stack.pop();
                    out.push_str("</ol>\n</li>\n");
                }
            }
        }
        out.push_str(&format!(
            "<li><a href=\"#{}\">{}</a>",
            escape_html(&e.id),
            escape_html(&e.text)
        ));
    }
    out.push_str("</li>\n");
    while stack.len() > 1 {
        stack.pop();
        out.push_str("</ol>\n</li>\n");
    }
    out.push_str("</ol>\n");
    out
}

/// Errors from templates and file access; rendering Markdown itself never
/// fails.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// Reading a template or note failed.
    #[error("cannot read {0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),
    /// A template is missing or invalid, or failed to render.
    #[error("template error: {0}")]
    Template(String),
}

/// Finds the Markdown of a note named in `![[Note]]` (for inline embeds).
pub trait Resolver: Send + Sync {
    /// The note's Markdown, or `None` when there is no such note.
    fn resolve(&self, name: &str) -> Option<String>;
}

/// Resolves notes as Obsidian does: by name anywhere under a vault folder
/// (`Note` finds `sub/Note.md`), or by a relative path.
#[derive(Debug)]
pub struct FsResolver {
    root: PathBuf,
    index: OnceLock<HashMap<String, PathBuf>>,
}

impl FsResolver {
    /// A resolver over the folder `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        FsResolver {
            root: root.into(),
            index: OnceLock::new(),
        }
    }

    fn index(&self) -> &HashMap<String, PathBuf> {
        self.index.get_or_init(|| {
            let mut map = HashMap::new();
            let mut stack = vec![self.root.clone()];
            while let Some(dir) = stack.pop() {
                let Ok(entries) = std::fs::read_dir(&dir) else {
                    continue;
                };
                for e in entries.flatten() {
                    let p = e.path();
                    let hidden = p
                        .file_name()
                        .is_some_and(|n| n.to_string_lossy().starts_with('.'));
                    if hidden {
                        continue;
                    }
                    if p.is_dir() {
                        stack.push(p);
                    } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("md"))
                        && let Some(stem) = p.file_stem()
                    {
                        map.entry(stem.to_string_lossy().to_lowercase())
                            .or_insert(p);
                    }
                }
            }
            map
        })
    }
}

impl Resolver for FsResolver {
    fn resolve(&self, name: &str) -> Option<String> {
        let name = name.trim();
        let direct = self.root.join(name);
        let with_ext = if Path::new(name).extension().is_some() {
            direct.clone()
        } else {
            self.root.join(format!("{name}.md"))
        };
        for p in [&with_ext, &direct] {
            if p.is_file() {
                return std::fs::read_to_string(p).ok();
            }
        }
        let stem = Path::new(name)
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())?;
        let p = self.index().get(&stem)?;
        std::fs::read_to_string(p).ok()
    }
}

/// Renders Markdown to HTML.
pub fn render(src: &str, opts: &RenderOptions) -> Rendered {
    render_with(src, opts, None)
}

/// Renders Markdown to HTML, finding embedded notes with `resolver`.
pub fn render_with(src: &str, opts: &RenderOptions, resolver: Option<&dyn Resolver>) -> Rendered {
    render_at(src, opts, resolver, 0)
}

pub(crate) fn render_at(
    src: &str,
    opts: &RenderOptions,
    resolver: Option<&dyn Resolver>,
    depth: usize,
) -> Rendered {
    let (meta, body) = if opts.front_matter && opts.flavor != Flavor::CommonMark {
        frontmatter::split(src, opts.flavor == Flavor::Pandoc)
    } else {
        (Map::new(), src)
    };
    let blocks = preprocess::Blocks {
        callouts: opts.flavor == Flavor::Obsidian,
        alerts_only: opts.flavor == Flavor::Gfm,
        fenced_divs: opts.flavor == Flavor::Pandoc,
    };
    let rewritten = preprocess::rewrite(body, blocks);
    let body = rewritten.as_deref().unwrap_or(body);
    let events = engine::parse(body, opts.engine, opts.flavor, opts.smart_punctuation);
    let (mut html, facts) = pipeline::to_html(events, opts, resolver, depth);
    if opts.sanitize {
        html = sanitize::sanitize(&html);
    }
    Rendered {
        html,
        meta,
        has_h1: facts.first_h1.is_some(),
        first_h1: facts.first_h1,
        toc: facts.toc,
        tags: facts.tags,
        has_math: facts.has_math,
    }
}

/// Escapes text for HTML content and attribute values.
pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toc_nests_by_level() {
        let e = |level, id: &str| TocEntry {
            level,
            id: id.into(),
            text: id.to_uppercase(),
        };
        let html = toc_html(&[e(1, "a"), e(2, "b"), e(3, "c"), e(2, "d"), e(1, "f")]);
        assert_eq!(
            html,
            "<ol>\n<li><a href=\"#a\">A</a>\n<ol>\n<li><a href=\"#b\">B</a>\n<ol>\n<li><a href=\"#c\">C</a></li>\n</ol>\n</li>\n<li><a href=\"#d\">D</a></li>\n</ol>\n</li>\n<li><a href=\"#f\">F</a></li>\n</ol>\n"
        );
    }

    #[test]
    fn engine_and_flavor_names_round_trip() {
        for e in [Engine::PulldownCmark, Engine::Comrak] {
            assert_eq!(Engine::parse(e.name()), Some(e));
        }
        for f in [
            Flavor::CommonMark,
            Flavor::Gfm,
            Flavor::Obsidian,
            Flavor::Pandoc,
        ] {
            assert_eq!(Flavor::parse(f.name()), Some(f));
        }
    }
}
