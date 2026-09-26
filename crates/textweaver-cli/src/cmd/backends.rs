//! `tw backends`. Owner: Agent B.
//!
//! Lists every speech backend compiled into this build, highest priority
//! first, with availability, which one automatic selection picks, and which
//! one your settings (`[speech] backend`) choose. Each backend is one
//! sentence, so the list reads well aloud.
//!
//! Discovery (finding Eloquence, SAPI voices, DECtalk, eSpeak) runs once:
//! every backend is probed a single time and both choices are made from
//! that one list. The registry is configured from the user's settings, as
//! the reader and `tw export-audio` configure it.

use std::path::PathBuf;

use serde::Serialize;
use textweaver_app::speech::backends::select_from;
use textweaver_app::speech::{BackendInfo, BackendRegistry, Caps};

/// Arguments for `tw backends`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
    /// Read settings from this directory (like `TEXTWEAVER_HOME`).
    #[arg(long)]
    pub home: Option<PathBuf>,
}

/// What `tw backends --json` prints.
#[derive(Debug, Serialize)]
pub struct Report {
    /// The backend automatic selection chooses.
    pub auto: &'static str,
    /// The backend your settings choose (`[speech] backend`, falling back
    /// to automatic selection when it is not available).
    pub selected: &'static str,
    /// Every backend, highest priority first.
    pub backends: Vec<BackendInfo>,
}

/// Builds the report, probing every backend once. `configured` is the
/// settings' backend (`"auto"` or empty for automatic selection).
pub fn report(registry: &BackendRegistry, configured: Option<&str>) -> Report {
    let backends = registry.list();
    Report {
        auto: select_from(&backends, None).backend.id,
        selected: select_from(&backends, configured).backend.id,
        backends,
    }
}

/// What a backend can do, as a phrase list ("word highlighting, pause,
/// audio files"); empty when it can do none of the things listed.
pub fn features(caps: Caps) -> Vec<&'static str> {
    [
        (Caps::WORD_EVENTS, "word highlighting"),
        (Caps::PAUSE, "pause"),
        (Caps::PITCH, "pitch"),
        (Caps::VOLUME, "volume"),
        (Caps::SYNTH_TO_FILE, "audio files"),
        (Caps::TONES, "tones"),
        (
            Caps::NATIVE_NORMALIZATION,
            "reads numbers and abbreviations itself",
        ),
    ]
    .into_iter()
    .filter(|(c, _)| caps.contains(*c))
    .map(|(_, s)| s)
    .collect()
}

/// "a, b, and c".
fn and_list(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [a] => (*a).to_owned(),
        [a, b] => format!("{a} and {b}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}

/// One sentence describing a backend; `auto` is the automatic choice and
/// `selected` the one the settings choose.
pub fn describe(b: &BackendInfo, auto: &str, selected: &str) -> String {
    let mut s = format!(
        "{}: {}. {}.",
        b.id,
        b.name,
        if b.available {
            "Available"
        } else {
            "Not available"
        }
    );
    if b.priority != i32::MIN {
        s.push_str(&format!(" Priority {}.", b.priority));
    } else {
        s.push_str(" Last resort.");
    }
    let f = features(b.caps);
    if !f.is_empty() {
        s.push_str(&format!(" Supports {}.", and_list(&f)));
    }
    if b.opt_in {
        s.push_str(" Only when chosen by name.");
    }
    if b.id == auto {
        s.push_str(" Chosen automatically.");
    }
    if b.id == selected && selected != auto {
        s.push_str(" Chosen by your settings.");
    }
    s
}

/// Runs `tw backends`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let (settings, message) = super::export_audio::load_settings(args.home.as_deref());
    if let Some(m) = message {
        eprintln!("{m}");
    }
    let registry = textweaver_engines::speech_registry_for(&settings);
    let r = report(&registry, Some(settings.speech.backend.as_str()));
    if args.json {
        println!("{}", serde_json::to_string_pretty(&r)?);
        return Ok(());
    }
    for b in &r.backends {
        println!("{}", describe(b, r.auto, r.selected));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_are_listed_with_a_choice() {
        let r = report(&BackendRegistry::with_builtins(), None);
        assert!(r.backends.iter().any(|b| b.id == "null"));
        let null = r.backends.iter().find(|b| b.id == "null").unwrap();
        let line = describe(null, "null", "null");
        assert_eq!(
            line,
            "null: Silent (no audio). Available. Last resort. \
             Supports pause, pitch, and volume. Chosen automatically."
        );
        let rec = r.backends.iter().find(|b| b.id == "recording").unwrap();
        assert_eq!(
            describe(rec, "null", "null"),
            "recording: Recording (test double). Available. Priority 0. Supports word \
             highlighting, pitch, volume, audio files, and tones. Only when chosen by name."
        );
        let json = serde_json::to_value(&r).unwrap();
        assert!(json["backends"][0]["caps"].is_string());
        let r = report(&BackendRegistry::with_builtins(), Some("recording"));
        assert_eq!(r.selected, "recording");
        let rec = r.backends.iter().find(|b| b.id == "recording").unwrap();
        assert!(describe(rec, r.auto, r.selected).ends_with(" Chosen by your settings."));
    }

    #[test]
    fn discovery_runs_once_per_backend() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        let probes = Arc::new(AtomicUsize::new(0));
        let mut registry = BackendRegistry::with_builtins();
        let counter = Arc::clone(&probes);
        registry.register(
            BackendInfo {
                id: "counted",
                name: "Counted",
                priority: 7,
                opt_in: false,
                available: false,
                caps: Caps::empty(),
            },
            move || {
                counter.fetch_add(1, Ordering::SeqCst);
                true
            },
            || Ok(Box::new(textweaver_app::speech::NullBackend::default()) as _),
        );
        let r = report(&registry, Some("counted"));
        assert_eq!(probes.load(Ordering::SeqCst), 1);
        assert_eq!(r.selected, "counted");
    }

    #[test]
    fn feature_lists() {
        assert_eq!(and_list(&features(Caps::empty())), "");
        assert_eq!(and_list(&features(Caps::TONES)), "tones");
        assert_eq!(
            and_list(&features(Caps::PAUSE | Caps::TONES)),
            "pause and tones"
        );
    }
}
