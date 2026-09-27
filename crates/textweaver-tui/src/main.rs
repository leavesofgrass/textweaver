//! `textweaver`: the self-voicing terminal reader.
//!
//! Owner: Agent D.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use textweaver_tui::{Options, launch};

/// Read documents aloud in the terminal.
#[derive(Parser, Debug)]
#[command(
    name = "textweaver",
    version,
    about = "Read documents aloud in the terminal: text, Markdown, HTML, EPUB, Word, and PDF. Inside, Space reads and pauses, Escape stops, h moves by heading, F1 opens the help, ? lists every key, and Ctrl+Q quits."
)]
struct Args {
    /// Document to open.
    file: Option<PathBuf>,
    /// Do not speak: no self-voicing and no reading aloud (use with a
    /// screen reader, which reads the status line and follows the cursor).
    #[arg(long)]
    no_speech: bool,
    /// Accessibility mode for this run: self-voicing (textweaver speaks
    /// everything), hybrid (textweaver reads documents aloud; your screen
    /// reader speaks messages and typing), or screen-reader (textweaver is
    /// silent). Not saved; Alt+Shift+A changes and saves it.
    #[arg(long, value_name = "MODE")]
    mode: Option<textweaver_tui::AccessMode>,
    /// Speech backend id (see `tw backends`); overrides the settings.
    #[arg(long)]
    backend: Option<String>,
    /// Keep settings and reading positions under this directory.
    #[arg(long)]
    home: Option<PathBuf>,
    /// Color theme for this run; the help lists every theme.
    #[arg(long, help = textweaver_tui::theme_help())]
    theme: Option<String>,
    /// Write a log to textweaver.log in the state folder at this level:
    /// off, error, warn (the default), info, debug, or trace. --log alone
    /// means debug.
    #[arg(long, value_name = "LEVEL", num_args = 0..=1, default_missing_value = "debug")]
    log: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let opts = Options {
        no_speech: args.no_speech,
        mode: args.mode,
        backend: args.backend,
        home: args.home,
        theme: args.theme,
        log: args.log,
    };
    launch(&opts, args.file.as_deref())
}
