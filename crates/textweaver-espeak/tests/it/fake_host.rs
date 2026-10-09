//! The backend against the real host binary running its fake engine
//! (`--engine fake`): the pipe protocol, the playback clock, and the event
//! contract, without libespeak-ng and without a sound (the null output
//! consumes samples on a timer, as `TEXTWEAVER_ESPEAK_OUTPUT=virtual`
//! does).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use textweaver_core::{CharPos, Rate, Utterance, UtteranceId, UtteranceKind};
use textweaver_espeak::host::fake;
use textweaver_espeak::{AudioOutput, EspeakHostBackend, EspeakHostConfig};
use textweaver_speech::{Caps, EventSink, RawEvent, SpeechBackend, SpeechError, VoiceParams};

/// The recording sink: every event with the moment it arrived.
#[derive(Default)]
pub(crate) struct Rec {
    pub events: Vec<(UtteranceId, RawEvent, Instant)>,
}

impl EventSink for Rec {
    fn emit(&mut self, id: UtteranceId, event: RawEvent) {
        self.events.push((id, event, Instant::now()));
    }
    fn is_current(&self, _id: UtteranceId) -> bool {
        true
    }
}

impl Rec {
    pub fn of(&self, id: UtteranceId) -> Vec<RawEvent> {
        self.events
            .iter()
            .filter(|(i, ..)| *i == id)
            .map(|(_, e, _)| e.clone())
            .collect()
    }
    pub fn words(&self, id: UtteranceId) -> Vec<(std::ops::Range<u32>, u32)> {
        self.of(id)
            .into_iter()
            .filter_map(|e| match e {
                RawEvent::Word {
                    byte_range,
                    audio_ms,
                } => Some((byte_range, audio_ms.unwrap_or(u32::MAX))),
                _ => None,
            })
            .collect()
    }
    pub fn ended(&self, id: UtteranceId) -> bool {
        self.of(id)
            .iter()
            .any(|e| matches!(e, RawEvent::Finished | RawEvent::Cancelled))
    }
}

fn test_host() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_textweaver-espeak-host"))
}

fn config(speed: f32) -> EspeakHostConfig {
    EspeakHostConfig {
        host: Some(test_host()),
        output: AudioOutput::Null { speed },
        fake_engine: true,
        ..EspeakHostConfig::default()
    }
}

/// A backend at eSpeak NG's default rate, where the fake speaks 100
/// samples per byte.
fn backend(speed: f32) -> EspeakHostBackend {
    let mut b = EspeakHostBackend::new(config(speed)).expect("fake host starts");
    b.set_params(&VoiceParams {
        rate: Rate::Wpm(fake::DEFAULT_RATE),
        ..VoiceParams::default()
    })
    .unwrap();
    b
}

pub(crate) fn utt(text: &str, generation: u64) -> Utterance {
    let mut u = Utterance::literal(text, CharPos(0));
    u.id = UtteranceId {
        generation,
        chunk: 0,
    };
    u
}

/// Polls until `id` ends or ten seconds pass.
pub(crate) fn pump(b: &mut dyn SpeechBackend, rec: &mut Rec, id: UtteranceId) -> bool {
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(10) {
        b.poll(rec);
        if rec.ended(id) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    false
}

#[test]
fn speak_reports_started_each_word_at_its_sample_then_finished() {
    let mut b = backend(8.0);
    assert_eq!(b.engine(), Some("fake"));
    assert!(b.capabilities().contains(Caps::PLAYBACK_EVENTS));
    let mut rec = Rec::default();
    let text = "Hello brave new world";
    let u = utt(text, 1);
    b.speak(&u, &mut rec).unwrap();
    assert!(pump(&mut b, &mut rec, u.id));
    let ev = rec.of(u.id);
    assert_eq!(ev.first(), Some(&RawEvent::Started));
    assert_eq!(ev.last(), Some(&RawEvent::Finished));
    let words = rec.words(u.id);
    let spoken: Vec<&str> = words
        .iter()
        .map(|(r, _)| &text[r.start as usize..r.end as usize])
        .collect();
    assert_eq!(spoken, ["Hello", "brave", "new", "world"]);
    // Each word at its first sample: 100 samples a byte at 22,050 Hz.
    let ms: Vec<u32> = words.iter().map(|(_, ms)| *ms).collect();
    let want: Vec<u32> = [0u64, 6, 12, 16]
        .iter()
        .map(|b| u32::try_from(b * 100 * 1000 / 22_050).unwrap())
        .collect();
    assert_eq!(ms, want);
}

#[test]
fn voices_come_from_the_host_and_unknown_ones_are_refused() {
    let mut b = backend(8.0);
    let ids: Vec<String> = b.voices().unwrap().into_iter().map(|v| v.id).collect();
    assert_eq!(ids, ["fake/en", "fake/fr"]);
    let p = VoiceParams {
        voice: Some("nope".into()),
        ..VoiceParams::default()
    };
    assert!(matches!(
        b.set_params(&p),
        Err(SpeechError::UnknownVoice(_))
    ));
    let p = VoiceParams {
        voice: Some("fake/fr".into()),
        ..VoiceParams::default()
    };
    b.set_params(&p).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fr.wav");
    let s = b.synthesize_utterance(&utt("ab cd", 1), &path).unwrap();
    assert_eq!(s.words.len(), 2);
    let wav = std::fs::read(&path).unwrap();
    assert!(wav.len() > 44, "a WAV with samples");
    let first = i16::from_le_bytes([wav[44], wav[45]]);
    assert_eq!(first.abs(), 2000, "the French fake voice");
}

#[test]
fn stop_cancels_what_was_queued() {
    let mut b = backend(1.0);
    let mut rec = Rec::default();
    let long = utt(&"word ".repeat(200), 1);
    let next = utt("after", 2);
    b.speak(&long, &mut rec).unwrap();
    b.speak(&next, &mut rec).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    b.poll(&mut rec);
    b.stop();
    assert!(pump(&mut b, &mut rec, next.id));
    assert!(rec.of(long.id).contains(&RawEvent::Cancelled));
    assert!(rec.of(next.id).contains(&RawEvent::Cancelled));
    // The host still works.
    let again = utt("again", 3);
    b.speak(&again, &mut rec).unwrap();
    assert!(pump(&mut b, &mut rec, again.id));
    assert_eq!(rec.of(again.id).last(), Some(&RawEvent::Finished));
}

#[test]
fn an_engine_crash_fails_one_utterance_and_the_next_starts_a_new_helper() {
    let mut b = backend(8.0);
    let mut rec = Rec::default();
    let crash = utt("please __crash__ now", 1);
    b.speak(&crash, &mut rec).unwrap();
    assert!(pump(&mut b, &mut rec, crash.id));
    let ev = rec.of(crash.id);
    assert!(
        ev.iter()
            .any(|e| matches!(e, RawEvent::Error(m) if m.contains("eSpeak NG"))),
        "{ev:?}"
    );
    assert_eq!(ev.last(), Some(&RawEvent::Finished));
    let next = utt("still here", 2);
    b.speak(&next, &mut rec).unwrap();
    assert!(pump(&mut b, &mut rec, next.id));
    assert_eq!(rec.words(next.id).len(), 2);
    assert_eq!(rec.of(next.id).last(), Some(&RawEvent::Finished));
}

#[test]
fn a_hung_engine_is_killed_after_the_stall_timeout() {
    let mut b = EspeakHostBackend::new(EspeakHostConfig {
        stall_timeout: Some(Duration::from_millis(300)),
        ..config(8.0)
    })
    .unwrap();
    let mut rec = Rec::default();
    let hang = utt("__hang__", 1);
    b.speak(&hang, &mut rec).unwrap();
    assert!(pump(&mut b, &mut rec, hang.id));
    assert!(
        rec.of(hang.id)
            .iter()
            .any(|e| matches!(e, RawEvent::Error(m) if m.contains("stopped responding")))
    );
}

#[test]
fn a_single_character_is_sent_as_a_character() {
    let mut b = backend(8.0);
    let mut rec = Rec::default();
    let mut u = utt("x", 1);
    u.kind = UtteranceKind::Character;
    b.speak(&u, &mut rec).unwrap();
    assert!(pump(&mut b, &mut rec, u.id));
    assert_eq!(rec.of(u.id).last(), Some(&RawEvent::Finished));
}
