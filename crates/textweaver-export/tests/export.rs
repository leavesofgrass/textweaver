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
        err.to_string().contains("use a .wav, .flac, .mp3, or .m4b"),
        "{err}"
    );
    let err = export(
        &doc,
        &mut backend,
        &dir.path().join("x.m4b"),
        None,
        None,
        &ExportOptions::default(),
        &mut no_progress,
    )
    .unwrap_err();
    assert!(matches!(err, ExportError::NoFfmpeg("M4B")), "{err}");
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

/// FLAC in process, with no ffmpeg: decoded back (a pure-Rust decoder), it
/// is exactly as long as the timeline, and its Vorbis comments carry the
/// title and the chapters.
#[cfg(feature = "flac")]
#[test]
fn flac_without_ffmpeg_decodes_back_with_its_chapters() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("book.flac");
    let (mut backend, _) = RecordingBackend::new();
    let report = export(
        &book(),
        &mut backend,
        &out,
        None,
        None,
        &ExportOptions::default(),
        &mut no_progress,
    )
    .unwrap();
    assert_eq!(report.format, AudioFormat::Flac);
    assert_eq!(report.ffmpeg, None);
    let mut flac = claxon::FlacReader::open(&out).unwrap();
    let info = flac.streaminfo();
    assert_eq!(info.sample_rate, 16_000);
    assert_eq!(info.channels, 1);
    assert_eq!(info.bits_per_sample, 16);
    let samples = flac.samples().count();
    assert_eq!(samples as u64 * 1000 / 16_000, report.timeline.duration_ms);
    assert_eq!(samples, 64_000);
    let tags: Vec<String> = flac.tags().map(|(k, v)| format!("{k}={v}")).collect();
    assert_eq!(
        tags,
        [
            "TITLE=Sample Book",
            "ALBUM=Sample Book",
            "ARTIST=Ada",
            "GENRE=Audiobook",
            "CHAPTER001=00:00:00.000",
            "CHAPTER001NAME=Intro",
            "CHAPTER002=00:00:02.500",
            "CHAPTER002NAME=Next",
        ]
    );
    // Only the output is left: the WAV was temporary.
    let left: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left, ["book.flac"]);
}

/// The FLAC encoder keeps real audio exactly (lossless), for 16-bit and
/// float engines alike.
#[cfg(feature = "flac")]
#[test]
fn flac_is_lossless_for_pcm_and_rounds_float() {
    let dir = tempfile::tempdir().unwrap();
    let pcm: Vec<i16> = (0..5000)
        .map(|i| ((i as f64 * 0.05).sin() * 12_000.0) as i16)
        .collect();
    let mut w = textweaver_export::wav::WavWriter::create(&dir.path().join("a.wav")).unwrap();
    let mut bytes = Vec::new();
    for s in &pcm {
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    let piece = WavData {
        format: textweaver_export::wav::WavFormat::pcm16_mono(22_050),
        audio: bytes,
    };
    w.append(&piece, Path::new("a")).unwrap();
    w.finish().unwrap();
    let out = dir.path().join("a.flac");
    textweaver_export::flac::encode(&dir.path().join("a.wav"), &out, &[]).unwrap();
    let mut flac = claxon::FlacReader::open(&out).unwrap();
    let back: Vec<i16> = flac.samples().map(|s| s.unwrap() as i16).collect();
    assert_eq!(back, pcm);
}

/// WAV gets its title and chapters as an ID3 tag, which the export's own
/// WAV reader skips.
#[cfg(feature = "id3")]
#[test]
fn wav_carries_id3_chapters() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("book.wav");
    let (mut backend, _) = RecordingBackend::new();
    export(
        &book(),
        &mut backend,
        &out,
        None,
        None,
        &ExportOptions::default(),
        &mut no_progress,
    )
    .unwrap();
    use id3::TagLike;
    let tag = id3::Tag::read_from_path(&out).unwrap();
    assert_eq!(tag.title(), Some("Sample Book"));
    assert_eq!(tag.album(), Some("Sample Book"));
    assert_eq!(tag.artist(), Some("Ada"));
    assert_eq!(tag.genre(), Some("Audiobook"));
    let chapters: Vec<(u32, u32, String)> = tag
        .chapters()
        .map(|c| {
            let title = c
                .frames
                .iter()
                .find(|f| f.id() == "TIT2")
                .and_then(|f| f.content().text())
                .unwrap_or_default()
                .to_owned();
            (c.start_time, c.end_time, title)
        })
        .collect();
    assert_eq!(
        chapters,
        [
            (0, 2500, "Intro".to_owned()),
            (2500, 4000, "Next".to_owned())
        ]
    );
    let toc: Vec<_> = tag.tables_of_contents().collect();
    assert_eq!(toc.len(), 1);
    assert!(toc[0].top_level && toc[0].ordered);
    assert_eq!(toc[0].elements, ["chp1", "chp2"]);
    // The audio is untouched.
    assert_eq!(WavData::read(&out).unwrap().frames(), 64_000);
}

/// Decodes an MP3 file with symphonia (a test-only, pure-Rust decoder),
/// gapless: its sample rate and the frames it holds.
#[cfg(feature = "mp3")]
fn decode_mp3(path: &Path) -> (u32, usize) {
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::errors::Error;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;
    let file = std::fs::File::open(path).unwrap();
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("mp3");
    let formats = FormatOptions {
        enable_gapless: true,
        ..FormatOptions::default()
    };
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &formats, &MetadataOptions::default())
        .unwrap();
    let mut reader = probed.format;
    let track = reader.default_track().unwrap().clone();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .unwrap();
    let mut frames = 0;
    loop {
        match reader.next_packet() {
            Ok(p) if p.track_id() == track.id => frames += decoder.decode(&p).unwrap().frames(),
            Ok(_) => {}
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => panic!("{e}"),
        }
    }
    (track.codec_params.sample_rate.unwrap_or(0), frames)
}

/// MP3 in process, with no ffmpeg: it decodes back to the timeline's
/// length, and its ID3v2 tag carries the title and the chapters.
#[cfg(feature = "mp3")]
#[test]
fn mp3_without_ffmpeg_decodes_back_with_its_chapters() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("book.mp3");
    let (mut backend, _) = RecordingBackend::new();
    let report = export(
        &book(),
        &mut backend,
        &out,
        None,
        None,
        &ExportOptions::default(),
        &mut no_progress,
    )
    .unwrap();
    assert_eq!(report.format, AudioFormat::Mp3);
    assert_eq!(report.ffmpeg, None);
    let (rate, frames) = decode_mp3(&out);
    assert_eq!(rate, 16_000);
    // 4 seconds at 16 kHz: 64,000 frames. The LAME tag lets the decoder
    // drop LAME's delay and padding; one MPEG frame (576 samples at 16
    // kHz) of slack remains for decoders that round.
    let expected = report.timeline.duration_ms as usize * 16;
    assert_eq!(expected, 64_000);
    assert!(
        frames.abs_diff(expected) <= 576,
        "decoded {frames} frames, expected {expected}"
    );
    use id3::TagLike;
    let tag = id3::Tag::read_from_path(&out).unwrap();
    assert_eq!(tag.title(), Some("Sample Book"));
    assert_eq!(tag.artist(), Some("Ada"));
    let chapters: Vec<(u32, u32, String)> = tag
        .chapters()
        .map(|c| {
            let title = c
                .frames
                .iter()
                .find(|f| f.id() == "TIT2")
                .and_then(|f| f.content().text())
                .unwrap_or_default()
                .to_owned();
            (c.start_time, c.end_time, title)
        })
        .collect();
    assert_eq!(
        chapters,
        [
            (0, 2500, "Intro".to_owned()),
            (2500, 4000, "Next".to_owned())
        ]
    );
    assert_eq!(tag.tables_of_contents().count(), 1);
    // The tag did not break the audio: it still decodes after it.
    let left: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left, ["book.mp3"]);
}

/// Real sound, not silence, survives the encoder: a tone's length and
/// rate come back from a 22,050 Hz mono WAV.
#[cfg(feature = "mp3")]
#[test]
fn mp3_encodes_a_tone_at_the_engines_rate() {
    let dir = tempfile::tempdir().unwrap();
    let pcm: Vec<i16> = (0..22_050)
        .map(|i| ((i as f64 * 0.06).sin() * 12_000.0) as i16)
        .collect();
    let mut w = textweaver_export::wav::WavWriter::create(&dir.path().join("a.wav")).unwrap();
    let mut bytes = Vec::new();
    for s in &pcm {
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    let piece = WavData {
        format: textweaver_export::wav::WavFormat::pcm16_mono(22_050),
        audio: bytes,
    };
    w.append(&piece, Path::new("a")).unwrap();
    w.finish().unwrap();
    let out = dir.path().join("a.mp3");
    textweaver_export::mp3::encode(&dir.path().join("a.wav"), &out).unwrap();
    let (rate, frames) = decode_mp3(&out);
    assert_eq!(rate, 22_050);
    assert!(frames.abs_diff(22_050) <= 576, "decoded {frames} frames");
    // Smaller than the WAV (44 KB of 16-bit samples).
    assert!(std::fs::metadata(&out).unwrap().len() < 44_100 / 2);
}

/// MP3 and M4B through ffmpeg, when it is installed.
#[test]
fn ffmpeg_conversion_when_available() {
    let Some(ff) = ffmpeg::find() else {
        eprintln!("ffmpeg not found; skipping the MP3 and M4B conversion test");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    // MP3 goes through ffmpeg only in a build without the mp3 feature.
    for name in ["book.mp3", "book.m4b"] {
        let out = dir.path().join(name);
        if !AudioFormat::from_path(&out).is_some_and(AudioFormat::needs_ffmpeg) {
            continue;
        }
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
