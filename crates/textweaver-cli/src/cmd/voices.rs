//! `tw voices`. Owner: Agent B.
//!
//! Lists the voices of a backend (default: the one your settings choose,
//! `[speech] backend`, else the one automatic selection picks), one voice
//! per line so a screen reader reads each as a sentence. The backends are
//! configured from your settings (the Eloquence library and dictionaries,
//! SAPI's OneCore voices), as the reader configures them.

use std::path::PathBuf;

use anyhow::Context;
use serde::Serialize;
use textweaver_app::speech::{BackendRegistry, Selection, Voice};
use textweaver_app::store::Settings;

/// Arguments for `tw voices`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Backend id (default: the settings' backend, else auto).
    #[arg(long)]
    pub backend: Option<String>,
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
    /// Read settings from this directory (like `TEXTWEAVER_HOME`).
    #[arg(long)]
    pub home: Option<PathBuf>,
}

/// What `tw voices --json` prints.
#[derive(Debug, Serialize)]
pub struct Report {
    /// The backend asked.
    pub backend: Selection,
    /// Its voices.
    pub voices: Vec<Voice>,
}

/// Lists the voices of the selected backend: `--backend`, else the
/// settings' backend.
pub fn voices(
    args: &Args,
    settings: &Settings,
    registry: &BackendRegistry,
) -> anyhow::Result<Report> {
    let asked = args
        .backend
        .as_deref()
        .or(Some(settings.speech.backend.as_str()));
    let selection = registry.select(asked);
    let factory = registry
        .factory(selection.backend.id)
        .with_context(|| format!("backend {} is not built in", selection.backend.id))?;
    let backend = factory()?;
    let voices = backend.voices()?;
    Ok(Report {
        backend: selection,
        voices,
    })
}

/// One line describing a voice, for reading aloud.
pub fn describe(v: &Voice) -> String {
    let mut s = format!("{}: {}", v.id, v.name);
    if !v.languages.is_empty() {
        s.push_str(&format!(", {}", v.languages.join(", ")));
    }
    if let Some(g) = &v.gender {
        s.push_str(&format!(", {g}"));
    }
    for t in &v.tags {
        // Names often already say it ("Zira (OneCore)"); do not repeat.
        if !v.name.to_lowercase().contains(&t.to_lowercase()) {
            s.push_str(&format!(", {t}"));
        }
    }
    s.push('.');
    s
}

/// Runs `tw voices`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let (settings, message) = super::export_audio::load_settings(args.home.as_deref());
    if let Some(m) = message {
        eprintln!("{m}");
    }
    let registry = textweaver_engines::speech_registry_for(&settings);
    let report = voices(&args, &settings, &registry)?;
    if let Some(msg) = report.backend.fallback_message() {
        eprintln!("{msg}");
    }
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    let n = report.voices.len();
    println!(
        "{} has {n} voice{}.",
        report.backend.backend.name,
        if n == 1 { "" } else { "s" }
    );
    for v in &report.voices {
        println!("{}", describe(v));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_recording_voices() {
        let args = Args {
            backend: Some("recording".into()),
            json: true,
            home: None,
        };
        let r = voices(
            &args,
            &Settings::default(),
            &BackendRegistry::with_builtins(),
        )
        .unwrap();
        assert_eq!(r.backend.backend.id, "recording");
        assert_eq!(describe(&r.voices[1]), "rec-en-us: Recording US, en-US.");
    }

    #[test]
    fn the_settings_backend_is_the_default() {
        let args = Args {
            backend: None,
            json: true,
            home: None,
        };
        let mut settings = Settings::default();
        settings.speech.backend = "recording".into();
        let r = voices(&args, &settings, &BackendRegistry::with_builtins()).unwrap();
        // The recording backend is opt-in: only the settings choose it.
        assert_eq!(r.backend.backend.id, "recording");
        assert!(!r.backend.fell_back);
    }

    #[test]
    fn tags_are_read_unless_the_name_says_them() {
        let v = Voice {
            id: "eci:enu:reed".into(),
            name: "Eloquence Reed, American English".into(),
            languages: vec!["en-US".into()],
            gender: Some("male".into()),
            tags: vec!["Eloquence".into(), "OpenEVV".into()],
        };
        assert_eq!(
            describe(&v),
            "eci:enu:reed: Eloquence Reed, American English, en-US, male, OpenEVV."
        );
    }
}
