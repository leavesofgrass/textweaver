//! Native writers (ADR-0017): accessible EPUB 3, DOCX with real heading
//! styles, BRF braille, and tagged PDF, written from a
//! [`Document`] in pure Rust.
//!
//! Every writer implements [`Writer`]: give it a document, [`WriteOptions`],
//! and any byte sink; it returns a [`WriteReport`] with the warnings a user
//! should hear (an image that could not be embedded, characters braille
//! cannot show). [`writer_for`] picks the writer for a [`Format`].
//!
//! ```
//! use textweaver_text::Document;
//! use textweaver_writers::{Format, WriteOptions, writer_for};
//!
//! let doc = Document::from_plain_text("Hello, world.");
//! let mut brf = Vec::new();
//! writer_for(Format::Brf)
//!     .write(&doc, &WriteOptions::default(), &mut brf)
//!     .unwrap();
//! // A paragraph starts in cell 3.
//! assert!(brf.starts_with(b"  ,HELLO1 WORLD4"));
//! ```
//!
//! All writers share one block tree ([`model::blocks`]), so they agree on
//! what is a heading, a list item, or a table's header row.

use std::io::Write;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use textweaver_text::Document;

pub mod brf;
pub mod docx;
pub mod epub;
mod math;
pub mod model;
pub mod pdf;
mod resource;
pub mod ueb;
mod xml;

pub use brf::BrfWriter;
pub use docx::DocxWriter;
pub use epub::EpubWriter;
pub use pdf::PdfWriter;

/// Output formats with a native writer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    /// EPUB 3 (`.epub`).
    Epub,
    /// Office Open XML WordprocessingML (`.docx`).
    Docx,
    /// Braille Ready Format (`.brf`).
    Brf,
    /// Tagged PDF (`.pdf`).
    Pdf,
}

impl Format {
    /// Every format, in a stable order.
    pub const ALL: [Format; 4] = [Format::Epub, Format::Docx, Format::Brf, Format::Pdf];

    /// The file extension, without a dot.
    pub fn extension(self) -> &'static str {
        match self {
            Format::Epub => "epub",
            Format::Docx => "docx",
            Format::Brf => "brf",
            Format::Pdf => "pdf",
        }
    }

    /// The media type.
    pub fn media_type(self) -> &'static str {
        match self {
            Format::Epub => "application/epub+zip",
            Format::Docx => {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            }
            Format::Brf => "text/plain",
            Format::Pdf => "application/pdf",
        }
    }

    /// The format for an extension or name (`"epub"`, `".DOCX"`, `"braille"`).
    pub fn from_name(name: &str) -> Option<Format> {
        match name.trim_start_matches('.').to_ascii_lowercase().as_str() {
            "epub" | "epub3" => Some(Format::Epub),
            "docx" | "word" => Some(Format::Docx),
            "brf" | "braille" => Some(Format::Brf),
            "pdf" => Some(Format::Pdf),
            _ => None,
        }
    }

    /// The spoken, user-facing name ("EPUB", "Word document").
    pub fn spoken_name(self) -> &'static str {
        match self {
            Format::Epub => "EPUB book",
            Format::Docx => "Word document",
            Format::Brf => "braille file",
            Format::Pdf => "PDF",
        }
    }
}

/// Writes a document in one format.
pub trait Writer: Send + Sync {
    /// The format this writer produces.
    fn format(&self) -> Format;

    /// Writes `doc` to `out`. Writers that build a container (EPUB, DOCX,
    /// PDF) assemble it in memory and write it once at the end, so `out`
    /// needs no `Seek`.
    fn write(
        &self,
        doc: &Document,
        options: &WriteOptions,
        out: &mut dyn Write,
    ) -> Result<WriteReport, WriteError>;
}

/// The writer for `format`.
pub fn writer_for(format: Format) -> Box<dyn Writer> {
    match format {
        Format::Epub => Box::new(EpubWriter),
        Format::Docx => Box::new(DocxWriter),
        Format::Brf => Box::new(BrfWriter),
        Format::Pdf => Box::new(PdfWriter),
    }
}

/// Writes `doc` in `format` to a new byte vector.
pub fn write_to_vec(
    doc: &Document,
    format: Format,
    options: &WriteOptions,
) -> Result<(Vec<u8>, WriteReport), WriteError> {
    let mut out = Vec::new();
    let report = writer_for(format).write(doc, options, &mut out)?;
    Ok((out, report))
}

/// Options shared by every writer, plus one section per format.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WriteOptions {
    /// Title, overriding the document's.
    pub title: Option<String>,
    /// BCP 47 language, overriding the document's (default `en`).
    pub language: Option<String>,
    /// Author, overriding the document's.
    pub author: Option<String>,
    /// Modification time written into package metadata, in seconds since
    /// the Unix epoch; `None` means now. Set it for reproducible output.
    pub timestamp: Option<u64>,
    /// Folder that relative image paths resolve against; defaults to the
    /// folder of the document's source file.
    pub resource_dir: Option<PathBuf>,
    /// Embed images found on disk (EPUB, DOCX, PDF). Images that cannot be
    /// found are written as their alt text and reported.
    pub embed_images: bool,
    /// EPUB options.
    pub epub: EpubOptions,
    /// Braille options.
    pub braille: BrailleOptions,
    /// PDF options.
    pub pdf: PdfOptions,
}

impl Default for WriteOptions {
    fn default() -> Self {
        WriteOptions {
            title: None,
            language: None,
            author: None,
            timestamp: None,
            resource_dir: None,
            embed_images: true,
            epub: EpubOptions::default(),
            braille: BrailleOptions::default(),
            pdf: PdfOptions::default(),
        }
    }
}

/// EPUB options.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EpubOptions {
    /// Split the book into one content document per chapter: at section
    /// breaks when the document has them, otherwise before each level 1
    /// heading. Off writes one content document.
    pub split_chapters: bool,
    /// Embed a bundled font (by name or key, such as `"Atkinson
    /// Hyperlegible Next"` or `"opendyslexic"`) and use it for the text,
    /// with its licence in the book beside the font files. Only bundled
    /// fonts can be embedded: their licence (SIL OFL 1.1) allows it.
    /// Reading systems may let the reader override it.
    pub font: Option<String>,
    /// Embed a bundled font for code, as for [`EpubOptions::font`].
    pub code_font: Option<String>,
}

impl Default for EpubOptions {
    fn default() -> Self {
        EpubOptions {
            split_chapters: true,
            font: None,
            code_font: None,
        }
    }
}

/// Braille translation grade.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BrailleGrade {
    /// Uncontracted Unified English Braille, translated natively.
    #[default]
    One,
    /// Contracted Unified English Braille through liblouis (`lou_translate`
    /// on the `PATH`, cargo feature `liblouis`); falls back to grade 1 with a
    /// warning when liblouis is missing.
    Two,
}

/// The braille code math is written in (ADR-0036). It takes effect with
/// the `mathcat` feature; without it, math is written as its spoken
/// words in uncontracted braille.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MathCode {
    /// The Nemeth Code, between the Nemeth switch indicators of BANA's
    /// guidance for Nemeth in UEB contexts.
    #[default]
    Nemeth,
    /// Unified English Braille's own mathematics.
    Ueb,
}

impl MathCode {
    /// The code's name, as it is said: "Nemeth" or "UEB".
    pub fn name(self) -> &'static str {
        match self {
            MathCode::Nemeth => "Nemeth",
            MathCode::Ueb => "UEB",
        }
    }
}

/// Braille (BRF) options.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BrailleOptions {
    /// Cells per line (default 40).
    pub cells_per_line: usize,
    /// Lines per page (default 25).
    pub lines_per_page: usize,
    /// Number braille pages at the right of each page's last line.
    pub page_numbers: bool,
    /// Translation grade.
    pub grade: BrailleGrade,
    /// liblouis table for grade 2 (default `en-ueb-g2.ctb`).
    pub table: String,
    /// The braille code for math (default Nemeth).
    pub math_code: MathCode,
}

impl Default for BrailleOptions {
    fn default() -> Self {
        BrailleOptions {
            cells_per_line: 40,
            lines_per_page: 25,
            page_numbers: true,
            grade: BrailleGrade::One,
            table: "en-ueb-g2.ctb".to_owned(),
            math_code: MathCode::Nemeth,
        }
    }
}

/// PDF page size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PageSize {
    /// US Letter, 8.5 by 11 inches.
    #[default]
    Letter,
    /// ISO A4, 210 by 297 millimetres.
    A4,
    /// Width and height in points (1/72 inch).
    Custom {
        /// Width in points.
        width: f32,
        /// Height in points.
        height: f32,
    },
}

impl PageSize {
    /// Width and height in points.
    pub fn points(self) -> (f32, f32) {
        match self {
            PageSize::Letter => (612.0, 792.0),
            PageSize::A4 => (595.28, 841.89),
            PageSize::Custom { width, height } => (width.max(144.0), height.max(144.0)),
        }
    }

    /// Reads a page size: `letter`, `a4`, `a5`, `legal`, or
    /// `WIDTHxHEIGHT` with a unit on either number or after both (`6x9in`,
    /// `148mmx210mm`, `432x648pt`; no unit means points). Sizes under two
    /// inches or over 200 inches are refused.
    pub fn parse(s: &str) -> Result<PageSize, String> {
        let t = s.trim().to_ascii_lowercase();
        match t.as_str() {
            "letter" | "us-letter" | "usletter" => return Ok(PageSize::Letter),
            "a4" => return Ok(PageSize::A4),
            "a5" => {
                return Ok(PageSize::Custom {
                    width: 419.53,
                    height: 595.28,
                });
            }
            "legal" | "us-legal" => {
                return Ok(PageSize::Custom {
                    width: 612.0,
                    height: 1008.0,
                });
            }
            _ => {}
        }
        let bad = || {
            format!(
                "{s:?} is not a page size; use letter, a4, a5, legal, or WIDTHxHEIGHT with in, mm, cm, or pt"
            )
        };
        let (w, h) = t.split_once('x').ok_or_else(bad)?;
        // A unit written once at the end applies to both numbers.
        let unit = ["in", "mm", "cm", "pt"]
            .into_iter()
            .find(|u| h.trim().ends_with(u))
            .unwrap_or("pt");
        let w = if w.trim().ends_with(|c: char| c.is_ascii_alphabetic()) {
            w.to_owned()
        } else {
            format!("{}{unit}", w.trim())
        };
        let (width, height) = (
            parse_length(&w).map_err(|_| bad())?,
            parse_length(h).map_err(|_| bad())?,
        );
        if !(144.0..=14_400.0).contains(&width) || !(144.0..=14_400.0).contains(&height) {
            return Err(format!(
                "{s:?} is too small or too large for a page; use 2 to 200 inches"
            ));
        }
        Ok(PageSize::Custom { width, height })
    }
}

/// Reads a length and returns points: `1in`, `2.54cm`, `25mm`, `72pt`, or a
/// bare number of points.
pub fn parse_length(s: &str) -> Result<f32, String> {
    let t = s.trim().to_ascii_lowercase();
    let (num, scale) = if let Some(n) = t.strip_suffix("in") {
        (n, 72.0)
    } else if let Some(n) = t.strip_suffix("mm") {
        (n, 72.0 / 25.4)
    } else if let Some(n) = t.strip_suffix("cm") {
        (n, 72.0 / 2.54)
    } else if let Some(n) = t.strip_suffix("pt") {
        (n, 1.0)
    } else {
        (t.as_str(), 1.0)
    };
    let v: f32 = num
        .trim()
        .parse()
        .map_err(|_| format!("{s:?} is not a length; use a number with in, mm, cm, or pt"))?;
    if !v.is_finite() || v < 0.0 {
        return Err(format!("{s:?} is not a length"));
    }
    Ok(v * scale)
}

/// PDF options.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PdfOptions {
    /// Page size.
    pub page_size: PageSize,
    /// Body font size in points (default 12; at least 18 with
    /// [`PdfOptions::large_print`]).
    pub font_size: f32,
    /// Line height as a multiple of the font size (default 1.5, WCAG 1.4.12).
    pub line_spacing: f32,
    /// Page margin in points on every side (default 72, one inch).
    pub margin: f32,
    /// The text font family, by name: a bundled family (`"Atkinson
    /// Hyperlegible Next"`, the default; `"OpenDyslexic"`; `"Atkinson
    /// Hyperlegible Mono"`) or an installed one (`"Verdana"`), or a font
    /// file path. A name that is neither bundled nor installed is an error.
    pub font_family: Option<String>,
    /// The code font family, by name or file, as for
    /// [`PdfOptions::font_family`] (default: the bundled Atkinson
    /// Hyperlegible Mono).
    pub code_font_family: Option<String>,
    /// A TrueType or OpenType font file for body text, used for every
    /// style; overrides [`PdfOptions::font_family`]. Without either,
    /// `TEXTWEAVER_PDF_FONT` names a file, else the bundled font is used
    /// (see [`pdf`]).
    pub font: Option<PathBuf>,
    /// Large print: text of at least 18 points, line spacing of at least
    /// 1.5, more space between paragraphs, gentler heading sizes, and code
    /// at full size.
    pub large_print: bool,
    /// "Page N of M" in each page's footer, marked as an artifact so
    /// screen readers skip it.
    pub page_numbers: bool,
    /// Start with a title page: the title, the author, and the date.
    pub title_page: bool,
    /// The date on the title page, as written (`"September 25, 2026"`);
    /// otherwise the document's `date` property (front matter), else no
    /// date. textweaver does not print today's date itself: it cannot know
    /// the reader's time zone, and a wrong date is worse than none.
    pub date: Option<String>,
    /// A table of contents after the title page, from the headings, each
    /// entry a link to its heading with its page number.
    pub toc: bool,
    /// Heading levels the table of contents lists (default 3: levels 1 to
    /// 3).
    pub toc_depth: u8,
    /// Compress content streams (off only for inspecting the output).
    pub compress: bool,
    /// Validate against PDF/UA-1 while writing; validation failures are
    /// errors. Off writes a tagged PDF without the PDF/UA claim.
    pub pdf_ua: bool,
}

impl Default for PdfOptions {
    fn default() -> Self {
        PdfOptions {
            page_size: PageSize::Letter,
            font_size: 12.0,
            line_spacing: 1.5,
            margin: 72.0,
            font_family: None,
            code_font_family: None,
            font: None,
            large_print: false,
            page_numbers: true,
            title_page: false,
            date: None,
            toc: false,
            toc_depth: 3,
            compress: true,
            pdf_ua: true,
        }
    }
}

/// Smallest text size in large print, in points.
pub const LARGE_PRINT_MIN_SIZE: f32 = 18.0;

impl PdfOptions {
    /// The large-print preset: 18-point text, 1.6 line spacing, and
    /// three-quarter-inch margins, so more words fit on a line at the
    /// larger size.
    pub fn large_print() -> PdfOptions {
        PdfOptions {
            font_size: LARGE_PRINT_MIN_SIZE,
            line_spacing: 1.6,
            margin: 54.0,
            large_print: true,
            ..PdfOptions::default()
        }
    }
}

/// What a writer wants the user to know about a finished write.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteReport {
    /// Warnings, each a sentence that reads well aloud.
    pub warnings: Vec<String>,
}

impl WriteReport {
    fn warn(&mut self, message: impl Into<String>) {
        let message = message.into();
        if !self.warnings.contains(&message) {
            self.warnings.push(message);
        }
    }
}

/// Writer failures.
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    /// Writing the output failed.
    #[error("cannot write the output: {0}")]
    Io(#[from] std::io::Error),
    /// Building a zip container (EPUB, DOCX) failed.
    #[error("cannot build the package: {0}")]
    Zip(String),
    /// No usable font was found for PDF output.
    #[error(
        "no font for PDF output: this build has no bundled fonts and none of the usual fonts is installed; name a font family or file with the PDF font option, or set TEXTWEAVER_PDF_FONT to a font file"
    )]
    NoFont,
    /// A font file could not be read or parsed.
    #[error("cannot use the font {0}: {1}")]
    Font(String, String),
    /// PDF generation or PDF/UA validation failed.
    #[error("cannot write the PDF: {0}")]
    Pdf(String),
    /// Braille translation failed.
    #[error("braille translation failed: {0}")]
    Braille(String),
}

/// Seconds since the Unix epoch for `options.timestamp`, or now.
pub(crate) fn timestamp(options: &WriteOptions) -> u64 {
    options.timestamp.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    })
}

/// A UTC date and time from Unix seconds: (year, month, day, hour, minute,
/// second).
pub(crate) fn civil(secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let rem = secs % 86_400;
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (
        year,
        month,
        day,
        (rem / 3600) as u32,
        (rem % 3600 / 60) as u32,
        (rem % 60) as u32,
    )
}

/// `YYYY-MM-DDThh:mm:ssZ` for Unix seconds.
pub(crate) fn iso8601(secs: u64) -> String {
    let (y, mo, d, h, mi, s) = civil(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_dates() {
        assert_eq!(iso8601(0), "1970-01-01T00:00:00Z");
        // 2026-09-25T12:34:56Z
        assert_eq!(iso8601(1_790_339_696), "2026-09-25T12:34:56Z");
        // Leap day.
        assert_eq!(iso8601(951_782_400), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn format_names() {
        assert_eq!(Format::from_name(".DOCX"), Some(Format::Docx));
        assert_eq!(Format::from_name("braille"), Some(Format::Brf));
        assert_eq!(Format::from_name("odt"), None);
        for f in Format::ALL {
            assert_eq!(Format::from_name(f.extension()), Some(f));
            assert_eq!(writer_for(f).format(), f);
        }
    }

    #[test]
    fn page_sizes_and_lengths() {
        assert_eq!(PageSize::parse("A4"), Ok(PageSize::A4));
        assert_eq!(PageSize::parse(" letter "), Ok(PageSize::Letter));
        assert_eq!(
            PageSize::parse("6x9in"),
            Ok(PageSize::Custom {
                width: 432.0,
                height: 648.0
            })
        );
        assert_eq!(
            PageSize::parse("432x648"),
            Ok(PageSize::Custom {
                width: 432.0,
                height: 648.0
            })
        );
        let PageSize::Custom { width, height } = PageSize::parse("148mmx210mm").unwrap() else {
            panic!("custom");
        };
        assert!((width - 419.53).abs() < 0.1 && (height - 595.28).abs() < 0.1);
        assert!(PageSize::parse("1x1in").is_err());
        assert!(PageSize::parse("huge").is_err());
        assert_eq!(parse_length("1in"), Ok(72.0));
        assert_eq!(parse_length("36"), Ok(36.0));
        assert!((parse_length("2.54cm").unwrap() - 72.0).abs() < 1e-3);
        assert!(parse_length("-1in").is_err());
        assert!(parse_length("wide").is_err());
    }

    #[test]
    fn large_print_preset() {
        let p = PdfOptions::large_print();
        assert!(p.large_print && p.font_size >= LARGE_PRINT_MIN_SIZE && p.line_spacing >= 1.5);
        let from_json: PdfOptions =
            serde_json::from_str(r#"{"large_print":true,"toc":true}"#).unwrap();
        assert!(from_json.large_print && from_json.toc && from_json.page_numbers);
    }

    #[test]
    fn options_deserialize_with_defaults() {
        let o: WriteOptions = serde_json::from_str(r#"{"braille":{"cells_per_line":32}}"#).unwrap();
        assert_eq!(o.braille.cells_per_line, 32);
        assert_eq!(o.braille.lines_per_page, 25);
        assert!(o.embed_images);
    }
}
