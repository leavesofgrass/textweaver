//! Conversion reports with provenance, for an accommodation file.
//!
//! A report says where each output came from and what it is missing:
//!
//! - **Provenance**: the source file's name and its SHA-256, textweaver's
//!   version, the output format, and the date and time the report was
//!   written, from the clock, in UTC.
//! - **What could not be made accessible**: images with no description,
//!   tables whose columns do not line up (rows with different numbers of
//!   cells, as a scan's tables often come out of OCR), tables with no header
//!   row, and math that did not parse, each with where it is: the heading it
//!   is under, the print page when the source has pages, and the line of the
//!   converted text. These are found in the document ([`find_issues`]).
//! - **Other notes**: what the loaders and writers already warn about (an
//!   image that could not be embedded, characters braille cannot show).
//!
//! Reports come as Markdown ([`ReportFormat::Markdown`], the default) or
//! JSON. A batch writes one report for the run, with a section per file,
//! as [`REPORT_FILE`](crate::REPORT_FILE) (or
//! [`REPORT_JSON_FILE`](crate::REPORT_JSON_FILE)) in the output folder; a
//! file converted on its own gets one beside its output, named after it
//! ([`file_report_path`]: `essay.pdf.report.md`).
//!
//! **Privacy.** A report names files only by their name, or their path
//! inside the folder that was converted; never a full path, a user name,
//! or a machine name. Reasons and notes have the source's folder and the
//! home folder taken out. Nothing is sent anywhere.

use std::fmt::Write as _;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Serialize;
use sha2::{Digest, Sha256};
use textweaver_core::MarkerKind;
use textweaver_text::Document;

use crate::{FileResult, OutputFormat, Status, Summary, count, watch};

/// The most issues one file's report lists; the rest are counted.
pub const MAX_ISSUES: usize = 1000;

/// textweaver's version, as `tw --version` prints it.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// How a report is written.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    /// Markdown, for people (the default).
    #[default]
    Markdown,
    /// JSON, for programs and records systems.
    Json,
}

impl ReportFormat {
    /// Parses `md`, `markdown`, or `json`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "md" | "markdown" => Some(ReportFormat::Markdown),
            "json" => Some(ReportFormat::Json),
            _ => None,
        }
    }

    /// The file extension, without a dot.
    pub fn extension(self) -> &'static str {
        match self {
            ReportFormat::Markdown => "md",
            ReportFormat::Json => "json",
        }
    }

    /// The name of a batch's report in its output folder.
    pub fn file_name(self) -> &'static str {
        match self {
            ReportFormat::Markdown => crate::REPORT_FILE,
            ReportFormat::Json => crate::REPORT_JSON_FILE,
        }
    }
}

/// Where the report for a file converted on its own goes: beside its
/// output, named after it (`essay.pdf` gives `essay.pdf.report.md`).
pub fn file_report_path(output: &Path, format: ReportFormat) -> PathBuf {
    let name = output
        .file_name()
        .map_or_else(|| "output".to_owned(), |n| n.to_string_lossy().into_owned());
    output.with_file_name(format!("{name}.report.{}", format.extension()))
}

/// True for a report's file name, so planning never takes a report as a
/// source: a batch's report (including the plain text one of earlier
/// versions) or a file's own report (`essay.pdf.report.md`).
pub fn is_report_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "conversion-report.md" | "conversion-report.json" | "conversion-report.txt"
    ) {
        return true;
    }
    let Some(stem) = lower
        .strip_suffix(".report.md")
        .or_else(|| lower.strip_suffix(".report.json"))
    else {
        return false;
    };
    OutputFormat::ALL
        .iter()
        .any(|f| stem.ends_with(&format!(".{}", f.extension())))
}

/// What kind of item could not be made accessible.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueKind {
    /// An image with no description (alt text), or only its file name.
    ImageWithoutDescription,
    /// A table whose rows have different numbers of cells, so its columns
    /// do not line up (common in tables recognized by OCR).
    RaggedTable,
    /// A table with no header row: its cells cannot be read with their
    /// column headings.
    TableWithoutHeader,
    /// Math that did not parse; it is written as its source.
    MathNotParsed,
}

impl IssueKind {
    /// The kind in words, first on its line in the report.
    pub fn label(self) -> &'static str {
        match self {
            IssueKind::ImageWithoutDescription => "Image without a description",
            IssueKind::RaggedTable => "Table whose columns do not line up",
            IssueKind::TableWithoutHeader => "Table without a header row",
            IssueKind::MathNotParsed => "Math that did not parse",
        }
    }
}

/// Where an issue is in the document.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Location {
    /// The heading it is under, when there is one.
    pub heading: Option<String>,
    /// The print page, when the source has pages (PDF, DAISY, OCR).
    pub page: Option<String>,
    /// The line, from 1: of the source for a Markdown file, else of the
    /// converted text.
    pub line: usize,
}

impl Location {
    /// Where, in words: "Under the heading “Results”, page 4, line 37."
    pub fn sentence(&self) -> String {
        let mut parts = Vec::new();
        if let Some(h) = &self.heading {
            parts.push(format!("under the heading \u{201c}{h}\u{201d}"));
        }
        if let Some(p) = &self.page {
            parts.push(format!("page {p}"));
        }
        parts.push(format!("line {}", self.line));
        let mut s = parts.join(", ");
        if let Some(first) = s.get(..1) {
            s = first.to_uppercase() + &s[1..];
        }
        s.push('.');
        s
    }
}

/// One item that could not be made accessible.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Issue {
    /// What it is.
    pub kind: IssueKind,
    /// The formula's source, as written, for math.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// More about it: an image's file name, why a formula did not parse,
    /// the cell counts of a table's rows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Where it is.
    pub location: Location,
}

impl Issue {
    /// The issue in words, kind first: "Image without a description:
    /// chart.png. Under the heading “Results”, line 12."
    pub fn sentence(&self) -> String {
        let mut s = self.kind.label().to_owned();
        let more: Vec<&str> = [self.source.as_deref(), self.detail.as_deref()]
            .into_iter()
            .flatten()
            .collect();
        if !more.is_empty() {
            s.push_str(": ");
            s.push_str(&more.join(", "));
        }
        s.push_str(". ");
        s.push_str(&self.location.sentence());
        s
    }
}

/// The items in `doc` that could not be made accessible, in document
/// order, at most [`MAX_ISSUES`]; the second value counts the ones left
/// out. One pass over the markers.
pub fn find_issues(doc: &Document) -> (Vec<Issue>, usize) {
    let markers = doc.markers();
    let mut heading: Option<String> = None;
    let mut page: Option<String> = None;
    let mut issues = Vec::new();
    let mut omitted = 0;
    let mut push = |issues: &mut Vec<Issue>, issue: Issue| {
        if issues.len() < MAX_ISSUES {
            issues.push(issue);
        } else {
            omitted += 1;
        }
    };
    for (i, m) in markers.iter().enumerate() {
        let here = |heading: &Option<String>, page: &Option<String>| Location {
            heading: heading.clone(),
            page: page.clone(),
            line: doc.line_of(m.range.start) + 1,
        };
        match m.kind {
            MarkerKind::Heading => {
                let text = clip(&collapse(&doc.slice(m.range)), 80);
                heading = (!text.is_empty()).then_some(text);
            }
            MarkerKind::PageBreak => {
                page = m
                    .label
                    .as_deref()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(|l| clip(l, 20));
            }
            MarkerKind::Image => {
                let alt = collapse(&doc.slice(m.range));
                let name = m.reference.as_deref().and_then(reference_name);
                let detail = if alt.is_empty() {
                    Some(name.unwrap_or_else(|| "no file name".to_owned()))
                } else if name.as_deref().is_some_and(|n| n == alt) {
                    Some(format!("{alt}, described only by its file name"))
                } else {
                    None
                };
                if let Some(detail) = detail {
                    push(
                        &mut issues,
                        Issue {
                            kind: IssueKind::ImageWithoutDescription,
                            source: None,
                            detail: Some(detail),
                            location: here(&heading, &page),
                        },
                    );
                }
            }
            MarkerKind::Math => {
                let source = doc.slice(m.range);
                if let Some(problem) = math_problem(&source) {
                    push(
                        &mut issues,
                        Issue {
                            kind: IssueKind::MathNotParsed,
                            source: Some(clip(&collapse(&source), 80)),
                            detail: Some(problem),
                            location: here(&heading, &page),
                        },
                    );
                }
            }
            MarkerKind::Table => {
                let rows = table_rows(&markers[i + 1..], m.range.end);
                let counts: Vec<usize> = rows.iter().map(|r| r.1).collect();
                if counts.iter().any(|&c| c != counts[0]) {
                    let shown: Vec<String> =
                        counts.iter().take(12).map(ToString::to_string).collect();
                    let more = if counts.len() > 12 { ", and more" } else { "" };
                    push(
                        &mut issues,
                        Issue {
                            kind: IssueKind::RaggedTable,
                            source: None,
                            detail: Some(format!(
                                "{}, with {}{more} cells",
                                count(rows.len(), "row"),
                                shown.join(", ")
                            )),
                            location: here(&heading, &page),
                        },
                    );
                }
                if !rows.is_empty() && !rows.iter().any(|r| r.0) {
                    push(
                        &mut issues,
                        Issue {
                            kind: IssueKind::TableWithoutHeader,
                            source: None,
                            detail: Some(count(rows.len(), "row")),
                            location: here(&heading, &page),
                        },
                    );
                }
            }
            _ => {}
        }
    }
    (issues, omitted)
}

/// The items in Markdown source text that could not be made accessible:
/// images with no description and math that did not parse (a Markdown
/// table always lines up and has a header row). Read from the text itself,
/// because the loaders leave an image with no description out of the
/// document; the line is the line of the Markdown source, where an author
/// fixes it.
pub fn find_markdown_issues(text: &str) -> (Vec<Issue>, usize) {
    use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_MATH
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    let line = |offset: usize| line_starts.partition_point(|&s| s <= offset).max(1);
    let mut issues = Vec::new();
    let mut omitted = 0;
    let mut push = |issues: &mut Vec<Issue>, issue: Issue| {
        if issues.len() < MAX_ISSUES {
            issues.push(issue);
        } else {
            omitted += 1;
        }
    };
    let mut heading: Option<String> = None;
    let mut heading_text: Option<String> = None;
    // The image being read: its target, its alt text, and where it starts.
    let mut image: Option<(String, String, usize)> = None;
    for (event, range) in Parser::new_ext(text, options).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { .. }) => heading_text = Some(String::new()),
            Event::End(TagEnd::Heading(_)) => {
                if let Some(h) = heading_text.take() {
                    let h = clip(&collapse(&h), 80);
                    heading = (!h.is_empty()).then_some(h);
                }
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                image = Some((dest_url.to_string(), String::new(), range.start));
            }
            Event::End(TagEnd::Image) => {
                if let Some((dest, alt, at)) = image.take() {
                    let alt = collapse(&alt);
                    let name = reference_name(&dest);
                    let detail = if alt.is_empty() {
                        Some(name.unwrap_or_else(|| "no file name".to_owned()))
                    } else if name.as_deref().is_some_and(|n| n == alt) {
                        Some(format!("{alt}, described only by its file name"))
                    } else {
                        None
                    };
                    if let Some(detail) = detail {
                        push(
                            &mut issues,
                            Issue {
                                kind: IssueKind::ImageWithoutDescription,
                                source: None,
                                detail: Some(detail),
                                location: Location {
                                    heading: heading.clone(),
                                    page: None,
                                    line: line(at),
                                },
                            },
                        );
                    }
                }
            }
            Event::Text(t) | Event::Code(t) => {
                if let Some(h) = heading_text.as_mut() {
                    h.push_str(&t);
                }
                if let Some(i) = image.as_mut() {
                    i.1.push_str(&t);
                }
            }
            Event::InlineMath(m) | Event::DisplayMath(m) => {
                if let Some(h) = heading_text.as_mut() {
                    h.push_str(&m);
                }
                if let Some(problem) = math_problem(&m) {
                    push(
                        &mut issues,
                        Issue {
                            kind: IssueKind::MathNotParsed,
                            source: Some(clip(&collapse(&text[range.clone()]), 80)),
                            detail: Some(problem),
                            location: Location {
                                heading: heading.clone(),
                                page: None,
                                line: line(range.start),
                            },
                        },
                    );
                }
            }
            _ => {}
        }
    }
    (issues, omitted)
}

/// The rows of a table whose markers follow it: (header, cell count) each.
fn table_rows(
    after: &[textweaver_text::Marker],
    end: textweaver_core::CharPos,
) -> Vec<(bool, usize)> {
    let mut rows: Vec<(bool, usize, textweaver_core::CharPos)> = Vec::new();
    for m in after.iter().take_while(|m| m.range.start < end) {
        match m.kind {
            MarkerKind::TableRow => rows.push((m.is_header_row(), 0, m.range.end)),
            MarkerKind::TableCell => {
                if let Some(row) = rows.last_mut().filter(|r| m.range.start <= r.2) {
                    row.1 += 1;
                }
            }
            _ => {}
        }
    }
    rows.into_iter().map(|(h, n, _)| (h, n)).collect()
}

/// Why a formula did not parse, or `None` when it did.
fn math_problem(marked: &str) -> Option<String> {
    let t = marked.trim();
    let body = [("$$", "$$"), ("\\[", "\\]"), ("\\(", "\\)"), ("$", "$")]
        .iter()
        .find_map(|(open, close)| {
            t.strip_prefix(open)
                .and_then(|b| b.strip_suffix(close))
                .filter(|b| !b.is_empty())
        })
        .unwrap_or(t);
    let math = textweaver_math::parse_latex(body.trim());
    math.diagnostics.first().map(|d| {
        let msg = d.message.trim().trim_end_matches('.');
        clip(msg, 80)
    })
}

/// The file name in an image reference (a path or URL), never its folder.
fn reference_name(reference: &str) -> Option<String> {
    let r = reference.trim();
    if r.is_empty() || r.starts_with("data:") {
        return None;
    }
    let r = r.split(['?', '#']).next().unwrap_or(r);
    let name = r.rsplit(['/', '\\']).next().unwrap_or(r).trim();
    (!name.is_empty()).then(|| clip(name, 80))
}

/// Text with runs of white space made one space, trimmed.
fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// At most `max` chars, with an ellipsis when cut.
fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('\u{2026}');
    out
}

/// The SHA-256 of `bytes`, in lowercase hexadecimal.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// The SHA-256 of a file, read in blocks, in lowercase hexadecimal.
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// `path` relative to `base` when it is inside it, else its file name: a
/// report never shows a folder outside the one converted.
pub(crate) fn relative_name(path: &Path, base: Option<&Path>) -> String {
    if let Some(rel) = base.and_then(|b| path.strip_prefix(b).ok())
        && !rel.as_os_str().is_empty()
    {
        return rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
    }
    path.file_name().map_or_else(
        || "unnamed".to_owned(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// `text` with the folders of `paths` and the home folder taken out, so a
/// reason or note never carries a path outside the converted folder.
fn scrub(text: &str, paths: &[&Path]) -> String {
    let mut folders: Vec<String> = paths
        .iter()
        .filter_map(|p| p.parent())
        .filter(|p| !p.as_os_str().is_empty())
        .flat_map(|p| {
            let mut v = vec![p.display().to_string()];
            if let Ok(c) = p.canonicalize() {
                v.push(c.display().to_string());
            }
            v
        })
        .collect();
    for var in ["HOME", "USERPROFILE"] {
        if let Some(h) = std::env::var_os(var).filter(|h| !h.is_empty()) {
            folders.push(PathBuf::from(h).display().to_string());
        }
    }
    folders.sort_by_key(|f| std::cmp::Reverse(f.len()));
    let mut out = text.to_owned();
    for f in folders.iter().filter(|f| f.len() > 1) {
        for sep in ['/', '\\'] {
            out = out.replace(&format!("{f}{sep}"), "");
        }
        out = out.replace(f.as_str(), "");
    }
    out
}

/// "Friday, October 2, 2026, at 21:14 UTC", the weekday computed.
pub fn spoken_utc(t: SystemTime) -> String {
    const DAYS: [&str; 7] = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ];
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let secs = t
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    // 1970-01-01 was a Thursday.
    let weekday = DAYS[((secs / 86_400 + 4) % 7) as usize];
    let (y, mo, d, h, mi, _) = watch::utc_parts(t);
    let month = MONTHS[(mo.clamp(1, 12) - 1) as usize];
    format!("{weekday}, {month} {d}, {y}, at {h:02}:{mi:02} UTC")
}

/// One file's part of a report.
#[derive(Clone, Debug, Serialize)]
pub struct FileEntry {
    /// The source, by its path inside the converted folder.
    pub name: String,
    /// The output, by its path inside the report's folder.
    pub output: String,
    /// "converted", "skipped", or "failed".
    pub status: &'static str,
    /// Why it failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The source's SHA-256, for a converted file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// Bytes read from the source.
    pub bytes_in: u64,
    /// Bytes written.
    pub bytes_out: u64,
    /// What could not be made accessible.
    pub issues: Vec<IssueEntry>,
    /// Issues found but not listed (past [`MAX_ISSUES`]).
    #[serde(skip_serializing_if = "is_zero")]
    pub issues_omitted: usize,
    /// What the loader and writer warned about.
    pub notes: Vec<String>,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// An issue as a report shows it: the structured fields and the sentence.
#[derive(Clone, Debug, Serialize)]
pub struct IssueEntry {
    /// The issue.
    #[serde(flatten)]
    pub issue: Issue,
    /// The kind in words.
    pub description: &'static str,
    /// The whole issue as a sentence.
    pub sentence: String,
}

/// A whole report: provenance, totals, and every file.
#[derive(Clone, Debug, Serialize)]
pub struct Report {
    /// Always "textweaver conversion report".
    pub report: &'static str,
    /// The report layout's version.
    pub report_version: u32,
    /// textweaver's version.
    pub textweaver: &'static str,
    /// When the report was written, UTC, ISO 8601.
    pub written: String,
    /// The output format.
    pub format: Option<OutputFormat>,
    /// The run's summary sentence.
    pub summary: String,
    /// Files written.
    pub converted: usize,
    /// Files up to date, not converted again.
    pub skipped: usize,
    /// Files that failed.
    pub failed: usize,
    /// Files never started because the run was stopped.
    pub canceled: usize,
    /// Converted files with something that could not be made accessible.
    pub files_with_issues: usize,
    /// Every file, in plan order.
    pub files: Vec<FileEntry>,
    #[serde(skip)]
    written_at: Option<SystemTime>,
}

impl Report {
    /// The report of `summary`, written at `written`; outputs are named
    /// relative to `dir`, the folder the report goes in.
    pub fn new(summary: &Summary, dir: Option<&Path>, written: SystemTime) -> Self {
        let files: Vec<FileEntry> = summary
            .files
            .iter()
            .map(|f| FileEntry::new(f, dir))
            .collect();
        Report {
            report: "textweaver conversion report",
            report_version: 1,
            textweaver: VERSION,
            written: watch::iso_utc(written),
            format: summary.format,
            summary: summary.sentence(),
            converted: summary.converted,
            skipped: summary.skipped,
            failed: summary.failed,
            canceled: summary.canceled,
            files_with_issues: files.iter().filter(|f| !f.issues.is_empty()).count(),
            files,
            written_at: Some(written),
        }
    }

    /// The report as JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default() + "\n"
    }

    /// The report as Markdown: meaning first on each line, headings and
    /// lists, no tables, no symbols standing alone.
    pub fn to_markdown(&self) -> String {
        let mut o = String::new();
        let single = self.files.len() == 1 && self.canceled == 0;
        match (single, self.files.first()) {
            (true, Some(f)) => {
                let _ = writeln!(o, "# Conversion report for {}\n", md(&f.name));
            }
            _ => o.push_str("# Conversion report\n\n"),
        }
        let _ = writeln!(o, "{}\n", md(&self.summary));
        o.push_str("## About this report\n\n");
        let _ = writeln!(o, "- Made by: textweaver {}", self.textweaver);
        let when = self
            .written_at
            .map_or_else(|| self.written.clone(), spoken_utc);
        let _ = writeln!(o, "- Written: {when}");
        if let Some(f) = self.format {
            let _ = writeln!(o, "- Output format: {}", f.label());
        }
        let _ = writeln!(
            o,
            "- Files: {} converted, {} up to date, {} failed{}",
            self.converted,
            self.skipped,
            self.failed,
            if self.canceled > 0 {
                format!(
                    ", {} not converted because the run was stopped",
                    self.canceled
                )
            } else {
                String::new()
            }
        );
        o.push('\n');
        o.push_str("## Could not be made accessible\n\n");
        let with: Vec<&FileEntry> = self.files.iter().filter(|f| !f.issues.is_empty()).collect();
        if with.is_empty() {
            o.push_str(
                "Nothing was found that could not be made accessible: every image has a \
                 description, every table lines up and has a header row, and all math parsed.\n\n",
            );
        } else {
            let items: usize = with.iter().map(|f| f.issues.len() + f.issues_omitted).sum();
            if single {
                let _ = writeln!(o, "{} found; the list is below.\n", count(items, "item"));
            } else {
                let _ = writeln!(
                    o,
                    "{} with items that could not be made accessible, {} in all:\n",
                    count(with.len(), "file"),
                    count(items, "item")
                );
                for f in &with {
                    let _ = writeln!(
                        o,
                        "- {}: {}",
                        md(&f.name),
                        count(f.issues.len() + f.issues_omitted, "item")
                    );
                }
                o.push('\n');
            }
        }
        let failed: Vec<&FileEntry> = self.files.iter().filter(|f| f.status == "failed").collect();
        if !failed.is_empty() && !single {
            let _ = writeln!(o, "## Failed, {}\n", count(failed.len(), "file"));
            for f in &failed {
                let _ = writeln!(
                    o,
                    "- {}: {}",
                    md(&f.name),
                    md(f.reason.as_deref().unwrap_or("no reason given"))
                );
            }
            o.push('\n');
        }
        o.push_str(if single {
            "## The file\n\n"
        } else {
            "## Files\n\n"
        });
        for f in &self.files {
            if !single {
                let _ = writeln!(o, "### {}\n", md(&f.name));
            }
            match f.status {
                "converted" => {
                    let _ = writeln!(o, "- Result: converted to {}", md(&f.output));
                }
                "skipped" => {
                    let _ = writeln!(
                        o,
                        "- Result: up to date, not converted again; the output is {}",
                        md(&f.output)
                    );
                }
                _ => {
                    let _ = writeln!(
                        o,
                        "- Result: failed, {}",
                        md(f.reason.as_deref().unwrap_or("no reason given"))
                    );
                }
            }
            if single {
                let _ = writeln!(o, "- Source: {}", md(&f.name));
            }
            if let Some(h) = &f.sha256 {
                let _ = writeln!(o, "- Source SHA-256: {h}");
            }
            if f.bytes_in > 0 {
                let _ = writeln!(o, "- Source size: {}", count(f.bytes_in as usize, "byte"));
            }
            o.push('\n');
            if !f.issues.is_empty() {
                let total = f.issues.len() + f.issues_omitted;
                let _ = writeln!(
                    o,
                    "Could not be made accessible, {}:\n",
                    count(total, "item")
                );
                for (n, i) in f.issues.iter().enumerate() {
                    let _ = writeln!(o, "{}. {}", n + 1, issue_markdown(&i.issue));
                }
                if f.issues_omitted > 0 {
                    let _ = writeln!(
                        o,
                        "\n{} more not listed; the first {MAX_ISSUES} are above.",
                        count(f.issues_omitted, "item")
                    );
                }
                o.push('\n');
            }
            if !f.notes.is_empty() {
                o.push_str("Other notes:\n\n");
                for n in &f.notes {
                    let _ = writeln!(o, "- {}", md(n));
                }
                o.push('\n');
            }
        }
        let trimmed = o.trim_end().to_owned();
        trimmed + "\n"
    }
}

impl FileEntry {
    fn new(f: &FileResult, dir: Option<&Path>) -> Self {
        let paths = [f.source.as_path(), f.output.as_path()];
        let (status, reason) = match &f.status {
            Status::Converted => ("converted", None),
            Status::Skipped => ("skipped", None),
            Status::Failed(r) => ("failed", Some(scrub(r, &paths))),
        };
        FileEntry {
            name: f.name.clone(),
            output: relative_name(&f.output, dir),
            status,
            reason,
            sha256: f.sha256.clone(),
            bytes_in: f.bytes_in,
            bytes_out: f.bytes_out,
            issues: f
                .issues
                .iter()
                .map(|i| IssueEntry {
                    issue: i.clone(),
                    description: i.kind.label(),
                    sentence: i.sentence(),
                })
                .collect(),
            issues_omitted: f.issues_omitted,
            notes: f.warnings.iter().map(|w| scrub(w, &paths)).collect(),
        }
    }
}

/// An issue as a Markdown list item: a formula's source in a code span.
fn issue_markdown(i: &Issue) -> String {
    let mut s = i.kind.label().to_owned();
    let mut more = Vec::new();
    if let Some(src) = &i.source {
        more.push(code(src));
    }
    if let Some(d) = &i.detail {
        more.push(md(d));
    }
    if !more.is_empty() {
        s.push_str(": ");
        s.push_str(&more.join(", "));
    }
    s.push_str(". ");
    s.push_str(&md(&i.location.sentence()));
    s
}

/// Text that Markdown shows as written.
fn md(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(
            c,
            '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>' | '#' | '|' | '{' | '}' | '!'
        ) {
            out.push('\\');
        }
        if c == '\n' || c == '\r' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

/// A code span that holds `s` whatever backticks it has.
fn code(s: &str) -> String {
    let s = s.replace(['\n', '\r'], " ");
    let mut longest = 0;
    let mut run = 0;
    for c in s.chars() {
        if c == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    let fence = "`".repeat(longest + 1);
    if longest > 0 || s.starts_with(' ') || s.ends_with(' ') {
        format!("{fence} {s} {fence}")
    } else {
        format!("{fence}{s}{fence}")
    }
}

impl Summary {
    /// The report of this run in `format`, written at `written`; outputs
    /// are named relative to `dir`.
    pub fn report(&self, format: ReportFormat, dir: Option<&Path>, written: SystemTime) -> String {
        let r = Report::new(self, dir, written);
        match format {
            ReportFormat::Markdown => r.to_markdown(),
            ReportFormat::Json => r.to_json(),
        }
    }

    /// Writes the run's report in `format` to `dir`
    /// ([`ReportFormat::file_name`]), replacing any earlier report whole
    /// (never merged with it, so entries from an older run cannot come
    /// back), and returns its path. The file is written to a temporary
    /// name and renamed, so a reader never sees half a report.
    pub fn write_report(&self, dir: &Path, format: ReportFormat) -> std::io::Result<PathBuf> {
        let path = dir.join(format.file_name());
        let text = self.report(format, Some(dir), SystemTime::now());
        crate::write_atomic(&path, text.as_bytes())?;
        Ok(path)
    }

    /// Writes a report beside each converted or failed file's output
    /// ([`file_report_path`]) for files converted on their own, and returns
    /// each path or why it could not be written. Up-to-date files keep
    /// their earlier report.
    pub fn write_file_reports(&self, format: ReportFormat) -> Vec<Result<PathBuf, String>> {
        let now = SystemTime::now();
        self.files
            .iter()
            .filter(|f| !matches!(f.status, Status::Skipped))
            .map(|f| write_file_report(f, self.format, format, now))
            .collect()
    }
}

/// Writes the report for one file beside its output and returns its path.
pub fn write_file_report(
    file: &FileResult,
    output_format: Option<OutputFormat>,
    format: ReportFormat,
    written: SystemTime,
) -> Result<PathBuf, String> {
    let (converted, failed) = match file.status {
        Status::Converted => (1, 0),
        Status::Skipped => (0, 0),
        Status::Failed(_) => (0, 1),
    };
    let one = Summary {
        format: output_format,
        converted,
        failed,
        skipped: usize::from(matches!(file.status, Status::Skipped)),
        warned: usize::from(!file.warnings.is_empty()),
        seconds: file.micros as f64 / 1e6,
        files: vec![file.clone()],
        ..Summary::default()
    };
    let path = file_report_path(&file.output, format);
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty());
    let text = one.report(format, dir, written);
    crate::write_atomic(&path, text.as_bytes())
        .map(|()| path)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use textweaver_core::CharRange;
    use textweaver_text::{DocumentMeta, Marker};

    fn doc(text: &str, markers: Vec<Marker>) -> Document {
        Document::new(DocumentMeta::default(), text.into(), markers)
    }

    fn at(text: &str, needle: &str) -> CharRange {
        let start = text.find(needle).expect("in the text");
        let s = text[..start].chars().count();
        CharRange::new(s, s + needle.chars().count())
    }

    #[test]
    fn issues_have_their_heading_page_and_line() {
        let text = "Results\nA picture: .\nx | y\n1\n$\\frac{a}{$\nEnd\n";
        let image_at = text.find(": .").expect("image") + 2;
        let image_at = text[..image_at].chars().count();
        let table = at(text, "x | y\n1");
        let markers = vec![
            Marker::new(MarkerKind::PageBreak, CharRange::empty(0)).with_label("iv"),
            Marker::new(MarkerKind::Heading, at(text, "Results")).with_level(1),
            Marker::new(MarkerKind::Image, CharRange::empty(image_at))
                .with_reference("D:/private/folder/chart.png"),
            Marker::new(MarkerKind::Table, table),
            Marker::new(MarkerKind::TableRow, at(text, "x | y")),
            Marker::new(MarkerKind::TableCell, at(text, "x")),
            Marker::new(MarkerKind::TableCell, at(text, "y")),
            Marker::new(MarkerKind::TableRow, at(text, "1")),
            Marker::new(MarkerKind::TableCell, at(text, "1")),
            Marker::new(MarkerKind::Math, at(text, "$\\frac{a}{$")),
        ];
        let d = doc(text, markers);
        let (issues, omitted) = find_issues(&d);
        assert_eq!(omitted, 0);
        let kinds: Vec<IssueKind> = issues.iter().map(|i| i.kind).collect();
        assert_eq!(
            kinds,
            [
                IssueKind::ImageWithoutDescription,
                IssueKind::RaggedTable,
                IssueKind::TableWithoutHeader,
                IssueKind::MathNotParsed,
            ]
        );
        let image = &issues[0];
        assert_eq!(image.detail.as_deref(), Some("chart.png"));
        assert_eq!(
            image.location,
            Location {
                heading: Some("Results".into()),
                page: Some("iv".into()),
                line: 2
            }
        );
        assert_eq!(
            image.sentence(),
            "Image without a description: chart.png. Under the heading \u{201c}Results\u{201d}, page iv, line 2."
        );
        assert_eq!(issues[1].detail.as_deref(), Some("2 rows, with 2, 1 cells"));
        assert_eq!(issues[1].location.line, 3);
        let math = &issues[3];
        assert_eq!(math.source.as_deref(), Some("$\\frac{a}{$"));
        assert!(
            math.detail.as_deref().is_some_and(|d| !d.is_empty()),
            "{math:?}"
        );
        assert_eq!(math.location.line, 5);
        // No folder of the image's path, ever.
        assert!(!format!("{issues:?}").contains("private"));
    }

    #[test]
    fn described_images_and_good_math_are_fine() {
        let text = "A cat\n$x^2$\n";
        let markers = vec![
            Marker::new(MarkerKind::Image, at(text, "A cat")).with_reference("cat.png"),
            Marker::new(MarkerKind::Math, at(text, "$x^2$")),
        ];
        assert!(find_issues(&doc(text, markers)).0.is_empty());
        let text = "cat.png\n";
        let markers =
            vec![Marker::new(MarkerKind::Image, at(text, "cat.png")).with_reference("img/cat.png")];
        let (issues, _) = find_issues(&doc(text, markers));
        assert_eq!(
            issues[0].detail.as_deref(),
            Some("cat.png, described only by its file name")
        );
    }

    #[test]
    fn markdown_images_and_math_are_found_in_the_source() {
        let text = "# Results\n\n![](figures/chart.png)\n\n![A chart of doses](d.png)\n\n\
                    ## Model\n\nThe area is $\\left( a$ and $x^2$.\n\n![scan.png](img/scan.png)\n";
        let (issues, omitted) = find_markdown_issues(text);
        assert_eq!(omitted, 0);
        let got: Vec<(IssueKind, Option<&str>, usize)> = issues
            .iter()
            .map(|i| (i.kind, i.location.heading.as_deref(), i.location.line))
            .collect();
        assert_eq!(
            got,
            [
                (IssueKind::ImageWithoutDescription, Some("Results"), 3),
                (IssueKind::MathNotParsed, Some("Model"), 9),
                (IssueKind::ImageWithoutDescription, Some("Model"), 11),
            ]
        );
        assert_eq!(issues[0].detail.as_deref(), Some("chart.png"));
        assert_eq!(issues[1].source.as_deref(), Some("$\\left( a$"));
        assert_eq!(issues[1].detail.as_deref(), Some("\\left without \\right"));
        assert_eq!(
            issues[2].detail.as_deref(),
            Some("scan.png, described only by its file name")
        );
    }

    #[test]
    fn report_names_and_paths() {
        assert_eq!(
            file_report_path(Path::new("out/essay.pdf"), ReportFormat::Markdown),
            Path::new("out/essay.pdf.report.md")
        );
        assert!(is_report_name("essay.pdf.report.md"));
        assert!(is_report_name("notes.md.report.json"));
        assert!(is_report_name("conversion-report.txt"));
        assert!(is_report_name("Conversion-Report.md"));
        assert!(!is_report_name("weekly.report.md"));
        assert!(!is_report_name("report.md"));
        assert_eq!(
            relative_name(Path::new("in/a/b.md"), Some(Path::new("in"))),
            "a/b.md"
        );
        assert_eq!(
            relative_name(Path::new("D:/elsewhere/b.md"), Some(Path::new("in"))),
            "b.md"
        );
        assert_eq!(ReportFormat::parse("JSON"), Some(ReportFormat::Json));
        assert_eq!(ReportFormat::parse("txt"), None);
    }

    #[test]
    fn dates_are_spoken_with_a_computed_weekday() {
        // 2026-10-02 00:00 UTC is 20,728 days after the epoch: a Friday.
        let t = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(20_728 * 86_400 + 76_440);
        assert_eq!(spoken_utc(t), "Friday, October 2, 2026, at 21:14 UTC");
        assert_eq!(
            spoken_utc(SystemTime::UNIX_EPOCH),
            "Thursday, January 1, 1970, at 00:00 UTC"
        );
    }

    #[test]
    fn markdown_and_code_escapes() {
        assert_eq!(md("a_b *c* [d]"), "a\\_b \\*c\\* \\[d\\]");
        assert_eq!(code("\\frac{a}{"), "`\\frac{a}{`");
        assert_eq!(code("a`b"), "`` a`b ``");
    }

    #[test]
    fn scrub_takes_folders_out() {
        let src = Path::new("/home/ada/notes/essay.md");
        let out = scrub("cannot read /home/ada/notes/essay.md", &[src]);
        assert_eq!(out, "cannot read essay.md");
    }
}
