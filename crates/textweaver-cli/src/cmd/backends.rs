//! `tw backends`. Owner: Agent B.
//!
//! Lists every speech backend compiled into this build, highest priority
//! first, with availability and which one automatic selection picks. Each
//! backend is one sentence, so the list reads well aloud.

use serde::Serialize;
use textweaver_app::speech::{BackendInfo, BackendRegistry, Caps};

/// Arguments for `tw backends`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
}

/// What `tw backends --json` prints.
#[derive(Debug, Serialize)]
pub struct Report {
    /// The backend automatic selection chooses.
    pub auto: &'static str,
    /// Every backend, highest priority first.
    pub backends: Vec<BackendInfo>,
}

/// Builds the report.
pub fn report(registry: &BackendRegistry) -> Report {
    Report {
        auto: registry.select(None).backend.id,
        backends: registry.list(),
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

/// One sentence describing a backend.
pub fn describe(b: &BackendInfo, auto: &str) -> String {
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
    s
}

/// Runs `tw backends`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let r = report(&textweaver_app::speech_registry());
    if args.json {
        println!("{}", serde_json::to_string_pretty(&r)?);
        return Ok(());
    }
    for b in &r.backends {
        println!("{}", describe(b, r.auto));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_are_listed_with_a_choice() {
        let r = report(&BackendRegistry::with_builtins());
        assert!(r.backends.iter().any(|b| b.id == "null"));
        let null = r.backends.iter().find(|b| b.id == "null").unwrap();
        let line = describe(null, "null");
        assert_eq!(
            line,
            "null: Silent (no audio). Available. Last resort. \
             Supports pause, pitch, and volume. Chosen automatically."
        );
        let rec = r.backends.iter().find(|b| b.id == "recording").unwrap();
        assert_eq!(
            describe(rec, "null"),
            "recording: Recording (test double). Available. Priority 0. Supports word \
             highlighting, pitch, volume, audio files, and tones. Only when chosen by name."
        );
        let json = serde_json::to_value(&r).unwrap();
        assert!(json["backends"][0]["caps"].is_string());
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
