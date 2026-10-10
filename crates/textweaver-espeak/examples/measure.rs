//! Measures eSpeak NG in-process against the helper program, with the
//! virtual output (nothing is played):
//!
//! ```text
//! cargo run -p textweaver-espeak --example measure [-- RUNS]
//! ```
//!
//! - start: creating each backend (the helper's includes starting it);
//! - first audio: from the request to the first block of samples. In
//!   process, that is libespeak-ng's first retrieval callback (where its
//!   own playback would start writing to the device); for the helper, the
//!   playback client's `Started`, raised when the output takes the first
//!   sample, so the pipe and the process switch are included;
//! - word timing: for the helper, when each word event arrives against
//!   when its audio is played (`Started` plus the word's milliseconds);
//!   in process, how far ahead of its audio each word event arrives, which
//!   the speech service bridges with a fixed offset (120 ms by default);
//! - agreement: whether both paths report the same words at the same
//!   milliseconds.
//!
//! Medians and the 90th percentile over RUNS utterances (default 20).
//! Needs libespeak-ng and the host built beside this example
//! (`cargo build -p textweaver-espeak --bins`).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use textweaver_core::{CharPos, Utterance, UtteranceId};
use textweaver_enginehost::{HostMsg, HostProcess};
use textweaver_espeak::protocol::{Reply, Request};
use textweaver_espeak::{AudioOutput, EspeakHostBackend, EspeakHostConfig, HOST_NAME};
use textweaver_speech::backends::espeak::{self, EspeakBackend, EspeakOutput};
use textweaver_speech::{EventSink, RawEvent, SpeechBackend};

const TEXT: &str =
    "The committee reviewed the proposal on Tuesday, and it approved the revised budget.";

#[derive(Default)]
struct Rec(Vec<(RawEvent, Instant)>);

impl EventSink for Rec {
    fn emit(&mut self, _id: UtteranceId, event: RawEvent) {
        self.0.push((event, Instant::now()));
    }
    fn is_current(&self, _id: UtteranceId) -> bool {
        true
    }
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// Median and 90th percentile.
fn stats(mut v: Vec<f64>) -> String {
    if v.is_empty() {
        return "no samples".into();
    }
    v.sort_by(f64::total_cmp);
    let at = |q: f64| v[((v.len() - 1) as f64 * q).round() as usize];
    format!(
        "median {:.1} ms, 90th percentile {:.1} ms (n={})",
        at(0.5),
        at(0.9),
        v.len()
    )
}

fn utt(n: u64) -> Utterance {
    let mut u = Utterance::literal(TEXT, CharPos(0));
    u.id = UtteranceId {
        generation: n,
        chunk: 0,
    };
    u
}

/// Speaks one utterance and polls every millisecond until it ends.
fn run(b: &mut dyn SpeechBackend, n: u64) -> (Instant, Rec) {
    let mut rec = Rec::default();
    let t0 = Instant::now();
    b.speak(&utt(n), &mut rec).expect("speak");
    while !rec
        .0
        .iter()
        .any(|(e, _)| matches!(e, RawEvent::Finished | RawEvent::Cancelled))
    {
        b.poll(&mut rec);
        std::thread::sleep(Duration::from_millis(1));
        assert!(t0.elapsed() < Duration::from_secs(30), "no end");
    }
    (t0, rec)
}

/// Each word event's arrival against its audio: arrival minus (`Started`
/// plus the word's milliseconds), in milliseconds.
fn word_offsets(rec: &Rec) -> Vec<f64> {
    let Some(started) = rec
        .0
        .iter()
        .find(|(e, _)| *e == RawEvent::Started)
        .map(|(_, t)| *t)
    else {
        return Vec::new();
    };
    rec.0
        .iter()
        .filter_map(|(e, t)| match e {
            RawEvent::Word {
                audio_ms: Some(a), ..
            } => {
                let due = started + Duration::from_millis(u64::from(*a));
                Some(if *t >= due {
                    ms(*t - due)
                } else {
                    -ms(due - *t)
                })
            }
            _ => None,
        })
        .collect()
}

fn main() {
    let runs: u64 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(20);
    let host = std::env::current_exe()
        .ok()
        .and_then(|e| {
            e.parent()
                .and_then(|d| d.parent())
                .map(|d| d.join(HOST_NAME))
        })
        .unwrap_or_else(|| PathBuf::from(HOST_NAME));

    let t = Instant::now();
    let mut local = EspeakBackend::new(EspeakOutput::Virtual).expect("in-process eSpeak NG");
    let local_start = t.elapsed();
    let t = Instant::now();
    let mut helper = EspeakHostBackend::new(EspeakHostConfig {
        host: Some(host.clone()),
        output: AudioOutput::Null { speed: 1.0 },
        ..EspeakHostConfig::default()
    })
    .expect("the helper (build it with cargo build -p textweaver-espeak --bins)");
    let helper_start = t.elapsed();
    println!("Start, in process: {:.1} ms", ms(local_start));
    println!("Start, helper: {:.1} ms", ms(helper_start));

    // Warm both up once.
    run(&mut local, 0);
    run(&mut helper, 0);

    // Agreement first, while both engines have spoken the same things:
    // libespeak-ng carries a little state from one utterance to the next
    // (timings move by a few milliseconds when the same text is repeated),
    // so only equal histories compare. The files go beside the example,
    // in the build folder.
    let dir = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join("espeak-measure")))
        .expect("the build folder");
    std::fs::create_dir_all(&dir).expect("a folder for the WAV files");
    let a = helper
        .synthesize_utterance(&utt(1), &dir.join("helper.wav"))
        .expect("helper file");
    let b = local
        .synthesize_utterance(&utt(1), &dir.join("local.wav"))
        .expect("in-process file");
    for f in ["helper.wav", "local.wav"] {
        let _ = std::fs::remove_file(dir.join(f));
    }
    let _ = std::fs::remove_dir(&dir);
    for (x, y) in a.words.iter().zip(&b.words).filter(|(x, y)| x != y) {
        println!("  differs: helper {x:?}, in process {y:?}");
    }
    println!(
        "Words agree: {} ({} words in process, {} from the helper)",
        if a.words == b.words { "yes" } else { "no" },
        b.words.len(),
        a.words.len()
    );

    // A second helper driven directly over the protocol, to time the
    // first block of samples reaching this process.
    let mut raw: HostProcess<Reply> =
        HostProcess::spawn(&host, std::iter::empty::<&str>(), "espeak").expect("raw helper");
    match raw.recv_timeout(Duration::from_secs(10)) {
        Some(HostMsg::Reply(Reply::Ready { .. })) => {}
        other => panic!("the raw helper did not start: {other:?}"),
    }
    let mut pipe_first = Vec::new();

    let (mut local_first, mut helper_first) = (Vec::new(), Vec::new());
    let (mut local_words, mut helper_words) = (Vec::new(), Vec::new());
    for n in 1..=runs {
        // In process: the first retrieval callback with samples.
        let t0 = Instant::now();
        let first = std::sync::Arc::new(std::sync::Mutex::new(None::<Instant>));
        let f = std::sync::Arc::clone(&first);
        espeak::retrieval_synthesize(
            TEXT,
            None,
            Box::new(move |wav: &[i16], _| {
                let mut f = f.lock().unwrap();
                if f.is_none() && !wav.is_empty() {
                    *f = Some(Instant::now());
                }
                false
            }),
        )
        .expect("synthesis");
        if let Some(at) = *first.lock().unwrap() {
            local_first.push(ms(at - t0));
        }
        let (_, rec) = run(&mut local, n);
        local_words.extend(word_offsets(&rec));

        let t0 = Instant::now();
        raw.send(&Request::Speak {
            token: n,
            character: false,
            text: TEXT.into(),
        })
        .expect("send");
        let mut first = None;
        loop {
            match raw.recv_timeout(Duration::from_secs(10)) {
                Some(HostMsg::Reply(Reply::Audio { .. })) if first.is_none() => {
                    first = Some(t0.elapsed());
                }
                Some(HostMsg::Reply(Reply::End { .. })) => break,
                Some(HostMsg::Reply(_)) => {}
                other => panic!("the raw helper stopped: {other:?}"),
            }
        }
        pipe_first.extend(first.map(ms));

        let (t0, rec) = run(&mut helper, n);
        if let Some((_, at)) = rec.0.iter().find(|(e, _)| *e == RawEvent::Started) {
            helper_first.push(ms(*at - t0));
        }
        helper_words.extend(word_offsets(&rec));
    }
    println!("First audio, in process: {}", stats(local_first));
    println!(
        "First audio, helper, first samples in this process: {}",
        stats(pipe_first)
    );
    println!(
        "First audio, helper, first sample taken by the output: {}",
        stats(helper_first)
    );
    println!(
        "Word events, in process, arrival against audio (negative is early): {}",
        stats(local_words)
    );
    println!(
        "Word events, helper, arrival against audio: {}",
        stats(helper_words)
    );
}
