//! `tw convert`. Owner: Agent A (wave 2).

use std::path::PathBuf;

/// Arguments for `tw convert`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Files or folders to convert.
    pub inputs: Vec<PathBuf>,
    /// Output format.
    #[arg(long, value_parser = ["markdown", "html", "text"], default_value = "markdown")]
    pub to: String,
    /// Output directory.
    #[arg(long)]
    pub out: Option<PathBuf>,
    /// Keep watching the folders and convert new files as they arrive.
    #[arg(long)]
    pub watch: bool,
}

/// Runs `tw convert`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let _ = args;
    super::not_implemented("convert")
}
