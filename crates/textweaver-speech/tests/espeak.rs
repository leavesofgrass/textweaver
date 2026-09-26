//! The espeak-ng backend against the real library (Linux dev container and
//! CI). There is no sound device there, so these tests use the `Virtual`
//! output: real synthesis and real event timing, audio discarded.

#![cfg(feature = "espeak")]

use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use textweaver_speech::backends::espeak::{EspeakBackend, EspeakOutput};
use textweaver_speech::core::{CharPos, CharRange, Utterance, UtteranceId};
use textweaver_speech::{
    EventSink, RawEvent, ServiceConfig, SpeechBackend, SpeechService, SpeechStatus, VoiceParams,
};

/// libespeak-ng is a process-wide singleton: one test at a time.
fn engine() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

#[derive(Default)]
struct Collect(Vec<(UtteranceId, RawEvent)>);

impl EventSink for Collect {
    fn emit(&mut self, id: UtteranceId, event: RawEvent) {
        self.0.push((id, event));
    }
    fn is_current(&self, _: UtteranceId) -> bool {
        true
    }
}

fn utt(text: &str, chunk: u32) -> Utterance {
    let mut u = Utterance::literal(text, CharPos(0));
    u.id = UtteranceId {
        generation: 1,
        chunk,
    };
    u
}

/// Polls until `id` ends or `timeout` passes.
fn run_until_end(b: &mut EspeakBackend, sink: &mut Collect, id: UtteranceId, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        b.poll(sink);
        if sink
            .0
            .iter()
            .any(|(i, e)| *i == id && matches!(e, RawEvent::Finished | RawEvent::Cancelled))
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("utterance did not end: {:?}", sink.0);
}

#[test]
fn word_events_carry_byte_ranges_and_audio_times() {
    let _g = engine();
    let mut b = EspeakBackend::new(EspeakOutput::Virtual).expect("espeak-ng initializes");
    b.set_params(&VoiceParams::default()).unwrap();
    let u = utt("Café pour Émile, s'il vous plaît.", 0);
    let mut sink = Collect::default();
    let t0 = Instant::now();
    b.speak(&u, &mut sink).unwrap();
    run_until_end(&mut b, &mut sink, u.id, Duration::from_secs(20));
    let elapsed = t0.elapsed();
    assert_eq!(sink.0.first().map(|(_, e)| e), Some(&RawEvent::Started));
    assert_eq!(sink.0.last().map(|(_, e)| e), Some(&RawEvent::Finished));
    let words: Vec<(&str, u32)> = sink
        .0
        .iter()
        .filter_map(|(_, e)| match e {
            RawEvent::Word {
                byte_range,
                audio_ms,
            } => Some((
                &u.text[byte_range.start as usize..byte_range.end as usize],
                audio_ms.expect("espeak reports audio time"),
            )),
            _ => None,
        })
        .collect();
    let texts: Vec<&str> = words.iter().map(|(w, _)| *w).collect();
    assert_eq!(texts[..3], ["Café", "pour", "Émile"], "{words:?}");
    assert!(words.windows(2).all(|w| w[0].1 <= w[1].1), "{words:?}");
    assert!(words.last().unwrap().1 > 500, "{words:?}");
    // Virtual output paces Finished to the audio's duration.
    assert!(elapsed >= Duration::from_millis(u64::from(words.last().unwrap().1)));
    let voices = b.voices().unwrap();
    assert!(
        voices
            .iter()
            .any(|v| v.languages.iter().any(|l| l == "en-us")),
        "{voices:?}"
    );
}

#[test]
fn stop_cancels_a_long_utterance() {
    let _g = engine();
    let mut b = EspeakBackend::new(EspeakOutput::Virtual).expect("espeak-ng initializes");
    let text = "This is a long sentence that would take several seconds to speak aloud. ".repeat(4);
    let u = utt(&text, 0);
    let mut sink = Collect::default();
    b.speak(&u, &mut sink).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    let t0 = Instant::now();
    b.stop();
    run_until_end(&mut b, &mut sink, u.id, Duration::from_secs(5));
    assert!(t0.elapsed() < Duration::from_secs(2));
    assert_eq!(sink.0.last().map(|(_, e)| e), Some(&RawEvent::Cancelled));
    // Jobs queued before the stop are cancelled without speaking.
    let later = utt("Queued.", 1);
    b.speak(&later, &mut sink).unwrap();
    run_until_end(&mut b, &mut sink, later.id, Duration::from_secs(10));
}

#[test]
fn unknown_voices_are_rejected() {
    let _g = engine();
    let mut b = EspeakBackend::new(EspeakOutput::Virtual).expect("espeak-ng initializes");
    let err = b
        .set_params(&VoiceParams {
            voice: Some("no-such-voice".into()),
            ..VoiceParams::default()
        })
        .unwrap_err();
    assert!(matches!(
        err,
        textweaver_speech::SpeechError::UnknownVoice(_)
    ));
    b.set_params(&VoiceParams {
        voice: Some("en-us".into()),
        ..VoiceParams::default()
    })
    .unwrap();
}

#[test]
fn synthesize_to_a_wav_file() {
    let _g = engine();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.wav");
    let mut b = EspeakBackend::new(EspeakOutput::Virtual).expect("espeak-ng initializes");
    b.synthesize_to_file("Hello from textweaver.", &path)
        .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    assert!(bytes.len() > 44 + 10_000, "{} bytes", bytes.len());
    // The backend still speaks afterwards.
    let u = utt("Still here.", 0);
    let mut sink = Collect::default();
    b.speak(&u, &mut sink).unwrap();
    run_until_end(&mut b, &mut sink, u.id, Duration::from_secs(10));
    assert!(
        sink.0
            .iter()
            .any(|(_, e)| matches!(e, RawEvent::Word { .. }))
    );
}

#[test]
fn the_service_highlights_each_word_on_the_audio_clock() {
    let _g = engine();
    let factory = Box::new(|| {
        EspeakBackend::new(EspeakOutput::Virtual).map(|b| Box::new(b) as Box<dyn SpeechBackend>)
    });
    let s = SpeechService::spawn(factory, ServiceConfig::default()).expect("service starts");
    assert_eq!(s.backend_id(), "espeak");
    let t0 = Instant::now();
    s.read(vec![
        Utterance::literal("One small step.", CharPos(0)),
        Utterance::literal("It is 2024.", CharPos(16)),
    ]);
    let mut got = Vec::new();
    loop {
        match s.statuses().recv_timeout(Duration::from_secs(20)) {
            Ok(SpeechStatus::Position { source_range, .. }) => {
                got.push((source_range, t0.elapsed()))
            }
            Ok(SpeechStatus::Finished) => break,
            Ok(other) => panic!("unexpected {other:?}"),
            Err(e) => panic!("{e}: {got:?}"),
        }
    }
    let ranges: Vec<Option<CharRange>> = got.iter().map(|(r, _)| *r).collect();
    let c = |a, b| Some(CharRange::new(a, b));
    // "2024" is spoken as "twenty twenty-four": both words highlight it,
    // reported once.
    assert_eq!(
        ranges,
        [c(0, 3), c(4, 9), c(10, 14), c(16, 18), c(19, 21), c(22, 26)]
    );
    // Words are spread over the audio, not fired in a burst.
    let first = got[1].1;
    let last = got[got.len() - 1].1;
    assert!(last > first + Duration::from_millis(300), "{got:?}");
}
