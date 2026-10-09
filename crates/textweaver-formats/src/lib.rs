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
//! | [`DocxLoader`], with comments and tracked changes | `docx`, `docm` | [`NATIVE_PRIORITY`] (10) |
//! | [`RtfLoader`]: Rich Text Format | `rtf` | [`NATIVE_PRIORITY`] (10) |
//! | [`OdtLoader`]: OpenDocument text, with comments and tracked changes | `odt`, `ott`, `fodt` | [`NATIVE_PRIORITY`] (10) |
//! | [`LatexLoader`]: a LaTeX subset, with math and `\input` inside the folder | `tex`, `latex`, `ltx` | [`NATIVE_PRIORITY`] (10) |
//! | [`EmlLoader`]: email, headers then body, attachments listed | `eml` | [`NATIVE_PRIORITY`] (10) |
//! | [`MhtmlLoader`]: web archives (RFC 2557) through the HTML loader | `mhtml`, `mht` | [`NATIVE_PRIORITY`] (10) |
//! | `PdfLoader` (feature `pdf`, on by default; ADR-0010), with OCR of scanned pages (feature `images`, and `ocr` for the in-process engine; ADR-0026) | `pdf` | [`NATIVE_PRIORITY`] (10) |
//! | `ImageLoader` (feature `images`; `ocr` adds the in-process engine): OCR of an image file | `png`, `jpg`, `jpeg` | [`NATIVE_PRIORITY`] (10) |
//! | [`DaisyLoader`]: DAISY 3 books and DTBook files | `opf`, `xml`, `dtbook` | [`NATIVE_PRIORITY`] (10) |
//! | [`BrfLoader`]: braille files (BRF), back-translated to print through liblouis | `brf`, `brl` | [`NATIVE_PRIORITY`] (10) |
//! | [`PptxLoader`]: PowerPoint slides and speaker notes | `pptx`, `pptm`, `ppsx` | [`NATIVE_PRIORITY`] (10) |
//! | [`SheetLoader`]: spreadsheets as tables | `csv`, `tsv`, `tab`, `ods`, and (feature `spreadsheets`) `xlsx`, `xlsm`, `xlsb` | [`NATIVE_PRIORITY`] (10) |
//! | [`ArchiveLoader`]: a list of the files inside, or the DAISY book or EPUB it holds | `zip`, and (feature `archives`) `tar`, `tgz`, `gz`, `7z` | [`NATIVE_PRIORITY`] (10) |
//! | [`JsonLoader`]: JSON with a heading per key, and JSON Lines with a heading per line | `json`, `jsonl`, `ndjson`, `geojson`, `webmanifest` | [`NATIVE_PRIORITY`] (10) |
//! | [`NotebookLoader`]: Jupyter notebooks, cell by cell | `ipynb` | [`NATIVE_PRIORITY`] (10) |
//! | [`SvgLoader`]: a drawing's title, description, titled parts, and text | `svg` | [`NATIVE_PRIORITY`] (10) |
//! | [`MathMlLoader`]: one formula, presentation or content MathML | `mml`, `mathml` | [`NATIVE_PRIORITY`] (10) |
//! | [`TextLoader`] | `txt`, `text`, `log` (and the fallback for everything else) | 0 |
//!
//! Two kinds of path are not plain files:
//!
//! - **Archive members**, `book.zip!chapter.pdf` (nested ones too,
//!   `outer.zip!inner.tar!notes.md`): [`Source::read`] reads the member,
//!   so every loader opens one, and the document's path (the key for
//!   notes and positions) is the whole form. See [`archive`].
//! - **Web addresses**, `https://...` (feature `url`): [`Registry::load`]
//!   fetches the page and reads it as HTML, or the PDF, EPUB, or other
//!   file it is. See `web`.
//!
//! Every built-in loader is native Rust. The Pandoc loader (feature
//! `pandoc`; `rst`, `org`, `tex`, `dbk`, `textile`,
//! `mediawiki`, `fb2`, `opml`, `ipynb`, and more; priority 5) is not among
//! the built-ins: a caller that wants Pandoc registers it
//! ([`Registry::with_pandoc`]), as `tw convert` does, so the reader never
//! runs a subprocess to open a file. It ranks below the native loaders, so
//! it never displaces one (star preferred Pandoc for HTML and DOCX and
//! inherited its bugs).
//!
//! Word comments and ODT annotations travel with the document as
//! [`DocumentComment`]s (see [`comments`]); tracked changes are read as
//! the final text, or said in place with [`RevisionMode::Marked`].
//!
//! Owner: Agent A.
//!
// `Registry::with_pandoc` exists only with the `pandoc` feature; without it
// the link above goes to `Registry`, so the docs build either way.
#![cfg_attr(
    feature = "pandoc",
    doc = "[`Registry::with_pandoc`]: Registry::with_pandoc"
)]
#![cfg_attr(not(feature = "pandoc"), doc = "[`Registry::with_pandoc`]: Registry")]

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use textweaver_text::{Document, DocumentMeta};

pub mod annotations;
pub mod archive;
pub mod brf;
mod builder;
pub mod cache;
pub mod callout;
mod counter;
pub mod daisy;
pub mod docx;
pub mod eml;
pub mod encoding;
pub mod epub;
pub mod export;
pub mod fulltext;
pub mod html;
pub mod json;
pub mod latex;
pub mod markdown;
mod mathml;
pub mod obsidian;
pub mod odt;
mod omml;
mod package;
#[cfg(feature = "pandoc")]
pub mod pandoc;
pub mod pause_markup;
#[cfg(feature = "pdf")]
pub mod pdf;
pub mod pptx;
pub mod progress;
mod revision;
pub mod rtf;
pub mod sheet;
pub mod svg;
mod text;
#[cfg(feature = "url")]
pub mod web;
mod xmldepth;

pub use annotations::{
    COMMENTS_PROPERTY, CommentReply, DocumentComment, REVISIONS_PROPERTY, comments, revision_count,
};
pub use archive::ArchiveLoader;
pub use brf::{BrfCode, BrfLoader};
pub use cache::{CacheKey, DocumentCache};
pub use daisy::DaisyLoader;
pub use docx::DocxLoader;
pub use eml::{EmlLoader, MhtmlLoader};
pub use epub::EpubLoader;
pub use export::{
    ExportFormat, HtmlOptions, MarkdownOptions, TextOptions, export, to_html, to_markdown,
    to_markdown_with, to_text,
};
pub use fulltext::{FullTextIndex, IndexedDocument, RefreshReport, SearchHit};
pub use html::HtmlLoader;
pub use json::{JsonLoader, NotebookLoader};
pub use latex::LatexLoader;
pub use markdown::MarkdownLoader;
pub use mathml::MathMlLoader;
pub use odt::OdtLoader;
#[cfg(feature = "pandoc")]
pub use pandoc::PandocLoader;
#[cfg(feature = "pdf")]
pub use pdf::PdfLoader;
#[cfg(feature = "images")]
pub use pdf::image::ImageLoader;
pub use pptx::PptxLoader;
pub use progress::{Progress, ProgressReport};
pub use rtf::RtfLoader;
pub use sheet::SheetLoader;
pub use svg::SvgLoader;
pub use text::TextLoader;

/// Priority of the built-in native loaders for their formats.
pub const NATIVE_PRIORITY: i32 = 10;

/// Separator between table cells on a row of canonical text.
pub const CELL_SEPARATOR: &str = " | ";

/// The deepest element nesting the HTML, EPUB, and DOCX walkers follow.
/// Content nested deeper is read as plain text without structure, so a
/// malformed or hostile file cannot overflow the stack (and take a whole
/// `tw convert` batch down with it); the document then carries
/// [`NESTING_WARNING`].
pub const MAX_NESTING: usize = 256;

/// The warning a document carries when content past [`MAX_NESTING`] was
/// flattened.
pub const NESTING_WARNING: &str =
    "Some content was nested too deeply to keep its structure, so it is read as plain text.";

/// The `DocumentMeta::properties` key holding loader warnings, one sentence
/// per line (see [`warnings`]).
pub const WARNINGS_PROPERTY: &str = "textweaver.warnings";

/// Adds a warning sentence to `meta` (once), and logs it.
pub fn add_warning(meta: &mut DocumentMeta, sentence: &str) {
    let name = meta
        .path
        .as_deref()
        .map_or_else(|| "document".into(), |p| p.display().to_string());
    let entry = meta
        .properties
        .entry(WARNINGS_PROPERTY.to_owned())
        .or_default();
    if entry.lines().any(|l| l == sentence) {
        return;
    }
    log::warn!("{name}: {sentence}");
    if !entry.is_empty() {
        entry.push('\n');
    }
    entry.push_str(sentence);
}

/// The warnings a loader left on a document (for example
/// [`NESTING_WARNING`]), each a sentence that reads well aloud.
pub fn warnings(meta: &DocumentMeta) -> Vec<String> {
    meta.properties
        .get(WARNINGS_PROPERTY)
        .map(|w| w.lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

/// Version of the canonical text the loaders produce. Bumped whenever a
/// loader's output changes, which invalidates cached documents.
pub const CANONICAL_VERSION: u32 = 8;

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

    /// The web address this source names: a [`Source::Url`], or a path
    /// written as `http://` or `https://` (how the reader passes one on).
    pub fn url(&self) -> Option<String> {
        match self {
            Source::Url(u) => Some(u.clone()),
            Source::Path(p) => {
                let s = p.to_string_lossy();
                let lower = s.to_ascii_lowercase();
                (lower.starts_with("http://") || lower.starts_with("https://"))
                    .then(|| s.replace('\\', "/"))
            }
            Source::Bytes { .. } => None,
        }
    }

    /// Reads the source's bytes (paths, archive members such as
    /// `book.zip!chapter.xml`, and in-memory data).
    pub fn read(&self) -> Result<Vec<u8>, LoadError> {
        match self {
            Source::Path(p) => archive::read_path(p).map_err(|e| LoadError::Io(p.clone(), e)),
            Source::Bytes { data, .. } => Ok(data.clone()),
            Source::Url(u) => Err(LoadError::Unsupported(format!("URL sources: {u}"))),
        }
    }

    /// The first `n` bytes of the source (fewer when it is shorter), read
    /// without reading the rest of a file.
    pub fn read_head(&self, n: usize) -> Result<Vec<u8>, LoadError> {
        use std::io::Read;
        match self {
            Source::Path(p) if !p.exists() && archive::split_member(p).is_some() => {
                let mut bytes = archive::read_path(p).map_err(|e| LoadError::Io(p.clone(), e))?;
                bytes.truncate(n);
                Ok(bytes)
            }
            Source::Path(p) => {
                let io = |e| LoadError::Io(p.clone(), e);
                let file = std::fs::File::open(p).map_err(io)?;
                let mut head = Vec::with_capacity(n.min(1 << 16));
                file.take(u64::try_from(n).unwrap_or(u64::MAX))
                    .read_to_end(&mut head)
                    .map_err(io)?;
                Ok(head)
            }
            Source::Bytes { data, .. } => Ok(data[..data.len().min(n)].to_vec()),
            Source::Url(u) => Err(LoadError::Unsupported(format!("URL sources: {u}"))),
        }
    }
}

/// Where footnotes go in the canonical text (star's `footnote_mode`, whose
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

/// How tracked changes are read: Word insertions and deletions, OpenDocument
/// change regions, and RTF revision marks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RevisionMode {
    /// The text as it reads with every change accepted: insertions are
    /// plain text, deletions are left out.
    #[default]
    Final,
    /// Each change is said where it is: "(inserted by Ada Example: new
    /// words)" with the new words under an `Underline` marker, "(deleted by
    /// Ada Example: old words)" with the old words under a `Strikethrough`
    /// marker, and moves as "moved here" and "moved away". The marker's
    /// label is the phrase, and its reference the change's date when there
    /// is one.
    Marked,
}

/// Options that affect how a document is loaded (part of the cache key,
/// except [`progress`](Self::progress)).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct LoadOptions {
    /// Drop code blocks (text under block `Code` markers) from the canonical
    /// text. Inline code is kept, as in star.
    pub skip_code: bool,
    /// Where footnotes go.
    pub footnotes: FootnoteMode,
    /// Text recognition for scanned pages and images.
    pub ocr: OcrOptions,
    /// How tracked changes are read.
    pub revisions: RevisionMode,
    /// Say the name of each command a loader leaves out, where it was
    /// ("(command hl)" in LaTeX), for high verbosity. Off: only the
    /// document's warnings name them.
    pub name_skipped_commands: bool,
    /// Keep SSML-style pause markup (`<break time="500ms"/>`) as text
    /// instead of reading it as a pause (`[speech] markup_pauses = false`,
    /// for documents that quote SSML). See [`pause_markup`].
    pub keep_pause_markup: bool,
    /// The braille code BRF files are read in (`[braille] brf_code`).
    pub brf_code: BrfCode,
    /// Progress reports and cancelling (not part of the cache key).
    #[serde(skip)]
    pub progress: Progress,
}

/// Which OCR engine reads scanned pages (the `ocr_engine` setting).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OcrEngineChoice {
    /// ocrs (in process) for English, Tesseract for other languages.
    #[default]
    Auto,
    /// Always ocrs.
    Ocrs,
    /// Always Tesseract.
    Tesseract,
    /// PaddleOCR's Latin model (experimental).
    Paddle,
}

impl OcrEngineChoice {
    /// Parses `auto`, `ocrs`, `tesseract`, or `paddle` (any case).
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "" | "auto" => Some(Self::Auto),
            "ocrs" => Some(Self::Ocrs),
            "tesseract" => Some(Self::Tesseract),
            "paddle" | "paddleocr" => Some(Self::Paddle),
            _ => None,
        }
    }
}

/// How scanned pages and images are read (ADR-0026).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct OcrOptions {
    /// Recognize text in pages that have none (on by default).
    pub enabled: bool,
    /// The text's language: Tesseract codes (`eng`, `fra+eng`) or language
    /// tags (`fr`); empty means the document's own language, else English
    /// (the `ocr_lang` setting).
    pub lang: String,
    /// The engine.
    pub engine: OcrEngineChoice,
}

impl Default for OcrOptions {
    fn default() -> Self {
        OcrOptions {
            enabled: true,
            lang: String::new(),
            engine: OcrEngineChoice::Auto,
        }
    }
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
    /// The file is not text (and no loader for its kind is available):
    /// the file's name and what it looks like.
    #[error("{0} is not a text file; it looks like {1}. textweaver cannot read it as text.")]
    Binary(String, &'static str),
}

/// Turns a source into a document.
pub trait Loader: Send + Sync {
    /// Stable id ("text", "markdown", "html").
    fn id(&self) -> &'static str;
    /// Lowercase extensions this loader claims, without dots.
    fn extensions(&self) -> &'static [&'static str];
    /// The extensions a folder scan (the library, `tw convert` on a
    /// folder, a hot folder) treats as documents: by default all of
    /// [`extensions`](Self::extensions). Loaders for files that are usually
    /// not documents (pictures, archives, any XML) claim fewer, so a folder
    /// of photos or zips does not fill the library; they still open when
    /// asked for by name.
    fn scan_extensions(&self) -> &'static [&'static str] {
        self.extensions()
    }
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
        r.register(Box::new(RtfLoader));
        r.register(Box::new(OdtLoader));
        r.register(Box::new(LatexLoader));
        r.register(Box::new(EmlLoader));
        r.register(Box::new(MhtmlLoader));
        #[cfg(feature = "pdf")]
        r.register(Box::new(PdfLoader));
        #[cfg(feature = "images")]
        r.register(Box::new(ImageLoader));
        r.register(Box::new(DaisyLoader));
        r.register(Box::new(BrfLoader));
        r.register(Box::new(PptxLoader));
        r.register(Box::new(SheetLoader));
        r.register(Box::new(ArchiveLoader));
        r.register(Box::new(JsonLoader));
        r.register(Box::new(NotebookLoader));
        r.register(Box::new(SvgLoader));
        r.register(Box::new(MathMlLoader));
        r
    }

    /// The built-in loaders plus Pandoc for the formats textweaver has no
    /// reader for (used when Pandoc is installed), stopped after `timeout`
    /// on one document (`None`: `TEXTWEAVER_PANDOC_TIMEOUT`, else two
    /// minutes).
    #[cfg(feature = "pandoc")]
    pub fn with_pandoc(timeout: Option<std::time::Duration>) -> Self {
        let mut r = Registry::with_builtins();
        r.register(Box::new(
            timeout.map_or_else(PandocLoader::default, PandocLoader::with_timeout),
        ));
        r
    }

    /// Adds a loader.
    pub fn register(&mut self, loader: Box<dyn Loader>) {
        self.loaders.push(loader);
    }

    /// Removes the loaders registered under `id` (to drop an optional
    /// loader, or to register it again with other settings).
    pub fn remove(&mut self, id: &str) {
        self.loaders.retain(|l| l.id() != id);
    }

    /// Ids of the registered loaders.
    pub fn ids(&self) -> Vec<&'static str> {
        self.loaders.iter().map(|l| l.id()).collect()
    }

    /// Every extension a folder scan treats as a document, sorted: those
    /// some available loader lists in its
    /// [`scan_extensions`](Loader::scan_extensions). Pictures, archives,
    /// and XML other than DAISY are left out, though they open by name.
    pub fn extensions(&self) -> Vec<&'static str> {
        let mut v: Vec<&'static str> = self
            .loaders
            .iter()
            .filter(|l| l.available())
            .flat_map(|l| l.scan_extensions().iter().copied())
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
            // Availability last: for Pandoc it runs a program once.
            if l.extensions().contains(&hint.as_str())
                && best.is_none_or(|b| l.priority() > b.priority())
                && l.available()
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
    /// unknown extensions. A web address is fetched first (feature `url`).
    pub fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        if let Some(url) = source.url() {
            return self.load_url(&url, source, options);
        }
        self.resolve(source).load(source, options)
    }

    #[cfg(feature = "url")]
    fn load_url(
        &self,
        url: &str,
        source: &Source,
        options: &LoadOptions,
    ) -> Result<Document, LoadError> {
        web::load(self, url, source, options)
    }

    #[cfg(not(feature = "url"))]
    fn load_url(&self, url: &str, _: &Source, _: &LoadOptions) -> Result<Document, LoadError> {
        Err(LoadError::Unsupported(format!(
            "this build of textweaver cannot open web addresses such as {url}"
        )))
    }
}

static CACHE_DIR: std::sync::RwLock<Option<PathBuf>> = std::sync::RwLock::new(None);

/// Keeps downloaded web files and OCR results under `dir` instead of
/// textweaver's cache folder (the app's own paths, or tests); `None` goes
/// back to the default.
pub fn set_cache_dir(dir: Option<PathBuf>) {
    *CACHE_DIR.write().unwrap_or_else(|p| p.into_inner()) = dir;
}

/// textweaver's cache folder, as the store places it (`TEXTWEAVER_HOME`'s
/// `cache/` when that is set): downloaded web files and OCR results live
/// under it.
#[cfg_attr(not(any(feature = "images", feature = "url")), allow(dead_code))]
pub(crate) fn cache_dir() -> Option<PathBuf> {
    if let Some(dir) = CACHE_DIR.read().unwrap_or_else(|p| p.into_inner()).clone() {
        return Some(dir);
    }
    if let Some(home) =
        std::env::var_os("TEXTWEAVER_HOME").filter(|v| !v.to_string_lossy().trim().is_empty())
    {
        return Some(PathBuf::from(home).join("cache"));
    }
    directories::ProjectDirs::from("org", "leavesofgrass", "textweaver")
        .map(|d| d.cache_dir().to_owned())
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
/// turned into `\n`. A source that is not text is refused
/// ([`LoadError::Binary`]).
pub fn source_text(source: &Source) -> Result<String, LoadError> {
    Ok(decode_source(source, None)?.text)
}

/// The source decoded with an optional declared encoding label (see
/// [`encoding::decode`]), line endings normalized to `\n`.
pub fn decode_source(
    source: &Source,
    declared: Option<&str>,
) -> Result<encoding::Decoded, LoadError> {
    // Binary detection looks only at the first `SNIFF_BYTES`, so a file is
    // refused before the rest of it is read (a 2 GB video costs 8 KB).
    let head = source.read_head(encoding::SNIFF_BYTES)?;
    if let Some(kind) = encoding::binary_kind(&head) {
        return Err(LoadError::Binary(source_name(source), kind));
    }
    let bytes = source.read()?;
    Ok(decode_bytes(&bytes, declared))
}

/// How a source is named in messages: its file name, else "This document".
fn source_name(source: &Source) -> String {
    title_from_path(source).unwrap_or_else(|| "This document".to_owned())
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
        // Pictures, archives, and plain XML open by name but are not
        // documents to a folder scan.
        for not in ["png", "zip", "xml", "json", "svg"] {
            assert!(!r.extensions().contains(&not), "{not}");
        }
        assert!(r.extensions().contains(&"opf"));
        let mut ids = vec![
            "text", "markdown", "html", "epub", "docx", "rtf", "odt", "latex", "eml", "mhtml",
        ];
        if cfg!(feature = "pdf") {
            ids.push("pdf");
        }
        if cfg!(feature = "images") {
            ids.push("image");
        }
        ids.extend([
            "daisy", "pptx", "sheet", "archive", "json", "notebook", "svg", "mathml",
        ]);
        // Pandoc is never a built-in (see `Registry::with_pandoc`).
        ids.extend(["low", "high"]);
        assert_eq!(r.ids(), ids);
    }
}
