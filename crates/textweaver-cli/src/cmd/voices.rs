//! `tw voices`. Owner: Agent B.
//!
//! Lists the voices of a backend (default: the one automatic selection
//! picks), one voice per line so a screen reader reads each as a sentence.

use anyhow::Context;
use serde::Serialize;
use textweaver_app::speech::{BackendRegistry, Selection, Voice};

/// Arguments for `tw voices`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Backend id (default: auto).
    #[arg(long)]
    pub backend: Option<String>,
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
}

/// What `tw voices --json` prints.
#[derive(Debug, Serialize)]
pub struct Report {
    /// The backend asked.
    pub backend: Selection,
    /// Its voices.
    pub voices: Vec<Voice>,
}

/// Lists the voices of the selected backend.
pub fn voices(args: &Args, registry: &BackendRegistry) -> anyhow::Result<Report> {
    let selection = registry.select(args.backend.as_deref());
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
    let report = voices(&args, &textweaver_app::speech_registry())?;
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
        };
        let r = voices(&args, &BackendRegistry::with_builtins()).unwrap();
        assert_eq!(r.backend.backend.id, "recording");
        assert_eq!(describe(&r.voices[1]), "rec-en-us: Recording US, en-US.");
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
