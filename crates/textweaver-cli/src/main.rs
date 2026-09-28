//! `tw`: the textweaver command line.
//!
//! One module per subcommand under `cmd/`: `open`, `text`, `info`,
//! `search`, `speak`, `voices`, `backends`, `eloquence`, `convert` (with
//! `convert_layout` for the PDF and EPUB layout flags), `export-audio`,
//! `library`, `vault`, `dictate`, `marks`, `lint`, `migrate-star`, `cite`,
//! `settings` (with `profile`), `define`, `stats`, `serve`, and `ocr`. Each module's docs name the ADR and crate it
//! wraps; the user guides are listed in `docs/README.md`.

use anyhow::Result;
use clap::{Parser, Subcommand};

mod cmd;

/// Read, extract, and speak documents from the command line.
#[derive(Parser, Debug)]
#[command(name = "tw", version, about, propagate_version = true)]
struct Cli {
    /// The command; with none, `tw` prints [`NO_COMMAND_HINT`].
    #[command(subcommand)]
    command: Option<Cmd>,
}

/// What `tw` with no arguments prints, on standard output, before exiting
/// with status 0: two lines instead of the whole command list as an error
/// (usability pass, item 6). `tw --help` still lists every command.
const NO_COMMAND_HINT: &str = "tw open FILE reads a document aloud in the terminal reader.
tw --help lists every command.
";

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
    /// Find ETI-Eloquence on this computer and explain how to get it.
    Eloquence(cmd::eloquence::Args),
    /// Convert documents and folders (Markdown, HTML, text, EPUB, Word, braille, PDF), or watch a folder.
    Convert(cmd::convert::Args),
    /// Read a document aloud into an audio file, with optional subtitles.
    #[command(name = "export-audio")]
    ExportAudio(cmd::export_audio::Args),
    /// Manage the library: folders, recent documents, full-text search.
    Library(cmd::library::Args),
    /// Import from or export to an Obsidian vault.
    Vault(cmd::vault::Args),
    /// Voice typing: transcribe speech to text.
    Dictate(cmd::dictate::Args),
    /// List a document's saved position and bookmarks, or export its notes as references.
    Marks(cmd::marks::Args),
    /// Check Markdown files for problems a listener would miss: heading levels, list markers, trailing spaces, link references, bare web addresses.
    Lint(cmd::lint::Args),
    /// Import settings and reading positions from Star.
    #[command(name = "migrate-star")]
    MigrateStar(cmd::migrate::Args),
    /// Manage references: add by DOI or ISBN, import, export, format, list.
    Cite(cmd::cite::Args),
    /// Export, import, locate, or reset settings and key overrides (JSON or TOML), and manage settings profiles.
    Settings(cmd::settings::Args),
    /// Define words offline: your glossary, then Open English WordNet, with pronunciations.
    Define(cmd::define::Args),
    /// Reading statistics: time read aloud, the furthest point, and sessions per document.
    Stats(cmd::stats::Args),
    /// Serve the app over JSON-RPC 2.0 on stdin and stdout, for editors and other tools.
    Serve(cmd::serve::Args),
    /// Text recognition for scanned pages: engine status, model downloads, and reading a scan.
    Ocr(cmd::ocr::Args),
}

fn main() -> Result<()> {
    let Some(command) = Cli::parse().command else {
        return cmd::print_all(NO_COMMAND_HINT);
    };
    match command {
        Cmd::Open(a) => cmd::open::run(a),
        Cmd::Text(a) => cmd::text::run(a),
        Cmd::Info(a) => cmd::info::run(a),
        Cmd::Search(a) => cmd::search::run(a),
        Cmd::Speak(a) => cmd::speak::run(a),
        Cmd::Voices(a) => cmd::voices::run(a),
        Cmd::Backends(a) => cmd::backends::run(a),
        Cmd::Eloquence(a) => cmd::eloquence::run(a),
        Cmd::Convert(a) => cmd::convert::run(a),
        Cmd::ExportAudio(a) => cmd::export_audio::run(a),
        Cmd::Library(a) => cmd::library::run(a),
        Cmd::Vault(a) => cmd::vault::run(a),
        Cmd::Dictate(a) => cmd::dictate::run(a),
        Cmd::Marks(a) => cmd::marks::run(a),
        Cmd::Lint(a) => cmd::lint::run(a),
        Cmd::MigrateStar(a) => cmd::migrate::run(a),
        Cmd::Cite(a) => cmd::cite::run(a),
        Cmd::Settings(a) => cmd::settings::run(a),
        Cmd::Define(a) => cmd::define::run(a),
        Cmd::Stats(a) => cmd::stats::run(a),
        Cmd::Serve(a) => cmd::serve::run(a),
        Cmd::Ocr(a) => cmd::ocr::run(a),
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser as _;

    #[test]
    fn no_arguments_is_not_an_error() {
        let cli = super::Cli::try_parse_from(["tw"]).unwrap();
        assert!(cli.command.is_none());
        assert_eq!(super::NO_COMMAND_HINT.lines().count(), 2);
    }
}
