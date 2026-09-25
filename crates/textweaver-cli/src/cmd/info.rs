//! `tw info`. Owner: Agent A.

use std::path::PathBuf;

/// Arguments for `tw info`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Document to describe.
    pub file: PathBuf,
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
}

/// Runs `tw info`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let _ = args;
    super::not_implemented("info")
}
