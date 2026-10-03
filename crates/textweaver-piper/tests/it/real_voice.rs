//! Tests against a real Piper voice. Ignored unless run with `--ignored`
//! and `TEXTWEAVER_PIPER_VOICE` naming a voice's `.onnx` file (its
//! `.onnx.json` beside it). Nothing is played: audio stays in memory.
//!
//! ```text
//! TEXTWEAVER_PIPER_VOICE=/path/en_US-joe-medium.onnx \
//!   cargo test -p textweaver-piper --all-features --test it -- real_voice:: --ignored --nocapture
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

/// How the time to a piece's audio grows with its length, on a warm
/// model: the numbers behind the backend's first-piece split
/// (`docs/dev/testing.md`, "Piper's first audio").
#[test]
#[ignore = "needs a Piper voice (TEXTWEAVER_PIPER_VOICE)"]
fn first_audio_by_length() {
    use textweaver_piper::model::PiperModel;
    use textweaver_piper::phonemes::prepare;
    use textweaver_piper::text::clauses;

    let Some((onnx, json)) = voice() else { return };
    let (_, p) = phonemizers().into_iter().next().unwrap();
    let mut s = Synthesizer::load(&onnx, &json, p).unwrap();
    s.speak_all("Warm up, twice over.", &params(210)).unwrap();
    // The two steps on their own: phonemes, then one model run.
    let (_, mut p) = phonemizers().into_iter().next().unwrap();
    let model = PiperModel::load(&onnx).unwrap();
    let config = s.config().clone();
    let best_of = |f: &mut dyn FnMut()| {
        (0..5)
            .map(|_| {
                let t = Instant::now();
                f();
                t.elapsed()
            })
            .min()
            .unwrap_or_default()
            .as_millis()
    };
    // Commas taken out, so each text is one clause and one model run.
    let plain = PROSE.replace([',', ';', '.'], "");
    let all: Vec<&str> = plain.split_whitespace().collect();
    for n in [2usize, 4, 6, 8, 12, 16, 24, 32] {
        let text = all[..n].join(" ");
        let cs = clauses(&text);
        let mut audio_ms = 0u64;
        let total = best_of(&mut || {
            let c = s.speak_all(&text, &params(210)).unwrap();
            audio_ms = c.samples.len() as u64 * 1000 / u64::from(s.sample_rate());
        });
        let phonemes = best_of(&mut || {
            prepare(&config, p.as_mut(), &text, &cs).unwrap();
        });
        let ids = prepare(&config, p.as_mut(), &text, &cs).unwrap().ids;
        let run = best_of(&mut || {
            model.infer(&config, &ids, 1.0, 0).unwrap();
        });
        eprintln!(
            "{n:>2} words, {audio_ms:>5} ms of audio: {total:>4} ms to synthesize \
             (phonemes {phonemes} ms, model {run} ms; fastest of 5)"
        );
    }
}

/// Records which utterances have started (first audio), and when.
#[derive(Default)]
struct Starts(Vec<(textweaver_core::UtteranceId, Instant)>);

impl textweaver_speech::EventSink for Starts {
    fn emit(&mut self, id: textweaver_core::UtteranceId, event: textweaver_speech::RawEvent) {
        if event == textweaver_speech::RawEvent::Started {
            self.0.push((id, Instant::now()));
        }
    }
    fn is_current(&self, _id: textweaver_core::UtteranceId) -> bool {
        true
    }
}

/// The backend's stop-to-first-audio, as `cargo xtask bench --engine
/// piper` measures it through the whole application, here on the backend
/// alone: read three sentences (the playing one and the service's two of
/// lookahead), listen for 30 ms after the first audio, Stop, and read
/// from the next sentence. Played to a silent output in real time.
#[test]
#[ignore = "needs a Piper voice (TEXTWEAVER_PIPER_VOICE)"]
fn stop_to_first_audio() {
    use textweaver_core::{CharPos, Utterance, UtteranceId};
    use textweaver_enginehost::AudioOutput;
    use textweaver_piper::{PiperBackend, PiperConfig};
    use textweaver_speech::SpeechBackend;

    let Some((onnx, _)) = voice() else { return };
    let tmp = tempfile::tempdir().unwrap();
    let mut config = PiperConfig::in_data_dir(tmp.path());
    config.voices_dir = onnx.parent().unwrap().to_owned();
    config.espeak_data = espeak_data();
    config.output = AudioOutput::Null { speed: 1.0 };
    config.default_voice = onnx.file_stem().and_then(|s| s.to_str()).map(str::to_owned);
    let mut b = PiperBackend::new(config).unwrap();
    let mut sink = Starts::default();
    let mut generation = 0u64;
    let mut restarts = |label: &str, texts: &dyn Fn(usize, usize) -> String| {
        let mut times = Vec::new();
        for round in 0..12 {
            b.stop();
            generation += 1;
            let t0 = Instant::now();
            for k in 0..3 {
                let mut u = Utterance::literal(texts(round, k), CharPos(0));
                u.id = UtteranceId {
                    generation,
                    chunk: k as u32,
                };
                b.speak(&u, &mut sink).unwrap();
            }
            let deadline = t0 + std::time::Duration::from_secs(20);
            let started = loop {
                b.poll(&mut sink);
                if let Some((_, at)) = sink.0.iter().find(|(id, _)| id.generation == generation) {
                    break *at;
                }
                assert!(Instant::now() < deadline, "no audio in 20 s");
                std::thread::sleep(std::time::Duration::from_millis(1));
            };
            // The first round warms the model: not counted.
            if round > 0 {
                times.push(started - t0);
            }
            std::thread::sleep(std::time::Duration::from_millis(30));
            b.poll(&mut sink);
        }
        times.sort();
        let ms = |d: std::time::Duration| d.as_millis();
        eprintln!(
            "stop to first audio, {label}, {} restarts: median {} ms, fastest {} ms, slowest {} ms",
            times.len(),
            ms(times[times.len() / 2]),
            ms(times[0]),
            ms(times[times.len() - 1])
        );
    };
    // Each restart reads text never spoken before (nothing cached).
    restarts("new text each time", &|round, k| {
        let i = round * 3 + k;
        format!(
            "Room {i} on the second floor holds {} chairs and a long table by the window.",
            i * 7 + 3
        )
    });
    // Each restart reads from the next sentence: it was handed over as
    // lookahead before the Stop, as "next sentence" while reading does.
    let sentences: Vec<&str> = PROSE.split_inclusive(". ").map(str::trim).collect();
    restarts("the next sentence", &|round, k| {
        sentences[(round + k) % sentences.len()].to_owned()
    });
}

/// The pacing never lets the audio run dry: a reading of six sentences,
/// played in real time to a silent output, takes as long as its audio,
/// plus the first piece's synthesis. A gap would add to the wall time.
#[test]
#[ignore = "needs a Piper voice (TEXTWEAVER_PIPER_VOICE)"]
fn a_paced_reading_has_no_gaps() {
    use textweaver_core::{CharPos, Utterance, UtteranceId};
    use textweaver_enginehost::AudioOutput;
    use textweaver_piper::{PiperBackend, PiperConfig};
    use textweaver_speech::{EventSink, RawEvent, SpeechBackend};

    #[derive(Default)]
    struct Ends(Vec<(UtteranceId, RawEvent, Instant)>);
    impl EventSink for Ends {
        fn emit(&mut self, id: UtteranceId, event: RawEvent) {
            if matches!(event, RawEvent::Started | RawEvent::Finished) {
                self.0.push((id, event, Instant::now()));
            }
        }
        fn is_current(&self, _id: UtteranceId) -> bool {
            true
        }
    }

    let Some((onnx, json)) = voice() else { return };
    let tmp = tempfile::tempdir().unwrap();
    let mut config = PiperConfig::in_data_dir(tmp.path());
    config.voices_dir = onnx.parent().unwrap().to_owned();
    config.espeak_data = espeak_data();
    config.output = AudioOutput::Null { speed: 1.0 };
    config.default_voice = onnx.file_stem().and_then(|s| s.to_str()).map(str::to_owned);
    let sentences: Vec<&str> = PROSE.split_inclusive(". ").map(str::trim).collect();
    let texts: Vec<&str> = sentences.iter().chain(&sentences[..1]).copied().collect();
    // The audio's length, synthesized apart (the noise makes each run a
    // little different, so it is compared with a margin).
    let (_, p) = phonemizers().into_iter().next().unwrap();
    let mut s = Synthesizer::load(&onnx, &json, p).unwrap();
    let audio_ms: u64 = texts
        .iter()
        .map(|t| {
            // The backend's default rate.
            let c = s.speak_all(t, &params(Rate::DEFAULT_WPM)).unwrap();
            c.samples.len() as u64 * 1000 / u64::from(s.sample_rate())
        })
        .sum();
    drop(s);
    let mut b = PiperBackend::new(config).unwrap();
    let mut sink = Ends::default();
    // Warm up, as the first reading after loading is slower.
    b.speak(&Utterance::literal("Warm up.", CharPos(0)), &mut sink)
        .unwrap();
    let deadline = Instant::now() + std::time::Duration::from_secs(30);
    while !sink.0.iter().any(|(_, e, _)| *e == RawEvent::Finished) {
        assert!(Instant::now() < deadline);
        b.poll(&mut sink);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    sink.0.clear();
    let t0 = Instant::now();
    for (k, t) in texts.iter().enumerate() {
        let mut u = Utterance::literal(*t, CharPos(0));
        u.id = UtteranceId {
            generation: 1,
            chunk: k as u32,
        };
        b.speak(&u, &mut sink).unwrap();
    }
    let last = UtteranceId {
        generation: 1,
        chunk: texts.len() as u32 - 1,
    };
    let deadline = t0 + std::time::Duration::from_millis(audio_ms * 2 + 10_000);
    while !sink
        .0
        .iter()
        .any(|(id, e, _)| *id == last && *e == RawEvent::Finished)
    {
        assert!(Instant::now() < deadline, "the reading did not finish");
        b.poll(&mut sink);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let first = sink
        .0
        .iter()
        .find(|(_, e, _)| *e == RawEvent::Started)
        .unwrap()
        .2;
    let end = sink.0.last().unwrap().2;
    let heard = (end - first).as_millis() as u64;
    eprintln!(
        "a paced reading: {audio_ms} ms of audio heard in {heard} ms (first audio after {} ms)",
        (first - t0).as_millis()
    );
    // 3 percent for the noise, and 600 ms for the comma pauses of the
    // first phrases (the backend speaks a reading's start a phrase at a
    // time; the reference above, a sentence at a time) and the polling.
    assert!(
        heard <= audio_ms * 103 / 100 + 600,
        "a gap: {heard} ms for {audio_ms} ms of audio"
    );
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
