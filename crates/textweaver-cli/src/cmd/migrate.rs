//! `tw migrate-star`. Owner: Agent C.

use std::path::PathBuf;

/// Arguments for `tw migrate-star`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Star configuration directory (default: the platform location).
    #[arg(long)]
    pub from: Option<PathBuf>,
    /// Show what would be imported without writing anything.
    #[arg(long)]
    pub dry_run: bool,
}

/// Runs `tw migrate-star`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let _ = args;
    super::not_implemented("migrate-star")
}
