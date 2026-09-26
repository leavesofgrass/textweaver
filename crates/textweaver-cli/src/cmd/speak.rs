//! `tw speak`. Owner: Agent B.
//!
//! Plans sentence-sized utterances for the text (or file), picks a backend
//! (an unavailable `--backend` falls back to automatic selection, with a
//! note on stderr), and either speaks through the speech service or, with
//! `--out`, writes audio to a file. `--json` prints the normalized
//! utterances with their offset maps and every status the service reported.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, bail};
use serde::Serialize;
use textweaver_app::core::{CharRange, Pitch, Rate, Utterance};
use textweaver_app::formats;
use textweaver_app::speech::{
    BackendRegistry, Caps, Pipeline, Selection, ServiceConfig, SpeechService, SpeechStatus,
    VoiceParams,
};
use textweaver_app::text::{Document, NarrationPolicy, plan};

/// Arguments for `tw speak`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Text to speak (omit with --file).
    pub text: Option<String>,
    /// Speak this file instead.
    #[arg(long)]
    pub file: Option<PathBuf>,
    /// Backend id (default: auto).
    #[arg(long)]
    pub backend: Option<String>,
    /// Voice id.
    #[arg(long)]
    pub voice: Option<String>,
    /// Rate in words per minute.
    #[arg(long)]
    pub rate: Option<u16>,
    /// Pitch offset in semitones.
    #[arg(long, allow_hyphen_values = true)]
    pub pitch: Option<i8>,
    /// Write audio to this file instead of playing it.
    #[arg(long)]
    pub out: Option<PathBuf>,
    /// Print the utterances, offset maps, and status events as JSON.
    #[arg(long)]
    pub json: bool,
}

/// What `tw speak --json` prints.
#[derive(Debug, Serialize)]
pub struct Report {
    /// The backend used and whether the request fell back.
    pub backend: Selection,
    /// The utterances as spoken: normalized text and offset maps.
    pub utterances: Vec<Utterance>,
    /// Every status the service reported, in order (empty with `--out`).
    pub statuses: Vec<SpeechStatus>,
    /// The audio file written, with `--out`.
    pub out: Option<PathBuf>,
}

/// How long to wait for the next status before giving up.
const STATUS_TIMEOUT: Duration = Duration::from_secs(60);

fn load(args: &Args) -> anyhow::Result<Document> {
    match (&args.text, &args.file) {
        (Some(t), None) => Ok(Document::from_plain_text(t)),
        (None, Some(f)) => {
            formats::load_path(f).with_context(|| format!("cannot open {}", f.display()))
        }
        (Some(_), Some(_)) => bail!("give either TEXT or --file, not both"),
        (None, None) => bail!("nothing to speak: give TEXT or --file"),
    }
}

fn config(args: &Args) -> ServiceConfig {
    let mut params = VoiceParams {
        voice: args.voice.clone(),
        ..VoiceParams::default()
    };
    if let Some(r) = args.rate {
        params.rate = Rate::Wpm(r).clamped();
    }
    if let Some(p) = args.pitch {
        params.pitch = Pitch::Semitones(p).clamped();
    }
    ServiceConfig {
        params,
        ..ServiceConfig::default()
    }
}

/// Speaks (or writes) and returns what happened.
pub fn speak(args: &Args, registry: &BackendRegistry) -> anyhow::Result<Report> {
    let doc = load(args)?;
    let planned = plan(
        &doc,
        CharRange::new(0, doc.len_chars()),
        &NarrationPolicy::default(),
    );
    let selection = registry.select(args.backend.as_deref());
    let factory = registry
        .factory(selection.backend.id)
        .with_context(|| format!("backend {} is not built in", selection.backend.id))?;
    let config = config(args);
    let pipeline = |caps: Caps| {
        Pipeline::for_settings(
            &config.normalize,
            config.punctuation,
            config.split_caps,
            caps.contains(Caps::NATIVE_NORMALIZATION),
        )
    };

    if let Some(out) = &args.out {
        let mut backend = factory()?;
        backend.set_params(&config.params)?;
        let caps = backend.capabilities();
        if !caps.contains(Caps::SYNTH_TO_FILE) {
            bail!(
                "backend {} cannot write audio files; try --backend espeak",
                selection.backend.id
            );
        }
        let p = pipeline(caps);
        let utterances: Vec<Utterance> = planned.into_iter().map(|u| p.apply(u)).collect();
        let text = utterances
            .iter()
            .map(|u| u.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        backend
            .synthesize_to_file(&text, out)
            .with_context(|| format!("cannot write {}", out.display()))?;
        return Ok(Report {
            backend: selection,
            utterances,
            statuses: Vec::new(),
            out: Some(out.clone()),
        });
    }

    let service = SpeechService::spawn(factory, config.clone())?;
    let p = pipeline(service.capabilities());
    let utterances: Vec<Utterance> = planned.iter().cloned().map(|u| p.apply(u)).collect();
    service.read(planned);
    let mut statuses = Vec::new();
    loop {
        let status = service
            .statuses()
            .recv_timeout(STATUS_TIMEOUT)
            .context("the speech service stopped answering")?;
        let done = matches!(status, SpeechStatus::Finished | SpeechStatus::Stopped);
        statuses.push(status);
        if done {
            break;
        }
    }
    service.shutdown();
    // Show the ids the service stamped, so statuses match utterances.
    let generation = statuses.iter().find_map(|s| match s {
        SpeechStatus::Position { utterance, .. } => Some(utterance.generation),
        _ => None,
    });
    let mut utterances = utterances;
    if let Some(generation) = generation {
        for (i, u) in utterances.iter_mut().enumerate() {
            u.id.generation = generation;
            u.id.chunk = u32::try_from(i).unwrap_or(u32::MAX);
        }
    }
    Ok(Report {
        backend: selection,
        utterances,
        statuses,
        out: None,
    })
}

/// Runs `tw speak`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let report = speak(&args, &BackendRegistry::with_builtins())?;
    if let Some(msg) = report.backend.fallback_message() {
        eprintln!("{msg}");
    }
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    let errors: Vec<&String> = report
        .statuses
        .iter()
        .filter_map(|s| match s {
            SpeechStatus::BackendError(e) => Some(e),
            _ => None,
        })
        .collect();
    for e in &errors {
        eprintln!("Speech error: {e}");
    }
    let n = report.utterances.len();
    let sentences = if n == 1 { "sentence" } else { "sentences" };
    match &report.out {
        Some(out) => eprintln!(
            "Wrote {n} {sentences} to {} with {}.",
            out.display(),
            report.backend.backend.name
        ),
        None => eprintln!(
            "Spoke {n} {sentences} with {}.",
            report.backend.backend.name
        ),
    }
    if errors.is_empty() {
        Ok(())
    } else {
        bail!("the speech backend reported errors")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(text: &str, backend: &str) -> Args {
        Args {
            text: Some(text.into()),
            file: None,
            backend: Some(backend.into()),
            voice: None,
            rate: None,
            pitch: None,
            out: None,
            json: true,
        }
    }

    #[test]
    fn null_backend_json_report() {
        let r = speak(
            &args("Dr. Smith paid $5. Done.", "null"),
            &BackendRegistry::with_builtins(),
        )
        .unwrap();
        assert_eq!(r.backend.backend.id, "null");
        let spoken: String = r.utterances.iter().map(|u| u.text.as_str()).collect();
        assert!(spoken.contains("Doctor"), "{spoken}");
        assert!(spoken.contains("five dollars"), "{spoken}");
        for u in &r.utterances {
            u.offset_map.check_invariants(&u.text).unwrap();
        }
        assert_eq!(r.statuses.last(), Some(&SpeechStatus::Finished));
        let json = serde_json::to_value(&r).unwrap();
        assert!(json["utterances"][0]["offset_map"]["spans"].is_array());
        assert_eq!(
            json["statuses"].as_array().unwrap().last().unwrap(),
            "finished"
        );
    }

    #[test]
    fn recording_backend_reports_every_word() {
        let r = speak(
            &args("one two three", "recording"),
            &BackendRegistry::with_builtins(),
        )
        .unwrap();
        let words = r
            .statuses
            .iter()
            .filter(|s| matches!(s, SpeechStatus::Position { .. }))
            .count();
        assert_eq!(words, 3);
    }

    #[test]
    fn unknown_backend_falls_back() {
        let r = speak(
            &args("hi", "no-such-engine"),
            &BackendRegistry::with_builtins(),
        )
        .unwrap();
        assert!(r.backend.fell_back);
        assert!(r.backend.fallback_message().is_some());
    }

    #[test]
    fn out_writes_audio_or_explains() {
        let path = std::env::temp_dir().join(format!("tw-speak-test-{}.wav", std::process::id()));
        let mut a = args("Hello.", "recording");
        a.out = Some(path.clone());
        let r = speak(&a, &BackendRegistry::with_builtins()).unwrap();
        assert_eq!(r.out.as_deref(), Some(path.as_path()));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "Hello.");
        let _ = std::fs::remove_file(&path);
        let mut a = args("Hello.", "null");
        a.out = Some(path);
        let err = speak(&a, &BackendRegistry::with_builtins()).unwrap_err();
        assert!(err.to_string().contains("cannot write audio files"));
    }

    #[test]
    fn needs_something_to_speak() {
        let mut a = args("x", "null");
        a.text = None;
        assert!(speak(&a, &BackendRegistry::with_builtins()).is_err());
    }
}
