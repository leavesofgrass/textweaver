//! `tw text`. Owner: Agent A.

use std::path::PathBuf;

/// Arguments for `tw text`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Document to extract.
    pub file: PathBuf,
    /// Output format.
    #[arg(long, value_parser = ["text", "markdown", "json"], default_value = "text")]
    pub format: String,
    /// Include markers (structure) in the output.
    #[arg(long)]
    pub structure: bool,
}

/// Runs `tw text`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let _ = args;
    super::not_implemented("text")
}
