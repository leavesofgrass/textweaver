//! Bulk conversion (ADR-0016): convert files and whole folder trees on all
//! cores, mirroring the tree and skipping outputs newer than their source,
//! with a timing summary; the engine behind `tw convert`.
//!
//! ```no_run
//! use textweaver_convert::{ConvertOptions, Converter, OutputFormat};
//!
//! let options = ConvertOptions {
//!     to: OutputFormat::Html,
//!     out_dir: Some("site".into()),
//!     ..ConvertOptions::default()
//! };
//! let converter = Converter::new(options).expect("templates");
//! let summary = converter.run(&["notes".into()]).expect("plan");
//! println!("{}", summary.sentence());
//! ```
//!
//! **Pipeline per file.** Markdown sources are rendered straight from their
//! text by `textweaver-render` (engine, flavor, math, templates) or copied
//! for Markdown output. Every other source is loaded by the
//! `textweaver-formats` registry into a `Document`, exported to Markdown for
//! HTML, or given to a native writer (EPUB, DOCX, BRF, PDF). Formats with no
//! native loader go through Pandoc when it is installed.
//!
//! **Memory.** Each worker holds one document at a time; the batch keeps
//! only paths and per-file results, so memory is proportional to the
//! largest documents in flight, not to the batch.
//!
//! **Incremental.** An output newer than its source is skipped unless
//! `force` is set; outputs are written to a temporary file and renamed, so
//! an interrupted run never leaves a half-written file behind.
//!
//! Owner: Agent L.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use serde::Serialize;
use textweaver_formats::{LoadOptions, Loader, MarkdownLoader, Registry, Source};
use textweaver_render::{
    EmbedMode, FsResolver, PageOptions, RenderOptions, Resolver, TemplateChoice, Templates,
};
use textweaver_text::Document;

pub mod pandoc;
mod plan;
pub mod watch;
pub mod writer;

pub use plan::{Job, Plan};
pub use watch::{WatchEvent, WatchOptions, watch};
pub use writer::{DocumentWriter, WriteError, WriteOptions, Writers};

/// An output format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Markdown (`.md`).
    Markdown,
    /// HTML (`.html`), through a template.
    Html,
    /// Plain text (`.txt`): the canonical text.
    Text,
    /// EPUB 3 (`.epub`).
    Epub,
    /// Word (`.docx`).
    Docx,
    /// Braille Ready Format (`.brf`).
    Brf,
    /// Tagged PDF (`.pdf`).
    Pdf,
}

impl OutputFormat {
    /// Every format, in display order.
    pub const ALL: [OutputFormat; 7] = [
        OutputFormat::Markdown,
        OutputFormat::Html,
        OutputFormat::Text,
        OutputFormat::Epub,
        OutputFormat::Docx,
        OutputFormat::Brf,
        OutputFormat::Pdf,
    ];

    /// Parses a name or extension (`md`, `markdown`, `html`, `htm`, `txt`,
    /// `text`, `epub`, `docx`, `brf`, `braille`, `pdf`).
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().trim_start_matches('.').to_ascii_lowercase().as_str() {
            "md" | "markdown" => Some(OutputFormat::Markdown),
            "html" | "htm" => Some(OutputFormat::Html),
            "txt" | "text" | "plain" => Some(OutputFormat::Text),
            "epub" => Some(OutputFormat::Epub),
            "docx" | "word" => Some(OutputFormat::Docx),
            "brf" | "braille" => Some(OutputFormat::Brf),
            "pdf" => Some(OutputFormat::Pdf),
            _ => None,
        }
    }

    /// The output file extension, without a dot.
    pub fn extension(self) -> &'static str {
        match self {
            OutputFormat::Markdown => "md",
            OutputFormat::Html => "html",
            OutputFormat::Text => "txt",
            OutputFormat::Epub => "epub",
            OutputFormat::Docx => "docx",
            OutputFormat::Brf => "brf",
            OutputFormat::Pdf => "pdf",
        }
    }

    /// The name people hear: "Markdown", "HTML", "plain text", …
    pub fn label(self) -> &'static str {
        match self {
            OutputFormat::Markdown => "Markdown",
            OutputFormat::Html => "HTML",
            OutputFormat::Text => "plain text",
            OutputFormat::Epub => "EPUB",
            OutputFormat::Docx => "Word",
            OutputFormat::Brf => "braille",
            OutputFormat::Pdf => "PDF",
        }
    }

    /// True for the formats written by the native writers.
    pub fn needs_writer(self) -> bool {
        matches!(
            self,
            OutputFormat::Epub | OutputFormat::Docx | OutputFormat::Brf | OutputFormat::Pdf
        )
    }
}

/// Settings for a conversion run.
#[derive(Clone, Debug)]
pub struct ConvertOptions {
    /// Output format.
    pub to: OutputFormat,
    /// Output folder; `None` writes next to each source.
    pub out_dir: Option<PathBuf>,
    /// Markdown rendering (engine, flavor, math, sanitization, embeds).
    pub render: RenderOptions,
    /// Page template for HTML.
    pub template: TemplateChoice,
    /// A folder of user templates, loaded before `template` is resolved.
    pub template_dir: Option<PathBuf>,
    /// Include a table of contents in HTML pages.
    pub toc: bool,
    /// Worker threads; `None` uses every core.
    pub jobs: Option<usize>,
    /// Convert even when the output is newer than the source.
    pub force: bool,
    /// Loader options for non-Markdown sources.
    pub load: LoadOptions,
    /// Use Pandoc for formats with no native loader, when installed.
    pub pandoc: bool,
    /// Options passed to the native writers.
    pub write: WriteOptions,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        ConvertOptions {
            to: OutputFormat::Markdown,
            out_dir: None,
            render: RenderOptions::default(),
            template: TemplateChoice::default(),
            template_dir: None,
            toc: true,
            jobs: None,
            force: false,
            load: LoadOptions::default(),
            pandoc: true,
            write: WriteOptions::default(),
        }
    }
}

/// Conversion failures that stop a run before it starts.
#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    /// An input does not exist.
    #[error("{0} does not exist")]
    Missing(PathBuf),
    /// The output format has no writer in this build.
    #[error("{0} output is not available in this build yet; choose md, html, or txt")]
    Unavailable(&'static str),
    /// A template could not be loaded.
    #[error(transparent)]
    Template(#[from] textweaver_render::RenderError),
    /// The worker pool could not start.
    #[error("cannot start worker threads: {0}")]
    Threads(String),
    /// Watching a folder failed.
    #[error("cannot watch {0}: {1}")]
    Watch(PathBuf, String),
    /// Reading or writing a file failed.
    #[error("{0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),
}

/// What happened to one file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", content = "reason", rename_all = "lowercase")]
pub enum Status {
    /// Written.
    Converted,
    /// The output was newer than the source.
    Skipped,
    /// Not written; the reason.
    Failed(String),
}

/// The outcome for one file.
#[derive(Clone, Debug, Serialize)]
pub struct FileResult {
    /// The source file.
    pub source: PathBuf,
    /// The output file (written or planned).
    pub output: PathBuf,
    /// What happened.
    #[serde(flatten)]
    pub status: Status,
    /// Bytes read from the source.
    pub bytes_in: u64,
    /// Bytes written.
    pub bytes_out: u64,
    /// Time spent on this file, in microseconds.
    pub micros: u64,
}

impl FileResult {
    fn failed(job: &Job, reason: impl Into<String>) -> Self {
        FileResult {
            source: job.source.clone(),
            output: job.output.clone(),
            status: Status::Failed(reason.into()),
            bytes_in: 0,
            bytes_out: 0,
            micros: 0,
        }
    }
}

/// Totals for a run.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Summary {
    /// Output format.
    pub format: Option<OutputFormat>,
    /// Files written.
    pub converted: usize,
    /// Files skipped because their output was newer.
    pub skipped: usize,
    /// Files that failed.
    pub failed: usize,
    /// Bytes read from converted sources.
    pub bytes_in: u64,
    /// Bytes written.
    pub bytes_out: u64,
    /// Wall-clock time of the run, in seconds.
    pub seconds: f64,
    /// Worker threads used.
    pub threads: usize,
    /// Every file, in plan order.
    pub files: Vec<FileResult>,
}

impl Summary {
    /// Files considered.
    pub fn total(&self) -> usize {
        self.converted + self.skipped + self.failed
    }

    /// Converted files per second of wall-clock time.
    pub fn files_per_second(&self) -> f64 {
        if self.seconds > 0.0 {
            self.converted as f64 / self.seconds
        } else {
            0.0
        }
    }

    /// The failures.
    pub fn failures(&self) -> impl Iterator<Item = &FileResult> {
        self.files
            .iter()
            .filter(|f| matches!(f.status, Status::Failed(_)))
    }

    /// One sentence that reads well aloud, for example "Converted 12 files
    /// to HTML in 0.4 seconds, 30 files per second. Skipped 3 unchanged
    /// files. No failures."
    pub fn sentence(&self) -> String {
        let label = self.format.map_or("the output format", OutputFormat::label);
        if self.converted == 0 && self.failed == 0 && self.skipped > 0 {
            return format!(
                "Nothing to convert: {} already up to date. Use --force to convert again.",
                if self.skipped == 1 {
                    "1 file is".to_owned()
                } else {
                    format!("all {} files are", self.skipped)
                }
            );
        }
        let mut s = format!(
            "Converted {} to {} in {}",
            count(self.converted, "file"),
            label,
            seconds(self.seconds)
        );
        if self.converted > 0 && self.seconds > 0.0 {
            s.push_str(&format!(
                ", {} files per second",
                round_sig(self.files_per_second())
            ));
        }
        s.push('.');
        if self.skipped > 0 {
            s.push_str(&format!(
                " Skipped {} whose output is up to date.",
                count(self.skipped, "file")
            ));
        }
        if self.failed == 0 {
            s.push_str(" No failures.");
        } else {
            s.push_str(&format!(" {} failed.", count(self.failed, "file")));
        }
        s
    }
}

fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

fn seconds(s: f64) -> String {
    if s < 0.1 {
        count((s * 1000.0).round() as usize, "millisecond")
    } else if (s - 1.0).abs() < 0.05 {
        "1 second".to_owned()
    } else {
        format!("{} seconds", round_sig(s))
    }
}

/// A number with about three significant digits, without trailing zeros.
fn round_sig(x: f64) -> String {
    if x >= 100.0 {
        format!("{}", x.round() as u64)
    } else if x >= 10.0 {
        format!("{:.1}", x).trim_end_matches(".0").to_owned()
    } else {
        let s = format!("{:.2}", x);
        s.trim_end_matches('0').trim_end_matches('.').to_owned()
    }
}

/// Converts files with one set of options; shared by all workers.
pub struct Converter {
    options: ConvertOptions,
    registry: Registry,
    templates: Templates,
    template: String,
    writers: Writers,
    /// Embed resolvers, one per input root (built on first use).
    resolvers: std::sync::Mutex<HashMap<PathBuf, std::sync::Arc<FsResolver>>>,
}

impl std::fmt::Debug for Converter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Converter")
            .field("options", &self.options)
            .field("template", &self.template)
            .finish()
    }
}

impl Converter {
    /// A converter with the built-in loaders, templates, and writers.
    pub fn new(options: ConvertOptions) -> Result<Self, ConvertError> {
        Self::with_writers(options, Writers::builtin())
    }

    /// A converter with the given writers.
    pub fn with_writers(options: ConvertOptions, writers: Writers) -> Result<Self, ConvertError> {
        if options.to.needs_writer() && writers.get(options.to).is_none() {
            return Err(ConvertError::Unavailable(options.to.label()));
        }
        let mut templates = Templates::builtin();
        if let Some(dir) = &options.template_dir {
            templates.load_dir(dir)?;
        }
        let template = templates.resolve(&options.template)?;
        Ok(Converter {
            options,
            registry: Registry::with_builtins(),
            templates,
            template,
            writers,
            resolvers: std::sync::Mutex::new(HashMap::new()),
        })
    }

    /// The options.
    pub fn options(&self) -> &ConvertOptions {
        &self.options
    }

    /// Lowercase extensions this converter reads: every native loader's,
    /// plus Pandoc's when Pandoc is enabled and installed.
    pub fn source_extensions(&self) -> Vec<&'static str> {
        let mut v = self.registry.extensions();
        if self.options.pandoc && pandoc::available() {
            v.extend(pandoc::EXTENSIONS.iter().copied());
        }
        v.sort_unstable();
        v.dedup();
        v
    }

    /// Plans the jobs for `inputs` (files and folders).
    pub fn plan(&self, inputs: &[PathBuf]) -> Result<Plan, ConvertError> {
        plan::plan(self, inputs)
    }

    /// Plans and runs a batch on the worker pool.
    pub fn run(&self, inputs: &[PathBuf]) -> Result<Summary, ConvertError> {
        let start = Instant::now();
        let plan = self.plan(inputs)?;
        self.execute(plan, start)
    }

    /// Runs planned jobs in parallel; results keep plan order.
    pub fn run_plan(&self, plan: Plan) -> Result<Summary, ConvertError> {
        self.execute(plan, Instant::now())
    }

    fn execute(&self, plan: Plan, start: Instant) -> Result<Summary, ConvertError> {
        use rayon::prelude::*;
        let threads = self
            .options
            .jobs
            .filter(|&n| n > 0)
            .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get()));
        // Up-to-date outputs are found first, on this thread: the checks
        // are two metadata calls each, and many threads issuing them at
        // once contend (measured on NTFS: 3.5 times slower on 12 threads).
        let mut files: Vec<Option<FileResult>> = Vec::with_capacity(plan.jobs.len());
        let mut todo: Vec<usize> = Vec::new();
        for (i, job) in plan.jobs.iter().enumerate() {
            if !self.options.force && is_up_to_date(&job.source, &job.output) {
                files.push(Some(FileResult {
                    source: job.source.clone(),
                    output: job.output.clone(),
                    status: Status::Skipped,
                    bytes_in: 0,
                    bytes_out: 0,
                    micros: 0,
                }));
            } else {
                files.push(None);
                todo.push(i);
            }
        }
        if !todo.is_empty() {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads.min(todo.len()))
                .thread_name(|i| format!("tw-convert-{i}"))
                .build()
                .map_err(|e| ConvertError::Threads(e.to_string()))?;
            let done: Vec<(usize, FileResult)> = pool.install(|| {
                todo.par_iter()
                    .map(|&i| (i, self.convert_now(&plan.jobs[i])))
                    .collect()
            });
            for (i, r) in done {
                files[i] = Some(r);
            }
        }
        let mut files: Vec<FileResult> = files.into_iter().flatten().collect();
        files.extend(plan.rejected);
        let mut s = Summary {
            format: Some(self.options.to),
            threads,
            ..Summary::default()
        };
        for f in &files {
            match f.status {
                Status::Converted => {
                    s.converted += 1;
                    s.bytes_in += f.bytes_in;
                    s.bytes_out += f.bytes_out;
                }
                Status::Skipped => s.skipped += 1,
                Status::Failed(_) => s.failed += 1,
            }
        }
        s.files = files;
        s.seconds = start.elapsed().as_secs_f64();
        Ok(s)
    }

    /// Converts one planned file: skips it when its output is newer,
    /// otherwise converts and writes it atomically. Never panics on a bad
    /// file; the reason goes into the result.
    pub fn convert_job(&self, job: &Job) -> FileResult {
        if !self.options.force && is_up_to_date(&job.source, &job.output) {
            return FileResult {
                source: job.source.clone(),
                output: job.output.clone(),
                status: Status::Skipped,
                bytes_in: 0,
                bytes_out: 0,
                micros: 0,
            };
        }
        self.convert_now(job)
    }

    /// Converts and writes one file, whatever the output's age.
    fn convert_now(&self, job: &Job) -> FileResult {
        let start = Instant::now();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.convert_bytes(job)
        }));
        let result = match outcome {
            Ok(Ok((bytes_in, data))) => match write_atomic(&job.output, &data) {
                Ok(()) => FileResult {
                    source: job.source.clone(),
                    output: job.output.clone(),
                    status: Status::Converted,
                    bytes_in,
                    bytes_out: data.len() as u64,
                    micros: 0,
                },
                Err(e) => FileResult::failed(job, format!("cannot write the output: {e}")),
            },
            Ok(Err(reason)) => FileResult::failed(job, reason),
            Err(_) => FileResult::failed(job, "internal error while converting this file"),
        };
        FileResult {
            micros: start.elapsed().as_micros() as u64,
            ..result
        }
    }

    /// The converted bytes of a job's source, and the source size.
    fn convert_bytes(&self, job: &Job) -> Result<(u64, Vec<u8>), String> {
        let ext = extension(&job.source);
        let is_markdown = MarkdownLoader.extensions().contains(&ext.as_str());
        let native = self
            .registry
            .loader_for(&Source::Path(job.source.clone()))
            .is_some();
        if is_markdown {
            let bytes = std::fs::read(&job.source).map_err(|e| format!("cannot read: {e}"))?;
            let text = decode(&bytes);
            return Ok((bytes.len() as u64, self.from_markdown(job, &text)?));
        }
        if !native && self.options.pandoc && pandoc::EXTENSIONS.contains(&ext.as_str()) {
            if !pandoc::available() {
                return Err(format!(
                    "no native reader for .{ext} files, and Pandoc is not installed"
                ));
            }
            let size = std::fs::metadata(&job.source).map_or(0, |m| m.len());
            let md = pandoc::to_markdown(&job.source, self.options.render.flavor)?;
            return Ok((size, self.from_markdown(job, &md)?));
        }
        let size = std::fs::metadata(&job.source).map_or(0, |m| m.len());
        let doc = self
            .registry
            .load(&Source::Path(job.source.clone()), &self.options.load)
            .map_err(|e| e.to_string())?;
        Ok((size, self.from_document(job, &doc, None)?))
    }

    /// Output for Markdown text (a Markdown file, or Pandoc's output).
    fn from_markdown(&self, job: &Job, text: &str) -> Result<Vec<u8>, String> {
        match self.options.to {
            OutputFormat::Markdown => Ok(text.as_bytes().to_vec()),
            OutputFormat::Html => {
                let resolver = self.resolver_for(job);
                let rendered = textweaver_render::render_with(
                    text,
                    &self.options.render,
                    resolver.as_deref().map(|r| r as &dyn Resolver),
                );
                self.page(job, &rendered)
            }
            _ => {
                let source = Source::Bytes {
                    data: text.as_bytes().to_vec(),
                    hint: "md".to_owned(),
                };
                let mut doc = MarkdownLoader
                    .load(&source, &self.options.load)
                    .map_err(|e| e.to_string())?;
                doc.meta.path = Some(job.source.clone());
                self.from_document(job, &doc, Some(text))
            }
        }
    }

    /// Output for a loaded document.
    fn from_document(
        &self,
        job: &Job,
        doc: &Document,
        markdown: Option<&str>,
    ) -> Result<Vec<u8>, String> {
        match self.options.to {
            OutputFormat::Text => {
                let mut t = doc.text().to_string();
                if !t.ends_with('\n') {
                    t.push('\n');
                }
                Ok(t.into_bytes())
            }
            OutputFormat::Markdown => Ok(textweaver_formats::to_markdown(doc).into_bytes()),
            OutputFormat::Html => {
                let md = match markdown {
                    Some(m) => m.to_owned(),
                    None => textweaver_formats::to_markdown(doc),
                };
                let mut rendered = textweaver_render::render(&md, &self.options.render);
                // A title or language the source declared (HTML <title>,
                // EPUB metadata) survives the trip through Markdown.
                if let Some(t) = &doc.meta.title {
                    rendered
                        .meta
                        .entry("title")
                        .or_insert_with(|| serde_json::Value::String(t.clone()));
                }
                if let Some(l) = &doc.meta.language {
                    rendered
                        .meta
                        .entry("lang")
                        .or_insert_with(|| serde_json::Value::String(l.clone()));
                }
                if let Some(a) = &doc.meta.author {
                    rendered
                        .meta
                        .entry("author")
                        .or_insert_with(|| serde_json::Value::String(a.clone()));
                }
                self.page(job, &rendered)
            }
            format => {
                let writer = self
                    .writers
                    .get(format)
                    .ok_or_else(|| format!("{} output is not available", format.label()))?;
                let options = WriteOptions {
                    title: doc.meta.title.clone().or_else(|| stem(&job.source)),
                    language: doc
                        .meta
                        .language
                        .clone()
                        .unwrap_or_else(|| self.options.write.language.clone()),
                    author: doc.meta.author.clone(),
                    ..self.options.write.clone()
                };
                let mut out = Vec::new();
                writer
                    .write(doc, &options, &mut out)
                    .map_err(|e| e.to_string())?;
                Ok(out)
            }
        }
    }

    fn page(&self, job: &Job, rendered: &textweaver_render::Rendered) -> Result<Vec<u8>, String> {
        let page = PageOptions {
            fallback_title: stem(&job.source),
            toc: self.options.toc,
            ..PageOptions::default()
        };
        self.templates
            .render(&self.template, rendered, &page)
            .map(String::into_bytes)
            .map_err(|e| e.to_string())
    }

    /// The embed resolver for a job's input root, when embeds are inlined.
    fn resolver_for(&self, job: &Job) -> Option<std::sync::Arc<FsResolver>> {
        if self.options.render.embeds != EmbedMode::Inline {
            return None;
        }
        let mut map = self.resolvers.lock().ok()?;
        Some(
            map.entry(job.root.clone())
                .or_insert_with(|| std::sync::Arc::new(FsResolver::new(job.root.clone())))
                .clone(),
        )
    }
}

/// Lowercase extension of a path, or empty.
pub(crate) fn extension(p: &Path) -> String {
    p.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn stem(p: &Path) -> Option<String> {
    p.file_stem().map(|s| s.to_string_lossy().into_owned())
}

/// Markdown bytes as text: UTF-8 (a BOM dropped), line endings kept.
fn decode(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    text.strip_prefix('\u{feff}').unwrap_or(&text).to_owned()
}

/// True when `output` exists and was modified at or after `source`.
pub fn is_up_to_date(source: &Path, output: &Path) -> bool {
    let mtime = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    match (mtime(source), mtime(output)) {
        (Some(s), Some(o)) => o >= s,
        _ => false,
    }
}

/// Writes `data` to a temporary file beside `path`, then renames it over
/// `path`, creating parent folders as needed.
pub fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let tmp_name = format!(
        ".{}.{}-{}.tmp",
        path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
        std::process::id(),
        nanos
    );
    let tmp = path.with_file_name(tmp_name);
    let result = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(data)?;
        f.flush()?;
        drop(f);
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Wall-clock duration helper for callers that time their own work.
pub fn elapsed_seconds(since: Instant) -> f64 {
    let d: Duration = since.elapsed();
    d.as_secs_f64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_parse_and_name() {
        for f in OutputFormat::ALL {
            assert_eq!(OutputFormat::parse(f.extension()), Some(f));
        }
        assert_eq!(OutputFormat::parse("braille"), Some(OutputFormat::Brf));
        assert_eq!(OutputFormat::parse(".MD"), Some(OutputFormat::Markdown));
        assert_eq!(OutputFormat::parse("rtf"), None);
    }

    #[test]
    fn summary_sentence_reads_well() {
        let s = Summary {
            format: Some(OutputFormat::Html),
            converted: 12,
            skipped: 1,
            failed: 0,
            seconds: 0.4,
            ..Summary::default()
        };
        assert_eq!(
            s.sentence(),
            "Converted 12 files to HTML in 0.4 seconds, 30 files per second. Skipped 1 file whose output is up to date. No failures."
        );
        let one = Summary {
            format: Some(OutputFormat::Text),
            converted: 1,
            failed: 2,
            seconds: 0.012,
            ..Summary::default()
        };
        assert_eq!(
            one.sentence(),
            "Converted 1 file to plain text in 12 milliseconds, 83.3 files per second. 2 files failed."
        );
    }

    #[test]
    fn writer_formats_need_a_writer() {
        let err = Converter::new(ConvertOptions {
            to: OutputFormat::Epub,
            ..ConvertOptions::default()
        })
        .unwrap_err();
        assert_eq!(
            err.to_string(),
            "EPUB output is not available in this build yet; choose md, html, or txt"
        );
        let skipped = Summary {
            format: Some(OutputFormat::Html),
            skipped: 3,
            ..Summary::default()
        };
        assert_eq!(
            skipped.sentence(),
            "Nothing to convert: all 3 files are already up to date. Use --force to convert again."
        );
    }
}
