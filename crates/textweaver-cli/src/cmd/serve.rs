//! `tw serve`. Owner: Agent D.

/// Arguments for `tw serve`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Serve JSON-RPC over stdin and stdout.
    #[arg(long)]
    pub stdio: bool,
}

/// Runs `tw serve`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let _ = args;
    super::not_implemented("serve")
}
