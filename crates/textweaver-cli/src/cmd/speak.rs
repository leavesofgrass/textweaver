//! `tw speak`. Owner: Agent B.
//!
//! Plans sentence-sized utterances for the text (or file), picks a backend
//! (an unavailable `--backend` falls back to automatic selection, with a
//! note on stderr), and either speaks through the speech service or, with
//! `--out`, writes audio to a file. `--json` prints the normalized
//! utterances with their offset maps and every status the service reported.
//!
//! The user's settings apply, as in the reader and `tw export-audio`: the
//! backend, voice, rate, pitch, and volume of `[speech]` (the flags override
//! them), normalization, punctuation, table narration, and where footnotes
//! go. A configured voice is used only with the configured backend.
//!
//! `tw speak -` (or `--file -`) reads the text from standard input, so
//! `Get-Clipboard | tw speak -` or `pbpaste | tw speak -` reads the
//! clipboard aloud.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, bail};
use serde::Serialize;
use textweaver_app::core::{CharRange, Pitch, Rate, Utterance};
use textweaver_app::formats;
use textweaver_app::speech::{
    BackendRegistry, Caps, Pipeline, Selection, ServiceConfig, SpeechService, SpeechStatus,
    resolve_voice,
};
use textweaver_app::store::Settings;
use textweaver_app::text::{Document, plan};

/// Arguments for `tw speak`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Text to speak (omit with --file); - reads standard input.
    pub text: Option<String>,
    /// Speak this file instead; - reads standard input.
    #[arg(long)]
    pub file: Option<PathBuf>,
    /// Backend id (default: the settings' backend, else auto).
    #[arg(long)]
    pub backend: Option<String>,
    /// Voice id or name (default: the settings' voice, with the settings'
    /// backend).
    #[arg(long)]
    pub voice: Option<String>,
    /// Rate in words per minute.
    #[arg(long)]
    pub rate: Option<u16>,
    /// Pitch offset in semitones.
    #[arg(long, allow_hyphen_values = true)]
    pub pitch: Option<i8>,
    /// Write audio to this file instead of playing it.
    #[arg(long = "out", short = 'o', alias = "output")]
    pub out: Option<PathBuf>,
    /// Print the utterances, offset maps, and status events as JSON.
    #[arg(long)]
    pub json: bool,
    /// Read settings from this directory (like `TEXTWEAVER_HOME`).
    #[arg(long)]
    pub home: Option<PathBuf>,
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

fn load(args: &Args, settings: &Settings) -> anyhow::Result<Document> {
    let stdin = || -> anyhow::Result<Document> {
        let mut text = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin().lock(), &mut text)
            .context("could not read standard input")?;
        if text.trim().is_empty() {
            return Err(super::Plain(
                "Nothing to speak: standard input was empty. Pipe text in, as in echo hello | tw speak -.".into(),
            )
            .into());
        }
        Ok(Document::from_plain_text(&text))
    };
    match (&args.text, &args.file) {
        (Some(t), None) if t == "-" => stdin(),
        (None, Some(f)) if f.as_os_str() == "-" => stdin(),
        (Some(t), None) => Ok(Document::from_plain_text(t)),
        (None, Some(f)) => formats::Registry::with_builtins()
            .load(
                &formats::Source::Path(f.clone()),
                &textweaver_app::load_options(settings),
            )
            .with_context(|| format!("cannot open {}", f.display())),
        (Some(_), Some(_)) => Err(super::Plain(
            "Two things to speak: TEXT and --file were both given. Give one of them.".into(),
        )
        .into()),
        (None, None) => Err(super::Plain(
            "Nothing to speak: no TEXT and no --file. Give text, a file with --file, or - to read standard input.".into(),
        )
        .into()),
    }
}

/// The service configuration: the settings', with the flags on top. The
/// settings' voice belongs to the settings' backend, so it applies only
/// when that backend was chosen.
fn config(args: &Args, settings: &Settings, backend: &str) -> ServiceConfig {
    let mut config = textweaver_engines::service_config(settings);
    let same_backend = settings.speech.backend == backend;
    config.params.voice = args.voice.clone().or_else(|| {
        same_backend
            .then(|| settings.speech.voice.clone())
            .flatten()
    });
    if !same_backend {
        config.prefer_voice = None;
    }
    if let Some(r) = args.rate {
        config.params.rate = Rate::Wpm(r).clamped();
    }
    if let Some(p) = args.pitch {
        config.params.pitch = Pitch::Semitones(p).clamped();
    }
    config
}

/// Speaks (or writes) with `settings` and returns what happened.
pub fn speak(
    args: &Args,
    settings: &Settings,
    registry: &BackendRegistry,
) -> anyhow::Result<Report> {
    let doc = load(args, settings)?;
    let planned = plan(
        &doc,
        CharRange::new(0, doc.len_chars()),
        &textweaver_app::narration_policy(settings),
    );
    let asked = args
        .backend
        .as_deref()
        .or(Some(settings.speech.backend.as_str()));
    let selection = registry.select(asked);
    let factory = registry
        .factory(selection.backend.id)
        .with_context(|| format!("backend {} is not built in", selection.backend.id))?;
    let config = config(args, settings, selection.backend.id);
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
        let mut params = config.params.clone();
        if let Some(asked) = &params.voice {
            // Plain names ("Zira", "Reed") resolve to the backend's voice id.
            let voices = backend.voices().unwrap_or_default();
            if let Some(id) = resolve_voice(&voices, asked) {
                params.voice = Some(id);
            }
        }
        backend.set_params(&params)?;
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
    // Pauses after headings, paragraphs and list items, as the reader has.
    let pauses = textweaver_app::structural_pauses(&doc, &planned);
    let reading = service.read_with_pauses(planned, pauses);
    let mut statuses = Vec::new();
    loop {
        let status = service
            .statuses()
            .recv_timeout(STATUS_TIMEOUT)
            .context("the speech service stopped answering")?;
        let done = matches!(
            status,
            SpeechStatus::Finished { generation } | SpeechStatus::Stopped { generation }
                if generation == reading
        );
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
    let (settings, message) = super::export_audio::load_settings(args.home.as_deref());
    if let Some(m) = message {
        eprintln!("{m}");
    }
    let registry = textweaver_engines::speech_registry_for(&settings);
    let report = speak(&args, &settings, &registry)?;
    if let Some(msg) = report.backend.fallback_message() {
        eprintln!("{msg}");
    }
    if args.json {
        crate::cmd::outln!("{}", serde_json::to_string_pretty(&report)?);
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
            home: None,
        }
    }

    fn speak(args: &Args, registry: &BackendRegistry) -> anyhow::Result<Report> {
        super::speak(args, &Settings::default(), registry)
    }

    #[test]
    fn settings_choose_the_backend_voice_and_rate() {
        let mut settings = Settings::default();
        settings.speech.backend = "recording".into();
        settings.speech.voice = Some("rec-en-us".into());
        let mut a = args("One. Two.", "x");
        a.backend = None;
        let r = super::speak(&a, &settings, &BackendRegistry::test_doubles()).unwrap();
        // The opt-in recording backend is chosen only by the settings.
        assert_eq!(r.backend.backend.id, "recording");
        assert!(!r.backend.fell_back);
        let c = config(&a, &settings, "recording");
        assert_eq!(c.params.voice.as_deref(), Some("rec-en-us"));
        // Another backend does not get the settings' voice; flags win.
        a.rate = Some(300);
        let c = config(&a, &settings, "null");
        assert_eq!(c.params.voice, None);
        assert_eq!(c.params.rate, Rate::Wpm(300).clamped());
    }

    #[test]
    fn null_backend_json_report() {
        let r = speak(
            &args("Dr. Smith paid $5. Done.", "null"),
            &BackendRegistry::test_doubles(),
        )
        .unwrap();
        assert_eq!(r.backend.backend.id, "null");
        let spoken: String = r.utterances.iter().map(|u| u.text.as_str()).collect();
        assert!(spoken.contains("Doctor"), "{spoken}");
        assert!(spoken.contains("five dollars"), "{spoken}");
        for u in &r.utterances {
            u.offset_map.check_invariants(&u.text).unwrap();
        }
        assert_eq!(
            r.statuses.last(),
            Some(&SpeechStatus::Finished { generation: 1 })
        );
        let json = serde_json::to_value(&r).unwrap();
        assert!(json["utterances"][0]["offset_map"]["spans"].is_array());
        assert_eq!(
            json["statuses"].as_array().unwrap().last().unwrap()["finished"]["generation"],
            1
        );
    }

    #[test]
    fn recording_backend_reports_every_word() {
        let r = speak(
            &args("one two three", "recording"),
            &BackendRegistry::test_doubles(),
        )
        .unwrap();
        let words = r
            .statuses
            .iter()
            .filter(|s| matches!(s, SpeechStatus::Position { .. }))
            .count();
        assert_eq!(words, 3);
    }

    /// With only the doubles registered, the fallback is `null`: with
    /// every built-in it would be a real engine, speaking aloud.
    #[test]
    fn unknown_backend_falls_back() {
        let r = speak(
            &args("hi", "no-such-engine"),
            &BackendRegistry::test_doubles(),
        )
        .unwrap();
        assert!(r.backend.fell_back);
        assert_eq!(r.backend.backend.id, "null");
        assert!(r.backend.fallback_message().is_some());
    }

    #[test]
    fn out_writes_audio_or_explains() {
        let path = std::env::temp_dir().join(format!("tw-speak-test-{}.wav", std::process::id()));
        let mut a = args("Hello.", "recording");
        a.out = Some(path.clone());
        let r = speak(&a, &BackendRegistry::test_doubles()).unwrap();
        assert_eq!(r.out.as_deref(), Some(path.as_path()));
        // The recording backend writes one word-length of silence as WAV.
        let wav = std::fs::read(&path).unwrap();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(wav.len(), 44 + 4000 * 2);
        let _ = std::fs::remove_file(&path);
        let mut a = args("Hello.", "null");
        a.out = Some(path);
        let err = speak(&a, &BackendRegistry::test_doubles()).unwrap_err();
        assert!(err.to_string().contains("cannot write audio files"));
    }

    #[test]
    fn needs_something_to_speak() {
        let mut a = args("x", "null");
        a.text = None;
        assert!(speak(&a, &BackendRegistry::test_doubles()).is_err());
    }
}
