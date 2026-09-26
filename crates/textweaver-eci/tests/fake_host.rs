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
use textweaver_eci::{AudioOutput, Dictionaries, EciBackend, EciConfig};
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
    let t0 = Instant::now();
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
    // cannot arrive before its audio has played (at 4x speed). Measured
    // from before `speak`, so a slow machine only makes it later. (Before,
    // the spread between the first and last word was measured, which a
    // late first word shrank.)
    let last = rec
        .events
        .iter()
        .rev()
        .find(|(_, e, _)| matches!(e, RawEvent::Word { .. }))
        .map(|(.., t)| *t)
        .unwrap();
    let heard = last - t0;
    let expected = Duration::from_millis(u64::from(words[words.len() - 1].1) / 4);
    assert!(
        heard + Duration::from_millis(40) >= expected,
        "the last word came {heard:?} after speak, audio says {expected:?}"
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
        Duration::from_secs(10),
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
    // Paused for longer than the rest of the audio lasts (about half a
    // second at this speed): if the pause did not hold, the utterance
    // would finish. A word whose audio had already played may still
    // arrive late on a busy machine, so the check is that the reading is
    // held, not that nothing at all arrives. (Before: a fixed 20 ms sleep
    // was meant to let such words arrive, and failed when one came later.)
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(1) {
        b.poll(&mut rec);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        !rec.has(u.id, |e| *e == RawEvent::Finished),
        "the utterance went on while paused: {:?}",
        rec.of(u.id)
    );
    assert!(rec.words(u.id).len() < 10, "every word came while paused");
    assert!(
        rec.of(u.id)
            .iter()
            .all(|e| matches!(e, RawEvent::Started | RawEvent::Word { .. })),
        "{:?}",
        rec.of(u.id)
    );
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
fn synthesize_utterance_reports_word_timings_on_the_files_clock() {
    let mut b = backend(1.0);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("utt.wav");
    let text = "Dr. Smith opened the café.";
    let u = utt(text, 1, 0);
    let fs = b.synthesize_utterance(&u, &path).unwrap();
    let (rate, samples) = textweaver_eci::wav::read_wav(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(rate, 8000);
    let spoken: Vec<&str> = fs
        .words
        .iter()
        .map(|w| &text[w.byte_range.start as usize..w.byte_range.end as usize])
        .collect();
    assert_eq!(spoken, ["Dr", "Smith", "opened", "the", "café"]);
    // Each time is its index mark's sample offset in this very file.
    let s = b.synthesize(text).unwrap();
    assert_eq!(s.samples.len(), samples.len());
    for (w, (range, sample)) in fs.words.iter().zip(&s.words) {
        assert_eq!(&w.byte_range, range);
        assert_eq!(u64::from(w.audio_ms), sample * 1000 / 8000);
    }
    assert_eq!(fs.words[0].audio_ms, 0);
    for pair in fs.words.windows(2) {
        assert!(pair[0].audio_ms < pair[1].audio_ms);
    }
    let duration_ms = samples.len() as u64 * 1000 / 8000;
    assert!(u64::from(fs.words[4].audio_ms) < duration_ms);
    // The fake engine gives every byte the same length: "Smith" starts
    // after the 4 bytes of "Dr. ", "opened" after 10.
    let per_byte = f64::from(fs.words[1].audio_ms) / 4.0;
    assert!((f64::from(fs.words[2].audio_ms) - per_byte * 10.0).abs() <= 1.0);
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
fn word_events_are_timed_by_playback_and_normalization_is_native() {
    let caps = backend(1.0).capabilities();
    assert!(caps.contains(Caps::PLAYBACK_EVENTS | Caps::NATIVE_NORMALIZATION));
    assert!(
        !caps.contains(Caps::TONES),
        "a silent output plays no tones"
    );
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

#[test]
fn a_hung_engine_is_killed_and_the_utterance_fails() {
    let mut b = EciBackend::new(EciConfig {
        stall_timeout: Some(Duration::from_millis(500)),
        ..config(8.0)
    })
    .unwrap();
    let mut rec = Rec::default();
    let u = utt("this will __hang__ forever", 11, 0);
    b.speak(&u, &mut rec).unwrap();
    assert!(pump(
        &mut b,
        &mut rec,
        Duration::from_secs(10),
        finished(u.id)
    ));
    assert!(rec.has(
        u.id,
        |e| matches!(e, RawEvent::Error(m) if m.contains("stopped responding"))
    ));
    assert!(matches!(
        b.synthesize("again __hang__"),
        Err(SpeechError::Engine(m)) if m.contains("stopped responding")
    ));
    // A fresh host serves the next request. It gets the normal stall
    // timeout: under 500 ms, a new host on a busy machine (a virus scan of
    // the new process, parallel builds) could be taken for a hung one, which
    // made this test flaky.
    b.set_stall_timeout(None);
    assert_eq!(b.synthesize("fine now").unwrap().words.len(), 2);
}

fn repo_dictionaries() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../third_party/ibmtts-dictionaries")
}

#[test]
fn dictionaries_load_per_language_and_can_be_turned_off() {
    let mut b = EciBackend::new(EciConfig {
        dictionaries: Dictionaries::Dir(repo_dictionaries()),
        ..config(8.0)
    })
    .unwrap();
    let loads: Vec<(u32, u8, i32)> = b
        .dictionary_loads()
        .iter()
        .map(|l| (l.dialect, l.volume, l.status))
        .collect();
    assert_eq!(
        loads,
        [
            (0x0001_0000, 0, 0),
            (0x0001_0000, 1, 0),
            (0x0001_0000, 2, 0)
        ]
    );
    assert!(b.dictionary_loads()[1].path.ends_with("ENURoot.dic"));

    // Switching to German loads its dictionaries once.
    let p = VoiceParams {
        voice: Some("eci:deu:reed".into()),
        ..VoiceParams::default()
    };
    b.set_params(&p).unwrap();
    b.synthesize("Guten Tag").unwrap();
    let deu = b
        .dictionary_loads()
        .iter()
        .filter(|l| l.dialect == 0x0004_0000)
        .count();
    assert_eq!(deu, 3);

    let mut off = EciBackend::new(EciConfig {
        dictionaries: Dictionaries::Off,
        ..config(8.0)
    })
    .unwrap();
    off.synthesize("x").unwrap();
    assert!(off.dictionary_loads().is_empty());
}

#[test]
fn an_utterance_over_the_frame_limit_is_refused_and_the_host_keeps_working() {
    // Before: the 17 MB request reached the host, whose reader failed on
    // its length, so the host exited and the reading restarted it.
    let mut b = backend(8.0);
    let mut rec = Rec::default();
    let huge = utt(&"a".repeat(17 * 1024 * 1024), 6, 0);
    let e = b.speak(&huge, &mut rec).unwrap_err();
    assert_eq!(
        e,
        SpeechError::Engine(
            "this text is too long to speak in one piece (more than 17 MB; the limit is 16 MB)"
                .into()
        )
    );
    let path = b.host_path().map(std::path::Path::to_path_buf);
    let next = utt("Still here.", 6, 1);
    b.speak(&next, &mut rec).unwrap();
    assert!(pump(
        &mut b,
        &mut rec,
        Duration::from_secs(10),
        finished(next.id)
    ));
    assert_eq!(rec.of(next.id).last(), Some(&RawEvent::Finished));
    assert_eq!(b.host_path().map(std::path::Path::to_path_buf), path);
}

#[test]
fn a_host_stuck_in_synthesis_exits_when_its_input_closes() {
    // Before: the host of an engine stuck in synthesis lived on after
    // textweaver closed its input (textweaver exited or crashed).
    use textweaver_eci::protocol::{Piece, Reply, Request};
    use textweaver_enginehost::{HostMsg, HostProcess};
    let mut h = HostProcess::<Reply>::spawn(&test_host(), ["--engine", "fake"], "eci-test")
        .expect("the fake host starts");
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut next = || h.recv_until(deadline).expect("the host answers in time");
    while !matches!(next(), HostMsg::Reply(Reply::Ready { .. })) {}
    h.send(&Request::Speak {
        token: 1,
        pieces: vec![Piece::Index(0), Piece::Text("__hang__".into())],
    })
    .unwrap();
    // The mark before the hang says synthesis has started.
    loop {
        match h.recv_until(deadline) {
            Some(HostMsg::Reply(Reply::Mark { token: 1, .. })) => break,
            Some(HostMsg::Reply(_)) => {}
            other => panic!("expected the engine to start, got {other:?}"),
        }
    }
    h.close_input();
    loop {
        match h.recv_until(deadline) {
            Some(HostMsg::Closed(_)) => break,
            Some(HostMsg::Reply(_)) => {}
            None => panic!("the stuck host did not exit after its input closed"),
        }
    }
}
