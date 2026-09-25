//! `tw`: the textweaver scripting CLI.
//!
//! One module per subcommand under `cmd/`, each owned by one agent
//! (docs/tasks.md): text, info, search (A); speak, voices, backends (B);
//! marks, migrate-star (C); open, serve (D).

use anyhow::Result;
use clap::{Parser, Subcommand};

mod cmd;

/// Read, extract, and speak documents from the command line.
#[derive(Parser, Debug)]
#[command(name = "tw", version, about, propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Open a document in the terminal reader.
    Open(cmd::open::Args),
    /// Print a document's canonical text.
    Text(cmd::text::Args),
    /// Print facts about a document (format, title, length, structure counts).
    Info(cmd::info::Args),
    /// Search a document.
    Search(cmd::search::Args),
    /// Speak text or a file, or write it to an audio file.
    Speak(cmd::speak::Args),
    /// List the voices of a backend.
    Voices(cmd::voices::Args),
    /// List speech backends and their availability.
    Backends(cmd::backends::Args),
    /// List a document's saved position and bookmarks.
    Marks(cmd::marks::Args),
    /// Import settings and reading positions from Star.
    #[command(name = "migrate-star")]
    MigrateStar(cmd::migrate::Args),
    /// Serve the app over JSON-RPC on stdio (wave 2).
    Serve(cmd::serve::Args),
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Cmd::Open(a) => cmd::open::run(a),
        Cmd::Text(a) => cmd::text::run(a),
        Cmd::Info(a) => cmd::info::run(a),
        Cmd::Search(a) => cmd::search::run(a),
        Cmd::Speak(a) => cmd::speak::run(a),
        Cmd::Voices(a) => cmd::voices::run(a),
        Cmd::Backends(a) => cmd::backends::run(a),
        Cmd::Marks(a) => cmd::marks::run(a),
        Cmd::MigrateStar(a) => cmd::migrate::run(a),
        Cmd::Serve(a) => cmd::serve::run(a),
    }
}
