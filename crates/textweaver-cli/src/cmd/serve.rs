//! `tw serve`. Owner: Agent D.
//!
//! Wave 2 serves the app over JSON-RPC on stdin and stdout. The argument
//! surface is fixed now; running it reports that it is not available yet.

/// Arguments for `tw serve`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Serve JSON-RPC over stdin and stdout.
    #[arg(long)]
    pub stdio: bool,
}

/// Runs `tw serve`.
pub fn run(args: Args) -> anyhow::Result<()> {
    if !args.stdio {
        anyhow::bail!("`tw serve` needs a transport; the only one planned is --stdio");
    }
    anyhow::bail!("`tw serve --stdio` (JSON-RPC over stdio) arrives in wave 2")
}
