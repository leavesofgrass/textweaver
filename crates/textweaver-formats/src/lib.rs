//! Document loaders.
//!
//! A [`Loader`] turns a [`Source`] into a [`Document`] in the canonical shape
//! of ADR-0002. The [`Registry`] picks the highest-priority available loader
//! for a source's extension or hint; unknown extensions load as plain text.
//! [`DocumentCache`] keeps loaded documents on disk, keyed by path,
//! modification time, size, and an options fingerprint.
//!
//! Built-in loaders and their priorities (higher wins for an extension):
//!
//! | Loader | Extensions | Priority |
//! |---|---|---|
//! | [`MarkdownLoader`] | `md`, `markdown`, `mdown`, `mkd`, `mkdn`, `mdwn`, `mdtxt`, `rmd` | [`NATIVE_PRIORITY`] (10) |
//! | [`HtmlLoader`] | `html`, `htm`, `xhtml`, `xht` | [`NATIVE_PRIORITY`] (10) |
//! | [`EpubLoader`] | `epub` | [`NATIVE_PRIORITY`] (10) |
//! | [`DocxLoader`] | `docx`, `docm` | [`NATIVE_PRIORITY`] (10) |
//! | `PdfLoader` (feature `pdf`, on by default; ADR-0010) | `pdf` | [`NATIVE_PRIORITY`] (10) |
//! | [`TextLoader`] | `txt`, `text`, `log` (and the fallback for everything else) | 0 |
//!
//! Every built-in loader is native Rust. Optional loaders rank below them,
//! so they never displace a native loader (Star preferred Pandoc for HTML
//! and DOCX and inherited its bugs).
//!
//! Owner: Agent A.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use textweaver_text::{Document, DocumentMeta};

mod builder;
pub mod cache;
pub mod docx;
pub mod encoding;
pub mod epub;
pub mod export;
pub mod html;
pub mod markdown;
mod package;
#[cfg(feature = "pdf")]
pub mod pdf;
mod text;

pub use cache::{CacheKey, DocumentCache};
pub use docx::DocxLoader;
pub use epub::EpubLoader;
pub use export::{
    ExportFormat, HtmlOptions, MarkdownOptions, TextOptions, export, to_html, to_markdown,
    to_markdown_with, to_text,
};
pub use html::HtmlLoader;
pub use markdown::MarkdownLoader;
#[cfg(feature = "pdf")]
pub use pdf::PdfLoader;
pub use text::TextLoader;

/// Priority of the built-in native loaders for their formats.
pub const NATIVE_PRIORITY: i32 = 10;

/// Separator between table cells on a row of canonical text.
pub const CELL_SEPARATOR: &str = " | ";

/// Version of the canonical text the loaders produce. Bumped whenever a
/// loader's output changes, which invalidates cached documents.
pub const CANONICAL_VERSION: u32 = 2;

/// Where a document comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// A file on disk.
    Path(PathBuf),
    /// In-memory bytes with a format hint (an extension such as `"md"`).
    Bytes {
        /// The content.
        data: Vec<u8>,
        /// Extension-like hint used to pick a loader.
        hint: String,
    },
    /// A URL (wave 2; loaders may refuse it).
    Url(String),
}

impl Source {
    /// The lowercase extension or hint used to choose a loader.
    pub fn hint(&self) -> Option<String> {
        match self {
            Source::Path(p) => p.extension().map(|e| e.to_string_lossy().to_lowercase()),
            Source::Bytes { hint, .. } => Some(hint.trim_start_matches('.').to_lowercase()),
            Source::Url(u) => Path::new(u.split(['?', '#']).next().unwrap_or(u))
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase()),
        }
    }

    /// Reads the source's bytes (paths and in-memory data only).
    pub fn read(&self) -> Result<Vec<u8>, LoadError> {
        match self {
            Source::Path(p) => std::fs::read(p).map_err(|e| LoadError::Io(p.clone(), e)),
            Source::Bytes { data, .. } => Ok(data.clone()),
            Source::Url(u) => Err(LoadError::Unsupported(format!("URL sources: {u}"))),
        }
    }
}

/// Where footnotes go in the canonical text (Star's `footnote_mode`, whose
/// default `inline` did nothing at all; here every mode works).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FootnoteMode {
    /// References read `[label]` where they are; the notes follow the text
    /// under a "Footnotes" heading, one per line.
    #[default]
    Deferred,
    /// Each note replaces its reference as `(footnote: text)`.
    Inline,
    /// References and notes are left out.
    Skip,
}

/// Options that affect how a document is loaded (part of the cache key).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct LoadOptions {
    /// Drop code blocks (text under block `Code` markers) from the canonical
    /// text. Inline code is kept, as in Star.
    pub skip_code: bool,
    /// Where footnotes go.
    pub footnotes: FootnoteMode,
}

/// Loader failures.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// Reading the file failed.
    #[error("cannot read {0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),
    /// No loader handles this source.
    #[error("no loader for {0}")]
    NoLoader(String),
    /// The loader cannot handle this kind of source.
    #[error("unsupported: {0}")]
    Unsupported(String),
    /// The content is malformed.
    #[error("parse error: {0}")]
    Parse(String),
}

/// Turns a source into a document.
pub trait Loader: Send + Sync {
    /// Stable id ("text", "markdown", "html").
    fn id(&self) -> &'static str;
    /// Lowercase extensions this loader claims, without dots.
    fn extensions(&self) -> &'static [&'static str];
    /// False when a runtime requirement (a subprocess, a library) is missing.
    fn available(&self) -> bool {
        true
    }
    /// Higher wins when several loaders claim an extension.
    fn priority(&self) -> i32 {
        0
    }
    /// Loads the document.
    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError>;
}

/// The set of loaders, chosen by extension and priority.
#[derive(Default)]
pub struct Registry {
    loaders: Vec<Box<dyn Loader>>,
}

impl Registry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// A registry with every built-in loader enabled by the current features.
    pub fn with_builtins() -> Self {
        let mut r = Registry::new();
        r.register(Box::new(TextLoader));
        r.register(Box::new(MarkdownLoader));
        r.register(Box::new(HtmlLoader));
        r.register(Box::new(EpubLoader));
        r.register(Box::new(DocxLoader));
        #[cfg(feature = "pdf")]
        r.register(Box::new(PdfLoader));
        r
    }

    /// Adds a loader.
    pub fn register(&mut self, loader: Box<dyn Loader>) {
        self.loaders.push(loader);
    }

    /// Ids of the registered loaders.
    pub fn ids(&self) -> Vec<&'static str> {
        self.loaders.iter().map(|l| l.id()).collect()
    }

    /// Every extension some available loader claims, sorted.
    pub fn extensions(&self) -> Vec<&'static str> {
        let mut v: Vec<&'static str> = self
            .loaders
            .iter()
            .filter(|l| l.available())
            .flat_map(|l| l.extensions().iter().copied())
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    /// The loader registered under `id`.
    pub fn loader_by_id(&self, id: &str) -> Option<&dyn Loader> {
        self.loaders
            .iter()
            .find(|l| l.id() == id)
            .map(|l| l.as_ref())
    }

    /// The best available loader for `source`: the highest priority among
    /// those claiming its extension; the first registered wins a tie.
    pub fn loader_for(&self, source: &Source) -> Option<&dyn Loader> {
        let hint = source.hint().unwrap_or_default();
        let mut best: Option<&dyn Loader> = None;
        for l in &self.loaders {
            if l.available()
                && l.extensions().contains(&hint.as_str())
                && best.is_none_or(|b| l.priority() > b.priority())
            {
                best = Some(l.as_ref());
            }
        }
        best
    }

    /// The loader that [`load`](Self::load) would use: the best one for the
    /// extension, or plain text.
    pub fn resolve(&self, source: &Source) -> &dyn Loader {
        self.loader_for(source).unwrap_or(&TextLoader)
    }

    /// Loads `source` with the best loader, falling back to plain text for
    /// unknown extensions.
    pub fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        self.resolve(source).load(source, options)
    }
}

/// Loads a file with the built-in registry and default options.
pub fn load_path(path: impl AsRef<Path>) -> Result<Document, LoadError> {
    Registry::with_builtins().load(
        &Source::Path(path.as_ref().to_owned()),
        &LoadOptions::default(),
    )
}

/// Metadata for a document loaded from `source` by loader `id`.
pub fn meta_for(source: &Source, id: &str) -> DocumentMeta {
    DocumentMeta {
        path: match source {
            Source::Path(p) => Some(p.clone()),
            _ => None,
        },
        format: id.to_owned(),
        ..DocumentMeta::default()
    }
}

/// The source's text: decoded as [`encoding::decode`] decides (BOM, UTF-8,
/// else Windows-1252), without a byte order mark, with `\r\n` and `\r`
/// turned into `\n`.
pub fn source_text(source: &Source) -> Result<String, LoadError> {
    Ok(decode_source(source, None)?.text)
}

/// The source decoded with an optional declared encoding label (see
/// [`encoding::decode`]), line endings normalized to `\n`.
pub fn decode_source(
    source: &Source,
    declared: Option<&str>,
) -> Result<encoding::Decoded, LoadError> {
    let bytes = source.read()?;
    Ok(decode_bytes(&bytes, declared))
}

/// [`encoding::decode`] with line endings normalized to `\n` and any stray
/// BOM character at the start dropped.
pub fn decode_bytes(bytes: &[u8], declared: Option<&str>) -> encoding::Decoded {
    let mut d = encoding::decode(bytes, declared);
    let text = d.text.strip_prefix('\u{feff}').unwrap_or(&d.text);
    d.text = normalize_newlines(text);
    d
}

/// `\r\n` and lone `\r` turned into `\n`.
pub fn normalize_newlines(text: &str) -> String {
    if text.contains('\r') {
        text.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        text.to_owned()
    }
}

/// Records a non-UTF-8 source encoding in `meta.properties["encoding"]`.
pub fn note_encoding(meta: &mut DocumentMeta, decoded: &encoding::Decoded) {
    if decoded.is_legacy() {
        meta.properties
            .insert("encoding".to_owned(), decoded.encoding.to_owned());
    }
}

/// A title from the file name (with its extension), for sources with a path.
pub fn title_from_path(source: &Source) -> Option<String> {
    match source {
        Source::Path(p) => p
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(&'static str, i32);

    impl Loader for Fake {
        fn id(&self) -> &'static str {
            self.0
        }
        fn extensions(&self) -> &'static [&'static str] {
            &["md"]
        }
        fn priority(&self) -> i32 {
            self.1
        }
        fn load(&self, _: &Source, _: &LoadOptions) -> Result<Document, LoadError> {
            Ok(Document::from_plain_text(self.0))
        }
    }

    #[test]
    fn registry_picks_by_priority_and_falls_back_to_text() {
        let mut r = Registry::with_builtins();
        r.register(Box::new(Fake("low", 1)));
        let md = Source::Bytes {
            data: b"# Hi".to_vec(),
            hint: "md".into(),
        };
        assert_eq!(r.resolve(&md).id(), "markdown");
        r.register(Box::new(Fake("high", 20)));
        assert_eq!(r.resolve(&md).id(), "high");
        let unknown = Source::Bytes {
            data: b"x".to_vec(),
            hint: "weird".into(),
        };
        assert_eq!(r.resolve(&unknown).id(), "text");
        assert!(r.extensions().contains(&"html"));
        let mut ids = vec!["text", "markdown", "html", "epub", "docx"];
        if cfg!(feature = "pdf") {
            ids.push("pdf");
        }
        ids.extend(["low", "high"]);
        assert_eq!(r.ids(), ids);
    }
}
