//! `tw open`. Owner: Agent D.

use std::path::PathBuf;

/// Arguments for `tw open`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Document to open.
    pub file: PathBuf,
}

/// Runs `tw open`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let _ = args;
    super::not_implemented("open")
}
