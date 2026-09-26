//! `tw export-audio`. Owner: Agent B2.
//!
//! Reads a document aloud into an audio file (`.wav`, or `.mp3` and `.m4b`
//! through ffmpeg) with optional subtitles (`.srt` or `.vtt`), through any
//! backend that can write audio files (ADR-0011). Without `--backend`, the
//! highest-priority available backend that can write files is used, so an
//! engine that can only play (Omnivox, speech-dispatcher) is passed over.
//! Progress goes to stderr in whole tens of percent, one short sentence
//! each, so a screen reader is not flooded.

use std::ops::ControlFlow;
use std::path::PathBuf;

use anyhow::{Context, bail};
use serde::Serialize;
use textweaver_app::core::{Pitch, Rate};
use textweaver_app::formats;
use textweaver_app::speech::{BackendRegistry, Caps, Selection, VoiceParams, resolve_voice};
use textweaver_export::{
    AudioFormat, CueOptions, ExportOptions, ExportReport, SubtitleRequest, export, ffmpeg,
};

/// Arguments for `tw export-audio`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Document to read aloud.
    pub file: PathBuf,
    /// Output file (.wav, .mp3, .m4b).
    #[arg(long)]
    pub out: PathBuf,
    /// Also write subtitles (.srt or .vtt).
    #[arg(long)]
    pub subtitles: Option<PathBuf>,
    /// One subtitle cue per word instead of caption lines.
    #[arg(long)]
    pub word_level: bool,
    /// Backend id (default: the best available one that can write files).
    #[arg(long)]
    pub backend: Option<String>,
    /// Voice id or name.
    #[arg(long)]
    pub voice: Option<String>,
    /// Rate in words per minute.
    #[arg(long)]
    pub rate: Option<u16>,
    /// Pitch offset in semitones.
    #[arg(long, allow_hyphen_values = true)]
    pub pitch: Option<i8>,
    /// Print the report (sentences, words, chapters, times) as JSON.
    #[arg(long)]
    pub json: bool,
    /// No progress messages.
    #[arg(long)]
    pub quiet: bool,
}

/// What `tw export-audio --json` prints.
#[derive(Debug, Serialize)]
pub struct Report {
    /// The backend used and whether the request fell back.
    pub backend: Selection,
    /// What was written, with the timeline.
    pub export: ExportReport,
}

/// Picks the backend: the one asked for (falling back to automatic
/// selection when unavailable), else the highest-priority available
/// backend that can write audio files.
pub fn choose(registry: &BackendRegistry, asked: Option<&str>) -> anyhow::Result<Selection> {
    if asked.is_some() {
        return Ok(registry.select(asked));
    }
    registry
        .list()
        .into_iter()
        .find(|b| b.available && !b.opt_in && b.caps.contains(Caps::SYNTH_TO_FILE))
        .map(|backend| Selection {
            backend,
            requested: None,
            fell_back: false,
        })
        .context(
            "no installed voice can write audio files; install espeak-ng, or choose one with --backend",
        )
}

/// "1 hour, 2 minutes, 5 seconds" for a length in ms.
pub fn spoken_duration(ms: u64) -> String {
    let s = (ms + 500) / 1000;
    let (h, m, s) = (s / 3600, s / 60 % 60, s % 60);
    let unit = |n: u64, one: &str| format!("{n} {one}{}", if n == 1 { "" } else { "s" });
    let mut parts = Vec::new();
    if h > 0 {
        parts.push(unit(h, "hour"));
    }
    if m > 0 {
        parts.push(unit(m, "minute"));
    }
    if s > 0 || parts.is_empty() {
        parts.push(unit(s, "second"));
    }
    parts.join(", ")
}

/// Exports and returns what happened.
pub fn export_audio(
    args: &Args,
    registry: &BackendRegistry,
    ffmpeg_path: Option<PathBuf>,
    progress: &mut dyn FnMut(&str),
) -> anyhow::Result<Report> {
    let format = AudioFormat::from_path(&args.out).with_context(|| {
        format!(
            "cannot write {}: use a .wav, .mp3, or .m4b file name",
            args.out.display()
        )
    })?;
    if format != AudioFormat::Wav && ffmpeg_path.is_none() {
        bail!(
            "writing {} needs ffmpeg, which was not found; install ffmpeg, set TEXTWEAVER_FFMPEG to its path, or export to .wav",
            format.name()
        );
    }
    let doc = formats::load_path(&args.file)
        .with_context(|| format!("cannot open {}", args.file.display()))?;
    let selection = choose(registry, args.backend.as_deref())?;
    let factory = registry
        .factory(selection.backend.id)
        .with_context(|| format!("backend {} is not built in", selection.backend.id))?;
    let mut backend = factory()?;
    let mut params = VoiceParams::default();
    if let Some(asked) = &args.voice {
        let voices = backend.voices().unwrap_or_default();
        params.voice = Some(resolve_voice(&voices, asked).unwrap_or_else(|| asked.clone()));
    }
    if let Some(r) = args.rate {
        params.rate = Rate::Wpm(r).clamped();
    }
    if let Some(p) = args.pitch {
        params.pitch = Pitch::Semitones(p).clamped();
    }
    backend.set_params(&params)?;
    if !backend.capabilities().contains(Caps::SYNTH_TO_FILE) {
        bail!(
            "{} cannot write audio files; choose another voice with --backend",
            selection.backend.name
        );
    }
    let subtitles = args.subtitles.as_ref().map(|path| SubtitleRequest {
        path: path.clone(),
        cues: CueOptions {
            word_level: args.word_level,
            ..CueOptions::default()
        },
    });
    let mut last_tenth = 0;
    let report = export(
        &doc,
        backend.as_mut(),
        &args.out,
        subtitles.as_ref(),
        ffmpeg_path.as_deref(),
        &ExportOptions::default(),
        &mut |p| {
            let tenth = p.percent() / 10;
            if p.total > 0 && tenth > last_tenth && p.done < p.total {
                last_tenth = tenth;
                progress(&format!("{} percent done.", tenth * 10));
            }
            ControlFlow::Continue(())
        },
    )?;
    Ok(Report {
        backend: selection,
        export: report,
    })
}

/// One or two sentences saying what was written.
pub fn summary(r: &Report) -> String {
    let t = &r.export.timeline;
    let n = t.sentences.len();
    let chapters = t.chapters.len();
    let mut s = format!(
        "Wrote {}: {}, {n} sentence{}, {chapters} chapter{}, with {}.",
        r.export.out.display(),
        spoken_duration(t.duration_ms),
        if n == 1 { "" } else { "s" },
        if chapters == 1 { "" } else { "s" },
        r.backend.backend.name
    );
    if let Some(sub) = &r.export.subtitles {
        s.push_str(&format!(" Subtitles in {}.", sub.display()));
    }
    s
}

/// Runs `tw export-audio`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let quiet = args.quiet || args.json;
    let report = export_audio(
        &args,
        &textweaver_app::speech_registry(),
        ffmpeg::find(),
        &mut |msg| {
            if !quiet {
                eprintln!("{msg}");
            }
        },
    )?;
    if let Some(msg) = report.backend.fallback_message() {
        eprintln!("{msg}");
    }
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", summary(&report));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch folder removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let p =
                std::env::temp_dir().join(format!("tw-export-audio-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(&p).unwrap();
            Scratch(p)
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn args(dir: &std::path::Path, out: &str) -> Args {
        let file = dir.join("doc.md");
        std::fs::write(&file, "# Intro\n\nHello world. Paid $5 today.\n").unwrap();
        Args {
            file,
            out: dir.join(out),
            subtitles: Some(dir.join("doc.srt")),
            word_level: false,
            backend: Some("recording".into()),
            voice: Some("Recording US".into()),
            rate: Some(200),
            pitch: None,
            json: true,
            quiet: true,
        }
    }

    #[test]
    fn exports_wav_and_subtitles_with_the_recording_backend() {
        let dir = Scratch::new("wav");
        let a = args(dir.path(), "doc.wav");
        let mut messages = Vec::new();
        let r = export_audio(&a, &BackendRegistry::with_builtins(), None, &mut |m| {
            messages.push(m.to_owned())
        })
        .unwrap();
        assert_eq!(r.backend.backend.id, "recording");
        assert_eq!(r.export.timeline.duration_ms, 2500);
        let srt = std::fs::read_to_string(dir.path().join("doc.srt")).unwrap();
        assert!(srt.contains("Paid $5 today."), "{srt}");
        assert!(
            messages.iter().any(|m| m == "30 percent done."),
            "{messages:?}"
        );
        assert_eq!(
            summary(&r),
            format!(
                "Wrote {}: 3 seconds, 3 sentences, 1 chapter, with Recording (test double). Subtitles in {}.",
                a.out.display(),
                dir.path().join("doc.srt").display()
            )
        );
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["export"]["format"], "wav");
        assert_eq!(json["export"]["timeline"]["chapters"][0]["title"], "Intro");
    }

    #[test]
    fn explains_what_cannot_be_done() {
        let dir = Scratch::new("errors");
        let reg = BackendRegistry::with_builtins();
        let mut a = args(dir.path(), "doc.mp3");
        let e = export_audio(&a, &reg, None, &mut |_| {}).unwrap_err();
        assert!(e.to_string().contains("needs ffmpeg"), "{e}");
        a.out = dir.path().join("doc.ogg");
        let e = export_audio(&a, &reg, None, &mut |_| {}).unwrap_err();
        assert!(e.to_string().contains("use a .wav, .mp3, or .m4b"), "{e}");
        a.out = dir.path().join("doc.wav");
        a.backend = Some("null".into());
        let e = export_audio(&a, &reg, None, &mut |_| {}).unwrap_err();
        assert!(e.to_string().contains("cannot write audio files"), "{e}");
    }

    #[test]
    fn automatic_choice_needs_a_backend_that_writes_files() {
        // The built-ins here: null (cannot write) and recording (opt-in).
        let e = choose(&BackendRegistry::with_builtins(), None);
        if cfg!(feature = "espeak") {
            return;
        }
        assert!(e.is_err());
        assert_eq!(
            choose(&BackendRegistry::with_builtins(), Some("recording"))
                .unwrap()
                .backend
                .id,
            "recording"
        );
    }

    #[test]
    fn durations_read_well() {
        assert_eq!(spoken_duration(0), "0 seconds");
        assert_eq!(spoken_duration(1400), "1 second");
        assert_eq!(spoken_duration(61_000), "1 minute, 1 second");
        assert_eq!(spoken_duration(7_200_000), "2 hours");
        assert_eq!(spoken_duration(3_725_000), "1 hour, 2 minutes, 5 seconds");
    }
}
