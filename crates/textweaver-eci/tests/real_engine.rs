//! Tests against a real, licensed ETI-Eloquence engine. They are
//! `#[ignore]`d and additionally do nothing unless `TEXTWEAVER_ECI=1`:
//!
//! ```text
//! TEXTWEAVER_ECI=1 cargo test -p textweaver-eci -- --ignored
//! ```
//!
//! Linux: Voxin (`TEXTWEAVER_ECI_LIBRARY`, set by `compose.voxin.yaml` in the
//! dev container). Windows: Code Factory's `eci.dll` through the 32-bit host
//! (`cargo xtask eci-host`); these need a licensed Code Factory engine and
//! have not been run on Windows yet.
//!
//! They never play audio: output goes to the null sink or to a file.

use std::time::{Duration, Instant};

use textweaver_core::{CharPos, Rate, Utterance, UtteranceId};
use textweaver_eci::{AudioOutput, EciBackend, EciConfig};
use textweaver_speech::{EventSink, RawEvent, SpeechBackend, VoiceParams};

/// The feasibility spike's sentence.
const SPIKE: &str = "Dr. Smith opened the library at 9:30 a.m. Café crème, naïve résumé.";

fn enabled() -> bool {
    let on = std::env::var("TEXTWEAVER_ECI").is_ok_and(|v| v == "1");
    if !on {
        eprintln!("skipped: set TEXTWEAVER_ECI=1 to run against the installed engine");
    }
    on
}

fn backend(speed: f32) -> EciBackend {
    EciBackend::new(EciConfig {
        output: AudioOutput::Null { speed },
        ..EciConfig::default()
    })
    .expect("the ECI engine starts (is it installed and licensed?)")
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
#[ignore = "needs a licensed ETI-Eloquence engine; run with TEXTWEAVER_ECI=1"]
fn spike_sentence_to_wav_marks_every_word_with_rising_offsets() {
    if !enabled() {
        return;
    }
    let mut b = backend(1.0);
    println!(
        "engine ECI {} at {} Hz via {}",
        b.engine_version().unwrap_or("?"),
        b.sample_rate().unwrap_or(0),
        b.host_path()
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    );
    let s = b.synthesize(SPIKE).unwrap();
    let rate = u64::from(s.sample_rate);
    for (r, sample) in &s.words {
        println!(
            "{:>6} ms  {}",
            sample * 1000 / rate,
            &SPIKE[r.start as usize..r.end as usize]
        );
    }
    println!("total {} ms", s.samples.len() as u64 * 1000 / rate);
    let expected = textweaver_eci::words::words(SPIKE).len();
    assert_eq!(s.words.len(), expected, "one mark per word");
    for pair in s.words.windows(2) {
        assert!(pair[0].1 < pair[1].1, "offsets rise: {pair:?}");
    }
    assert!(*s.words.last().map(|(_, at)| at).unwrap() < s.samples.len() as u64);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("spike.wav");
    b.synthesize_to_file(SPIKE, &path).unwrap();
    let (r, samples) = textweaver_eci::wav::read_wav(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(r, s.sample_rate);
    assert_eq!(samples.len(), s.samples.len());
}

#[test]
#[ignore = "needs a licensed ETI-Eloquence engine; run with TEXTWEAVER_ECI=1"]
fn speak_plays_on_the_clock_and_stop_cancels() {
    if !enabled() {
        return;
    }
    let mut b = backend(4.0);
    let mut rec = Rec::default();
    let mut u = Utterance::literal(SPIKE, CharPos(0));
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
    let words: Vec<u32> = rec
        .0
        .iter()
        .filter_map(|(_, e)| match e {
            RawEvent::Word { audio_ms, .. } => *audio_ms,
            _ => None,
        })
        .collect();
    assert_eq!(rec.0.first().map(|e| &e.1), Some(&RawEvent::Started));
    assert_eq!(words.len(), textweaver_eci::words::words(SPIKE).len());
    assert!(words.windows(2).all(|w| w[0] < w[1]), "{words:?}");

    // Stop partway through a long utterance.
    let long = "The quick brown fox jumps over the lazy dog. ".repeat(20);
    let mut v = Utterance::literal(long, CharPos(0));
    v.id = UtteranceId {
        generation: 2,
        chunk: 0,
    };
    b.speak(&v, &mut rec).unwrap();
    let t0 = Instant::now();
    while rec
        .0
        .iter()
        .filter(|(i, e)| *i == v.id && matches!(e, RawEvent::Word { .. }))
        .count()
        < 3
    {
        assert!(t0.elapsed() < Duration::from_secs(20));
        b.poll(&mut rec);
        std::thread::sleep(Duration::from_millis(2));
    }
    b.stop();
    b.poll(&mut rec);
    let n = rec.0.len();
    std::thread::sleep(Duration::from_millis(300));
    b.poll(&mut rec);
    assert_eq!(rec.0.len(), n, "no events after Cancelled");
    assert_eq!(rec.0.last().map(|e| &e.1), Some(&RawEvent::Cancelled));
}

#[test]
#[ignore = "needs a licensed ETI-Eloquence engine; run with TEXTWEAVER_ECI=1"]
fn voices_include_reed_and_rate_changes_duration() {
    if !enabled() {
        return;
    }
    let mut b = backend(1.0);
    let voices = b.voices().unwrap();
    for v in &voices {
        println!("{:<16} {}", v.id, v.name);
    }
    assert!(voices.iter().any(|v| v.id == "eci:enu:reed"));
    let text = "Reading quickly is a skill that improves with practice.";
    let slow = {
        b.set_params(&VoiceParams {
            rate: Rate::Wpm(150),
            ..VoiceParams::default()
        })
        .unwrap();
        b.synthesize(text).unwrap()
    };
    let fast = {
        b.set_params(&VoiceParams {
            rate: Rate::Wpm(400),
            voice: Some("eci:enu:shelley".into()),
            ..VoiceParams::default()
        })
        .unwrap();
        b.synthesize(text).unwrap()
    };
    println!(
        "150 wpm: {} samples; 400 wpm (Shelley): {} samples",
        slow.samples.len(),
        fast.samples.len()
    );
    assert!(fast.samples.len() * 2 < slow.samples.len());
}

#[test]
#[ignore = "needs a licensed ETI-Eloquence engine; run with TEXTWEAVER_ECI=1"]
fn windows_1252_text_reaches_the_engine_as_accented_letters() {
    if !enabled() {
        return;
    }
    // "résumé" (rez-oo-may) and "resume" (rih-zoom) sound different, so
    // their durations differ when the engine reads Windows-1252 0xE9 as
    // "é". The UTF-8 bytes of "résumé" (sent here as the Windows-1252
    // characters "Ã©") must not be read as "é": if they were, the engine
    // would be decoding UTF-8 and our encoding would be wrong.
    let mut b = backend(1.0);
    let accented = b.synthesize("résumé").unwrap().samples.len();
    let plain = b.synthesize("resume").unwrap().samples.len();
    let utf8_bytes = b.synthesize("rÃ©sumÃ©").unwrap().samples.len();
    println!("résumé {accented}, resume {plain}, UTF-8 bytes of résumé {utf8_bytes} samples");
    assert_ne!(accented, plain, "é was not read as an accented letter");
    assert_ne!(utf8_bytes, accented, "the engine decoded UTF-8");
    assert!(accented < plain * 3 / 2, "accented letters read as symbols");
}

#[test]
#[ignore = "needs a licensed ETI-Eloquence engine; run with TEXTWEAVER_ECI=1"]
fn text_ending_in_an_accented_letter_finishes() {
    if !enabled() {
        return;
    }
    // libvoxin 1.5.8 never finished input whose last byte was a Latin-1
    // letter; the host now always ends the text with a space.
    let mut b = backend(1.0);
    for text in ["café", "a é", "résumé", "Ü"] {
        // Finishing at all is the point (a lone "Ü" is silent in Voxin).
        let s = b.synthesize(text).unwrap();
        println!("{text:?}: {} samples", s.samples.len());
        assert_eq!(s.words.len(), textweaver_eci::words::words(text).len());
        if text != "Ü" {
            assert!(!s.samples.is_empty(), "{text}");
        }
    }
}
