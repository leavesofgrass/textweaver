//! Export through the recording backend, whose files are silence with
//! exact word timings (250 ms per word, 16 kHz), so cue files can be
//! compared byte for byte. ffmpeg conversion runs when ffmpeg is installed
//! and is skipped (with a note) where it is not.

use std::ops::ControlFlow;
use std::path::Path;

use textweaver_core::{CharRange, MarkerKind};
use textweaver_export::{
    AudioFormat, CueOptions, ExportError, ExportOptions, Progress, SubtitleFormat, SubtitleRequest,
    export, ffmpeg, subtitles, synthesize_wav, wav::WavData,
};
use textweaver_speech::{Caps, NormalizeConfig, RecordingBackend, RecordingMode, SpeechBackend};
use textweaver_text::{Document, DocumentData, DocumentMeta, Marker};

/// "Intro" (heading 1), a paragraph, "Next" (heading 2), a paragraph.
fn book() -> Document {
    let text = "Intro\n\nHello world. Paid $5 today.\n\nNext\n\nThe end.";
    let at = |s: &str| {
        let b = text.find(s).unwrap();
        let start = text[..b].chars().count();
        CharRange::new(start, start + s.chars().count())
    };
    Document::from(DocumentData {
        meta: DocumentMeta {
            title: Some("Sample Book".into()),
            author: Some("Ada".into()),
            ..DocumentMeta::default()
        },
        text: text.into(),
        markers: vec![
            Marker::new(MarkerKind::Heading, at("Intro")).with_level(1),
            Marker::new(MarkerKind::Paragraph, at("Hello world. Paid $5 today.")),
            Marker::new(MarkerKind::Heading, at("Next")).with_level(2),
            Marker::new(MarkerKind::Paragraph, at("The end.")),
        ],
    })
}

fn no_progress(_: Progress) -> ControlFlow<()> {
    ControlFlow::Continue(())
}

#[test]
fn wav_and_cues_from_the_recording_backend() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("book.wav");
    let (mut backend, rec) = RecordingBackend::new();
    let mut seen = Vec::new();
    let timeline = synthesize_wav(
        &book(),
        &mut backend,
        &out,
        &ExportOptions::default(),
        &mut |p| {
            seen.push(p);
            ControlFlow::Continue(())
        },
    )
    .unwrap();

    // What the engine was asked to say, one file per utterance.
    let spoken = rec
        .calls()
        .into_iter()
        .filter_map(|c| match c {
            textweaver_speech::backends::Call::SynthesizeToFile { text, .. } => Some(text),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        spoken,
        [
            "heading level 1, Intro",
            "Hello world.",
            "Paid five dollars today.",
            "heading level 2, Next",
            "The end."
        ]
    );
    // 4 + 2 + 4 + 4 + 2 words at 250 ms.
    assert_eq!(timeline.duration_ms, 4000);
    let audio = WavData::read(&out).unwrap();
    assert_eq!(audio.format.sample_rate, 16_000);
    assert_eq!(audio.frames(), 64_000);
    assert_eq!(seen.first(), Some(&Progress { done: 0, total: 5 }));
    assert_eq!(seen.last(), Some(&Progress { done: 5, total: 5 }));

    let chapters: Vec<(&str, u64, u64)> = timeline
        .chapters
        .iter()
        .map(|c| (c.title.as_str(), c.start_ms, c.end_ms))
        .collect();
    assert_eq!(chapters, [("Intro", 0, 2500), ("Next", 2500, 4000)]);

    // Captions show the document's text when its words sound: the spoken
    // "heading level 1," before "Intro" has no caption.
    let srt = subtitles(&timeline, SubtitleFormat::Srt, &CueOptions::default());
    assert_eq!(
        srt,
        "1\n00:00:00,750 --> 00:00:01,000\nIntro\n\n\
         2\n00:00:01,000 --> 00:00:01,500\nHello world.\n\n\
         3\n00:00:01,500 --> 00:00:02,500\nPaid $5 today.\n\n\
         4\n00:00:03,250 --> 00:00:03,500\nNext\n\n\
         5\n00:00:03,500 --> 00:00:04,000\nThe end.\n"
    );
    let words = CueOptions {
        word_level: true,
        ..CueOptions::default()
    };
    let vtt = subtitles(&timeline, SubtitleFormat::Vtt, &words);
    assert_eq!(
        vtt,
        "WEBVTT\n\n\
         00:00:00.750 --> 00:00:01.000\nIntro\n\n\
         00:00:01.000 --> 00:00:01.250\nHello\n\n\
         00:00:01.250 --> 00:00:01.500\nworld.\n\n\
         00:00:01.500 --> 00:00:01.750\nPaid\n\n\
         00:00:01.750 --> 00:00:02.250\n$5\n\n\
         00:00:02.250 --> 00:00:02.500\ntoday.\n\n\
         00:00:03.250 --> 00:00:03.500\nNext\n\n\
         00:00:03.500 --> 00:00:03.750\nThe\n\n\
         00:00:03.750 --> 00:00:04.000\nend.\n"
    );
}

#[test]
fn caption_lines_split_long_sentences_by_word_times() {
    let doc = Document::from_plain_text("one two three four five six seven.");
    let dir = tempfile::tempdir().unwrap();
    let (mut backend, _rec) = RecordingBackend::new();
    let timeline = synthesize_wav(
        &doc,
        &mut backend,
        &dir.path().join("a.wav"),
        &ExportOptions::default(),
        &mut no_progress,
    )
    .unwrap();
    let three = CueOptions {
        max_words: 3,
        ..CueOptions::default()
    };
    assert_eq!(
        subtitles(&timeline, SubtitleFormat::Srt, &three),
        "1\n00:00:00,000 --> 00:00:00,750\none two three\n\n\
         2\n00:00:00,750 --> 00:00:01,500\nfour five six\n\n\
         3\n00:00:01,500 --> 00:00:01,750\nseven.\n"
    );
}

/// A backend that writes files but reports no word timings (the trait's
/// default `synthesize_utterance`): cues fall back to Star's weighting
/// within each sentence.
struct NoWordTimes(RecordingBackend);

impl SpeechBackend for NoWordTimes {
    fn id(&self) -> textweaver_speech::BackendId {
        "no-word-times"
    }
    fn capabilities(&self) -> Caps {
        Caps::SYNTH_TO_FILE
    }
    fn voices(&self) -> Result<Vec<textweaver_speech::Voice>, textweaver_speech::SpeechError> {
        self.0.voices()
    }
    fn set_params(
        &mut self,
        p: &textweaver_speech::VoiceParams,
    ) -> Result<(), textweaver_speech::SpeechError> {
        self.0.set_params(p)
    }
    fn effective_wpm(&self) -> u16 {
        self.0.effective_wpm()
    }
    fn speak(
        &mut self,
        u: &textweaver_core::Utterance,
        sink: &mut dyn textweaver_speech::EventSink,
    ) -> Result<(), textweaver_speech::SpeechError> {
        self.0.speak(u, sink)
    }
    fn stop(&mut self) {
        self.0.stop();
    }
    fn synthesize_to_file(
        &mut self,
        text: &str,
        path: &Path,
    ) -> Result<(), textweaver_speech::SpeechError> {
        self.0.synthesize_to_file(text, path)
    }
}

#[test]
fn without_word_times_tokens_share_their_sentence_by_length() {
    let doc = Document::from_plain_text("Hi there everyone.");
    let dir = tempfile::tempdir().unwrap();
    let mut backend = NoWordTimes(RecordingBackend::new().0);
    let timeline = synthesize_wav(
        &doc,
        &mut backend,
        &dir.path().join("a.wav"),
        &ExportOptions::default(),
        &mut no_progress,
    )
    .unwrap();
    assert!(timeline.sentences[0].words.is_empty());
    let words = CueOptions {
        word_level: true,
        ..CueOptions::default()
    };
    // 750 ms over weights 3, 6, 10 (of 19).
    assert_eq!(
        subtitles(&timeline, SubtitleFormat::Srt, &words),
        "1\n00:00:00,000 --> 00:00:00,118\nHi\n\n\
         2\n00:00:00,118 --> 00:00:00,355\nthere\n\n\
         3\n00:00:00,355 --> 00:00:00,750\neveryone.\n"
    );
}

#[test]
fn gaps_cancellation_and_errors() {
    let dir = tempfile::tempdir().unwrap();
    let doc = Document::from_plain_text("One. Two.");
    let (mut backend, _) = RecordingBackend::new();
    let gap = ExportOptions {
        gap_ms: 100,
        ..ExportOptions::default()
    };
    let t = synthesize_wav(
        &doc,
        &mut backend,
        &dir.path().join("g.wav"),
        &gap,
        &mut no_progress,
    )
    .unwrap();
    let times: Vec<(u64, u64)> = t.sentences.iter().map(|s| (s.start_ms, s.end_ms)).collect();
    assert_eq!(times, [(0, 250), (350, 600)]);

    let out = dir.path().join("c.wav");
    let err = synthesize_wav(
        &doc,
        &mut backend,
        &out,
        &ExportOptions::default(),
        &mut |p| {
            if p.done == 1 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        },
    )
    .unwrap_err();
    assert!(matches!(err, ExportError::Cancelled));
    assert!(!out.exists(), "a cancelled export leaves no file");

    let mut null = textweaver_speech::NullBackend::default();
    let err = synthesize_wav(
        &doc,
        &mut null,
        &dir.path().join("n.wav"),
        &ExportOptions::default(),
        &mut no_progress,
    )
    .unwrap_err();
    assert_eq!(err.to_string(), "The null voice cannot write audio files.");

    let err = synthesize_wav(
        &Document::from_plain_text("  \n "),
        &mut backend,
        &dir.path().join("e.wav"),
        &ExportOptions::default(),
        &mut no_progress,
    )
    .unwrap_err();
    assert!(matches!(err, ExportError::Empty));

    let err = export(
        &doc,
        &mut backend,
        &dir.path().join("x.ogg"),
        None,
        None,
        &ExportOptions::default(),
        &mut no_progress,
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("use a .wav, .mp3, or .m4b"),
        "{err}"
    );
    let err = export(
        &doc,
        &mut backend,
        &dir.path().join("x.mp3"),
        None,
        None,
        &ExportOptions::default(),
        &mut no_progress,
    )
    .unwrap_err();
    assert!(matches!(err, ExportError::NoFfmpeg("MP3")), "{err}");
}

#[test]
fn normalization_can_be_turned_off_and_timed_mode_sets_the_word_length() {
    let dir = tempfile::tempdir().unwrap();
    let (backend, rec) = RecordingBackend::with(
        RecordingMode::Timed { ms_per_word: 100 },
        RecordingBackend::DEFAULT_CAPS,
    );
    let mut backend = backend;
    let opts = ExportOptions {
        normalize: NormalizeConfig::none(),
        ..ExportOptions::default()
    };
    let t = synthesize_wav(
        &Document::from_plain_text("Paid $5 today."),
        &mut backend,
        &dir.path().join("a.wav"),
        &opts,
        &mut no_progress,
    )
    .unwrap();
    assert_eq!(
        rec.spoken_texts(),
        Vec::<String>::new(),
        "files, not speech"
    );
    assert_eq!(t.sentences[0].spoken, "Paid $5 today.");
    assert_eq!(t.duration_ms, 300);
}

#[test]
fn export_writes_wav_and_subtitles() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("book.wav");
    let srt = dir.path().join("book.srt");
    let (mut backend, _) = RecordingBackend::new();
    let report = export(
        &book(),
        &mut backend,
        &out,
        Some(&SubtitleRequest {
            path: srt.clone(),
            cues: CueOptions::default(),
        }),
        None,
        &ExportOptions::default(),
        &mut no_progress,
    )
    .unwrap();
    assert_eq!(report.format, AudioFormat::Wav);
    assert_eq!(report.subtitles.as_deref(), Some(srt.as_path()));
    assert_eq!(
        std::fs::read_to_string(&srt).unwrap(),
        subtitles(
            &report.timeline,
            SubtitleFormat::Srt,
            &CueOptions::default()
        )
    );
    assert!(
        std::fs::read_to_string(&srt)
            .unwrap()
            .starts_with("1\n00:00:00,750 --> 00:00:01,000\nIntro\n")
    );
    assert!(WavData::read(&out).is_ok());
}

/// MP3 and M4B through ffmpeg, when it is installed.
#[test]
fn ffmpeg_conversion_when_available() {
    let Some(ff) = ffmpeg::find() else {
        eprintln!("ffmpeg not found; skipping the MP3 and M4B conversion test");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    for name in ["book.mp3", "book.m4b"] {
        let out = dir.path().join(name);
        let (mut backend, _) = RecordingBackend::new();
        let report = export(
            &book(),
            &mut backend,
            &out,
            None,
            Some(&ff),
            &ExportOptions::default(),
            &mut no_progress,
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(report.ffmpeg.as_deref(), Some(ff.as_path()));
        let len = std::fs::metadata(&out).unwrap().len();
        assert!(len > 0, "{name} is empty");
        // Only the output is left: the WAV and metadata were temporary.
        let left: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert!(
            left.iter()
                .all(|n| n.to_string_lossy().starts_with("book.")),
            "{left:?}"
        );
        let probe = ff.with_file_name(if cfg!(windows) {
            "ffprobe.exe"
        } else {
            "ffprobe"
        });
        if name.ends_with("m4b") && probe.is_file() {
            let o = std::process::Command::new(&probe)
                .args(["-v", "error", "-show_chapters", "-of", "csv=p=0"])
                .arg(&out)
                .output()
                .unwrap();
            let chapters = String::from_utf8_lossy(&o.stdout);
            assert!(chapters.contains("Intro"), "{chapters}");
            assert!(chapters.contains("Next"), "{chapters}");
        }
    }
}
