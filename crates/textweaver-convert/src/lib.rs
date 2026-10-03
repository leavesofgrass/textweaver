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
//! native loader go through Pandoc when it is installed, by way of the
//! formats crate's Pandoc loader: the one Pandoc path in textweaver, run with
//! `--sandbox`, its output pipes read on their own threads, and a timeout
//! (`TEXTWEAVER_PANDOC` names the program, `TEXTWEAVER_PANDOC_TIMEOUT` or
//! [`ConvertOptions::pandoc_timeout`] the time limit).
//!
//! **Citations.** Pandoc citations in Markdown sources (`[@doe2020]`) are
//! formatted with `textweaver-cite` in a CSL style, and a References
//! section is appended, for every output but Markdown (see [`citations`]
//! and ADR-0019).
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
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

use serde::Serialize;
use textweaver_formats::{LoadOptions, Loader, MarkdownLoader, Registry, Source};
use textweaver_render::{
    EmbedMode, FsResolver, PageOptions, RenderOptions, Resolver, TemplateChoice, Templates,
};
use textweaver_text::Document;

pub mod citations;
mod plan;
pub mod watch;
pub mod writer;

pub use citations::CitationOptions;
pub use plan::{Job, Plan};
pub use watch::{WatchEvent, WatchOptions, watch};
pub use writer::{
    BrailleGrade, BrailleOptions, BrailleTableFormat, EpubOptions, MathCode, PageSize, PdfOptions,
    WriteError, WriteOptions, WriteReport, Writer, Writers,
};

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
        match s
            .trim()
            .trim_start_matches('.')
            .to_ascii_lowercase()
            .as_str()
        {
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
    /// How long Pandoc may run on one file; `None` uses
    /// `TEXTWEAVER_PANDOC_TIMEOUT` or two minutes.
    pub pandoc_timeout: Option<Duration>,
    /// Options passed to the native writers.
    pub write: WriteOptions,
    /// How Pandoc citations in Markdown are formatted.
    pub citations: CitationOptions,
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
            pandoc_timeout: None,
            write: WriteOptions::default(),
            citations: CitationOptions::default(),
        }
    }
}

/// Conversion failures that stop a run before it starts.
#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    /// An input does not exist.
    #[error("{0} does not exist")]
    Missing(PathBuf),
    /// The output format has no writer in this converter.
    #[error("{0} output is not available in this build; choose md, html, or txt")]
    Unavailable(&'static str),
    /// The output format cannot be written on this system (PDF output with
    /// no font); the message says what to do.
    #[error("{0}")]
    Output(String),
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
    /// What the writer wants the user to know about a converted file (an
    /// image that could not be embedded, characters braille cannot show),
    /// each a sentence that reads well aloud.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl FileResult {
    fn failed(job: &Job, reason: impl Into<String>) -> Self {
        FileResult {
            status: Status::Failed(reason.into()),
            ..FileResult::skipped(job)
        }
    }

    fn skipped(job: &Job) -> Self {
        FileResult {
            source: job.source.clone(),
            output: job.output.clone(),
            status: Status::Skipped,
            bytes_in: 0,
            bytes_out: 0,
            micros: 0,
            warnings: Vec::new(),
        }
    }
}

/// One converted file in memory, before it is written.
struct Output {
    /// Bytes read from the source.
    bytes_in: u64,
    /// The output file's bytes.
    data: Vec<u8>,
    /// The writer's warnings.
    warnings: Vec<String>,
}

impl Output {
    fn new(data: Vec<u8>) -> Self {
        Output {
            bytes_in: 0,
            data,
            warnings: Vec::new(),
        }
    }

    /// The same output, recording the source size.
    fn read_from(self, bytes_in: u64) -> Self {
        Output { bytes_in, ..self }
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
    /// Converted files that came with writer warnings.
    pub warned: usize,
    /// Bytes read from converted sources.
    pub bytes_in: u64,
    /// Bytes written.
    pub bytes_out: u64,
    /// Wall-clock time of the run, in seconds.
    pub seconds: f64,
    /// Worker threads used.
    pub threads: usize,
    /// Files never started because the run was canceled
    /// ([`Converter::run_plan_with`]); they are not in `files`.
    pub canceled: usize,
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

    /// The converted files that came with writer warnings.
    pub fn warnings(&self) -> impl Iterator<Item = &FileResult> {
        self.files.iter().filter(|f| !f.warnings.is_empty())
    }

    /// One sentence that reads well aloud, for example "Converted 12 files
    /// to HTML in 0.4 seconds, 30 files per second. Skipped 3 unchanged
    /// files. No failures."
    ///
    /// A canceled run says so first: "Stopped. Converted 3 files to PDF
    /// ... 45 files were not converted."
    pub fn sentence(&self) -> String {
        let label = self.format.map_or("the output format", OutputFormat::label);
        if self.canceled > 0 {
            let mut s = format!(
                "Stopped. Converted {} to {} in {}.",
                count(self.converted, "file"),
                label,
                seconds(self.seconds)
            );
            if self.skipped > 0 {
                s.push_str(&format!(" {} up to date.", count(self.skipped, "file")));
            }
            if self.failed > 0 {
                s.push_str(&format!(" {} failed.", count(self.failed, "file")));
            }
            s.push_str(&format!(
                " {} not converted.",
                if self.canceled == 1 {
                    "1 file was".to_owned()
                } else {
                    format!("{} files were", self.canceled)
                }
            ));
            return s;
        }
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
        if self.warned == 1 {
            s.push_str(" 1 file has warnings.");
        } else if self.warned > 1 {
            s.push_str(&format!(" {} files have warnings.", self.warned));
        }
        if self.failed == 0 {
            s.push_str(" No failures.");
        } else {
            s.push_str(&format!(" {} failed.", count(self.failed, "file")));
        }
        s
    }
}

/// The name of the report a batch leaves in its output folder.
pub const REPORT_FILE: &str = "conversion-report.txt";

impl Summary {
    /// The report of this run as plain text, for [`REPORT_FILE`]: the
    /// summary sentence, then each failure and each file with warnings.
    ///
    /// Every entry starts with the file's name and what happened, so the
    /// key fact is at the start of the line on a braille display; the full
    /// path follows on its own line. `written` is when the report is
    /// written, given as a UTC time on the last line.
    pub fn report_text(&self, written: SystemTime) -> String {
        let mut out = String::new();
        out.push_str(&self.sentence());
        out.push('\n');
        let failures: Vec<&FileResult> = self.failures().collect();
        if !failures.is_empty() {
            out.push_str(&format!("\nFailed, {}:\n", count(failures.len(), "file")));
            for f in failures {
                let reason = match &f.status {
                    Status::Failed(r) => r.as_str(),
                    _ => "",
                };
                out.push_str(&format!("{}: {reason}\n", file_name(&f.source)));
                out.push_str(&format!("  {}\n", f.source.display()));
            }
        }
        let warned: Vec<&FileResult> = self.warnings().collect();
        if !warned.is_empty() {
            out.push_str(&format!("\nWarnings, {}:\n", count(warned.len(), "file")));
            for f in warned {
                for w in &f.warnings {
                    out.push_str(&format!("{}: {w}\n", file_name(&f.source)));
                }
                out.push_str(&format!("  {}\n", f.source.display()));
            }
        }
        let (y, mo, d, h, mi, _) = watch::utc_parts(written);
        out.push_str(&format!(
            "\nReport written {y:04}-{mo:02}-{d:02} at {h:02}:{mi:02} UTC.\n"
        ));
        out
    }

    /// Writes [`report_text`](Self::report_text) to [`REPORT_FILE`] in
    /// `dir`, replacing any earlier report whole (never merged with it, so
    /// entries from an older run cannot come back), and returns its path.
    /// The file is written to a temporary name and renamed, so a reader
    /// never sees half a report.
    pub fn write_report(&self, dir: &Path) -> std::io::Result<PathBuf> {
        let path = dir.join(REPORT_FILE);
        write_atomic(&path, self.report_text(SystemTime::now()).as_bytes())?;
        Ok(path)
    }
}

/// A file's name for a report line, or its whole path when it has none.
fn file_name(p: &Path) -> String {
    p.file_name().map_or_else(
        || p.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
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

/// What to do when PDF output finds no font.
const NO_FONT: &str = "PDF output needs a font, and none was found. Install Atkinson Hyperlegible, Verdana, Arial, DejaVu Sans, or Noto Sans, or name a TrueType font file with the PDF font option (tw convert --pdf-font) or the TEXTWEAVER_PDF_FONT environment variable.";

/// Converts files with one set of options; shared by all workers.
pub struct Converter {
    options: ConvertOptions,
    registry: Registry,
    templates: Templates,
    template: String,
    writers: Writers,
    citations: citations::Citations,
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
        if options.to == OutputFormat::Pdf {
            // Once, before any file: without a font every file would fail.
            textweaver_writers::pdf::check_fonts(&options.write).map_err(|e| match e {
                WriteError::NoFont => ConvertError::Output(NO_FONT.to_owned()),
                other => ConvertError::Output(format!("Cannot write PDF files: {other}.")),
            })?;
        }
        let mut templates = Templates::builtin();
        if let Some(dir) = &options.template_dir {
            templates.load_dir(dir)?;
        }
        let template = templates.resolve(&options.template)?;
        let citations =
            citations::Citations::new(options.citations.clone()).map_err(ConvertError::Output)?;
        let registry = if options.pandoc {
            Registry::with_pandoc(options.pandoc_timeout)
        } else {
            Registry::with_builtins()
        };
        Ok(Converter {
            options,
            registry,
            templates,
            template,
            writers,
            citations,
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
        self.registry.extensions()
    }

    /// Plans the jobs for `inputs` (files and folders).
    pub fn plan(&self, inputs: &[PathBuf]) -> Result<Plan, ConvertError> {
        plan::plan(self, inputs)
    }

    /// Plans and runs a batch on the worker pool.
    pub fn run(&self, inputs: &[PathBuf]) -> Result<Summary, ConvertError> {
        let start = Instant::now();
        let plan = self.plan(inputs)?;
        self.execute(plan, start, &|_| {}, &AtomicBool::new(false))
    }

    /// Runs planned jobs in parallel; results keep plan order.
    pub fn run_plan(&self, plan: Plan) -> Result<Summary, ConvertError> {
        self.execute(plan, Instant::now(), &|_| {}, &AtomicBool::new(false))
    }

    /// Runs planned jobs in parallel like [`run_plan`](Self::run_plan),
    /// telling `on_file` about each file as it finishes and stopping when
    /// `cancel` is set.
    ///
    /// `on_file` is called once for every file in the plan (converted,
    /// skipped because it is up to date, failed, or rejected by the plan),
    /// from the worker threads and in no particular order, so a caller can
    /// count them against [`Plan::len`] for progress. It must be quick.
    ///
    /// Workers look at `cancel` before each file: a file already being
    /// converted finishes and is written whole (outputs are written to a
    /// temporary file and renamed), and no file is started after it is
    /// set. Files never started are counted in [`Summary::canceled`] and
    /// left out of [`Summary::files`]; `on_file` is not called for them.
    ///
    /// ```no_run
    /// use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    /// use textweaver_convert::{ConvertOptions, Converter};
    ///
    /// let converter = Converter::new(ConvertOptions::default()).expect("templates");
    /// let plan = converter.plan(&["notes".into()]).expect("plan");
    /// let total = plan.len();
    /// let done = AtomicUsize::new(0);
    /// let cancel = AtomicBool::new(false);
    /// let summary = converter
    ///     .run_plan_with(
    ///         plan,
    ///         |_file| {
    ///             let n = done.fetch_add(1, Ordering::Relaxed) + 1;
    ///             eprintln!("{n} of {total}");
    ///         },
    ///         &cancel,
    ///     )
    ///     .expect("threads");
    /// println!("{}", summary.sentence());
    /// ```
    pub fn run_plan_with(
        &self,
        plan: Plan,
        on_file: impl Fn(&FileResult) + Sync,
        cancel: &AtomicBool,
    ) -> Result<Summary, ConvertError> {
        self.execute(plan, Instant::now(), &on_file, cancel)
    }

    fn execute(
        &self,
        plan: Plan,
        start: Instant,
        on_file: &(dyn Fn(&FileResult) + Sync),
        cancel: &AtomicBool,
    ) -> Result<Summary, ConvertError> {
        use rayon::prelude::*;
        for r in &plan.rejected {
            on_file(r);
        }
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
        let mut canceled = 0;
        for (i, job) in plan.jobs.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                files.push(None);
                canceled += 1;
            } else if !self.options.force && is_up_to_date(&job.source, &job.output) {
                let r = FileResult::skipped(job);
                on_file(&r);
                files.push(Some(r));
            } else {
                files.push(None);
                todo.push(i);
            }
        }
        if !todo.is_empty() && !cancel.load(Ordering::Relaxed) {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads.min(todo.len()))
                .thread_name(|i| format!("tw-convert-{i}"))
                .build()
                .map_err(|e| ConvertError::Threads(e.to_string()))?;
            let done: Vec<(usize, FileResult)> = pool.install(|| {
                todo.par_iter()
                    .filter_map(|&i| {
                        if cancel.load(Ordering::Relaxed) {
                            return None;
                        }
                        let r = self.convert_now(&plan.jobs[i]);
                        on_file(&r);
                        Some((i, r))
                    })
                    .collect()
            });
            canceled += todo.len() - done.len();
            for (i, r) in done {
                files[i] = Some(r);
            }
        } else {
            canceled += todo.len();
        }
        let mut files: Vec<FileResult> = files.into_iter().flatten().collect();
        files.extend(plan.rejected);
        let mut s = Summary {
            format: Some(self.options.to),
            threads,
            canceled,
            ..Summary::default()
        };
        for f in &files {
            match f.status {
                Status::Converted => {
                    s.converted += 1;
                    s.bytes_in += f.bytes_in;
                    s.bytes_out += f.bytes_out;
                    if !f.warnings.is_empty() {
                        s.warned += 1;
                    }
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
            return FileResult::skipped(job);
        }
        self.convert_now(job)
    }

    /// Converts and writes one file, whatever the output's age.
    fn convert_now(&self, job: &Job) -> FileResult {
        let start = Instant::now();
        let outcome =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.convert_bytes(job)));
        let result = match outcome {
            Ok(Ok(out)) => match write_atomic(&job.output, &out.data) {
                Ok(()) => FileResult {
                    status: Status::Converted,
                    bytes_in: out.bytes_in,
                    bytes_out: out.data.len() as u64,
                    warnings: out.warnings,
                    ..FileResult::skipped(job)
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

    /// The converted bytes of a job's source, with the source size and the
    /// writer's warnings.
    fn convert_bytes(&self, job: &Job) -> Result<Output, String> {
        let ext = extension(&job.source);
        let is_markdown = MarkdownLoader.extensions().contains(&ext.as_str());
        let loader = self.registry.loader_for(&Source::Path(job.source.clone()));
        if is_markdown {
            let bytes = std::fs::read(&job.source).map_err(|e| format!("cannot read: {e}"))?;
            let text = decode(&bytes);
            return Ok(self
                .convert_markdown(job, &text)?
                .read_from(bytes.len() as u64));
        }
        if loader.is_none() && textweaver_formats::pandoc::EXTENSIONS.contains(&ext.as_str()) {
            return Err(if self.options.pandoc {
                format!("no native reader for .{ext} files, and Pandoc is not installed")
            } else {
                format!("no native reader for .{ext} files, and Pandoc is turned off")
            });
        }
        let size = std::fs::metadata(&job.source).map_or(0, |m| m.len());
        let doc = self
            .registry
            .load(&Source::Path(job.source.clone()), &self.options.load)
            .map_err(|e| e.to_string())?;
        let mut out = self.convert_document(job, &doc, None)?.read_from(size);
        // What the loader had to leave out (content nested too deeply).
        out.warnings
            .splice(0..0, textweaver_formats::warnings(&doc.meta));
        Ok(out)
    }

    /// Output for Markdown text (a Markdown file, or Pandoc's output).
    fn convert_markdown(&self, job: &Job, text: &str) -> Result<Output, String> {
        // Citations are formatted for every output but Markdown itself.
        let cited = if self.options.to == OutputFormat::Markdown {
            citations::Cited::default()
        } else {
            self.citations.apply(
                &job.source,
                text,
                self.options.to == OutputFormat::Html,
                self.options.render.flavor == textweaver_render::Flavor::Pandoc,
            )
        };
        let text = cited.markdown.as_deref().unwrap_or(text);
        let mut out = self.convert_markdown_text(job, text)?;
        out.warnings.extend(cited.warnings);
        Ok(out)
    }

    fn convert_markdown_text(&self, job: &Job, text: &str) -> Result<Output, String> {
        match self.options.to {
            OutputFormat::Markdown => Ok(Output::new(text.as_bytes().to_vec())),
            OutputFormat::Html => {
                let resolver = self.resolver_for(job);
                let rendered = textweaver_render::render_with(
                    text,
                    &self.options.render,
                    resolver.as_deref().map(|r| r as &dyn Resolver),
                );
                self.page(job, &rendered).map(Output::new)
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
                self.convert_document(job, &doc, Some(text))
            }
        }
    }

    /// Output for a loaded document.
    fn convert_document(
        &self,
        job: &Job,
        doc: &Document,
        markdown: Option<&str>,
    ) -> Result<Output, String> {
        match self.options.to {
            OutputFormat::Text => {
                let mut t = doc.text().to_string();
                if !t.ends_with('\n') {
                    t.push('\n');
                }
                Ok(Output::new(t.into_bytes()))
            }
            OutputFormat::Markdown => Ok(Output::new(
                textweaver_formats::to_markdown(doc).into_bytes(),
            )),
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
                self.page(job, &rendered).map(Output::new)
            }
            format => {
                let writer = self
                    .writers
                    .get(format)
                    .ok_or_else(|| format!("{} output is not available", format.label()))?;
                // The writer takes title, language, and author from the
                // options when set, else from the document (front matter,
                // HTML or EPUB metadata), its first heading, and its file
                // name; images resolve beside the source.
                let mut data = Vec::new();
                let report = writer
                    .write(doc, &self.options.write, &mut data)
                    .map_err(|e| e.to_string())?;
                Ok(Output {
                    warnings: report.warnings,
                    ..Output::new(data)
                })
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

/// Writes `data` to a temporary file beside `path`, synced, then renames
/// it over `path`, creating parent folders as needed and retrying while
/// Windows reports the file in use ([`textweaver_core::fs::write_atomic`]).
pub use textweaver_core::fs::write_atomic;

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
        let warned = Summary {
            format: Some(OutputFormat::Brf),
            converted: 4,
            warned: 2,
            seconds: 2.0,
            ..Summary::default()
        };
        assert_eq!(
            warned.sentence(),
            "Converted 4 files to braille in 2 seconds, 2 files per second. 2 files have warnings. No failures."
        );
        let one_warned = Summary {
            warned: 1,
            ..warned
        };
        assert!(
            one_warned
                .sentence()
                .ends_with(" 1 file has warnings. No failures.")
        );
    }

    #[test]
    fn a_canceled_run_says_so_first() {
        let s = Summary {
            format: Some(OutputFormat::Pdf),
            converted: 3,
            skipped: 1,
            failed: 1,
            canceled: 45,
            seconds: 2.0,
            ..Summary::default()
        };
        assert_eq!(
            s.sentence(),
            "Stopped. Converted 3 files to PDF in 2 seconds. 1 file up to date. 1 file failed. 45 files were not converted."
        );
        let one = Summary {
            format: Some(OutputFormat::Markdown),
            canceled: 1,
            seconds: 0.05,
            ..Summary::default()
        };
        assert_eq!(
            one.sentence(),
            "Stopped. Converted 0 files to Markdown in 50 milliseconds. 1 file was not converted."
        );
    }

    #[test]
    fn the_report_lists_failures_and_warnings_name_first() {
        let file = |name: &str, status: Status, warnings: Vec<String>| FileResult {
            source: PathBuf::from("notes").join(name),
            output: PathBuf::from("out").join(name),
            status,
            bytes_in: 0,
            bytes_out: 0,
            micros: 0,
            warnings,
        };
        let s = Summary {
            format: Some(OutputFormat::Pdf),
            converted: 2,
            failed: 1,
            warned: 1,
            seconds: 1.0,
            files: vec![
                file("a.md", Status::Converted, Vec::new()),
                file(
                    "report.docx",
                    Status::Failed("the file is damaged".into()),
                    Vec::new(),
                ),
                file(
                    "b.md",
                    Status::Converted,
                    vec!["An image could not be embedded.".into()],
                ),
            ],
            ..Summary::default()
        };
        let text = s.report_text(SystemTime::UNIX_EPOCH);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], s.sentence());
        let failed = lines
            .iter()
            .position(|l| *l == "Failed, 1 file:")
            .expect("a failures heading");
        assert_eq!(lines[failed + 1], "report.docx: the file is damaged");
        assert!(lines[failed + 2].trim_start().ends_with("report.docx"));
        let warned = lines
            .iter()
            .position(|l| *l == "Warnings, 1 file:")
            .expect("a warnings heading");
        assert_eq!(lines[warned + 1], "b.md: An image could not be embedded.");
        assert_eq!(
            lines.last(),
            Some(&"Report written 1970-01-01 at 00:00 UTC.")
        );
        assert!(!text.contains("a.md:"), "converted files are not listed");
    }

    #[test]
    fn a_new_report_replaces_the_old_one_whole() {
        let dir = tempfile::tempdir().expect("temp dir");
        let failed = Summary {
            format: Some(OutputFormat::Html),
            failed: 1,
            files: vec![FileResult {
                source: PathBuf::from("old.docx"),
                output: PathBuf::from("old.html"),
                status: Status::Failed("the file is damaged".into()),
                bytes_in: 0,
                bytes_out: 0,
                micros: 0,
                warnings: Vec::new(),
            }],
            ..Summary::default()
        };
        let path = failed.write_report(dir.path()).expect("written");
        assert_eq!(path, dir.path().join(REPORT_FILE));
        let clean = Summary {
            format: Some(OutputFormat::Html),
            converted: 1,
            seconds: 0.5,
            ..Summary::default()
        };
        clean.write_report(dir.path()).expect("written again");
        let text = std::fs::read_to_string(&path).expect("read");
        assert!(!text.contains("old.docx"), "{text}");
        assert!(text.starts_with("Converted 1 file to HTML"), "{text}");
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .expect("list")
            .map(|e| e.expect("entry").file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from(REPORT_FILE)]);
    }

    #[test]
    fn writer_formats_need_a_writer() {
        let epub = || ConvertOptions {
            to: OutputFormat::Epub,
            ..ConvertOptions::default()
        };
        assert!(Converter::new(epub()).is_ok(), "built-in writers");
        let err = Converter::with_writers(epub(), Writers::none()).unwrap_err();
        assert_eq!(
            err.to_string(),
            "EPUB output is not available in this build; choose md, html, or txt"
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
