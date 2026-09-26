//! Tests against a real, licensed DECtalk the user installed. They are
//! `#[ignore]`d and additionally do nothing unless `TEXTWEAVER_DECTALK=1`:
//!
//! ```text
//! cargo xtask hosts
//! TEXTWEAVER_DECTALK=1 cargo test -p textweaver-dectalk --test real_engine -- --ignored --nocapture
//! ```
//!
//! DECtalk is found as the backend finds it (`TEXTWEAVER_DECTALK_LIBRARY`,
//! then the usual install folders). Never point these tests at the
//! community DECtalk source build: textweaver does not test against it
//! (ADR-0021). They never play audio: output goes to the null sink or to
//! files in a temporary folder, which are deleted.

use std::time::{Duration, Instant};

use textweaver_core::{CharPos, Rate, Utterance, UtteranceId};
use textweaver_dectalk::{AudioOutput, DectalkBackend, DectalkConfig};
use textweaver_speech::{EventSink, RawEvent, SpeechBackend, VoiceParams};

const SENTENCE: &str = "Dr. Smith opened the library at 9:30 a.m. Café crème, naïve résumé.";

fn enabled() -> bool {
    let on = std::env::var("TEXTWEAVER_DECTALK").is_ok_and(|v| v == "1");
    if !on {
        eprintln!("skipped: set TEXTWEAVER_DECTALK=1 to run against the installed DECtalk");
    }
    on
}

fn backend(speed: f32) -> DectalkBackend {
    DectalkBackend::new(DectalkConfig {
        output: AudioOutput::Null { speed },
        ..DectalkConfig::default()
    })
    .expect("DECtalk starts (is it installed and licensed, and were the hosts built?)")
}

#[derive(Default)]
struct Rec(Vec<(UtteranceId, RawEvent)>);

impl EventSink for Rec {
    fn emit(&mut self, id: UtteranceId, event: RawEvent) {
        self.0.push((id, event));
    }
    fn is_current(&self, _id: UtteranceId) -> bool {
        true
    }
}

#[test]
#[ignore = "needs a licensed DECtalk; run with TEXTWEAVER_DECTALK=1"]
fn a_sentence_to_wav_marks_every_word_with_rising_offsets() {
    if !enabled() {
        return;
    }
    let mut b = backend(1.0);
    println!(
        "{} via {}",
        b.library().map_or("?", |c| c.reason.as_str()),
        b.host_path()
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sentence.wav");
    let mut u = Utterance::literal(SENTENCE, CharPos(0));
    u.id = UtteranceId {
        generation: 1,
        chunk: 0,
    };
    let fs = b.synthesize_utterance(&u, &path).unwrap();
    let (rate, samples) =
        textweaver_dectalk::wav::read_wav(&std::fs::read(&path).unwrap()).unwrap();
    let duration_ms = samples.len() as u64 * 1000 / u64::from(rate);
    for w in &fs.words {
        println!(
            "{:>6} ms  {}",
            w.audio_ms,
            &SENTENCE[w.byte_range.start as usize..w.byte_range.end as usize]
        );
    }
    println!("file {duration_ms} ms at {rate} Hz");
    assert_eq!(
        fs.words.len(),
        textweaver_dectalk::words::words(SENTENCE).len(),
        "one mark per word"
    );
    assert!(fs.words.windows(2).all(|w| w[0].audio_ms < w[1].audio_ms));
    assert!(fs.words.iter().all(|w| u64::from(w.audio_ms) < duration_ms));
}

#[test]
#[ignore = "needs a licensed DECtalk; run with TEXTWEAVER_DECTALK=1"]
fn every_speaker_speaks_and_rate_changes_the_length() {
    if !enabled() {
        return;
    }
    let mut b = backend(1.0);
    for v in b.voices().unwrap() {
        b.set_params(&VoiceParams {
            voice: Some(v.id.clone()),
            ..VoiceParams::default()
        })
        .unwrap();
        let s = b.synthesize("Hello from DECtalk.").unwrap();
        println!("{}: {} samples", v.name, s.samples.len());
        assert!(!s.samples.is_empty(), "{}", v.name);
    }
    let len_at = |b: &mut DectalkBackend, wpm| {
        b.set_params(&VoiceParams {
            rate: Rate::Wpm(wpm),
            ..VoiceParams::default()
        })
        .unwrap();
        b.synthesize(SENTENCE).unwrap().samples.len()
    };
    let slow = len_at(&mut b, 150);
    let fast = len_at(&mut b, 400);
    println!("150 wpm: {slow} samples; 400 wpm: {fast} samples");
    assert!(fast * 3 / 2 < slow);
}

#[test]
#[ignore = "needs a licensed DECtalk; run with TEXTWEAVER_DECTALK=1"]
fn speak_plays_on_the_clock_and_stop_cancels() {
    if !enabled() {
        return;
    }
    let mut b = backend(4.0);
    let mut rec = Rec::default();
    let mut u = Utterance::literal(SENTENCE, CharPos(0));
    u.id = UtteranceId {
        generation: 1,
        chunk: 0,
    };
    b.speak(&u, &mut rec).unwrap();
    let t0 = Instant::now();
    while !rec.0.iter().any(|(_, e)| *e == RawEvent::Finished) {
        assert!(t0.elapsed() < Duration::from_secs(20), "{:?}", rec.0);
        b.poll(&mut rec);
        std::thread::sleep(Duration::from_millis(2));
    }
    let words = rec
        .0
        .iter()
        .filter(|(_, e)| matches!(e, RawEvent::Word { .. }))
        .count();
    assert_eq!(words, textweaver_dectalk::words::words(SENTENCE).len());

    let long = "The quick brown fox jumps over the lazy dog. ".repeat(10);
    let mut v = Utterance::literal(long, CharPos(0));
    v.id = UtteranceId {
        generation: 2,
        chunk: 0,
    };
    b.speak(&v, &mut rec).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    b.poll(&mut rec);
    b.stop();
    b.poll(&mut rec);
    assert_eq!(
        rec.0.iter().rev().find(|(id, _)| *id == v.id).map(|e| &e.1),
        Some(&RawEvent::Cancelled)
    );
}
