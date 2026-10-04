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
/// default `synthesize_utterance`): cues fall back to star's weighting
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
    assert!(!out.exists(), "a canceled export leaves no file");

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
        &dir.path().join("x.aac"),
        None,
        None,
        &ExportOptions::default(),
        &mut no_progress,
    )
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("use a .wav, .flac, .mp3, .opus, .ogg, .m4b, or .mp4"),
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

/// An Ogg Opus file read back: its header fields, its comments, and its
/// audio decoded at 48 kHz with the pre-skip and end trimming applied.
#[cfg(feature = "opus")]
struct OggOpus {
    input_rate: u32,
    channels: u8,
    comments: Vec<String>,
    audio: Vec<f32>,
}

/// Reads every Ogg page of `path`, checking each one's CRC, the first
/// page's start flag and the last page's end flag, then decodes the
/// packets with libopus.
#[cfg(feature = "opus")]
fn read_ogg_opus(path: &Path) -> OggOpus {
    let bytes = std::fs::read(path).unwrap();
    let mut packets: Vec<Vec<u8>> = Vec::new();
    let mut partial: Vec<u8> = Vec::new();
    let mut last_granule = 0i64;
    let mut flags_seen = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        assert_eq!(&bytes[at..at + 4], b"OggS", "page at byte {at}");
        let flags = bytes[at + 5];
        let granule = i64::from_le_bytes(bytes[at + 6..at + 14].try_into().unwrap());
        let segs = usize::from(bytes[at + 26]);
        let lacing = &bytes[at + 27..at + 27 + segs];
        let len: usize = lacing.iter().map(|&b| usize::from(b)).sum();
        let mut page = bytes[at..at + 27 + segs + len].to_vec();
        let crc = u32::from_le_bytes(page[22..26].try_into().unwrap());
        page[22..26].copy_from_slice(&[0; 4]);
        assert_eq!(
            textweaver_export::opus::ogg_crc(&page),
            crc,
            "CRC of the page at byte {at}"
        );
        let mut data = &bytes[at + 27 + segs..at + 27 + segs + len];
        for &l in lacing {
            let (piece, rest) = data.split_at(usize::from(l));
            partial.extend_from_slice(piece);
            data = rest;
            if l < 255 {
                packets.push(std::mem::take(&mut partial));
            }
        }
        if granule >= 0 {
            assert!(granule >= last_granule, "granule positions only grow");
            last_granule = granule;
        }
        flags_seen.push(flags);
        at += 27 + segs + len;
    }
    assert_eq!(
        flags_seen.first(),
        Some(&0x02),
        "the first page starts the stream"
    );
    assert_eq!(
        flags_seen.last().map(|f| f & 0x04),
        Some(0x04),
        "the last page ends it"
    );
    assert!(partial.is_empty(), "no packet is cut off");
    let head = &packets[0];
    assert_eq!(&head[..8], b"OpusHead");
    let pre_skip = usize::from(u16::from_le_bytes([head[10], head[11]]));
    let tags = &packets[1];
    assert_eq!(&tags[..8], b"OpusTags");
    let u32_at = |b: &[u8], i: usize| u32::from_le_bytes(b[i..i + 4].try_into().unwrap()) as usize;
    let vendor = u32_at(tags, 8);
    let mut i = 12 + vendor;
    let count = u32_at(tags, i);
    i += 4;
    let mut comments = Vec::new();
    for _ in 0..count {
        let n = u32_at(tags, i);
        comments.push(String::from_utf8(tags[i + 4..i + 4 + n].to_vec()).unwrap());
        i += 4 + n;
    }
    let mut decoder =
        opusic_c::Decoder::new(opusic_c::Channels::Mono, opusic_c::SampleRate::Hz48000).unwrap();
    let mut audio = Vec::new();
    let mut frame = vec![0f32; 5760];
    for p in &packets[2..] {
        let n = decoder.decode_float_to_slice(p, &mut frame, false).unwrap();
        audio.extend_from_slice(&frame[..n]);
    }
    let end = usize::try_from(last_granule).unwrap();
    assert!(audio.len() >= end, "{} samples, granule {end}", audio.len());
    audio.truncate(end);
    audio.drain(..pre_skip);
    OggOpus {
        input_rate: u32::from_le_bytes(head[12..16].try_into().unwrap()),
        channels: head[9],
        comments,
        audio,
    }
}

/// Ogg Opus in process, with no ffmpeg: decoded back, it is exactly as
/// long as the timeline, and its comments carry the title and chapters.
#[cfg(feature = "opus")]
#[test]
fn opus_without_ffmpeg_decodes_back_with_its_chapters() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("book.opus");
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
    assert_eq!(report.format, AudioFormat::Opus);
    assert_eq!(report.ffmpeg, None);
    let back = read_ogg_opus(&out);
    assert_eq!(back.input_rate, 16_000);
    assert_eq!(back.channels, 1);
    // 4 seconds: 192,000 samples at 48 kHz, exactly.
    assert_eq!(report.timeline.duration_ms, 4000);
    assert_eq!(back.audio.len(), 192_000);
    assert_eq!(
        back.comments,
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
    let left: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left, ["book.opus"]);
}

/// Real sound survives the encoder, resampled from the engine's 22,050 Hz:
/// half a second of silence, then a tone. Decoded, the tone starts where
/// it should (the pre-skip is right), keeps its pitch and loudness, and
/// the file is a small fraction of the WAV.
#[cfg(feature = "opus")]
#[test]
fn opus_keeps_a_tone_in_time_and_pitch() {
    let dir = tempfile::tempdir().unwrap();
    let rate = 22_050usize;
    // About 210.6 Hz at 0.37 of full scale.
    let step = 0.06f64;
    let pcm: Vec<i16> = (0..rate * 2)
        .map(|i| {
            if i < rate / 2 {
                0
            } else {
                (((i - rate / 2) as f64 * step).sin() * 12_000.0) as i16
            }
        })
        .collect();
    let mut w = textweaver_export::wav::WavWriter::create(&dir.path().join("a.wav")).unwrap();
    let mut bytes = Vec::new();
    for s in &pcm {
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    let piece = WavData {
        format: textweaver_export::wav::WavFormat::pcm16_mono(rate as u32),
        audio: bytes,
    };
    w.append(&piece, Path::new("a")).unwrap();
    w.finish().unwrap();
    let out = dir.path().join("a.opus");
    let comments = textweaver_export::vorbis::comments(Some("Tone"), None, &[]);
    textweaver_export::opus::encode(&dir.path().join("a.wav"), &out, &comments).unwrap();
    let back = read_ogg_opus(&out);
    assert_eq!(back.input_rate, 22_050);
    assert_eq!(
        back.comments,
        ["TITLE=Tone", "ALBUM=Tone", "GENRE=Audiobook"]
    );
    // Two seconds at 48 kHz.
    assert_eq!(back.audio.len(), 96_000);
    // The tone starts at half a second, within a millisecond.
    let onset = back.audio.iter().position(|s| s.abs() > 0.05).unwrap();
    assert!(
        onset.abs_diff(24_000) <= 48,
        "the tone starts at sample {onset}"
    );
    // Its pitch: zero crossings over the last second, 2 per cycle.
    let tone = &back.audio[48_000..96_000 - 960];
    let crossings = tone
        .windows(2)
        .filter(|w| (w[0] < 0.0) != (w[1] < 0.0))
        .count();
    let hz = crossings as f64 / 2.0 / (tone.len() as f64 / 48_000.0);
    let expected = step * rate as f64 / std::f64::consts::TAU;
    assert!(
        (hz - expected).abs() < expected * 0.02,
        "{hz:.1} Hz, expected {expected:.1} Hz"
    );
    // Its loudness: the RMS of a sine at 12,000 of 32,768.
    let rms = (tone.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / tone.len() as f64).sqrt();
    let want = 12_000.0 / 32_768.0 / std::f64::consts::SQRT_2;
    assert!(
        (rms - want).abs() < want * 0.15,
        "RMS {rms:.3}, expected {want:.3}"
    );
    // About 32 kbit/s: two seconds in under an eighth of the 16-bit WAV.
    let size = std::fs::metadata(&out).unwrap().len();
    assert!(size < (rate * 2 * 2 / 8) as u64, "{size} bytes");
}

/// MP3 and M4B through ffmpeg, when it is installed.
#[test]
fn ffmpeg_conversion_when_available() {
    let Some(ff) = ffmpeg::find() else {
        skip_or_fail(
            "ffmpeg",
            "ffmpeg not found, so the MP3 and M4B conversion test",
        );
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

/// Skips a test whose tool is missing, loudly, or fails it when CI says the
/// tool must be there: `TEXTWEAVER_REQUIRE_TOOLS` names the required tools,
/// comma-separated (`docs/dev/testing.md`, "Tests that need a tool"). A test
/// that skips where the tool should exist is a gate that never runs.
fn skip_or_fail(tool: &str, why: &str) {
    let required = std::env::var("TEXTWEAVER_REQUIRE_TOOLS")
        .is_ok_and(|v| v.split(',').any(|t| t.trim() == tool));
    assert!(
        !required,
        "Fail: {why}, but TEXTWEAVER_REQUIRE_TOOLS requires {tool} here"
    );
    eprintln!("SKIPPED, not checked: {why}");
}

/// A stop asked for after the last sentence, as the progress reaches
/// `done == total`, is a stop: `Cancelled`, and no file.
#[test]
fn a_stop_after_the_last_sentence_leaves_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("late.wav");
    let (mut backend, _) = RecordingBackend::new();
    let err = export(
        &book(),
        &mut backend,
        &out,
        None,
        None,
        &ExportOptions::default(),
        &mut |p| {
            if p.total > 0 && p.done == p.total {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        },
    )
    .unwrap_err();
    assert!(matches!(err, ExportError::Cancelled), "{err}");
    assert!(!out.exists(), "a stopped export leaves no file");
}

/// A stop during FLAC encoding (asked after synthesis, with the last
/// count again) ends as `Cancelled` with no file and no subtitles, never
/// as written.
#[cfg(feature = "flac")]
#[test]
fn a_stop_during_encoding_leaves_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("stopped.flac");
    let subs = textweaver_export::SubtitleRequest {
        path: dir.path().join("stopped.vtt"),
        cues: textweaver_export::CueOptions::default(),
    };
    let (mut backend, _) = RecordingBackend::new();
    let mut at_end = 0;
    let err = export(
        &book(),
        &mut backend,
        &out,
        Some(&subs),
        None,
        &ExportOptions::default(),
        &mut |p| {
            if p.total > 0 && p.done == p.total {
                at_end += 1;
            }
            // Synthesis ends normally; the first question while encoding
            // says stop.
            if at_end >= 2 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        },
    )
    .unwrap_err();
    assert!(matches!(err, ExportError::Cancelled), "{err}");
    assert!(at_end >= 2, "the encoder asked whether to stop");
    assert!(!out.exists(), "a stopped export leaves no file");
    assert!(!subs.path.exists(), "and no subtitles");
    // The encoder alone, told to stop at once.
    let wav = dir.path().join("a.wav");
    let (mut backend, _) = RecordingBackend::new();
    synthesize_wav(
        &book(),
        &mut backend,
        &wav,
        &ExportOptions::default(),
        &mut no_progress,
    )
    .unwrap();
    let flac = dir.path().join("a.flac");
    let err = textweaver_export::flac::encode_with_stop(&wav, &flac, &[], &|| true).unwrap_err();
    assert!(matches!(err, ExportError::Cancelled), "{err}");
    assert!(!flac.exists());
}

/// Ogg Vorbis in process, with no ffmpeg: the file opens with an Ogg page
/// (the `OggS` capture pattern, the stream's first page) holding the Vorbis
/// identification header with the engine's rate and channel count; the
/// comment header carries the title and chapters; and decoded back, the
/// audio is as long as the timeline.
#[cfg(feature = "vorbis")]
#[test]
fn ogg_vorbis_without_ffmpeg_reads_back_with_its_header_and_chapters() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("book.ogg");
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
    assert_eq!(report.format, AudioFormat::Ogg);
    assert_eq!(report.ffmpeg, None);
    assert_eq!(report.timeline.duration_ms, 4000);

    let bytes = std::fs::read(&out).unwrap();
    // The first page: capture pattern, version 0, the start-of-stream flag,
    // and one packet, the identification header.
    assert_eq!(&bytes[..4], b"OggS");
    assert_eq!(bytes[4], 0);
    assert_eq!(bytes[5], 0x02);
    let segs = usize::from(bytes[26]);
    let id = &bytes[27 + segs..];
    assert_eq!(&id[..7], b"\x01vorbis");
    let u32_at = |b: &[u8], i: usize| u32::from_le_bytes(b[i..i + 4].try_into().unwrap());
    assert_eq!(u32_at(id, 7), 0, "Vorbis version");
    assert_eq!(id[11], 1, "channels");
    assert_eq!(u32_at(id, 12), 16_000, "sample rate");

    // The comment header follows on the next page.
    let second = 27
        + segs
        + bytes[27..27 + segs]
            .iter()
            .map(|&b| usize::from(b))
            .sum::<usize>();
    assert_eq!(&bytes[second..second + 4], b"OggS");
    let segs2 = usize::from(bytes[second + 26]);
    let tags = &bytes[second + 27 + segs2..];
    assert_eq!(&tags[..7], b"\x03vorbis");
    let vendor = u32_at(tags, 7) as usize;
    let mut i = 11 + vendor;
    let count = u32_at(tags, i);
    i += 4;
    let mut comments = Vec::new();
    for _ in 0..count {
        let n = u32_at(tags, i) as usize;
        comments.push(String::from_utf8(tags[i + 4..i + 4 + n].to_vec()).unwrap());
        i += 4 + n;
    }
    assert_eq!(
        comments,
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

    // Decoded back: 4 seconds at 16 kHz, mono.
    let mut decoder = vorbis_rs::VorbisDecoder::new(std::fs::File::open(&out).unwrap()).unwrap();
    assert_eq!(decoder.sampling_frequency().get(), 16_000);
    assert_eq!(decoder.channels().get(), 1);
    let mut frames = 0;
    while let Some(block) = decoder.decode_audio_block().unwrap() {
        frames += block.samples()[0].len();
    }
    assert_eq!(frames, 64_000);
    let left: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left, ["book.ogg"]);
}
