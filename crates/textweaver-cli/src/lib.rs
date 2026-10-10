//! `tw`: textweaver's terminal program, as a library (B1-o1).
//!
//! `tw` alone, or `tw FILE`, opens the terminal reader; `tw COMMAND` runs
//! a command headless, with no terminal interface. `textweaver` is a
//! second name for the same program: a link on Linux and macOS, and on
//! Windows a small launcher (`src/bin/textweaver.rs`) that runs `tw.exe`
//! beside it with the same arguments. Both binaries are thin; `tw` calls
//! [`main`].
//!
//! A new subcommand goes here: a module in `cmd/`, a variant in `Cmd`
//! below, and its arm in `run`.
//!
//! One module per subcommand under `cmd/`: `open`, `text`, `info`,
//! `search`, `speak`, `voices`, `backends`, `eloquence`, `convert` (with
//! `convert_layout` for the PDF and EPUB layout flags), `export-audio`,
//! `library`, `vault`, `dictate`, `marks`, `notes`, `lint`, `migrate-star`, `cite`,
//! `settings` (with `profile`), `define`, `stats`, `study`, `summarize`, `changes`, `serve`, `ocr`, `components`, and `update`. Each module's docs name the ADR and crate it
//! wraps; the user guides are listed in `docs/README.md`.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

mod cmd;

/// The program's description at the top of `tw --help`.
const ABOUT: &str = "Read documents aloud in the terminal reader, or run a command without it. tw FILE opens a document in the reader; tw COMMAND runs a command, such as tw convert. textweaver is a second name for tw.";

/// The reader's main keys, at the end of `tw --help`.
const READER_KEYS: &str = "In the reader, Space reads and pauses, Escape stops, h moves by heading, F1 opens the help, ? lists every key, and Ctrl+Q quits.";

/// Read, extract, and speak documents, in the terminal reader or from the
/// command line. The name is fixed, so `tw --help` and `textweaver
/// --help` print the same text.
#[derive(Parser, Debug)]
#[command(
    name = "tw",
    bin_name = "tw",
    version = textweaver_app::VERSION_TEXT,
    about = ABOUT,
    after_help = READER_KEYS,
    propagate_version = true,
    args_conflicts_with_subcommands = true
)]
struct Cli {
    /// Document to open in the terminal reader: a file, a file inside an
    /// archive (`book.zip!chapter.pdf`), or a web address. With none, the
    /// reader opens empty.
    file: Option<PathBuf>,
    /// The reader's options.
    #[command(flatten)]
    reader: cmd::open::Reader,
    /// The command; with none, `tw` opens the terminal reader.
    #[command(subcommand)]
    command: Option<Cmd>,
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
    /// Links between notes: a document's links out and what links to its notes.
    Notes(cmd::notes::Args),
    /// Check Markdown files for problems a listener would miss: heading levels, list markers, trailing spaces, link references, bare web addresses.
    Lint(cmd::lint::Args),
    /// Import settings and reading positions from star.
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
    /// Study cards: how many are due today, and how many are new.
    Study(cmd::study::Args),
    /// Summarize a document: its most central sentences, one per line, without a model.
    Summarize(cmd::summarize::Args),
    /// List a document's tracked changes and comments, or write it with every change accepted or rejected.
    Changes(cmd::changes::Args),
    /// Sync notes, highlights, bookmarks, and places with your other computers: setup, status, now.
    Sync(cmd::sync::Args),
    /// Serve the app over JSON-RPC 2.0 on stdin and stdout, for editors and other tools.
    Serve(cmd::serve::Args),
    /// Text recognition for scanned pages: engine status, model downloads, and reading a scan.
    Ocr(cmd::ocr::Args),
    /// Optional components (models, fonts, voices): list, download, verify, remove, or install from a file.
    Components(cmd::components::Args),
    /// Check for a newer textweaver on GitHub, or download, check, and install it.
    Update(cmd::update::Args),
}

/// Runs `tw` and turns the result into its exit status: 0 done, 1 failed
/// or nothing found, 2 a usage error (clap exits with 2 itself before
/// `main` runs a command). An error is printed as one line on standard
/// error, "Error: what failed: why. What to do." (the "Command line"
/// page).
pub fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if e.downcast_ref::<cmd::NothingFound>().is_some() => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("{}", cmd::one_line(&e));
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    let Some(command) = cli.command else {
        return cmd::open::read(cli.file, &cli.reader);
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
        Cmd::Notes(a) => cmd::notes::run(a),
        Cmd::Lint(a) => cmd::lint::run(a),
        Cmd::MigrateStar(a) => cmd::migrate::run(a),
        Cmd::Cite(a) => cmd::cite::run(a),
        Cmd::Settings(a) => cmd::settings::run(a),
        Cmd::Define(a) => cmd::define::run(a),
        Cmd::Stats(a) => cmd::stats::run(a),
        Cmd::Study(a) => cmd::study::run(a),
        Cmd::Summarize(a) => cmd::summarize::run(a),
        Cmd::Changes(a) => cmd::changes::run(a),
        Cmd::Sync(a) => cmd::sync::run(a),
        Cmd::Serve(a) => cmd::serve::run(a),
        Cmd::Ocr(a) => cmd::ocr::run(a),
        Cmd::Components(a) => cmd::components::run(a),
        Cmd::Update(a) => cmd::update::run(a),
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser as _;

    use super::{Cli, Cmd};

    #[test]
    fn no_arguments_opens_the_reader_empty() {
        let cli = Cli::try_parse_from(["tw"]).unwrap();
        assert!(cli.command.is_none());
        assert!(cli.file.is_none());
    }

    /// A file opens the reader; a command name runs the command; the
    /// reader's options go with a file, never with a command.
    #[test]
    fn a_file_opens_the_reader_and_a_command_runs_headless() {
        let cli = Cli::try_parse_from(["tw", "book.md", "--no-speech"]).unwrap();
        assert!(cli.command.is_none());
        assert_eq!(cli.file.as_deref(), Some(std::path::Path::new("book.md")));
        assert!(cli.reader.no_speech);
        let cli = Cli::try_parse_from(["tw", "convert", "a.md", "--to", "html"]).unwrap();
        assert!(matches!(cli.command, Some(Cmd::Convert(_))));
        assert!(cli.file.is_none());
        let cli = Cli::try_parse_from(["tw", "open", "book.md"]).unwrap();
        assert!(matches!(cli.command, Some(Cmd::Open(_))));
        assert!(Cli::try_parse_from(["tw", "--no-speech", "convert", "a.md"]).is_err());
    }

    /// `textweaver` is the same program: started under either name, the
    /// help names `tw`, so the two agree.
    #[test]
    fn the_help_is_the_same_under_either_name() {
        use clap::CommandFactory as _;
        let help = |argv0: &str| {
            let err = Cli::command()
                .try_get_matches_from([argv0, "--help"])
                .unwrap_err();
            err.render().to_string()
        };
        let tw = help("tw");
        assert_eq!(tw, help("textweaver"));
        assert_eq!(tw, help("/usr/local/bin/textweaver"));
        assert!(tw.contains("Usage: tw [OPTIONS] [FILE]"), "{tw}");
    }
}
