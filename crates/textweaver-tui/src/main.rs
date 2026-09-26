//! `textweaver`: the self-voicing terminal reader.
//!
//! Owner: Agent D.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use textweaver_tui::{Options, launch};

/// Read documents aloud in the terminal.
#[derive(Parser, Debug)]
#[command(name = "textweaver", version, about)]
struct Args {
    /// Document to open.
    file: Option<PathBuf>,
    /// Do not speak: no self-voicing and no reading aloud (use with a
    /// screen reader, which reads the status line and follows the cursor).
    #[arg(long)]
    no_speech: bool,
    /// Speech backend id (see `tw backends`); overrides the settings.
    #[arg(long)]
    backend: Option<String>,
    /// Keep settings and reading positions under this directory.
    #[arg(long)]
    home: Option<PathBuf>,
    /// Color theme for this run: galaxy, light, or high-contrast.
    #[arg(long)]
    theme: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let opts = Options {
        no_speech: args.no_speech,
        backend: args.backend,
        home: args.home,
        theme: args.theme,
    };
    launch(&opts, args.file.as_deref())
}
