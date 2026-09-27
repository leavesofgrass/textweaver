//! Tests against a real Piper voice. Ignored unless run with `--ignored`
//! and `TEXTWEAVER_PIPER_VOICE` naming a voice's `.onnx` file (its
//! `.onnx.json` beside it). Nothing is played: audio stays in memory.
//!
//! ```text
//! TEXTWEAVER_PIPER_VOICE=/path/en_US-joe-medium.onnx \
//!   cargo test -p textweaver-piper --all-features --test real_voice -- --ignored --nocapture
//! ```
//!
//! They print the numbers `docs/adr/0023-in-process-neural-speech.md`
//! records: load time, time to first audio, real-time factor, and how
//! well the model's word timing matches the pauses in its own audio.

use std::path::PathBuf;
use std::time::Instant;

use textweaver_core::{Pitch, Rate};
use textweaver_piper::phonemes::{Phonemizer, PhonemizerChoice, phonemizer};
use textweaver_piper::text::words;
use textweaver_piper::{SynthParams, Synthesizer, Timing, measure};

/// Plain prose written for these tests (no outside text).
const PROSE: &str = "The library opens at nine in the morning, and it closes at six. \
Students may borrow up to twelve books at a time. If a book is late, the fine is small, \
but it adds up quickly. Quiet rooms on the second floor can be booked a week ahead; \
larger rooms need a teacher's signature. Ask at the front desk if you need help finding \
anything, whether it is a map, a newspaper, or an old recording.";

fn voice() -> Option<(PathBuf, PathBuf)> {
    let onnx = PathBuf::from(std::env::var_os("TEXTWEAVER_PIPER_VOICE")?);
    let json = PathBuf::from(format!("{}.json", onnx.display()));
    Some((onnx, json))
}

fn espeak_data() -> PathBuf {
    std::env::temp_dir().join("textweaver-piper-test-espeak-ng-data")
}

fn phonemizers() -> Vec<(&'static str, Box<dyn Phonemizer>)> {
    let mut out = Vec::new();
    for (name, choice) in [
        ("library", PhonemizerChoice::Library),
        ("rust", PhonemizerChoice::Rust),
    ] {
        match phonemizer(choice, &espeak_data()) {
            Ok(p) => out.push((name, p)),
            Err(e) => eprintln!("{name} phonemizer unavailable: {e}"),
        }
    }
    out
}

fn params(wpm: u16) -> SynthParams {
    SynthParams {
        rate: Rate::Wpm(wpm),
        pitch: Pitch::default(),
        speaker: 0,
    }
}

#[test]
#[ignore = "needs a Piper voice (TEXTWEAVER_PIPER_VOICE)"]
fn speed_and_first_audio() {
    let Some((onnx, json)) = voice() else { return };
    for (name, p) in phonemizers() {
        let m = measure(&onnx, &json, p, PROSE, &params(210)).unwrap();
        eprintln!(
            "{name} ({}): load {:?}, first audio {:?}, total {:?} for {:.1} s of audio, \
             real-time factor {:.3}, {:.0} wpm at length scale 1, timing {:?}",
            m.phonemizer,
            m.load,
            m.first_audio,
            m.total,
            m.audio.as_secs_f64(),
            m.real_time_factor(),
            m.wpm,
            m.timing
        );
        assert_eq!(m.timing, Timing::Model);
        assert!(m.real_time_factor() < 2.0);
    }
    // A warm model: the first chunk of a new utterance.
    let (_, p) = phonemizers().into_iter().next().unwrap();
    let mut s = Synthesizer::load(&onnx, &json, p).unwrap();
    s.speak_all("Warm up.", &params(265)).unwrap();
    for text in [
        "Next heading.",
        "Chapter three, in which the students find the old recording.",
    ] {
        let t = Instant::now();
        let mut first = None;
        s.speak(text, &params(265), |_| {
            first.get_or_insert(t.elapsed());
            true
        })
        .unwrap();
        eprintln!("warm first audio for {text:?}: {:?}", first.unwrap());
    }
}

/// 10 ms RMS energy frames of `samples` at `rate`.
fn energy(samples: &[i16], rate: u32) -> Vec<f64> {
    let n = (rate / 100) as usize;
    samples
        .chunks(n)
        .map(|c| {
            let s: f64 = c.iter().map(|&x| f64::from(x) * f64::from(x)).sum();
            (s / c.len() as f64).sqrt()
        })
        .collect()
}

#[test]
#[ignore = "needs a Piper voice (TEXTWEAVER_PIPER_VOICE)"]
fn word_timing_matches_the_audio() {
    let Some((onnx, json)) = voice() else { return };
    for (name, p) in phonemizers() {
        let mut s = Synthesizer::load(&onnx, &json, p).unwrap();
        let rate = s.sample_rate();
        let mut exact = 0usize;
        let mut chunks = 0usize;
        let mut timed = 0usize;
        let mut total_words = 0usize;
        let mut errors_ms: Vec<f64> = Vec::new();
        for wpm in [210u16, 265] {
            s.speak(PROSE, &params(wpm), |c| {
                chunks += 1;
                exact += usize::from(c.exact);
                timed += c.words.len();
                let e = energy(&c.samples, rate);
                let peak = e.iter().copied().fold(0.0, f64::max);
                let quiet = |i: usize| e.get(i).is_none_or(|&v| v < peak * 0.03);
                // A word that starts after a pause: its predicted start
                // should be where the sound comes back.
                for (_, sample) in &c.words {
                    let at = (*sample as usize) / (rate as usize / 100);
                    if at < 5 || !(1..=5).all(|k| quiet(at - k)) {
                        continue;
                    }
                    let onset = (at.saturating_sub(15)..at + 30).find(|&i| {
                        i >= 5 && (1..=5).all(|k| quiet(i - k)) && !quiet(i) && !quiet(i + 1)
                    });
                    if let Some(o) = onset {
                        errors_ms.push((o as f64 - at as f64) * 10.0);
                    }
                }
                true
            })
            .unwrap();
            total_words += words(PROSE).len();
        }
        let mean = errors_ms.iter().map(|e| e.abs()).sum::<f64>() / errors_ms.len().max(1) as f64;
        let worst = errors_ms.iter().map(|e| e.abs()).fold(0.0, f64::max);
        eprintln!(
            "{name}: {exact} of {chunks} chunks aligned word for word; {timed} of {total_words} \
             words timed; after pauses, predicted starts are {mean:.0} ms from the audio's \
             onsets on average (worst {worst:.0} ms, {} pauses)",
            errors_ms.len()
        );
        assert!(timed * 10 >= total_words * 9, "most words are timed");
        assert!(mean <= 60.0, "{errors_ms:?}");
    }
}

#[test]
#[ignore = "needs a Piper voice and libespeak-ng (TEXTWEAVER_PIPER_VOICE)"]
fn both_phonemizers_agree() {
    let (Ok(mut lib), Ok(mut rs)) = (
        phonemizer(PhonemizerChoice::Library, &espeak_data()),
        phonemizer(PhonemizerChoice::Rust, &espeak_data()),
    ) else {
        eprintln!("needs both phonemizers");
        return;
    };
    let clauses = [
        "The library opens at nine in the morning",
        "Students may borrow up to twelve books at a time",
        "Quiet rooms on the second floor can be booked a week ahead",
        "whether it is a map",
        "an old recording",
    ];
    let (mut same, mut same_sounds, mut chars, mut differ) = (0, 0, 0usize, 0usize);
    for c in clauses {
        let a = lib.phonemize("en-us", c).unwrap();
        let b = rs.phonemize("en-us", c).unwrap();
        eprintln!("{c}\n  lib: {a}\n  rs:  {b}");
        same += usize::from(a == b);
        // Word breaks aside (libespeak-ng joins "in the"), how many
        // phonemes differ.
        let (a, b): (Vec<char>, Vec<char>) = (
            a.chars().filter(|c| *c != ' ').collect(),
            b.chars().filter(|c| *c != ' ').collect(),
        );
        same_sounds += usize::from(a == b);
        chars += a.len().max(b.len());
        differ += edit_distance(&a, &b);
    }
    eprintln!(
        "{same} of {} clauses identical, {same_sounds} identical apart from word breaks; \
         {differ} of {chars} phonemes differ",
        clauses.len()
    );
    assert!(
        differ * 20 <= chars,
        "the phonemizers agree on 95% of phonemes"
    );
}

fn edit_distance(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(ca != cb);
            cur.push(sub.min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}
