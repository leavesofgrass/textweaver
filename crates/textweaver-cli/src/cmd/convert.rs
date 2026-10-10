//! `tw convert`: convert files and folders, on every core, or watch a
//! folder. Owner: Agent L.
//!
//! Output is written for listening: one summary sentence at the end, one
//! line per failure, and per-file lines only with `--verbose`. `--json`
//! prints the whole summary (every file, its status and timing) instead.
//! A conversion report (each source's name and SHA-256, textweaver's
//! version, the date, and what could not be made accessible, with where it
//! is) is saved as `conversion-report.md` in the output folder (or the one
//! folder being converted in place), replacing the last run's, or beside
//! each output for files named on their own (`essay.pdf.report.md`);
//! `--report-format json` writes JSON instead, `--no-report` leaves it out,
//! and `tw convert` says where it went. The exit status is 1 when any file
//! failed.
//!
//! Pandoc citations in Markdown (`[@doe2020]`) are formatted in a CSL style
//! (`--style`, APA by default) with a References section appended, from
//! `--bibliography`, the front matter's `bibliography`, the folder's
//! `references.json`, and your own library (`tw cite`), in that order.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use anyhow::{Context, bail};
use textweaver_convert::{
    BrailleOptions, BrailleTableFormat, CitationOptions, ConvertOptions, Converter, MathCode,
    OutputFormat, PdfOptions, ReportFormat, Status, WatchOptions, WriteOptions, watch,
};
use textweaver_render::{EmbedMode, Engine, Flavor, RenderOptions, TemplateChoice};
use textweaver_writers::Template;

/// Arguments for `tw convert`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Files or folders to convert. Folders are converted recursively and
    /// mirrored under --out.
    #[arg(required = true)]
    pub inputs: Vec<PathBuf>,
    /// Output format: md, html, txt, epub, docx, brf, or pdf; and, written
    /// by carta, adoc (AsciiDoc), typ (Typst), tex (LaTeX), wiki
    /// (MediaWiki), or org.
    #[arg(long, default_value = "md", value_parser = parse_format)]
    pub to: OutputFormat,
    /// Read every file as this format instead of by its extension: dokuwiki
    /// or jira (which have no extension of their own), or an extension
    /// such as org or rst.
    #[arg(long, value_name = "FORMAT")]
    pub from: Option<String>,
    /// Output folder (default: beside each source; for --watch, a
    /// "converted" folder inside the watched folder).
    #[arg(long = "out", short = 'o', alias = "output")]
    pub out: Option<PathBuf>,
    /// Markdown engine: pulldown (fastest) or comrak (complete GFM).
    #[arg(long, default_value = "pulldown", value_parser = parse_engine)]
    pub engine: Engine,
    /// Markdown flavor: gfm, obsidian, pandoc, or commonmark.
    #[arg(long, default_value = "gfm", value_parser = parse_flavor)]
    pub flavor: Flavor,
    /// A publishing template for EPUB, Word, and PDF: apa (APA 7 student
    /// paper), ama (AMA 11 manuscript), large-print, dyslexia-friendly,
    /// high-contrast, or manuscript; the layout options below still
    /// override it. For HTML: the page template, default, print,
    /// fragment, a name from --templates, or a file path.
    #[arg(long, default_value = "default")]
    pub template: String,
    /// Folder of your own templates (HTML files with MiniJinja syntax).
    #[arg(long)]
    pub templates: Option<PathBuf>,
    /// HTML: the theme for the page's colors, such as sepia or
    /// galaxy-light (default: the theme in your settings; Galaxy also
    /// follows the reader's system to Galaxy Light). Never asks.
    #[arg(long, value_name = "NAME")]
    pub theme: Option<String>,
    /// Worker threads (default: every core).
    #[arg(long, short = 'j')]
    pub jobs: Option<usize>,
    /// Convert even when the output is newer than the source.
    #[arg(long)]
    pub force: bool,
    /// Keep watching the folders and convert files as they arrive.
    #[arg(long)]
    pub watch: bool,
    /// Print the summary as JSON.
    #[arg(long)]
    pub json: bool,
    /// Use the files under this folder instead of the usual place (the
    /// settings, the reference library, the themes, and the fonts).
    #[arg(long, value_name = "DIR")]
    pub home: Option<PathBuf>,
    /// Print a line for every file.
    #[arg(long, short = 'v')]
    pub verbose: bool,
    /// Obsidian embeds of notes: link or inline.
    #[arg(long, default_value = "link", value_parser = parse_embeds)]
    pub embeds: EmbedMode,
    /// Remove scripts and unsafe HTML from the output (for Markdown from
    /// untrusted sources).
    #[arg(long)]
    pub sanitize: bool,
    /// Curly quotes, dashes, and ellipses.
    #[arg(long)]
    pub smart: bool,
    /// Leave LaTeX math as source instead of MathML.
    #[arg(long)]
    pub no_math: bool,
    /// Leave out the table of contents in HTML pages.
    #[arg(long)]
    pub no_toc: bool,
    /// Never use Pandoc, even for formats with no native reader.
    #[arg(long)]
    pub no_pandoc: bool,
    /// Seconds Pandoc may spend on one file before it is stopped (default:
    /// the TEXTWEAVER_PANDOC_TIMEOUT environment variable, else 120).
    #[arg(long, value_name = "SECONDS")]
    pub pandoc_timeout: Option<u64>,
    /// Citations: a bibliography (CSL-JSON, BibTeX, BibLaTeX, or RIS) to
    /// look keys up in first, before the folder's references.json and your
    /// own library.
    #[arg(long, value_name = "FILE")]
    pub bibliography: Option<PathBuf>,
    /// Citations: the CSL style, a name (apa, mla, chicago, chicago-notes,
    /// harvard, ieee, vancouver, ama, nature; tw cite styles lists all) or
    /// a .csl file.
    #[arg(long, value_name = "NAME", default_value = textweaver_convert::citations::DEFAULT_STYLE)]
    pub style: String,
    /// Leave Pandoc citations as written, without a References section.
    #[arg(long)]
    pub no_citations: bool,
    /// Read inline code spans as ASCIIMath (for course material written
    /// for MathJax); fenced blocks marked asciimath are read either way.
    #[arg(long)]
    pub asciimath: bool,
    /// PDF: a TrueType or OpenType font file for the text (default: the
    /// TEXTWEAVER_PDF_FONT environment variable, then the bundled Atkinson
    /// Hyperlegible Next, then an installed font).
    #[arg(long, value_name = "FILE")]
    pub pdf_font: Option<PathBuf>,
    /// BRF: the braille code for math, nemeth (the default) or ueb. Needs
    /// a build with MathCAT; otherwise math is written as spoken words.
    #[arg(long, value_name = "CODE", default_value = "nemeth", value_parser = parse_math_code)]
    pub math_code: MathCode,
    /// BRF: how tables are laid out: linear (the default, one row per
    /// line), listed (each entry after its column heading), or stairstep
    /// (each entry two cells right of the one before; four columns at
    /// most).
    #[arg(long, value_name = "FORMAT", default_value = "linear", value_parser = parse_table_format)]
    pub table_format: BrailleTableFormat,
    #[command(flatten)]
    pub layout: super::convert_layout::LayoutArgs,
    /// Do not save a conversion report (where each file came from, its
    /// SHA-256, and what could not be made accessible).
    #[arg(long)]
    pub no_report: bool,
    /// The conversion report's format: md (Markdown, the default) or json.
    #[arg(long, value_name = "FORMAT", default_value = "md", value_parser = parse_report_format)]
    pub report_format: ReportFormat,
    /// Watch: seconds a file's size must hold still before converting.
    #[arg(long, default_value_t = 2.0)]
    pub stable_seconds: f64,
    /// Watch: leave converted sources in place instead of moving them to
    /// "processed".
    #[arg(long)]
    pub keep_sources: bool,
}

fn parse_format(s: &str) -> Result<OutputFormat, String> {
    OutputFormat::parse(s).ok_or_else(|| {
        format!("unknown format {s:?}; use md, html, txt, epub, docx, brf, pdf, adoc, typ, tex, wiki, or org")
    })
}

fn parse_report_format(s: &str) -> Result<ReportFormat, String> {
    ReportFormat::parse(s).ok_or_else(|| format!("unknown report format {s:?}; use md or json"))
}

fn parse_math_code(s: &str) -> Result<MathCode, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "nemeth" => Ok(MathCode::Nemeth),
        "ueb" => Ok(MathCode::Ueb),
        _ => Err(format!("unknown math code {s:?}; use nemeth or ueb")),
    }
}

fn parse_table_format(s: &str) -> Result<BrailleTableFormat, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "linear" => Ok(BrailleTableFormat::Linear),
        "listed" => Ok(BrailleTableFormat::Listed),
        "stairstep" => Ok(BrailleTableFormat::Stairstep),
        _ => Err(format!(
            "unknown table format {s:?}; use linear, listed, or stairstep"
        )),
    }
}

/// The line `tw convert` prints after converting to BRF: the math code, or
/// that this build writes math as words.
fn math_braille_line(code: MathCode) -> String {
    if cfg!(feature = "mathcat") {
        format!("Math braille: {}.", code.name())
    } else {
        "Math braille: none in this build; math is written as spoken words.".to_owned()
    }
}

fn parse_engine(s: &str) -> Result<Engine, String> {
    Engine::parse(s).ok_or_else(|| format!("unknown engine {s:?}; use pulldown or comrak"))
}

fn parse_flavor(s: &str) -> Result<Flavor, String> {
    Flavor::parse(s)
        .ok_or_else(|| format!("unknown flavor {s:?}; use gfm, obsidian, pandoc, or commonmark"))
}

fn parse_embeds(s: &str) -> Result<EmbedMode, String> {
    match s.to_ascii_lowercase().as_str() {
        "link" | "links" => Ok(EmbedMode::Link),
        "inline" => Ok(EmbedMode::Inline),
        _ => Err(format!("unknown embed mode {s:?}; use link or inline")),
    }
}

/// The options for `args`, with a publishing template applied under the
/// layout options: the template sets its defaults, then the layout options
/// given on the command line are applied again on top (applying them is
/// idempotent), so `--template apa --line-spacing 1.5` keeps 1.5.
fn options(args: &Args) -> ConvertOptions {
    let mut o = command_options(args);
    if let Some(t) = Template::parse(&args.template) {
        t.apply(&mut o.write);
        o.write = args.layout.apply(o.write);
    }
    o
}

/// Where a publishing template has no effect, the one sentence saying so.
fn template_note(args: &Args) -> Option<String> {
    let t = Template::parse(&args.template)?;
    match args.to {
        OutputFormat::Epub | OutputFormat::Docx | OutputFormat::Pdf => None,
        OutputFormat::Brf => Some(format!(
            "Template {}: braille keeps its own layout; the template applies to EPUB, Word, and PDF.",
            t.name()
        )),
        _ => Some(format!(
            "Template {}: applies to EPUB, Word, and PDF; this output uses its usual layout.",
            t.name()
        )),
    }
}

fn command_options(args: &Args) -> ConvertOptions {
    ConvertOptions {
        to: args.to,
        out_dir: args.out.clone(),
        render: RenderOptions {
            engine: args.engine,
            flavor: args.flavor,
            math: !args.no_math,
            asciimath: args.asciimath,
            sanitize: args.sanitize,
            smart_punctuation: args.smart,
            embeds: args.embeds,
            ..RenderOptions::default()
        },
        template: match Template::parse(&args.template) {
            Some(_) => TemplateChoice::default(),
            None => TemplateChoice::parse(&args.template),
        },
        template_dir: args.templates.clone(),
        toc: !args.no_toc,
        jobs: args.jobs,
        force: args.force,
        from: args.from.clone(),
        pandoc: !args.no_pandoc,
        pandoc_timeout: args
            .pandoc_timeout
            .filter(|&s| s > 0)
            .map(Duration::from_secs),
        citations: CitationOptions {
            enabled: !args.no_citations,
            bibliography: args.bibliography.clone(),
            style: args.style.clone(),
            user_library: super::paths(args.home.as_deref())
                .ok()
                .map(|p| textweaver_cite::user_library_path(&p.data_dir)),
        },
        write: args.layout.apply(WriteOptions {
            pdf: PdfOptions {
                font: args.pdf_font.clone(),
                ..PdfOptions::default()
            },
            braille: BrailleOptions {
                math_code: args.math_code,
                table_format: args.table_format,
                ..BrailleOptions::default()
            },
            ..WriteOptions::default()
        }),
        // What could not be made accessible goes in the report and the
        // JSON summary.
        audit: !args.no_report || args.json,
        ..ConvertOptions::default()
    }
}

/// The theme CSS for HTML pages: `--theme`, else the settings' theme
/// (`settings_theme`), with user themes from `themes_dir`. Never asks, so
/// scripts and pipes work the same as a terminal.
fn page_theme(
    args: &Args,
    settings_theme: &str,
    themes_dir: Option<&std::path::Path>,
) -> anyhow::Result<Option<String>> {
    textweaver_app::page_theme_css(args.theme.as_deref(), settings_theme, themes_dir)
        .map_err(anyhow::Error::msg)
}

/// Runs `tw convert`.
pub fn run(args: Args) -> anyhow::Result<()> {
    // A Lexend the reader downloaded is found by name (`--font lexend`).
    textweaver_app::use_downloaded_fonts(super::paths(args.home.as_deref()).ok().as_ref());
    let mut options = options(&args);
    if args.to == OutputFormat::Html {
        let (settings, message) = super::export_audio::load_settings(args.home.as_deref());
        if let Some(m) = message {
            eprintln!("{m}");
        }
        let themes = super::paths(args.home.as_deref())
            .ok()
            .map(|p| p.themes_dir());
        options.theme_css = page_theme(&args, &settings.display.theme, themes.as_deref())?;
        // The reader's font, spacing, and line length: a page looks like
        // the reader (themes carry colors only).
        options.typography = Some(textweaver_app::page_typography(&settings));
    }
    let converter = Converter::new(options)?;
    if let Some(note) = template_note(&args) {
        eprintln!("{note}");
    }
    if args.watch {
        return run_watch(&args, &converter);
    }
    let summary = converter.run(&args.inputs)?;
    let report = if args.no_report {
        None
    } else {
        Some(save_report(&args, &summary))
    };
    if args.json {
        crate::cmd::outln!("{}", serde_json::to_string_pretty(&summary)?);
        if let Some(line) = &report {
            eprintln!("{line}");
        }
    } else {
        for f in &summary.files {
            match &f.status {
                Status::Failed(reason) => {
                    eprintln!("Failed: {}: {reason}", f.source.display());
                }
                Status::Converted if args.verbose => {
                    crate::cmd::outln!(
                        "Converted {} to {}",
                        f.source.display(),
                        f.output.display()
                    );
                }
                Status::Skipped if args.verbose => {
                    crate::cmd::outln!("Up to date: {}", f.output.display());
                }
                _ => {}
            }
            // Writer warnings are always printed: they say what the output
            // is missing (an image, a braille symbol).
            for w in &f.warnings {
                eprintln!("Warning: {}: {w}", f.source.display());
            }
        }
        crate::cmd::outln!("{}", summary.sentence());
        if args.to == OutputFormat::Brf && summary.converted > 0 {
            crate::cmd::outln!("{}", math_braille_line(args.math_code));
        }
        if let Some(line) = report {
            crate::cmd::outln!("{line}");
        }
    }
    if summary.failed > 0 {
        bail!("{} of {} files failed", summary.failed, summary.total());
    }
    Ok(())
}

/// Saves the conversion report and says where it is, in one sentence: in
/// the output folder (or the one folder converted in place) for a batch,
/// else beside each output.
fn save_report(args: &Args, summary: &textweaver_convert::Summary) -> String {
    let format = args.report_format;
    if let Some(dir) = report_dir(args) {
        return match summary.write_report(&dir, format) {
            Ok(path) => format!("Report saved as {}.", path.display()),
            Err(e) => format!(
                "Warning: the report could not be saved in {}: {e}",
                dir.display()
            ),
        };
    }
    let written = summary.write_file_reports(format);
    let mut saved = Vec::new();
    let mut lines = Vec::new();
    for r in written {
        match r {
            Ok(path) => saved.push(path),
            Err(e) => lines.push(format!("Warning: a report could not be saved: {e}")),
        }
    }
    match saved.as_slice() {
        [] => {}
        [one] => lines.insert(0, format!("Report saved as {}.", one.display())),
        many => lines.insert(
            0,
            format!(
                "Reports saved beside each output, {} of them, named like {}.",
                many.len(),
                many[0]
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
            ),
        ),
    }
    lines.join("\n")
}

/// Where `tw convert` saves a batch's report: the output folder, or the
/// one folder being converted in place. Files named on their own, without
/// `--out`, get a report each, beside their output.
fn report_dir(args: &Args) -> Option<PathBuf> {
    match (&args.out, args.inputs.as_slice()) {
        (Some(out), _) => Some(out.clone()),
        (None, [only]) if only.is_dir() => Some(only.clone()),
        _ => None,
    }
}

fn run_watch(args: &Args, converter: &Converter) -> anyhow::Result<()> {
    for input in &args.inputs {
        if !input.is_dir() {
            bail!("--watch needs folders; {} is not a folder", input.display());
        }
    }
    if args.inputs.len() > 1 && args.out.is_some() {
        bail!("watch one folder at a time when --out is given, or leave out --out");
    }
    let opts = WatchOptions {
        stable: Duration::from_secs_f64(args.stable_seconds.max(0.0)),
        move_processed: !args.keep_sources,
        ..WatchOptions::default()
    };
    // Stopped with Control+C, which ends the process; outputs are written
    // to a temporary file and renamed, so none is left half-written.
    let stop = AtomicBool::new(false);
    let json = args.json;
    std::thread::scope(|scope| -> anyhow::Result<()> {
        let mut handles = Vec::new();
        for input in &args.inputs {
            let output = args.out.clone().unwrap_or_else(|| input.join("converted"));
            let (opts, stop) = (&opts, &stop);
            handles.push(scope.spawn(move || {
                watch(converter, input, &output, opts, stop, &mut |e| {
                    if json {
                        if let textweaver_convert::WatchEvent::File(r) = e
                            && let Ok(line) = serde_json::to_string(r)
                        {
                            crate::cmd::outln!("{line}");
                        }
                    } else {
                        crate::cmd::outln!("{}", e.sentence());
                    }
                })
            }));
        }
        for h in handles {
            match h.join() {
                Ok(r) => r.context("watching stopped")?,
                Err(_) => bail!("a watcher stopped unexpectedly"),
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        args: Args,
    }

    fn parse(args: &[&str]) -> Args {
        Cli::try_parse_from(std::iter::once("tw").chain(args.iter().copied()))
            .expect("the arguments parse")
            .args
    }

    #[test]
    fn a_publishing_template_goes_to_the_writers_under_the_layout_options() {
        let args = parse(&[
            "paper.md",
            "--to",
            "docx",
            "--template",
            "apa",
            "--line-spacing",
            "1.5",
        ]);
        let o = options(&args);
        assert_eq!(o.write.template, Some(Template::ApaStudentPaper));
        assert!(o.write.pdf.title_page && o.write.epub.cover);
        // The command line wins over the template's double spacing.
        assert_eq!(o.write.pdf.line_spacing, 1.5);
        // The HTML page template stays the default.
        assert!(matches!(o.template, TemplateChoice::Named(ref n) if n == "default"));
        assert!(template_note(&args).is_none());
    }

    #[test]
    fn the_report_goes_to_the_output_folder_or_the_one_folder() {
        let dir = tempfile::tempdir().expect("temp dir");
        let folder = dir.path().to_string_lossy().into_owned();
        let args = parse(&["a.md", "b.md", "--out", "converted"]);
        assert_eq!(report_dir(&args), Some(PathBuf::from("converted")));
        let args = parse(&[&folder]);
        assert_eq!(report_dir(&args), Some(dir.path().to_owned()));
        assert_eq!(report_dir(&parse(&["a.md"])), None);
        assert_eq!(report_dir(&parse(&[&folder, "a.md"])), None);
        assert!(parse(&[&folder, "--no-report"]).no_report);
        assert_eq!(parse(&[&folder]).report_format, ReportFormat::Markdown);
        assert_eq!(
            parse(&[&folder, "--report-format", "json"]).report_format,
            ReportFormat::Json
        );
        assert!(Cli::try_parse_from(["tw", "a.md", "--report-format", "txt"]).is_err());
    }

    #[test]
    fn a_file_named_on_its_own_gets_a_report_beside_its_output() {
        let dir = tempfile::tempdir().expect("temp dir");
        let src = dir.path().join("essay.md");
        std::fs::write(&src, "# Results\n\n![](chart.png)\n").expect("written");
        let src_s = src.to_string_lossy().into_owned();
        let args = parse(&[&src_s, "--to", "html"]);
        let converter = Converter::new(options(&args)).expect("converter");
        let summary = converter.run(&args.inputs).expect("ran");
        let said = save_report(&args, &summary);
        let report = dir.path().join("essay.html.report.md");
        assert_eq!(said, format!("Report saved as {}.", report.display()));
        let text = std::fs::read_to_string(&report).expect("the report");
        assert!(
            text.starts_with("# Conversion report for essay.md\n"),
            "{text}"
        );
        assert!(
            text.contains(
                "1. Image without a description: chart.png. Under the heading \u{201c}Results\u{201d}, line 3."
            ),
            "{text}"
        );
        let folder = dir.path().to_string_lossy().into_owned();
        assert!(!text.contains(&folder), "no folder in the report: {text}");
    }

    #[test]
    fn html_page_templates_are_unchanged() {
        let o = options(&parse(&["page.md", "--to", "html", "--template", "print"]));
        assert_eq!(o.write.template, None);
        assert!(matches!(o.template, TemplateChoice::Named(ref n) if n == "print"));
        let args = parse(&["page.md", "--to", "html", "--template", "large-print"]);
        let note = template_note(&args).expect("a note for HTML");
        assert!(note.starts_with("Template large-print:"), "{note}");
    }

    #[test]
    fn the_braille_table_format_reaches_the_writer() {
        let o = options(&parse(&["notes.md", "--to", "brf"]));
        assert_eq!(o.write.braille.table_format, BrailleTableFormat::Linear);
        let o = options(&parse(&[
            "notes.md",
            "--to",
            "brf",
            "--table-format",
            "Stairstep",
        ]));
        assert_eq!(o.write.braille.table_format, BrailleTableFormat::Stairstep);
        let bad = Cli::try_parse_from(["tw", "notes.md", "--table-format", "spiral"]);
        assert!(bad.is_err());
    }

    /// `--theme` picks the page's theme; without it the settings' theme is
    /// used, Galaxy keeps the default stylesheet, and nothing asks. An
    /// unknown name is an error that lists the names.
    #[test]
    fn the_theme_comes_from_the_flag_or_the_settings() {
        let args = parse(&["notes.md", "--to", "html", "--theme", "sepia"]);
        let css = page_theme(&args, "galaxy", None).unwrap().unwrap();
        assert!(css.starts_with("/* textweaver theme: Sepia */"), "{css}");
        let plain = parse(&["notes.md", "--to", "html"]);
        assert_eq!(page_theme(&plain, "galaxy", None).unwrap(), None);
        let nord = page_theme(&plain, "nord", None).unwrap().unwrap();
        assert!(nord.starts_with("/* textweaver theme: Nord */"), "{nord}");
        let bad = parse(&["notes.md", "--to", "html", "--theme", "plaid"]);
        let err = page_theme(&bad, "galaxy", None).unwrap_err().to_string();
        assert!(
            err.contains("unknown theme \"plaid\"") && err.contains("sepia"),
            "{err}"
        );
    }

    /// A converted page carries the chosen theme's properties.
    #[test]
    fn the_chosen_theme_is_in_the_page() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("notes.md");
        std::fs::write(&src, "# Notes\n\nText.\n").unwrap();
        let args = parse(&["notes.md", "--to", "html", "--theme", "sepia"]);
        let mut o = options(&args);
        o.theme_css = page_theme(&args, "galaxy", None).unwrap();
        o.audit = false;
        let summary = Converter::new(o).unwrap().run(&[src]).unwrap();
        assert_eq!(summary.converted, 1);
        let page = std::fs::read_to_string(dir.path().join("notes.html")).unwrap();
        assert!(page.contains("/* textweaver theme: Sepia */"));
        assert!(!page.contains("textweaver themes: Galaxy Light"));
    }

    /// A converted page takes its font and line length from the reading
    /// settings, as `run` passes them.
    #[test]
    fn the_reading_settings_set_the_page_type() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("notes.md");
        std::fs::write(&src, "# Notes\n\nText.\n").unwrap();
        let args = parse(&["notes.md", "--to", "html"]);
        let mut settings = textweaver_app::store::Settings::default();
        settings.display.measure = 50;
        settings.reading_aids.font.size_pt = 18.0;
        let mut o = options(&args);
        o.typography = Some(textweaver_app::page_typography(&settings));
        o.audit = false;
        Converter::new(o).unwrap().run(&[src]).unwrap();
        let page = std::fs::read_to_string(dir.path().join("notes.html")).unwrap();
        assert!(page.contains("--tw-type-measure: 50ch;"), "{page}");
        assert!(page.contains("--tw-type-size: 150%;"), "{page}");
    }
}
