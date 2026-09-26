//! `tw export-audio`. Owner: Agent B (wave 2).

use std::path::PathBuf;

/// Arguments for `tw export-audio`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Document to read aloud.
    pub file: PathBuf,
    /// Output file (.wav, .mp3, .m4b).
    #[arg(long)]
    pub out: PathBuf,
    /// Also write subtitles (.srt or .vtt).
    #[arg(long)]
    pub subtitles: Option<PathBuf>,
    /// Backend id.
    #[arg(long)]
    pub backend: Option<String>,
    /// Voice id or name.
    #[arg(long)]
    pub voice: Option<String>,
    /// Rate in words per minute.
    #[arg(long)]
    pub rate: Option<u16>,
}

/// Runs `tw export-audio`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let _ = args;
    super::not_implemented("export-audio")
}
