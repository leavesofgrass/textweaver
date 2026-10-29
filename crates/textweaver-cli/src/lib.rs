//! `tw`: textweaver's terminal program, as a library (B1-o1).
//!
//! `tw` alone, or `tw FILE`, opens the terminal reader; `tw COMMAND` runs
//! a command headless, with no terminal interface. In the packages the
//! terminal program is part of the one program, `textweaver`, which opens
//! the app by default and runs this library with `textweaver --tw` (or
//! `--cli`), when started as `tw`, or when its first argument is a
//! command ([`is_command`]); it calls [`main_with`]. This crate's own `tw`
//! binary is the terminal program alone, without the app, for development
//! and the tests; it calls [`main`].
//!
//! A new subcommand goes here: a module in `cmd/`, a variant in `Cmd`
//! below, and its arm in `run`.
//!
//! One module per subcommand under `cmd/`: `open`, `text`, `info`,
//! `search`, `speak`, `voices`, `backends`, `eloquence`, `convert` (with
//! `convert_layout` for the PDF and EPUB layout flags), `export-audio`,
//! `library`, `vault`, `dictate`, `marks`, `notes`, `lint`, `migrate-star`, `cite`,
//! `settings` (with `profile`), `define`, `stats`, `study`, `summarize`, `changes`, `serve`, `ocr`, `components`, `update`, and, with the `mqtt` feature, `broker`, `publish` (`tw publish`, with the older names `tw send` and `tw share` as hidden aliases, and `live_conn` for their connection options) and `join_qr`. Each module's docs name the ADR and crate it
//! wraps; the user guides are listed in `docs/README.md`.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

mod cmd;

/// The program's description at the top of `tw --help`.
const ABOUT: &str = "Read documents aloud in the terminal reader, or run a command without it. tw FILE opens a document in the reader; tw COMMAND runs a command, such as tw convert. textweaver --tw does the same as tw; textweaver alone opens the textweaver app.";

/// The reader's main keys, at the end of `tw --help`.
const READER_KEYS: &str = "In the reader, Space reads and pauses, Escape stops, h moves by heading, F1 opens the help, ? lists every key, and Ctrl+Q quits.";

/// Read, extract, and speak documents, in the terminal reader or from the
/// command line. The name is fixed, so `tw --help` and `textweaver --tw
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
    /// CW (Morse code) tools: monitor what a key adapter sends (MIDI and keys), in words.
    Cw(cmd::cw::Args),
    /// Run textweaver's own MQTT broker for live documents and net logs, or manage its accounts.
    #[cfg(feature = "mqtt")]
    Broker(cmd::broker::Args),
    /// Publish a document live to a broker, again on each save, or with --check-in one check-in.
    #[cfg(feature = "mqtt")]
    Publish(cmd::publish::PublishArgs),
    /// The older name of tw publish --check-in: publish one entry, a JSON object.
    #[cfg(feature = "mqtt")]
    #[command(hide = true)]
    Send(cmd::publish::SendArgs),
    /// The older name of tw publish FILE: publish a document live.
    #[cfg(feature = "mqtt")]
    #[command(hide = true)]
    Share(cmd::publish::ShareArgs),
    /// Read the join link from a QR code in an image (PNG or JPEG) and print it.
    #[cfg(feature = "mqtt")]
    JoinQr(cmd::join_qr::Args),
}

/// Runs `tw` and turns the result into its exit status: 0 done, 1 failed
/// or nothing found, 2 a usage error (clap exits with 2 itself before
/// `main` runs a command). An error is printed as one line on standard
/// error, "Error: what failed: why. What to do." (the "Command line"
/// page).
pub fn main() -> ExitCode {
    main_with(std::env::args_os().skip(1))
}

/// Runs `tw` with `args`, the arguments after the program's name, as
/// [`main`] does: the combined program calls it with what follows `--tw`.
/// Help and errors name the program `tw` whatever it was started as.
pub fn main_with<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let argv = std::iter::once(OsString::from("tw")).chain(args.into_iter().map(Into::into));
    match run(Cli::parse_from(argv)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if e.downcast_ref::<cmd::NothingFound>().is_some() => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("{}", cmd::one_line(&e));
            ExitCode::FAILURE
        }
    }
}

/// True when `name` is one of `tw`'s commands (`convert`, `text`, and the
/// rest, or an alias of one), so `textweaver convert ...` runs the command
/// line as `tw convert ...` does.
pub fn is_command(name: &OsStr) -> bool {
    use clap::CommandFactory as _;
    let Some(name) = name.to_str() else {
        return false;
    };
    Cli::command()
        .get_subcommands()
        .any(|c| c.get_name() == name || c.get_all_aliases().any(|a| a == name))
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
        Cmd::Cw(a) => cmd::cw::run(a),
        #[cfg(feature = "mqtt")]
        Cmd::Broker(a) => cmd::broker::run(a),
        #[cfg(feature = "mqtt")]
        Cmd::Publish(a) => cmd::publish::run(a),
        #[cfg(feature = "mqtt")]
        Cmd::Send(a) => cmd::publish::send(a),
        #[cfg(feature = "mqtt")]
        Cmd::Share(a) => cmd::publish::share(a),
        #[cfg(feature = "mqtt")]
        Cmd::JoinQr(a) => cmd::join_qr::run(a),
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

    /// Started under any name (`textweaver --tw` passes it on), the help
    /// names `tw`, so the two agree.
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

    /// The commands are known by name; a file name or an option is not a
    /// command.
    #[test]
    fn commands_are_known_by_name() {
        use std::ffi::OsStr;
        for c in [
            "convert",
            "text",
            "speak",
            "export-audio",
            "migrate-star",
            "serve",
            "update",
        ] {
            assert!(super::is_command(OsStr::new(c)), "{c}");
        }
        for not in ["book.md", "--help", "", "Convert", "help"] {
            assert!(!super::is_command(OsStr::new(not)), "{not}");
        }
    }

    /// `tw publish FILE` publishes a document live; `tw publish
    /// --check-in` one entry; the options of one never go with the other.
    #[cfg(feature = "mqtt")]
    #[test]
    fn publish_takes_a_document_or_a_check_in() {
        let cli =
            Cli::try_parse_from(["tw", "publish", "agenda.md", "--topic", "docs/agenda"]).unwrap();
        let Some(Cmd::Publish(a)) = cli.command else {
            panic!("not publish");
        };
        assert_eq!(a.file.as_deref(), Some(std::path::Path::new("agenda.md")));
        assert!(!a.check_in);
        let cli = Cli::try_parse_from([
            "tw",
            "publish",
            "--check-in",
            "--topic",
            "netlog/checkins",
            "--field",
            "callsign=N0CALL",
            "--retain",
        ])
        .unwrap();
        let Some(Cmd::Publish(a)) = cli.command else {
            panic!("not publish");
        };
        assert!(a.check_in && a.retain && a.file.is_none());
        assert_eq!(a.field, ["callsign=N0CALL"]);
        for bad in [
            &["tw", "publish", "--topic", "t"][..],
            &["tw", "publish", "a.md", "--check-in", "--topic", "t"],
            &["tw", "publish", "a.md", "--topic", "t", "--field", "a=1"],
            &["tw", "publish", "a.md", "--topic", "t", "--retain"],
            &["tw", "publish", "--check-in", "--topic", "t", "--once"],
            &[
                "tw",
                "publish",
                "--check-in",
                "--topic",
                "t",
                "--title",
                "A",
            ],
            &[
                "tw",
                "publish",
                "--check-in",
                "--topic",
                "t",
                "--qr",
                "a.png",
            ],
            &[
                "tw",
                "publish",
                "--check-in",
                "--topic",
                "t",
                "--every",
                "2",
            ],
        ] {
            assert!(Cli::try_parse_from(bad).is_err(), "{bad:?}");
        }
    }

    /// The older names keep their arguments, stay out of the command list,
    /// and are named in `tw publish --help`.
    #[cfg(feature = "mqtt")]
    #[test]
    fn share_and_send_are_hidden_aliases_named_in_the_publish_help() {
        use clap::CommandFactory as _;
        use std::ffi::OsStr;
        let cli =
            Cli::try_parse_from(["tw", "share", "agenda.md", "--topic", "t", "--once"]).unwrap();
        assert!(matches!(cli.command, Some(Cmd::Share(_))));
        let cli = Cli::try_parse_from(["tw", "send", "--topic", "t", "--field", "a=1"]).unwrap();
        assert!(matches!(cli.command, Some(Cmd::Send(_))));
        for name in ["publish", "share", "send"] {
            assert!(super::is_command(OsStr::new(name)), "{name}");
        }
        let help = Cli::command().render_help().to_string();
        let listed: Vec<&str> = help
            .lines()
            .filter_map(|l| l.strip_prefix("  "))
            .filter_map(|l| l.split_whitespace().next())
            .collect();
        assert!(listed.contains(&"publish"), "{help}");
        assert!(
            !listed.contains(&"share") && !listed.contains(&"send"),
            "{help}"
        );
        let err = Cli::command()
            .try_get_matches_from(["tw", "publish", "--help"])
            .unwrap_err();
        let publish = err.render().to_string();
        assert!(publish.contains("Usage: tw publish"), "{publish}");
        assert!(publish.contains("--check-in"), "{publish}");
        assert!(
            publish.contains("Also: tw share and tw send, the older names"),
            "{publish}"
        );
    }
}
