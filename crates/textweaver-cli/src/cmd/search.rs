//! `tw search`. Owner: Agent A.

use std::path::PathBuf;

/// Arguments for `tw search`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Document to search.
    pub file: PathBuf,
    /// Text or pattern to find.
    pub pattern: String,
    /// Treat the pattern as a regular expression.
    #[arg(long)]
    pub regex: bool,
    /// Match case exactly.
    #[arg(long)]
    pub case_sensitive: bool,
    /// Match whole words only.
    #[arg(long)]
    pub whole_word: bool,
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
}

/// Runs `tw search`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let _ = args;
    super::not_implemented("search")
}
