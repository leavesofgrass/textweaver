//! A real export with espeak-ng (feature `espeak`; the dev container):
//! a WAV with real audio, and SRT cues whose word times come from
//! espeak-ng's `audio_position`. Nothing is played aloud; the audio stays
//! in a temporary folder.
#![cfg(feature = "espeak")]

use std::ops::ControlFlow;

use textweaver_core::{CharRange, MarkerKind};
use textweaver_export::{
    CueOptions, ExportOptions, SubtitleFormat, SubtitleRequest, export, wav::WavData,
};
use textweaver_speech::backends::espeak::{EspeakBackend, EspeakOutput};
use textweaver_text::{Document, DocumentData, DocumentMeta, Marker};

#[test]
fn espeak_exports_a_wav_and_srt_with_word_times() {
    let text = "Intro\n\nThe library opens at nine. Smith reads every book.";
    let doc = Document::from(DocumentData {
        meta: DocumentMeta::default(),
        text: text.into(),
        markers: vec![Marker::new(MarkerKind::Heading, CharRange::new(0, 5)).with_level(1)],
    });
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("book.wav");
    let srt = dir.path().join("book.srt");
    let mut backend = EspeakBackend::new(EspeakOutput::Virtual).unwrap();
    let report = export(
        &doc,
        &mut backend,
        &out,
        Some(&SubtitleRequest {
            path: srt.clone(),
            cues: CueOptions::default(),
        }),
        None,
        &ExportOptions::default(),
        &mut |_| ControlFlow::Continue(()),
    )
    .unwrap();
    let audio = WavData::read(&out).unwrap();
    assert!(audio.frames() > 0);
    let t = &report.timeline;
    assert_eq!(t.sentences.len(), 3);
    assert!(t.duration_ms > 1000, "{}", t.duration_ms);
    // Sentences follow one another; every document word got a time from
    // espeak-ng, rising within each sentence.
    for w in t.sentences.windows(2) {
        assert_eq!(w[0].end_ms, w[1].start_ms);
    }
    let words: Vec<&str> = t
        .sentences
        .iter()
        .flat_map(|s| s.words.iter().map(|w| w.text.as_str()))
        .collect();
    assert_eq!(
        words,
        [
            "Intro", "The", "library", "opens", "at", "nine.", "Smith", "reads", "every", "book."
        ]
    );
    for s in &t.sentences {
        for w in s.words.windows(2) {
            assert!(w[0].start_ms < w[1].start_ms, "{:?}", s.words);
        }
    }
    let cues = std::fs::read_to_string(&srt).unwrap();
    assert!(cues.starts_with("1\n00:00:"), "{cues}");
    assert!(cues.contains("\nThe library opens at nine.\n"), "{cues}");
    assert!(cues.contains("\nSmith reads every book.\n"), "{cues}");
    eprintln!("espeak export: {} ms of audio\n{cues}", t.duration_ms);
    let vtt = textweaver_export::subtitles(
        t,
        SubtitleFormat::Vtt,
        &CueOptions {
            word_level: true,
            ..CueOptions::default()
        },
    );
    assert_eq!(vtt.matches(" --> ").count(), 10, "{vtt}");
}
