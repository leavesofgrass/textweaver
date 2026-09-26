//! `tw open`. Owner: Agent D.
//!
//! Runs the terminal reader in this process (the same code as the
//! `textweaver` binary, through `textweaver_tui::launch`), so `tw` works on
//! its own without the reader binary installed next to it.

use std::path::PathBuf;

use anyhow::bail;

/// Arguments for `tw open`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Document to open: a file, a file inside an archive
    /// (`book.zip!chapter.pdf`), or a web address.
    pub file: PathBuf,
    /// Do not speak (use with a screen reader).
    #[arg(long)]
    pub no_speech: bool,
    /// Accessibility mode for this run: self-voicing, hybrid, or
    /// screen-reader (not saved).
    #[arg(long, value_name = "MODE")]
    pub mode: Option<textweaver_tui::AccessMode>,
    /// Speech backend id (see `tw backends`).
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

/// The reader's options for `args`.
fn options(args: &Args) -> textweaver_tui::Options {
    textweaver_tui::Options {
        no_speech: args.no_speech,
        mode: args.mode,
        backend: args.backend.clone(),
        home: args.home.clone(),
        theme: args.theme.clone(),
        log: args.log.clone(),
    }
}

/// Runs `tw open`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let source = textweaver_app::formats::Source::Path(args.file.clone());
    if source.url().is_none() && !textweaver_app::formats::archive::exists(&args.file) {
        bail!("{}: no such file", args.file.display());
    }
    textweaver_tui::launch(&options(&args), Some(&args.file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forwards_options_to_the_reader() {
        let args = Args {
            file: "doc.md".into(),
            no_speech: true,
            mode: Some(textweaver_tui::AccessMode::Hybrid),
            backend: Some("espeak".into()),
            home: Some("state".into()),
            theme: Some("light".into()),
            log: Some("info".into()),
        };
        let o = options(&args);
        assert_eq!(o.log.as_deref(), Some("info"));
        assert!(o.no_speech);
        assert_eq!(o.mode, Some(textweaver_tui::AccessMode::Hybrid));
        assert_eq!(o.backend.as_deref(), Some("espeak"));
        assert_eq!(o.home, Some(PathBuf::from("state")));
        assert_eq!(o.theme.as_deref(), Some("light"));
    }

    #[test]
    fn missing_file_is_an_error() {
        let args = Args {
            file: "definitely/not/here.txt".into(),
            no_speech: true,
            mode: None,
            backend: None,
            home: None,
            theme: None,
            log: None,
        };
        assert!(run(args).unwrap_err().to_string().contains("no such file"));
    }
}
