//! Document loaders.
//!
//! A [`Loader`] turns a [`Source`] into a [`Document`]. The [`Registry`] picks
//! the highest-priority available loader for a source's extension or hint.
//!
//! Owner: Agent A. Phase 0 ships only the plain-text loader; Markdown and HTML
//! (wave 1), EPUB, DOCX, `paperback`, and `pandoc` (wave 2) follow.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use textweaver_text::{Document, DocumentMeta};

mod text;

pub use text::TextLoader;

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
            Source::Bytes { hint, .. } => Some(hint.to_lowercase()),
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

/// Options that affect how a document is loaded (part of the cache key).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LoadOptions {
    /// Drop text under `Code` markers from the canonical text.
    pub skip_code: bool,
    /// Keep footnotes inline where referenced instead of at the end.
    pub footnotes_inline: bool,
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

    /// The best available loader for `source`.
    pub fn loader_for(&self, source: &Source) -> Option<&dyn Loader> {
        let hint = source.hint().unwrap_or_default();
        self.loaders
            .iter()
            .filter(|l| l.available() && l.extensions().contains(&hint.as_str()))
            .max_by_key(|l| l.priority())
            .map(|l| l.as_ref())
    }

    /// Loads `source` with the best loader, falling back to plain text for
    /// unknown extensions.
    pub fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        match self.loader_for(source) {
            Some(l) => l.load(source, options),
            None => TextLoader.load(source, options),
        }
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
