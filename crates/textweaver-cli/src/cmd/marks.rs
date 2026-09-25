//! `tw marks`. Owner: Agent C.

use std::path::PathBuf;

/// Arguments for `tw marks`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Document whose marks to list.
    pub file: PathBuf,
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
}

/// Runs `tw marks`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let _ = args;
    super::not_implemented("marks")
}
