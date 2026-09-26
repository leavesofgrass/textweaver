//! `tw library`. Owner: Agent C (wave 2).

use std::path::PathBuf;

/// Arguments for `tw library`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Search text across the library.
    #[arg(long)]
    pub search: Option<String>,
    /// Add a folder to the library.
    #[arg(long)]
    pub add: Option<PathBuf>,
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
}

/// Runs `tw library`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let _ = args;
    super::not_implemented("library")
}
