//! `tw open`. Owner: Agent D.
//!
//! Launches the terminal reader (the `textweaver` binary) on a document. The
//! binary is looked up next to `tw` first (both are built into the same
//! target directory and installed together), then on `PATH`. Running it as
//! a child keeps `tw` free of a dependency on the TUI crate; see the wave 1
//! report for the proposed library dependency instead.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};

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
    /// Color theme for this run: galaxy, light, or high-contrast.
    #[arg(long)]
    pub theme: Option<String>,
}

/// The reader binary's file name on this platform.
fn reader_name() -> String {
    format!("textweaver{}", std::env::consts::EXE_SUFFIX)
}

/// The reader next to `tw`, else the bare name for a `PATH` lookup.
fn reader_path() -> PathBuf {
    let beside = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(reader_name())));
    match beside {
        Some(p) if p.is_file() => p,
        _ => PathBuf::from(reader_name()),
    }
}

/// The reader's command line for `args`.
fn reader_args(args: &Args) -> Vec<std::ffi::OsString> {
    let mut out: Vec<std::ffi::OsString> = vec![args.file.clone().into()];
    if args.no_speech {
        out.push("--no-speech".into());
    }
    if let Some(b) = &args.backend {
        out.push("--backend".into());
        out.push(b.into());
    }
    if let Some(h) = &args.home {
        out.push("--home".into());
        out.push(h.clone().into());
    }
    if let Some(t) = &args.theme {
        out.push("--theme".into());
        out.push(t.into());
    }
    out
}

/// Runs `tw open`.
pub fn run(args: Args) -> anyhow::Result<()> {
    if !Path::new(&args.file).exists() {
        bail!("{}: no such file", args.file.display());
    }
    let reader = reader_path();
    let status = Command::new(&reader)
        .args(reader_args(&args))
        .status()
        .with_context(|| {
            format!(
                "cannot start the terminal reader {} (build it with `cargo build -p textweaver-tui`)",
                reader.display()
            )
        })?;
    if !status.success() {
        bail!("the terminal reader exited with {status}");
    }
    Ok(())
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
            home: None,
            theme: Some("light".into()),
        };
        let got: Vec<String> = reader_args(&args)
            .into_iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            got,
            [
                "doc.md",
                "--no-speech",
                "--backend",
                "espeak",
                "--theme",
                "light"
            ]
        );
        assert!(reader_name().starts_with("textweaver"));
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
