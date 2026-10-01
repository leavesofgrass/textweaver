//! Real SAPI voices. Ignored unless `TEXTWEAVER_SAPI=1`; run with both
//! hosts built:
//!
//! ```text
//! cargo build -p textweaver-sapi --bins --target i686-pc-windows-msvc --no-default-features
//! TEXTWEAVER_SAPI=1 cargo test -p textweaver-sapi --test it -- real_voices:: --ignored --nocapture
//! ```
//!
//! Only Microsoft voices and eSpeak are used (never the VW voices, which
//! belong to another product, and never Eloquence, which is not licensed on
//! the development machine). Nothing is played aloud: audio goes to WAV
//! files in a temporary directory or to the silent output.

#![cfg(windows)]

use std::ops::Range;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use textweaver_core::{CharPos, Utterance, UtteranceId};
use textweaver_sapi::voices::{Arch, Family, SapiVoice};
use textweaver_sapi::{AudioOutput, SapiBackend, SapiConfig, Synthesis};
use textweaver_speech::{EventSink, RawEvent, SpeechBackend, VoiceParams};

const SENTENCE: &str = "Dr. Smith opened the library at 9:30 a.m. Café crème, naïve résumé.";

/// Voices the tests may load, by exact name and architecture.
const ALLOWED: &[(&str, Arch)] = &[
    ("Microsoft David Desktop", Arch::X64),
    ("Microsoft Zira Desktop", Arch::X64),
    ("Microsoft David", Arch::X64),
    ("Microsoft Mark", Arch::X64),
    ("Microsoft Zira", Arch::X64),
    ("eSpeak-en", Arch::X86),
    ("eSpeak-en-us", Arch::X86),
    ("eSpeak-en+f2", Arch::X86),
];

fn enabled() -> bool {
    if std::env::var("TEXTWEAVER_SAPI").as_deref() == Ok("1") {
        true
    } else {
        eprintln!("skipped: set TEXTWEAVER_SAPI=1 to use real SAPI voices");
        false
    }
}

fn config() -> SapiConfig {
    SapiConfig {
        host_x64: Some(PathBuf::from(env!("CARGO_BIN_EXE_textweaver-sapi-host"))),
        output: AudioOutput::Null { speed: 4.0 },
        ..SapiConfig::default()
    }
}

/// The token name (without the listing's annotations) of a voice.
fn token_name(v: &SapiVoice) -> &str {
    v.voice
        .name
        .split(" (")
        .next()
        .unwrap_or(v.voice.name.as_str())
}

fn allowed(b: &SapiBackend) -> Vec<SapiVoice> {
    let all = b.voice_details().expect("voices");
    ALLOWED
        .iter()
        .filter_map(|(name, arch)| {
            all.iter()
                .find(|v| token_name(v) == *name && v.arch == *arch)
                .cloned()
        })
        .collect()
}

fn voice(b: &SapiBackend, name: &str) -> SapiVoice {
    assert!(
        ALLOWED.iter().any(|(n, _)| *n == name),
        "{name} not allowed"
    );
    allowed(b)
        .into_iter()
        .find(|v| token_name(v) == name)
        .unwrap_or_else(|| panic!("{name} is not installed"))
}

fn select(b: &mut SapiBackend, v: &SapiVoice) {
    b.set_params(&VoiceParams {
        voice: Some(v.voice.id.clone()),
        ..VoiceParams::default()
    })
    .expect("voice selects");
}

/// Every whitespace-separated token of `text` overlaps a reported word,
/// the words move forward, and their samples rise.
fn check_words(text: &str, s: &Synthesis) {
    assert!(!s.samples.is_empty());
    let words: &[(Range<u32>, u64)] = &s.words;
    assert!(
        words.windows(2).all(|w| w[0].0.start <= w[1].0.start),
        "{words:?}"
    );
    assert!(words.windows(2).all(|w| w[0].1 < w[1].1), "{words:?}");
    let mut at = 0usize;
    for token in text.split_whitespace() {
        let start = at + text[at..].find(token).unwrap();
        let end = start + token.len();
        at = end;
        assert!(
            words
                .iter()
                .any(|(r, _)| (r.start as usize) < end && (r.end as usize) > start),
            "no word event covers {token:?}: {words:?}"
        );
    }
}

fn show(text: &str, s: &Synthesis) -> String {
    s.words
        .iter()
        .map(|(r, sample)| {
            format!(
                "{} {}ms",
                &text[r.start as usize..r.end as usize],
                sample * 1000 / u64::from(s.sample_rate)
            )
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

#[test]
#[ignore = "needs real SAPI voices: TEXTWEAVER_SAPI=1"]
fn david_in_the_64_bit_host_reports_every_word() {
    if !enabled() {
        return;
    }
    let mut b = SapiBackend::new(config()).unwrap();
    let david = voice(&b, "Microsoft David Desktop");
    assert_eq!(david.arch, Arch::X64);
    select(&mut b, &david);
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("david.wav");
    b.synthesize_to_file(SENTENCE, &wav).unwrap();
    let bytes = std::fs::metadata(&wav).unwrap().len();
    let s = b.synthesize(SENTENCE).unwrap();
    println!("David (x64): wav {bytes} bytes; {}", show(SENTENCE, &s));
    // The file holds the same speech (synthesis length varies by a few
    // samples from run to run).
    let data = (bytes - 44) / 2;
    assert!(
        data.abs_diff(s.samples.len() as u64) < 100,
        "{data} vs {}",
        s.samples.len()
    );
    check_words(SENTENCE, &s);
    // "9:30 a.m." arrives four times from David and is reported once.
    let nine: Vec<_> = s
        .words
        .iter()
        .filter(|(r, _)| &SENTENCE[r.start as usize..r.end as usize] == "9:30 a.m.")
        .collect();
    assert_eq!(nine.len(), 1, "{:?}", s.words);
}

#[test]
#[ignore = "needs real SAPI voices: TEXTWEAVER_SAPI=1"]
fn david_exports_word_timings_on_the_files_clock() {
    if !enabled() {
        return;
    }
    let mut b = SapiBackend::new(config()).unwrap();
    let david = voice(&b, "Microsoft David Desktop");
    select(&mut b, &david);
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("david-utterance.wav");
    let mut u = Utterance::literal(SENTENCE, CharPos(0));
    u.id = UtteranceId {
        generation: 1,
        chunk: 0,
    };
    let fs = b.synthesize_utterance(&u, &wav).unwrap();
    let bytes = std::fs::metadata(&wav).unwrap().len();
    let duration_ms = (bytes - 44) / 2 * 1000 / 22050;
    let shown: Vec<String> = fs
        .words
        .iter()
        .map(|w| {
            format!(
                "{} {}ms",
                &SENTENCE[w.byte_range.start as usize..w.byte_range.end as usize],
                w.audio_ms
            )
        })
        .collect();
    println!("David export ({duration_ms} ms): {}", shown.join(" | "));
    assert!(fs.words.len() >= 10, "{shown:?}");
    assert!(fs.words.windows(2).all(|w| w[0].audio_ms < w[1].audio_ms));
    assert!(fs.words.iter().all(|w| u64::from(w.audio_ms) < duration_ms));
}

#[test]
#[ignore = "needs real SAPI voices: TEXTWEAVER_SAPI=1"]
fn espeak_in_the_32_bit_host_reports_every_word() {
    if !enabled() {
        return;
    }
    let mut b = SapiBackend::new(config()).unwrap();
    let espeak = voice(&b, "eSpeak-en");
    assert_eq!(espeak.arch, Arch::X86);
    assert_eq!(espeak.family, Family::Espeak);
    select(&mut b, &espeak);
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("espeak.wav");
    b.synthesize_to_file(SENTENCE, &wav).unwrap();
    assert!(b.host_path(Arch::X86).is_some());
    let s = b.synthesize(SENTENCE).unwrap();
    println!(
        "eSpeak-en (x86, {}): {}",
        b.host_path(Arch::X86).unwrap().display(),
        show(SENTENCE, &s)
    );
    check_words(SENTENCE, &s);
    // eSpeak's sub-token ranges pass through.
    let texts: Vec<&str> = s
        .words
        .iter()
        .map(|(r, _)| &SENTENCE[r.start as usize..r.end as usize])
        .collect();
    assert!(texts.contains(&"Dr") && texts.contains(&"30"), "{texts:?}");
}

#[test]
#[ignore = "needs real SAPI voices: TEXTWEAVER_SAPI=1"]
fn every_allowed_voice_reports_word_timing() {
    if !enabled() {
        return;
    }
    let mut b = SapiBackend::new(config()).unwrap();
    let voices = allowed(&b);
    println!("{} allowed voices installed", voices.len());
    let mut failures = Vec::new();
    for v in voices {
        select(&mut b, &v);
        let t0 = Instant::now();
        match b.synthesize(SENTENCE) {
            Ok(s) => {
                println!(
                    "{:<34} {:>3} words, {:>5} ms audio, synthesized in {:>4} ms: {}",
                    v.voice.name,
                    s.words.len(),
                    s.samples.len() as u64 * 1000 / u64::from(s.sample_rate),
                    t0.elapsed().as_millis(),
                    show(SENTENCE, &s)
                );
                check_words(SENTENCE, &s);
            }
            Err(e) => {
                println!("{:<34} FAILED: {e}", v.voice.name);
                failures.push(v.voice.name.clone());
            }
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
#[ignore = "needs real SAPI voices: TEXTWEAVER_SAPI=1"]
fn onecore_voices_load_through_sapi5() {
    if !enabled() {
        return;
    }
    let mut b = SapiBackend::new(config()).unwrap();
    let mark = voice(&b, "Microsoft Mark");
    assert!(mark.voice.name.contains("OneCore"), "{}", mark.voice.name);
    select(&mut b, &mark);
    let s = b.synthesize(SENTENCE).unwrap();
    println!("Mark (OneCore): {}", show(SENTENCE, &s));
    check_words(SENTENCE, &s);
}

/// The export path (`synthesize_utterance`) with several real voices in one
/// backend: each chosen voice reaches the host. Measured on Tuesday,
/// September 29, 2026 (Windows 11): Microsoft David Desktop and Microsoft
/// David (OneCore) write the same audio, byte for byte, while SAPI reports
/// the OneCore token as selected; Zira Desktop and Zira (OneCore) differ,
/// and Mark differs from both Davids. So the two David tokens speak with
/// the same voice data, and a check that voices differ must not pair them.
#[test]
#[ignore = "needs real SAPI voices: TEXTWEAVER_SAPI=1"]
fn chosen_voices_reach_the_host_in_the_export_path() {
    if !enabled() {
        return;
    }
    let mut b = SapiBackend::new(config()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut export = |name: &str| {
        let v = voice(&b, name);
        select(&mut b, &v);
        let path = dir.path().join(format!("{name}.wav"));
        let u = Utterance::literal(SENTENCE, CharPos(0));
        let fs = b.synthesize_utterance(&u, &path).unwrap();
        assert!(!fs.words.is_empty(), "{name}: no word times");
        std::fs::read(&path).unwrap()
    };
    let david = export("Microsoft David Desktop");
    let zira = export("Microsoft Zira Desktop");
    let mark = export("Microsoft Mark");
    let david_again = export("Microsoft David Desktop");
    // Compared without printing: the files are large.
    assert!(david != zira, "Zira Desktop spoke as David Desktop");
    assert!(david != mark, "Mark (OneCore) spoke as David Desktop");
    assert!(zira != mark, "Mark (OneCore) spoke as Zira Desktop");
    // A real engine need not repeat itself byte for byte in one process,
    // so the voice chosen again is checked against the other two.
    assert!(david_again != mark, "David Desktop stayed Mark");
    assert!(david_again != zira, "David Desktop stayed Zira");
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
#[ignore = "needs real SAPI voices: TEXTWEAVER_SAPI=1"]
fn speaking_and_stopping_a_real_voice() {
    if !enabled() {
        return;
    }
    let mut b = SapiBackend::new(config()).unwrap();
    let zira = voice(&b, "Microsoft Zira Desktop");
    select(&mut b, &zira);
    let mut rec = Rec::default();
    let mut u = Utterance::literal(SENTENCE, CharPos(0));
    u.id = UtteranceId {
        generation: 1,
        chunk: 0,
    };
    let t0 = Instant::now();
    b.speak(&u, &mut rec).unwrap();
    let mut first_word = None;
    while t0.elapsed() < Duration::from_secs(20)
        && !rec.0.iter().any(|(_, e)| *e == RawEvent::Finished)
    {
        b.poll(&mut rec);
        if first_word.is_none()
            && rec
                .0
                .iter()
                .any(|(_, e)| matches!(e, RawEvent::Word { .. }))
        {
            first_word = Some(t0.elapsed());
        }
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
    println!("Zira: first word event after {first_word:?}; audio_ms {words:?}");
    assert_eq!(rec.0.first().map(|(_, e)| e), Some(&RawEvent::Started));
    assert_eq!(rec.0.last().map(|(_, e)| e), Some(&RawEvent::Finished));
    assert!(words.windows(2).all(|w| w[0] < w[1]));
    assert!(words.len() >= 11);

    // Stop mid-utterance: Cancelled, and nothing after it.
    let mut rec = Rec::default();
    u.id.chunk = 1;
    b.speak(&u, &mut rec).unwrap();
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(10)
        && rec
            .0
            .iter()
            .filter(|(_, e)| matches!(e, RawEvent::Word { .. }))
            .count()
            < 3
    {
        b.poll(&mut rec);
        std::thread::sleep(Duration::from_millis(2));
    }
    b.stop();
    let n = rec.0.len();
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_millis(500) {
        b.poll(&mut rec);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(&rec.0[n..], &[(u.id, RawEvent::Cancelled)]);
}

/// Words per minute of `s` for a text of `words` words, from the first
/// word to the end of the audio.
fn wpm(s: &Synthesis, words: usize) -> f64 {
    let first = s.words.first().map_or(0, |w| w.1);
    let secs = (s.samples.len() as u64 - first) as f64 / f64::from(s.sample_rate);
    words as f64 * 60.0 / secs
}

/// Median fundamental frequency of voiced 40 ms frames (autocorrelation,
/// 60-400 Hz).
fn median_f0(s: &Synthesis) -> f64 {
    let rate = s.sample_rate as usize;
    let frame = rate / 25;
    let (lo, hi) = (rate / 400, rate / 60);
    let mut f0s = Vec::new();
    for chunk in s.samples.chunks_exact(frame) {
        let x: Vec<f64> = chunk.iter().map(|&v| f64::from(v)).collect();
        let energy: f64 = x.iter().map(|v| v * v).sum();
        if energy / (frame as f64) < 1.0e5 {
            continue;
        }
        let mut best = (0usize, 0.0f64);
        for lag in lo..hi {
            let c: f64 = x[..frame - lag]
                .iter()
                .zip(&x[lag..])
                .map(|(a, b)| a * b)
                .sum();
            if c > best.1 {
                best = (lag, c);
            }
        }
        if best.0 > 0 && best.1 / energy > 0.5 {
            f0s.push(rate as f64 / best.0 as f64);
        }
    }
    f0s.sort_by(|a, b| a.total_cmp(b));
    f0s.get(f0s.len() / 2).copied().unwrap_or(0.0)
}

const PASSAGE: &str = "The library opened early on Monday. Students came in from the rain, \
shook out their coats, and found seats near the windows. Some read quietly; others \
listened to their books through headphones, following each highlighted word on the \
screen. By noon every table was full, and the librarian brought out extra chairs from \
the back room so that nobody would have to leave.";

#[test]
#[ignore = "measurement: TEXTWEAVER_SAPI=1 ... calibrate -- --ignored --nocapture"]
fn calibrate() {
    if !enabled() {
        return;
    }
    let words = PASSAGE.split_whitespace().count();
    println!("passage: {words} words");
    let mut b = SapiBackend::new(config()).unwrap();
    for name in [
        "Microsoft David Desktop",
        "Microsoft Zira Desktop",
        "eSpeak-en",
    ] {
        let v = voice(&b, name);
        select(&mut b, &v);
        let mut row = Vec::new();
        for r in -10i8..=10 {
            let s = b.synthesize_at_rate(PASSAGE, r).unwrap();
            row.push(wpm(&s, words).round() as u32);
        }
        println!("{name:<26} wpm at rate -10..=10: {row:?}");
    }
    // Pitch: median F0 at a few semitone offsets (David), through the
    // mapping to SAPI's absmiddle.
    let david = voice(&b, "Microsoft David Desktop");
    let mut out = Vec::new();
    for p in [-5i8, -2, 0, 2, 5] {
        b.set_params(&VoiceParams {
            voice: Some(david.voice.id.clone()),
            pitch: textweaver_core::Pitch::Semitones(p),
            ..VoiceParams::default()
        })
        .unwrap();
        let s = b.synthesize(PASSAGE).unwrap();
        out.push((p, median_f0(&s).round()));
    }
    let base = out.iter().find(|(p, _)| *p == 0).map_or(1.0, |o| o.1);
    let semis: Vec<String> = out
        .iter()
        .map(|(p, f)| {
            format!(
                "{p:+} st (absmiddle {:+}): {f} Hz ({:+.1} st)",
                textweaver_sapi::calibration::sapi_pitch(*p),
                12.0 * (f / base).log2()
            )
        })
        .collect();
    println!("David pitch -> median F0: {}", semis.join(", "));
}
