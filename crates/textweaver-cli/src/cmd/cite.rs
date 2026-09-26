//! `tw cite`: the reference library. Owner: Agent P (wave 2).
//!
//! The work is done by `textweaver_cite::commands`, which is tested there
//! with a temporary library; this file only parses arguments, finds the
//! library, and prints.
//!
//! Libraries: your personal library is `references.json` in textweaver's
//! data folder (`TEXTWEAVER_HOME/data` when that is set). `--folder DIR`
//! uses `DIR/references.json` instead, falling back to the personal
//! library when formatting or checking; `--library FILE` uses any file.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context as _;
use textweaver_app::store::Paths;
use textweaver_cite::commands::{self, Context, Show};
use textweaver_cite::{Format, OutputFormat, UreqClient, folder_library_path, user_library_path};

/// Arguments for `tw cite`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Use this library file instead of your personal library.
    #[arg(long, global = true, value_name = "FILE")]
    pub library: Option<PathBuf>,
    /// Use the library of a document folder (FOLDER/references.json); keys
    /// it lacks are looked up in your personal library.
    #[arg(long, global = true, value_name = "FOLDER", conflicts_with = "library")]
    pub folder: Option<PathBuf>,
    /// What to do.
    #[command(subcommand)]
    pub command: CiteCommand,
}

/// `tw cite` subcommands.
#[derive(clap::Subcommand, Debug)]
pub enum CiteCommand {
    /// Look up a DOI (doi.org) or ISBN (Open Library) and add it.
    Add {
        /// A DOI such as 10.1038/nature12373, a doi.org link, or an ISBN.
        identifier: String,
        /// Seconds to wait for an answer.
        #[arg(long, default_value_t = 15)]
        timeout: u64,
    },
    /// Import references from a BibTeX, BibLaTeX, RIS, or CSL-JSON file.
    Import {
        /// The file (.bib, .ris, or .json).
        file: PathBuf,
    },
    /// Export the library, or some keys, as BibTeX, BibLaTeX, RIS, or CSL-JSON.
    Export {
        /// bibtex, biblatex, ris, or csl-json.
        #[arg(long, value_parser = parse_format)]
        to: Format,
        /// Keys to export; all references when none are given.
        keys: Vec<String>,
        /// Write to this file instead of standard output.
        #[arg(long, short)]
        output: Option<PathBuf>,
    },
    /// Format references with a citation style.
    Format {
        /// Keys to format; all references when none are given.
        keys: Vec<String>,
        /// apa, mla, chicago, chicago-notes, harvard, ieee, vancouver, ama,
        /// nature, any name from `tw cite styles`, or a .csl file.
        #[arg(long, default_value = "apa")]
        style: String,
        /// Output markup: plain, markdown, or html.
        #[arg(long = "as", value_name = "FORMAT", default_value = "plain", value_parser = parse_output)]
        output: OutputFormat,
        /// Print only the in-text citation of all the keys together.
        #[arg(long, conflicts_with = "bibliography")]
        citation: bool,
        /// Print only the bibliography entries.
        #[arg(long)]
        bibliography: bool,
    },
    /// List the references in the library.
    List {
        /// Print the library as CSL-JSON.
        #[arg(long)]
        json: bool,
    },
    /// Remove a reference by key.
    Remove {
        /// The citation key.
        key: String,
    },
    /// List the built-in citation styles.
    Styles,
    /// List a document's citations whose keys are not in the library.
    Check {
        /// A Markdown or text document with Pandoc citations ([@key]).
        file: PathBuf,
    },
}

fn parse_format(s: &str) -> Result<Format, String> {
    Format::from_name(s).ok_or_else(|| {
        format!("{s} is not a reference format; use bibtex, biblatex, ris, or csl-json")
    })
}

fn parse_output(s: &str) -> Result<OutputFormat, String> {
    OutputFormat::from_name(s)
        .ok_or_else(|| format!("{s} is not an output format; use plain, markdown, or html"))
}

/// Runs `tw cite`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let paths = Paths::platform().context("could not find textweaver's data folder")?;
    let user = user_library_path(&paths.data_dir);
    let (library, fallback) = match (&args.library, &args.folder) {
        (Some(file), _) => (file.clone(), vec![user]),
        (None, Some(dir)) => (folder_library_path(dir), vec![user]),
        (None, None) => (user, Vec::new()),
    };
    let timeout = match &args.command {
        CiteCommand::Add { timeout, .. } => Duration::from_secs(*timeout),
        _ => textweaver_cite::lookup::DEFAULT_TIMEOUT,
    };
    let client = UreqClient::new(timeout);
    let ctx = Context {
        library,
        fallback,
        cache_dir: Some(paths.cache_dir.join("citations")),
        client: &client,
    };
    let out = match args.command {
        CiteCommand::Add { identifier, .. } => commands::add(&ctx, &identifier)?,
        CiteCommand::Import { file } => commands::import(&ctx, &file)?,
        CiteCommand::Export { to, keys, output } => {
            let text = commands::export(&ctx, to, &keys)?;
            match output {
                Some(path) => {
                    std::fs::write(&path, &text)
                        .with_context(|| format!("could not write {}", path.display()))?;
                    format!(
                        "Exported the references as {} to {}.",
                        to.display_name(),
                        path.display()
                    )
                }
                None => text.trim_end().to_owned(),
            }
        }
        CiteCommand::Format {
            keys,
            style,
            output,
            citation,
            bibliography,
        } => {
            let show = if citation {
                Show::Citation
            } else if bibliography {
                Show::Bibliography
            } else {
                Show::Both
            };
            commands::format(&ctx, &keys, &style, output, show)?
        }
        CiteCommand::List { json } => commands::list(&ctx, json)?.trim_end().to_owned(),
        CiteCommand::Remove { key } => commands::remove(&ctx, &key)?,
        CiteCommand::Styles => commands::styles(),
        CiteCommand::Check { file } => {
            let text = std::fs::read_to_string(&file)
                .with_context(|| format!("could not read {}", file.display()))?;
            commands::check(&ctx, &text)?
        }
    };
    println!("{out}");
    Ok(())
}
