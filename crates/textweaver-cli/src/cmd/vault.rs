//! `tw vault`. Owner: Agent J (wave 2).

use std::path::PathBuf;

/// Arguments for `tw vault`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// `import` or `export`.
    #[arg(value_parser = ["import", "export"])]
    pub action: String,
    /// The Obsidian vault folder.
    pub vault: PathBuf,
    /// Document whose notes to export (export only).
    #[arg(long)]
    pub document: Option<PathBuf>,
}

/// Runs `tw vault`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let _ = args;
    super::not_implemented("vault")
}
