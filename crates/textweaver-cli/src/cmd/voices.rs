//! `tw voices`. Owner: Agent B.

/// Arguments for `tw voices`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Backend id (default: auto).
    #[arg(long)]
    pub backend: Option<String>,
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
}

/// Runs `tw voices`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let _ = args;
    super::not_implemented("voices")
}
