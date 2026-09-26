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
    /// Document to open.
    pub file: PathBuf,
    /// Do not speak (use with a screen reader).
    #[arg(long)]
    pub no_speech: bool,
    /// Speech backend id (see `tw backends`).
    #[arg(long)]
    pub backend: Option<String>,
    /// Keep settings and reading positions under this directory.
    #[arg(long)]
    pub home: Option<PathBuf>,
    /// Color theme for this run; the help lists every theme.
    #[arg(long, help = textweaver_tui::theme_help())]
    pub theme: Option<String>,
}

/// The reader's options for `args`.
fn options(args: &Args) -> textweaver_tui::Options {
    textweaver_tui::Options {
        no_speech: args.no_speech,
        backend: args.backend.clone(),
        home: args.home.clone(),
        theme: args.theme.clone(),
    }
}

/// Runs `tw open`.
pub fn run(args: Args) -> anyhow::Result<()> {
    if !args.file.exists() {
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
            backend: Some("espeak".into()),
            home: Some("state".into()),
            theme: Some("light".into()),
        };
        let o = options(&args);
        assert!(o.no_speech);
        assert_eq!(o.backend.as_deref(), Some("espeak"));
        assert_eq!(o.home, Some(PathBuf::from("state")));
        assert_eq!(o.theme.as_deref(), Some("light"));
    }

    #[test]
    fn missing_file_is_an_error() {
        let args = Args {
            file: "definitely/not/here.txt".into(),
            no_speech: true,
            backend: None,
            home: None,
            theme: None,
        };
        assert!(run(args).unwrap_err().to_string().contains("no such file"));
    }
}
