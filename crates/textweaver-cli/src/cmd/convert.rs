//! `tw convert`: convert files and folders, on every core, or watch a
//! folder. Owner: Agent L.
//!
//! Output is written for listening: one summary sentence at the end, one
//! line per failure, and per-file lines only with `--verbose`. `--json`
//! prints the whole summary (every file, its status and timing) instead.
//! The exit status is 1 when any file failed.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use anyhow::{Context, bail};
use textweaver_convert::{
    ConvertOptions, Converter, OutputFormat, Status, WatchOptions, watch,
};
use textweaver_render::{EmbedMode, Engine, Flavor, RenderOptions, TemplateChoice};

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
    /// HTML page template: default, print, fragment, a name from
    /// --templates, or a file path.
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
    /// Obsidian embeds of notes: link (default) or inline.
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

fn options(args: &Args) -> ConvertOptions {
    ConvertOptions {
        to: args.to,
        out_dir: args.out.clone(),
        render: RenderOptions {
            engine: args.engine,
            flavor: args.flavor,
            math: !args.no_math,
            sanitize: args.sanitize,
            smart_punctuation: args.smart,
            embeds: args.embeds,
            ..RenderOptions::default()
        },
        template: TemplateChoice::parse(&args.template),
        template_dir: args.templates.clone(),
        toc: !args.no_toc,
        jobs: args.jobs,
        force: args.force,
        pandoc: !args.no_pandoc,
        ..ConvertOptions::default()
    }
}

/// Runs `tw convert`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let converter = Converter::new(options(&args))?;
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
        }
        println!("{}", summary.sentence());
    }
    if summary.failed > 0 {
        bail!(
            "{} of {} files failed",
            summary.failed,
            summary.total()
        );
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
            let output = args
                .out
                .clone()
                .unwrap_or_else(|| input.join("converted"));
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
