//! The backend against the real host binary running its fake engine
//! (`--engine fake`): the whole pipe protocol, playback clock, and event
//! contract, without the proprietary engine and without making a sound
//! (the null audio output consumes samples on a timer).
//!
//! `TEXTWEAVER_ECI_TEST_HOST` runs the same tests against another host
//! build, for example the 32-bit Windows host:
//! `TEXTWEAVER_ECI_TEST_HOST=target/i686-pc-windows-msvc/debug/textweaver-eci-host.exe`.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use textweaver_core::{CharPos, Pitch, Rate, Utterance, UtteranceId, Volume};
use textweaver_eci::{AudioOutput, EciBackend, EciConfig};
use textweaver_speech::{Caps, EventSink, RawEvent, SpeechBackend, SpeechError, VoiceParams};

#[derive(Default)]
struct Rec {
    events: Vec<(UtteranceId, RawEvent, Instant)>,
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
    fn of(&self, id: UtteranceId) -> Vec<RawEvent> {
        self.events
            .iter()
            .filter(|(i, ..)| *i == id)
            .map(|(_, e, _)| e.clone())
            .collect()
    }
    fn has(&self, id: UtteranceId, pred: impl Fn(&RawEvent) -> bool) -> bool {
        self.events.iter().any(|(i, e, _)| *i == id && pred(e))
    }
    fn words(&self, id: UtteranceId) -> Vec<(std::ops::Range<u32>, u32)> {
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
}

fn test_host() -> PathBuf {
    std::env::var_os("TEXTWEAVER_ECI_TEST_HOST")
        .filter(|v| !v.is_empty())
        .map_or_else(
            || PathBuf::from(env!("CARGO_BIN_EXE_textweaver-eci-host")),
            PathBuf::from,
        )
}

fn config(speed: f32) -> EciConfig {
    EciConfig {
        host: Some(test_host()),
        output: AudioOutput::Null { speed },
        fake_engine: true,
        ..EciConfig::default()
    }
}

fn backend(speed: f32) -> EciBackend {
    EciBackend::new(config(speed)).expect("fake host starts")
}

fn utt(text: &str, generation: u64, chunk: u32) -> Utterance {
    let mut u = Utterance::literal(text, CharPos(0));
    u.id = UtteranceId { generation, chunk };
    u
}

/// Polls until `done` holds or `timeout` passes; returns whether it held.
fn pump(b: &mut EciBackend, rec: &mut Rec, timeout: Duration, done: impl Fn(&Rec) -> bool) -> bool {
    let t0 = Instant::now();
    loop {
        b.poll(rec);
        if done(rec) {
            return true;
        }
        if t0.elapsed() > timeout {
            return false;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn finished(id: UtteranceId) -> impl Fn(&Rec) -> bool {
    move |r: &Rec| {
        r.has(id, |e| {
            matches!(e, RawEvent::Finished | RawEvent::Cancelled)
        })
    }
}

#[test]
fn speak_reports_started_each_word_in_order_then_finished() {
    let mut b = backend(4.0);
    let mut rec = Rec::default();
    let text = "Hello brave new world, café naïve 日本 ok.";
    let u = utt(text, 1, 0);
    b.speak(&u, &mut rec).unwrap();
    assert!(pump(
        &mut b,
        &mut rec,
        Duration::from_secs(10),
        finished(u.id)
    ));
    let ev = rec.of(u.id);
    assert_eq!(ev.first(), Some(&RawEvent::Started));
    assert_eq!(ev.last(), Some(&RawEvent::Finished));
    let words = rec.words(u.id);
    let spoken: Vec<&str> = words
        .iter()
        .map(|(r, _)| &text[r.start as usize..r.end as usize])
        .collect();
    assert_eq!(
        spoken,
        [
            "Hello", "brave", "new", "world", "café", "naïve", "日本", "ok"
        ]
    );
    assert_eq!(words[0].1, 0);
    for pair in words.windows(2) {
        assert!(pair[0].1 < pair[1].1, "audio_ms must rise: {words:?}");
    }
    // Word events fire on the playback clock, not in a burst: the last word
    // arrives well after the first.
    let times: Vec<Instant> = rec
        .events
        .iter()
        .filter(|(_, e, _)| matches!(e, RawEvent::Word { .. }))
        .map(|(.., t)| *t)
        .collect();
    let spread = times[times.len() - 1] - times[0];
    let expected = Duration::from_millis(u64::from(words[words.len() - 1].1) / 4);
    assert!(
        spread + Duration::from_millis(40) >= expected,
        "words spread over {spread:?}, audio says {expected:?}"
    );
}

#[test]
fn queued_utterances_play_back_to_back_in_order() {
    let mut b = backend(8.0);
    let mut rec = Rec::default();
    let a = utt("First sentence here.", 2, 0);
    let c = utt("Second one.", 2, 1);
    b.speak(&a, &mut rec).unwrap();
    b.speak(&c, &mut rec).unwrap();
    assert!(pump(
        &mut b,
        &mut rec,
        Duration::from_secs(10),
        finished(c.id)
    ));
    let order: Vec<(UtteranceId, RawEvent)> =
        rec.events.iter().map(|(i, e, _)| (*i, e.clone())).collect();
    let a_fin = order
        .iter()
        .position(|(i, e)| *i == a.id && *e == RawEvent::Finished)
        .unwrap();
    let c_start = order
        .iter()
        .position(|(i, e)| *i == c.id && *e == RawEvent::Started)
        .unwrap();
    assert!(a_fin < c_start, "{order:?}");
    assert_eq!(rec.words(a.id).len(), 3);
    assert_eq!(rec.words(c.id).len(), 2);
}

#[test]
fn stop_mid_utterance_cancels_and_nothing_follows() {
    let mut b = backend(2.0);
    let mut rec = Rec::default();
    let long = "word ".repeat(60);
    let u = utt(&long, 3, 0);
    let queued = utt("never heard", 3, 1);
    b.speak(&u, &mut rec).unwrap();
    b.speak(&queued, &mut rec).unwrap();
    assert!(pump(&mut b, &mut rec, Duration::from_secs(10), |r| {
        r.words(u.id).len() >= 3
    }));
    b.stop();
    assert!(pump(
        &mut b,
        &mut rec,
        Duration::from_secs(2),
        finished(u.id)
    ));
    let n = rec.events.len();
    // Keep polling: no late word, finished, or started for either utterance.
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_millis(400) {
        b.poll(&mut rec);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(rec.events.len(), n, "late events: {:?}", &rec.events[n..]);
    assert_eq!(rec.of(u.id).last(), Some(&RawEvent::Cancelled));
    assert_eq!(rec.of(queued.id), [RawEvent::Cancelled]);
    assert!(rec.words(u.id).len() < 60);
    assert!(!rec.has(u.id, |e| *e == RawEvent::Finished));

    // The backend is usable again right away.
    let next = utt("Again now.", 4, 0);
    b.speak(&next, &mut rec).unwrap();
    assert!(pump(
        &mut b,
        &mut rec,
        Duration::from_secs(10),
        finished(next.id)
    ));
    assert_eq!(rec.of(next.id).last(), Some(&RawEvent::Finished));
    assert_eq!(rec.words(next.id).len(), 2);
}

#[test]
fn pause_holds_every_event_and_resume_continues_without_skipping() {
    let mut b = backend(4.0);
    let mut rec = Rec::default();
    let text = "one two three four five six seven eight nine ten";
    let u = utt(text, 5, 0);
    b.speak(&u, &mut rec).unwrap();
    assert!(pump(&mut b, &mut rec, Duration::from_secs(10), |r| {
        r.words(u.id).len() >= 2
    }));
    b.pause().unwrap();
    // Let anything already due drain, then expect silence.
    std::thread::sleep(Duration::from_millis(20));
    b.poll(&mut rec);
    let held = rec.events.len();
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_millis(400) {
        b.poll(&mut rec);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(rec.events.len(), held, "events while paused");
    b.resume().unwrap();
    assert!(pump(
        &mut b,
        &mut rec,
        Duration::from_secs(10),
        finished(u.id)
    ));
    let words = rec.words(u.id);
    assert_eq!(words.len(), 10, "no word skipped or repeated");
    for pair in words.windows(2) {
        assert!(pair[0].1 < pair[1].1);
    }
    assert_eq!(rec.of(u.id).last(), Some(&RawEvent::Finished));
}

#[test]
fn synthesize_to_file_writes_a_wav_and_marks_rise() {
    let mut b = backend(1.0);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.wav");
    b.synthesize_to_file("Dr. Smith opened the library.", &path)
        .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let (rate, samples) = textweaver_eci::wav::read_wav(&bytes).unwrap();
    assert_eq!(rate, 8000);
    assert!(!samples.is_empty());

    let s = b.synthesize("Dr. Smith opened the library.").unwrap();
    assert_eq!(s.samples.len(), samples.len());
    assert_eq!(s.words.len(), 5);
    for pair in s.words.windows(2) {
        assert!(pair[0].1 < pair[1].1);
    }
}

#[test]
fn rate_and_volume_change_the_audio() {
    let mut b = backend(1.0);
    let text = "The quick brown fox.";
    let normal = b.synthesize(text).unwrap();
    let mut p = VoiceParams {
        rate: Rate::Wpm(450),
        ..VoiceParams::default()
    };
    b.set_params(&p).unwrap();
    let fast = b.synthesize(text).unwrap();
    assert!(fast.samples.len() < normal.samples.len());
    assert!(b.effective_wpm() > 400, "{}", b.effective_wpm());

    p.volume = Volume::new(50);
    p.rate = Rate::default();
    b.set_params(&p).unwrap();
    let quiet = b.synthesize(text).unwrap();
    let peak = |s: &[i16]| s.iter().map(|v| v.unsigned_abs()).max().unwrap_or(0);
    assert_eq!(quiet.samples.len(), normal.samples.len());
    assert_eq!(peak(&quiet.samples), peak(&normal.samples) / 2);
}

#[test]
fn voices_list_languages_and_presets_and_reject_unknown_ids() {
    let mut b = backend(1.0);
    let voices = b.voices().unwrap();
    // The fake engine reports en-US, en-GB, and de-DE, eight presets each.
    assert_eq!(voices.len(), 24);
    assert_eq!(voices[0].id, "eci:enu:reed");
    assert!(voices.iter().any(|v| v.id == "eci:deu:shelley"));
    assert!(voices.iter().all(|v| v.name.starts_with("Eloquence ")));

    let mut p = VoiceParams {
        voice: Some("eci:deu:shelley".into()),
        pitch: Pitch::Semitones(2),
        ..VoiceParams::default()
    };
    b.set_params(&p).unwrap();
    assert!(b.synthesize("Guten Tag").is_ok());
    p.voice = Some("eci:fra:reed".into());
    assert!(matches!(
        b.set_params(&p),
        Err(SpeechError::UnknownVoice(_))
    ));
    p.voice = Some("en-US:9".into());
    assert!(matches!(
        b.set_params(&p),
        Err(SpeechError::UnknownVoice(_))
    ));
}

#[test]
fn a_host_crash_ends_the_utterance_and_the_next_speak_restarts_it() {
    let mut b = backend(8.0);
    let mut rec = Rec::default();
    let bad = utt("this will __crash__ now", 6, 0);
    b.speak(&bad, &mut rec).unwrap();
    assert!(pump(
        &mut b,
        &mut rec,
        Duration::from_secs(10),
        finished(bad.id)
    ));
    assert!(rec.has(bad.id, |e| matches!(e, RawEvent::Error(_))));
    assert_eq!(rec.of(bad.id).last(), Some(&RawEvent::Finished));

    let good = utt("Still here.", 7, 0);
    b.speak(&good, &mut rec).unwrap();
    assert!(pump(
        &mut b,
        &mut rec,
        Duration::from_secs(10),
        finished(good.id)
    ));
    assert_eq!(rec.words(good.id).len(), 2);

    let fail = utt("please __fail__", 8, 0);
    b.speak(&fail, &mut rec).unwrap();
    assert!(pump(
        &mut b,
        &mut rec,
        Duration::from_secs(10),
        finished(fail.id)
    ));
    assert!(rec.has(
        fail.id,
        |e| matches!(e, RawEvent::Error(m) if m.contains("fake engine failure"))
    ));
}

#[test]
fn empty_utterances_still_start_and_finish() {
    let mut b = backend(8.0);
    let mut rec = Rec::default();
    let u = utt("   ", 9, 0);
    b.speak(&u, &mut rec).unwrap();
    assert!(pump(
        &mut b,
        &mut rec,
        Duration::from_secs(5),
        finished(u.id)
    ));
    assert_eq!(rec.of(u.id), [RawEvent::Started, RawEvent::Finished]);
}

#[test]
fn capabilities_match_adr_0007() {
    let b = backend(1.0);
    let caps = b.capabilities();
    for c in [
        Caps::WORD_EVENTS,
        Caps::AUDIO_CLOCK,
        Caps::PAUSE,
        Caps::PITCH,
        Caps::VOLUME,
        Caps::SYNTH_TO_FILE,
    ] {
        assert!(caps.contains(c), "{c:?}");
    }
    assert!(!caps.contains(Caps::REQUIRES_MAIN_THREAD));
    assert_eq!(b.id(), "eci");
    // The host under test is the one requested (not a fallback).
    assert_eq!(b.host_path(), Some(test_host().as_path()));
}

#[test]
fn host_search_skips_files_that_do_not_exist() {
    let cfg = EciConfig {
        host: Some(PathBuf::from("definitely-not-here/textweaver-eci-host")),
        fake_engine: true,
        ..EciConfig::default()
    };
    let found = textweaver_eci::host_candidates(&cfg);
    assert!(found.iter().all(|p| p.is_file()));
    assert!(!found.iter().any(|p| p.starts_with("definitely-not-here")));
}

#[test]
fn the_factory_builds_a_working_backend() {
    let f = textweaver_eci::factory(config(8.0));
    let mut b = f().unwrap();
    let mut rec = Rec::default();
    let u = utt("Hi.", 10, 0);
    b.speak(&u, &mut rec).unwrap();
    let t0 = Instant::now();
    while !finished(u.id)(&rec) && t0.elapsed() < Duration::from_secs(5) {
        b.poll(&mut rec);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(rec.of(u.id).last(), Some(&RawEvent::Finished));
}
