//! `tw convert`: convert files and folders, on every core, or watch a
//! folder. Owner: Agent L.
//!
//! Output is written for listening: one summary sentence at the end, one
//! line per failure, and per-file lines only with `--verbose`. `--json`
//! prints the whole summary (every file, its status and timing) instead.
//! The exit status is 1 when any file failed.
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
    BrailleOptions, CitationOptions, ConvertOptions, Converter, MathCode, OutputFormat, PdfOptions,
    Status, WatchOptions, WriteOptions, watch,
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
    /// Output format: md, html, txt, epub, docx, brf, or pdf.
    #[arg(long, default_value = "md", value_parser = parse_format)]
    pub to: OutputFormat,
    /// Output folder (default: beside each source; for --watch, a
    /// "converted" folder inside the watched folder).
    #[arg(long)]
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
    #[command(flatten)]
    pub layout: super::convert_layout::LayoutArgs,
    /// Watch: seconds a file's size must hold still before converting.
    #[arg(long, default_value_t = 2.0)]
    pub stable_seconds: f64,
    /// Watch: leave converted sources in place instead of moving them to
    /// "processed".
    #[arg(long)]
    pub keep_sources: bool,
}

fn parse_format(s: &str) -> Result<OutputFormat, String> {
    OutputFormat::parse(s)
        .ok_or_else(|| format!("unknown format {s:?}; use md, html, txt, epub, docx, brf, or pdf"))
}

fn parse_math_code(s: &str) -> Result<MathCode, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "nemeth" => Ok(MathCode::Nemeth),
        "ueb" => Ok(MathCode::Ueb),
        _ => Err(format!("unknown math code {s:?}; use nemeth or ueb")),
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
        pandoc: !args.no_pandoc,
        pandoc_timeout: args
            .pandoc_timeout
            .filter(|&s| s > 0)
            .map(Duration::from_secs),
        citations: CitationOptions {
            enabled: !args.no_citations,
            bibliography: args.bibliography.clone(),
            style: args.style.clone(),
            user_library: textweaver_app::store::Paths::platform()
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
                ..BrailleOptions::default()
            },
            ..WriteOptions::default()
        }),
        ..ConvertOptions::default()
    }
}

/// Runs `tw convert`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let converter = Converter::new(options(&args))?;
    if let Some(note) = template_note(&args) {
        eprintln!("{note}");
    }
    if args.watch {
        return run_watch(&args, &converter);
    }
    let summary = converter.run(&args.inputs)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        for f in &summary.files {
            match &f.status {
                Status::Failed(reason) => {
                    eprintln!("Failed: {}: {reason}", f.source.display());
                }
                Status::Converted if args.verbose => {
                    println!("Converted {} to {}", f.source.display(), f.output.display());
                }
                Status::Skipped if args.verbose => {
                    println!("Up to date: {}", f.output.display());
                }
                _ => {}
            }
            // Writer warnings are always printed: they say what the output
            // is missing (an image, a braille symbol).
            for w in &f.warnings {
                eprintln!("Warning: {}: {w}", f.source.display());
            }
        }
        println!("{}", summary.sentence());
        if args.to == OutputFormat::Brf && summary.converted > 0 {
            println!("{}", math_braille_line(args.math_code));
        }
    }
    if summary.failed > 0 {
        bail!("{} of {} files failed", summary.failed, summary.total());
    }
    Ok(())
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
                            println!("{line}");
                        }
                    } else {
                        println!("{}", e.sentence());
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
    fn html_page_templates_are_unchanged() {
        let o = options(&parse(&["page.md", "--to", "html", "--template", "print"]));
        assert_eq!(o.write.template, None);
        assert!(matches!(o.template, TemplateChoice::Named(ref n) if n == "print"));
        let args = parse(&["page.md", "--to", "html", "--template", "large-print"]);
        let note = template_note(&args).expect("a note for HTML");
        assert!(note.starts_with("Template large-print:"), "{note}");
    }
}
