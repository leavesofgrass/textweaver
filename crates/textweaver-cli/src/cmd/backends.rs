//! `tw backends`. Owner: Agent B.

/// Arguments for `tw backends`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
}

/// Runs `tw backends`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let _ = args;
    super::not_implemented("backends")
}
