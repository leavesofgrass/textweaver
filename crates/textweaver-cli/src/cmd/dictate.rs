//! `tw dictate`. Owner: Agent J (wave 2).

use std::path::PathBuf;

/// Arguments for `tw dictate`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Transcribe this audio file instead of the microphone.
    #[arg(long)]
    pub file: Option<PathBuf>,
    /// Whisper model name.
    #[arg(long)]
    pub model: Option<String>,
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
}

/// Runs `tw dictate`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let _ = args;
    super::not_implemented("dictate")
}
