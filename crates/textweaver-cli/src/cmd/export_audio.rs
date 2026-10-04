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
use textweaver_app::lexicon::i18n::{Arg, Catalog};
use textweaver_app::speech::{BackendRegistry, Caps, Selection, VoiceParams, resolve_voice};
use textweaver_app::store::{Paths, Settings, SettingsStore, SubtitleKaraoke};
use textweaver_export::{
    AudioFormat, CaptionMeta, ChapterNames, CueOptions, ExportOptions, ExportReport, Karaoke,
    SubtitleRequest, export, ffmpeg, readalong,
};

/// Arguments for `tw export-audio`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Document to read aloud.
    pub file: PathBuf,
    /// Output file (.wav, .flac, .mp3, .opus, .m4b), or .html for a
    /// read-along page: the text with the MP3 inside, the spoken word
    /// marked as it plays.
    #[arg(long = "out", short = 'o', alias = "output")]
    pub out: PathBuf,
    /// Also write subtitles (.srt, .vtt, or .ass karaoke).
    #[arg(long)]
    pub subtitles: Option<PathBuf>,
    /// One subtitle cue per word instead of caption lines.
    #[arg(long)]
    pub word_level: bool,
    /// Karaoke in subtitle lines: off, tags (WebVTT, spoken words
    /// underlined), or lines (a cue per word in bold and underline).
    /// Default: `[export] subtitle_karaoke`.
    #[arg(long, value_enum)]
    pub karaoke: Option<KaraokeArg>,
    /// Also write a WebVTT chapters file here. With `[export]
    /// subtitle_chapters`, one is written beside the subtitles or the audio.
    #[arg(long, value_name = "FILE")]
    pub chapters: Option<PathBuf>,
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
    /// Read settings from this directory (like `TEXTWEAVER_HOME`).
    #[arg(long)]
    pub home: Option<PathBuf>,
}

/// `--karaoke`: how subtitle lines show the word being read.
#[derive(clap::ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KaraokeArg {
    /// Plain caption lines.
    #[default]
    Off,
    /// WebVTT timestamp tags, spoken words underlined.
    Tags,
    /// A cue per word: the line with that word in bold and underline.
    Lines,
}

impl From<KaraokeArg> for Karaoke {
    fn from(k: KaraokeArg) -> Self {
        match k {
            KaraokeArg::Off => Karaoke::Off,
            KaraokeArg::Tags => Karaoke::Tags,
            KaraokeArg::Lines => Karaoke::Lines,
        }
    }
}

/// `[export] subtitle_karaoke` as the exporter's style.
pub fn karaoke_setting(k: SubtitleKaraoke) -> Karaoke {
    match k {
        SubtitleKaraoke::Off => Karaoke::Off,
        SubtitleKaraoke::Tags => Karaoke::Tags,
        SubtitleKaraoke::Lines => Karaoke::Lines,
    }
}

/// Names for untitled chapters in the interface language.
pub fn chapter_names(language: &str) -> ChapterNames {
    let (c, _) = Catalog::for_language(language, None);
    ChapterNames {
        untitled_document: c.tr("export-chapter-untitled"),
        numbered: c.fmt(
            "export-chapter-numbered",
            &[("number", Arg::Str("{number}".into()))],
        ),
    }
}

/// What `tw export-audio --json` prints.
#[derive(Debug, Serialize)]
pub struct Report {
    /// The backend used and whether the request fell back.
    pub backend: Selection,
    /// What was written, with the timeline.
    pub export: ExportReport,
    /// The WebVTT chapters file, if one was written.
    pub chapters: Option<PathBuf>,
}

/// Picks the backend: the one asked for (falling back to automatic
/// selection when unavailable), else `configured` (`[speech] backend`)
/// when it is available and can write audio files, else the
/// highest-priority available backend that can write audio files.
pub fn choose(
    registry: &BackendRegistry,
    asked: Option<&str>,
    configured: Option<&str>,
) -> anyhow::Result<Selection> {
    if asked.is_some() {
        return Ok(registry.select(asked));
    }
    if let Some(id) = configured.filter(|id| !id.is_empty() && *id != "auto")
        && let Some(backend) = registry
            .list()
            .into_iter()
            .find(|b| b.id == id && b.available && b.caps.contains(Caps::SYNTH_TO_FILE))
    {
        return Ok(Selection {
            backend,
            requested: None,
            fell_back: false,
        });
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

/// Exports and returns what happened, with `settings`: the voice, rate,
/// pitch, and volume of `[speech]` (the flags override them), the preferred
/// backend, normalization and table narration, where footnotes go, and the
/// subtitles of `[export]`.
pub fn export_audio(
    args: &Args,
    settings: &Settings,
    registry: &BackendRegistry,
    ffmpeg_path: Option<PathBuf>,
    progress: &mut dyn FnMut(&str),
) -> anyhow::Result<Report> {
    // An .html file is the read-along page, with the MP3 inside.
    let page = readalong::is_page(&args.out);
    let format = if page {
        AudioFormat::Mp3
    } else {
        AudioFormat::from_path(&args.out).with_context(|| {
            format!(
                "cannot write {}: use a .wav, .flac, .mp3, .opus, or .m4b file name, or .html for a read-along page",
                args.out.display()
            )
        })?
    };
    if format.needs_ffmpeg() && ffmpeg_path.is_none() {
        bail!(
            "writing {} needs ffmpeg, which was not found; install ffmpeg, set TEXTWEAVER_FFMPEG to its path, or export to .flac, .mp3, .opus, or .wav",
            format.name()
        );
    }
    let doc = formats::Registry::with_builtins()
        .load(
            &formats::Source::Path(args.file.clone()),
            &textweaver_app::load_options(settings),
        )
        .with_context(|| format!("cannot open {}", args.file.display()))?;
    let selection = choose(
        registry,
        args.backend.as_deref(),
        Some(settings.speech.backend.as_str()),
    )?;
    let factory = registry
        .factory(selection.backend.id)
        .with_context(|| format!("backend {} is not built in", selection.backend.id))?;
    let mut backend = factory()?;
    let config = textweaver_engines::service_config(settings);
    let mut params = VoiceParams {
        // A configured voice belongs to the configured backend.
        voice: None,
        ..config.params.clone()
    };
    let same_backend = settings.speech.backend == selection.backend.id;
    let voice = args.voice.clone().or_else(|| {
        same_backend
            .then(|| settings.speech.voice.clone())
            .flatten()
    });
    if let Some(asked) = &voice {
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
    let plan = textweaver_app::subtitle_plan(
        settings,
        &args.out,
        args.subtitles.as_deref(),
        args.word_level,
    );
    let karaoke = args.karaoke.map_or_else(
        || karaoke_setting(settings.export.subtitle_karaoke),
        Karaoke::from,
    );
    let chapters_to = args.chapters.clone().or_else(|| {
        settings.export.subtitle_chapters.then(|| {
            textweaver_export::chapters::vtt_path(plan.path.as_deref().unwrap_or(&args.out))
        })
    });
    let subtitles = plan.path.map(|path| SubtitleRequest {
        path,
        cues: CueOptions {
            word_level: plan.word_level,
            karaoke,
            ..CueOptions::default()
        },
    });
    let voice_name = match &params.voice {
        Some(v) => format!("{v} ({})", selection.backend.name),
        None => selection.backend.name.to_string(),
    };
    let options = ExportOptions {
        narration: textweaver_app::narration_policy(settings),
        normalize: config.normalize,
        punctuation: config.punctuation,
        split_caps: config.split_caps,
        chapter_names: chapter_names(&settings.interface.language),
        captions: CaptionMeta {
            voice: Some(voice_name),
            words_per_minute: Some(u32::from(params.rate.wpm())),
            ..CaptionMeta::default()
        },
        ..ExportOptions::default()
    };
    let mut last_tenth = 0;
    let mut on_progress = |p: textweaver_export::Progress| {
        let tenth = p.percent() / 10;
        if p.total > 0 && tenth > last_tenth && p.done < p.total {
            last_tenth = tenth;
            progress(&format!("{} percent done.", tenth * 10));
        }
        ControlFlow::Continue(())
    };
    let report = if page {
        let themes = args
            .home
            .as_deref()
            .map(Paths::under)
            .or_else(|| Paths::platform().ok())
            .map(|p| p.themes_dir());
        let theme_css =
            textweaver_app::page_theme_css(None, &settings.display.theme, themes.as_deref())
                .map_err(anyhow::Error::msg)?;
        let (catalog, _) = Catalog::for_language(&settings.interface.language, None);
        let page_opts = readalong::PageOptions {
            theme_css,
            lang: settings.interface.language.clone(),
            fallback_title: args
                .file
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned()),
            labels: textweaver_app::read_along_labels(&catalog),
        };
        readalong::export_page(
            &doc,
            backend.as_mut(),
            &args.out,
            subtitles.as_ref(),
            ffmpeg_path.as_deref(),
            &options,
            &page_opts,
            &mut on_progress,
        )?
    } else {
        export(
            &doc,
            backend.as_mut(),
            &args.out,
            subtitles.as_ref(),
            ffmpeg_path.as_deref(),
            &options,
            &mut on_progress,
        )?
    };
    let chapters = match chapters_to {
        Some(path) => {
            std::fs::write(
                &path,
                textweaver_export::chapters::vtt(&report.timeline.chapters),
            )
            .with_context(|| format!("cannot write {}", path.display()))?;
            Some(path)
        }
        None => None,
    };
    Ok(Report {
        backend: selection,
        export: report,
        chapters,
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
    if let Some(ch) = &r.chapters {
        s.push_str(&format!(" Chapters in {}.", ch.display()));
    }
    s
}

/// The settings `tw export-audio` uses: from `--home`, else the platform
/// folders; defaults when there are none. A message when the file was
/// damaged (it is backed up and defaults are used).
pub fn load_settings(home: Option<&std::path::Path>) -> (Settings, Option<String>) {
    let paths = match home {
        Some(h) => Some(Paths::under(h)),
        None => Paths::platform().ok(),
    };
    match paths {
        Some(p) => SettingsStore::new(p).load(),
        None => (Settings::default(), None),
    }
}

/// Runs `tw export-audio`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let quiet = args.quiet || args.json;
    let (settings, message) = load_settings(args.home.as_deref());
    if let Some(m) = message {
        eprintln!("{m}");
    }
    let report = export_audio(
        &args,
        &settings,
        &textweaver_engines::speech_registry_for(&settings),
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
            karaoke: None,
            chapters: None,
            backend: Some("recording".into()),
            voice: Some("Recording US".into()),
            rate: Some(200),
            pitch: None,
            json: true,
            quiet: true,
            home: None,
        }
    }

    #[test]
    fn an_html_out_writes_the_read_along_page() {
        let dir = Scratch::new("readalong");
        let mut a = args(dir.path(), "doc.html");
        a.subtitles = None;
        let r = export_audio(
            &a,
            &Settings::default(),
            &BackendRegistry::test_doubles(),
            None,
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(r.export.out, dir.path().join("doc.html"));
        let html = std::fs::read_to_string(dir.path().join("doc.html")).unwrap();
        assert!(html.contains("data:audio/mpeg;base64,"));
        assert!(html.contains("id=\"s0\"") && html.contains("id=\"w0\""));
        assert!(html.contains(">Back a sentence</button>"));
        assert!(html.contains("Hello"));
        assert!(!dir.path().join("doc.mp3").exists());
    }

    #[test]
    fn karaoke_tags_a_note_and_a_chapters_file() {
        let dir = Scratch::new("karaoke");
        let mut a = args(dir.path(), "doc.wav");
        a.subtitles = Some(dir.path().join("doc.vtt"));
        a.karaoke = Some(KaraokeArg::Tags);
        let mut settings = Settings::default();
        settings.export.subtitle_chapters = true;
        let r = export_audio(
            &a,
            &settings,
            &BackendRegistry::test_doubles(),
            None,
            &mut |_| {},
        )
        .unwrap();
        let vtt = std::fs::read_to_string(dir.path().join("doc.vtt")).unwrap();
        assert!(vtt.contains("::cue(:past)"), "{vtt}");
        assert!(vtt.contains("\nNOTE\n"), "{vtt}");
        assert!(
            vtt.replace('\n', " ")
                .contains("(Recording (test double)), 200 words a minute. Made by textweaver."),
            "{vtt}"
        );
        assert!(vtt.contains("Hello <00:00:"), "{vtt}");
        let chapters = dir.path().join("doc.chapters.vtt");
        assert_eq!(r.chapters.as_deref(), Some(chapters.as_path()));
        let ch = std::fs::read_to_string(&chapters).unwrap();
        assert!(ch.starts_with("WEBVTT\n\n00:00:00.000 --> "), "{ch}");
        assert!(ch.ends_with("\nIntro\n"), "{ch}");
        assert!(summary(&r).ends_with(&format!(" Chapters in {}.", chapters.display())));
    }

    #[test]
    fn chapter_names_follow_the_interface_language() {
        let de = chapter_names("de");
        assert_eq!(de.untitled_document, "Hörbuch");
        assert_eq!(de.number(2), "Kapitel 2");
        assert_eq!(chapter_names("en").number(4), "Chapter 4");
        assert_eq!(chapter_names("es").untitled_document, "Audiolibro");
    }

    #[test]
    fn exports_wav_and_subtitles_with_the_recording_backend() {
        let dir = Scratch::new("wav");
        let a = args(dir.path(), "doc.wav");
        let mut messages = Vec::new();
        let r = export_audio(
            &a,
            &Settings::default(),
            &BackendRegistry::test_doubles(),
            None,
            &mut |m| messages.push(m.to_owned()),
        )
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
        let reg = BackendRegistry::test_doubles();
        let mut a = args(dir.path(), "doc.m4b");
        let e = export_audio(&a, &Settings::default(), &reg, None, &mut |_| {}).unwrap_err();
        assert!(e.to_string().contains("needs ffmpeg"), "{e}");
        a.out = dir.path().join("doc.ogg");
        let e = export_audio(&a, &Settings::default(), &reg, None, &mut |_| {}).unwrap_err();
        assert!(
            e.to_string()
                .contains("use a .wav, .flac, .mp3, .opus, or .m4b"),
            "{e}"
        );
        a.out = dir.path().join("doc.wav");
        a.backend = Some("null".into());
        let e = export_audio(&a, &Settings::default(), &reg, None, &mut |_| {}).unwrap_err();
        assert!(e.to_string().contains("cannot write audio files"), "{e}");
    }

    #[test]
    fn automatic_choice_needs_a_backend_that_writes_files() {
        // The doubles: null (cannot write) and recording (opt-in).
        let e = choose(&BackendRegistry::test_doubles(), None, None);
        assert!(e.is_err());
        assert_eq!(
            choose(&BackendRegistry::test_doubles(), Some("recording"), None)
                .unwrap()
                .backend
                .id,
            "recording"
        );
    }

    #[test]
    fn export_settings_and_speech_settings_are_used() {
        use textweaver_app::store::{FootnoteMode, SubtitleFormat, TableMode};
        let dir = Scratch::new("settings");
        let file = dir.path().join("doc.md");
        std::fs::write(
            &file,
            "Intro with a note[^1].\n\n| Name | Role |\n|---|---|\n| Ada | Engineer |\n\n[^1]: Secret footnote text.\n",
        )
        .unwrap();
        let mut settings = Settings::default();
        settings.export.subtitles_with_audio = true;
        settings.export.subtitle_format = SubtitleFormat::Vtt;
        settings.export.subtitle_word_level = true;
        settings.speech.backend = "recording".into();
        settings.normalization.table_mode = TableMode::Skip;
        settings.normalization.footnote_mode = FootnoteMode::Skip;
        let a = Args {
            file,
            out: dir.path().join("book.wav"),
            subtitles: None,
            word_level: false,
            karaoke: None,
            chapters: None,
            backend: None,
            voice: None,
            rate: None,
            pitch: None,
            json: true,
            quiet: true,
            home: None,
        };
        let r = export_audio(
            &a,
            &settings,
            &BackendRegistry::test_doubles(),
            None,
            &mut |_| {},
        )
        .unwrap();
        // [speech] backend chose the (opt-in) recording backend.
        assert_eq!(r.backend.backend.id, "recording");
        // [export]: WebVTT beside the audio, one cue per word.
        let vtt_path = dir.path().join("book.vtt");
        assert_eq!(r.export.subtitles.as_deref(), Some(vtt_path.as_path()));
        let vtt = std::fs::read_to_string(&vtt_path).unwrap();
        assert!(vtt.starts_with("WEBVTT"), "{vtt}");
        assert!(vtt.lines().any(|l| l.trim() == "Intro"), "{vtt}");
        // table_mode and footnote_mode shaped what was read (the "table
        // skipped" notice has no document text, so it has no cue).
        assert!(!vtt.contains("Ada") && !vtt.contains("Engineer"), "{vtt}");
        assert!(!vtt.contains("Secret"), "{vtt}");
    }

    #[test]
    fn a_configured_backend_that_cannot_write_files_is_passed_over() {
        let reg = BackendRegistry::test_doubles();
        let chosen = choose(&reg, None, Some("null"));
        // null cannot write files: the automatic choice applies instead,
        // and among the doubles nothing can be chosen automatically.
        assert!(chosen.is_err());
        assert_eq!(
            choose(&reg, None, Some("recording")).unwrap().backend.id,
            "recording"
        );
    }

    /// The sample document to FLAC with no ffmpeg at all: the file is FLAC,
    /// and the report counts its chapters.
    #[test]
    fn flac_needs_no_ffmpeg() {
        let dir = Scratch::new("flac");
        let sample =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/sample.md");
        let a = Args {
            file: sample,
            out: dir.path().join("sample.flac"),
            subtitles: None,
            ..args(dir.path(), "unused.wav")
        };
        let r = export_audio(
            &a,
            &Settings::default(),
            &BackendRegistry::test_doubles(),
            None,
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(r.export.format, AudioFormat::Flac);
        assert!(r.export.ffmpeg.is_none());
        assert!(r.export.timeline.duration_ms > 0);
        assert!(!r.export.timeline.chapters.is_empty());
        let bytes = std::fs::read(&a.out).unwrap();
        assert_eq!(&bytes[..4], b"fLaC");
        assert!(summary(&r).starts_with("Wrote "), "{}", summary(&r));
    }

    /// The sample document to Ogg Opus with no ffmpeg: an Ogg stream whose
    /// first packet is the Opus header.
    #[cfg(feature = "opus")]
    #[test]
    fn opus_needs_no_ffmpeg() {
        let dir = Scratch::new("opus");
        let sample =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/sample.md");
        let a = Args {
            file: sample,
            out: dir.path().join("sample.opus"),
            subtitles: None,
            ..args(dir.path(), "unused.wav")
        };
        let r = export_audio(
            &a,
            &Settings::default(),
            &BackendRegistry::test_doubles(),
            None,
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(r.export.format, AudioFormat::Opus);
        assert!(r.export.ffmpeg.is_none());
        assert!(!r.export.timeline.chapters.is_empty());
        let bytes = std::fs::read(&a.out).unwrap();
        assert_eq!(&bytes[..4], b"OggS");
        assert_eq!(&bytes[28..36], b"OpusHead");
        assert!(summary(&r).starts_with("Wrote "), "{}", summary(&r));
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
