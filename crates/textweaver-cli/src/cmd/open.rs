//! The terminal reader: `tw`, `tw FILE`, and `tw open FILE`. Owner: Agent D.
//!
//! Runs the terminal reader in this process (`textweaver_tui::launch`).
//! Since beta 1 (B1-o1) the reader is `tw`'s default mode: `tw` alone
//! opens it empty, `tw FILE` opens a document, and `tw open FILE`, the
//! older spelling, does the same. `textweaver` is a second name for the
//! same program.

use std::io::IsTerminal as _;
use std::path::PathBuf;

use anyhow::bail;

/// The reader's options, shared by `tw [FILE]` and `tw open FILE`.
#[derive(clap::Args, Debug, Clone, Default)]
pub struct Reader {
    /// Do not speak: no self-voicing and no reading aloud (use with a
    /// screen reader, which reads the status line and follows the cursor).
    #[arg(long)]
    pub no_speech: bool,
    /// Accessibility mode for this run: self-voicing (textweaver speaks
    /// everything), hybrid (textweaver reads documents aloud; your screen
    /// reader speaks messages and typing), or screen-reader (textweaver is
    /// silent). Not saved; Alt+Shift+A changes and saves it.
    #[arg(long, value_name = "MODE")]
    pub mode: Option<textweaver_tui::AccessMode>,
    /// Speech backend id (see `tw backends`); overrides the settings.
    #[arg(long)]
    pub backend: Option<String>,
    /// Keep settings and reading positions under this directory.
    #[arg(long)]
    pub home: Option<PathBuf>,
    /// Color theme for this run; the help lists every theme.
    #[arg(long, help = textweaver_tui::theme_help())]
    pub theme: Option<String>,
    /// Write a log to textweaver.log in the state folder at this level:
    /// off, error, warn (the default), info, debug, or trace. --log alone
    /// means debug.
    #[arg(long, value_name = "LEVEL", num_args = 0..=1, default_missing_value = "debug")]
    pub log: Option<String>,
}

/// Arguments for `tw open`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Document to open: a file, a file inside an archive
    /// (`book.zip!chapter.pdf`), or a web address.
    pub file: PathBuf,
    /// The reader's options.
    #[command(flatten)]
    pub reader: Reader,
}

/// The reader's options for `reader`.
fn options(reader: &Reader) -> textweaver_tui::Options {
    textweaver_tui::Options {
        no_speech: reader.no_speech,
        mode: reader.mode,
        backend: reader.backend.clone(),
        home: reader.home.clone(),
        theme: reader.theme.clone(),
        log: reader.log.clone(),
    }
}

/// Runs `tw open`.
pub fn run(args: Args) -> anyhow::Result<()> {
    read(Some(args.file), &args.reader)
}

/// Opens the terminal reader, on `file` or empty: `tw`, `tw FILE`, and
/// `tw open FILE`. A missing file or a folder is refused first; then,
/// with standard output not a terminal (`tw > out.txt`, a script), the
/// reader is refused in one line instead of drawing into a file or pipe.
pub fn read(file: Option<PathBuf>, reader: &Reader) -> anyhow::Result<()> {
    if let Some(file) = &file {
        check_file(file)?;
    }
    if !std::io::stdout().is_terminal() {
        return Err(anyhow::Error::new(super::Plain(NO_TERMINAL.to_owned())));
    }
    textweaver_tui::launch(&options(reader), file.as_deref())
}

/// The error when the reader is asked for without a terminal.
pub(crate) const NO_TERMINAL: &str = "The terminal reader needs a terminal, and standard output is not one. Run tw in a terminal, or give a command such as tw convert; tw --help lists them.";

/// Refuses a file that does not exist, or a folder. A web address, or a
/// member of an archive (`book.zip!inner.pdf`), is not a file on disk
/// under that exact name, so only plain paths are checked.
fn check_file(file: &std::path::Path) -> anyhow::Result<()> {
    let source = textweaver_app::formats::Source::Path(file.to_owned());
    if source.url().is_none() {
        if !textweaver_app::formats::archive::exists(file) {
            bail!(
                "{}: no such file. Check the name and the folder.",
                file.display()
            );
        }
        if file.is_dir() {
            bail!(
                "{} is a folder, not a document. Give the name of a file in it.",
                file.display()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(file: PathBuf) -> Args {
        Args {
            file,
            reader: Reader {
                no_speech: true,
                ..Reader::default()
            },
        }
    }

    #[test]
    fn forwards_options_to_the_reader() {
        let reader = Reader {
            no_speech: true,
            mode: Some(textweaver_tui::AccessMode::Hybrid),
            backend: Some("espeak".into()),
            home: Some("state".into()),
            theme: Some("light".into()),
            log: Some("info".into()),
        };
        let o = options(&reader);
        assert_eq!(o.log.as_deref(), Some("info"));
        assert!(o.no_speech);
        assert_eq!(o.mode, Some(textweaver_tui::AccessMode::Hybrid));
        assert_eq!(o.backend.as_deref(), Some("espeak"));
        assert_eq!(o.home, Some(PathBuf::from("state")));
        assert_eq!(o.theme.as_deref(), Some("light"));
    }

    #[test]
    fn missing_file_is_an_error() {
        let err = run(args("definitely/not/here.txt".into())).unwrap_err();
        assert!(err.to_string().contains("no such file"));
    }

    /// A folder is refused before the reader starts, with a reason.
    #[test]
    fn a_folder_is_not_a_document() {
        let dir = tempfile::tempdir().unwrap();
        let err = run(args(dir.path().to_owned())).unwrap_err().to_string();
        assert!(err.contains("is a folder, not a document"), "{err}");
    }

    /// The refusal without a terminal says what failed first, within the
    /// first 40 Braille cells, and names the way out.
    #[test]
    fn the_no_terminal_message_reads_first_things_first() {
        assert!(NO_TERMINAL.starts_with("The terminal reader needs a terminal"));
        assert!(NO_TERMINAL.contains("tw --help"));
    }
}
