//! `tw speak`. Owner: Agent B.

use std::path::PathBuf;

/// Arguments for `tw speak`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Text to speak (omit with --file).
    pub text: Option<String>,
    /// Speak this file instead.
    #[arg(long)]
    pub file: Option<PathBuf>,
    /// Backend id (default: auto).
    #[arg(long)]
    pub backend: Option<String>,
    /// Voice id.
    #[arg(long)]
    pub voice: Option<String>,
    /// Rate in words per minute.
    #[arg(long)]
    pub rate: Option<u16>,
    /// Pitch offset in semitones.
    #[arg(long, allow_hyphen_values = true)]
    pub pitch: Option<i8>,
    /// Write audio to this file instead of playing it.
    #[arg(long)]
    pub out: Option<PathBuf>,
    /// Print the utterances, offset maps, and status events as JSON.
    #[arg(long)]
    pub json: bool,
}

/// Runs `tw speak`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let _ = args;
    super::not_implemented("speak")
}
