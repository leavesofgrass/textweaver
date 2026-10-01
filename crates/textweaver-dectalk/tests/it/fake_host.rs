//! The backend against the real host binary running its fake engine
//! (`--engine fake`): the pipe protocol, the playback clock, and the event
//! contract, without DECtalk and without a sound (the null output consumes
//! samples on a timer).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use textweaver_core::{CharPos, Pitch, Rate, Utterance, UtteranceId, Volume};
use textweaver_dectalk::host::fake;
use textweaver_dectalk::{AudioOutput, DectalkBackend, DectalkConfig, Speaker};
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
    PathBuf::from(env!("CARGO_BIN_EXE_textweaver-dectalk-host"))
}

fn config(speed: f32) -> DectalkConfig {
    DectalkConfig {
        host: Some(test_host()),
        output: AudioOutput::Null { speed },
        fake_engine: true,
        ..DectalkConfig::default()
    }
}

/// DECtalk's own default rate (textweaver's default is faster).
fn at_180() -> VoiceParams {
    VoiceParams {
        rate: Rate::Wpm(180),
        ..VoiceParams::default()
    }
}

/// A backend at 180 wpm, where the fake speaks 200 samples per byte.
fn backend(speed: f32) -> DectalkBackend {
    let mut b = DectalkBackend::new(config(speed)).expect("fake host starts");
    b.set_params(&at_180()).unwrap();
    b
}

fn utt(text: &str, generation: u64, chunk: u32) -> Utterance {
    let mut u = Utterance::literal(text, CharPos(0));
    u.id = UtteranceId { generation, chunk };
    u
}

fn pump(
    b: &mut DectalkBackend,
    rec: &mut Rec,
    timeout: Duration,
    done: impl Fn(&Rec) -> bool,
) -> bool {
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

fn peak(s: &[i16]) -> u16 {
    s.iter().map(|v| v.unsigned_abs()).max().unwrap_or(0)
}

#[test]
fn speak_reports_started_each_word_in_order_then_finished() {
    let mut b = backend(8.0);
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
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_millis(400) {
        b.poll(&mut rec);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(rec.events.len(), n, "late events: {:?}", &rec.events[n..]);
    assert_eq!(rec.of(u.id).last(), Some(&RawEvent::Cancelled));
    assert_eq!(rec.of(queued.id), [RawEvent::Cancelled]);
    assert!(rec.words(u.id).len() < 60);

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
    assert_eq!(rec.of(u.id).last(), Some(&RawEvent::Finished));
}

#[test]
fn synthesize_to_file_and_utterance_write_wavs_with_word_timings() {
    let mut b = backend(1.0);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.wav");
    let text = "Dr. Smith opened the library.";
    b.synthesize_to_file(text, &path).unwrap();
    let (rate, samples) =
        textweaver_dectalk::wav::read_wav(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(rate, 11_025);
    // 200 samples per byte at DECtalk's default 180 wpm.
    assert_eq!(samples.len(), text.len() * fake::SAMPLES_PER_BYTE);

    let s = b.synthesize(text).unwrap();
    assert_eq!(s.samples.len(), samples.len());
    let at: Vec<u64> = s.words.iter().map(|(_, a)| *a).collect();
    assert_eq!(at, [0, 800, 2000, 3400, 4200]);

    let upath = dir.path().join("utt.wav");
    let fs = b.synthesize_utterance(&utt(text, 1, 0), &upath).unwrap();
    let ms: Vec<(&str, u32)> = fs
        .words
        .iter()
        .map(|w| {
            (
                &text[w.byte_range.start as usize..w.byte_range.end as usize],
                w.audio_ms,
            )
        })
        .collect();
    // Sample offsets at 11,025 Hz, rounded down.
    assert_eq!(
        ms,
        [
            ("Dr", 0),
            ("Smith", 72),
            ("opened", 181),
            ("the", 308),
            ("library", 380)
        ]
    );
    assert_eq!(
        std::fs::read(&upath).unwrap(),
        std::fs::read(&path).unwrap()
    );
}

#[test]
fn rate_voice_pitch_and_volume_reach_the_audio() {
    let mut b = backend(1.0);
    let text = "The quick brown fox.";
    let normal = b.synthesize(text).unwrap();
    assert_eq!(peak(&normal.samples), 500, "Paul");
    assert_eq!(b.effective_wpm(), 180);

    let mut p = VoiceParams {
        voice: Some("dectalk:betty".into()),
        rate: Rate::Wpm(360),
        ..VoiceParams::default()
    };
    b.set_params(&p).unwrap();
    assert_eq!(b.speaker(), Speaker::Betty);
    assert_eq!(b.effective_wpm(), 360);
    let fast = b.synthesize(text).unwrap();
    assert_eq!(fast.samples.len() * 2, normal.samples.len());
    assert_eq!(peak(&fast.samples), 900, "Betty");

    // DECtalk's range caps the rate.
    p.rate = Rate::Wpm(900);
    b.set_params(&p).unwrap();
    assert_eq!(b.effective_wpm(), 600);

    // An octave up halves the square wave's period.
    let crossings = |s: &[i16]| s.windows(2).filter(|w| (w[0] < 0) != (w[1] < 0)).count();
    p.rate = Rate::Wpm(180);
    p.voice = None;
    let base = crossings(&b.synthesize(text).unwrap().samples);
    p.pitch = Pitch::Semitones(12);
    b.set_params(&p).unwrap();
    let high = crossings(&b.synthesize(text).unwrap().samples);
    assert!(high > base * 19 / 10, "{base} -> {high}");

    p.pitch = Pitch::default();
    p.volume = Volume::new(50);
    b.set_params(&p).unwrap();
    let quiet = b.synthesize(text).unwrap();
    assert_eq!(peak(&quiet.samples), 250);
}

#[test]
fn nine_speakers_are_listed_and_unknown_voices_refused() {
    let mut b = backend(1.0);
    let voices = b.voices().unwrap();
    let names: Vec<&str> = voices.iter().map(|v| v.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "DECtalk Perfect Paul",
            "DECtalk Huge Harry",
            "DECtalk Frail Frank",
            "DECtalk Doctor Dennis",
            "DECtalk Beautiful Betty",
            "DECtalk Uppity Ursula",
            "DECtalk Whispering Wendy",
            "DECtalk Rough Rita",
            "DECtalk Kit the Kid"
        ]
    );
    for (i, v) in voices.iter().enumerate() {
        b.set_params(&VoiceParams {
            voice: Some(v.id.clone()),
            ..at_180()
        })
        .unwrap();
        let s = b.synthesize("x").unwrap();
        assert_eq!(peak(&s.samples), 500 + 100 * i as u16, "{}", v.id);
    }
    // Star's saved names work too.
    b.set_params(&VoiceParams {
        voice: Some("Paul".into()),
        ..VoiceParams::default()
    })
    .unwrap();
    assert!(matches!(
        b.set_params(&VoiceParams {
            voice: Some("eci:enu:reed".into()),
            ..VoiceParams::default()
        }),
        Err(SpeechError::UnknownVoice(_))
    ));
    assert_eq!(
        b.speaker(),
        Speaker::Paul,
        "a refused voice changes nothing"
    );
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

    // The voice survives the restart.
    b.set_params(&VoiceParams {
        voice: Some("dectalk:kit".into()),
        ..VoiceParams::default()
    })
    .unwrap();
    let good = utt("Still here.", 7, 0);
    b.speak(&good, &mut rec).unwrap();
    assert!(pump(
        &mut b,
        &mut rec,
        Duration::from_secs(10),
        finished(good.id)
    ));
    assert_eq!(rec.words(good.id).len(), 2);
    assert_eq!(peak(&b.synthesize("k").unwrap().samples), 1300);

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
fn a_hung_engine_is_killed_and_the_utterance_fails() {
    let mut b = DectalkBackend::new(DectalkConfig {
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
    assert_eq!(b.synthesize("fine now").unwrap().words.len(), 2);
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
fn capabilities_and_identity() {
    let b = backend(1.0);
    let caps = b.capabilities();
    assert!(caps.contains(
        Caps::WORD_EVENTS
            | Caps::AUDIO_CLOCK
            | Caps::PAUSE
            | Caps::PITCH
            | Caps::VOLUME
            | Caps::SYNTH_TO_FILE
            | Caps::PLAYBACK_EVENTS
    ));
    assert!(
        !caps.contains(Caps::TONES),
        "a silent output plays no tones"
    );
    assert!(!caps.contains(Caps::NATIVE_NORMALIZATION));
    assert!(!caps.contains(Caps::REQUIRES_MAIN_THREAD));
    assert_eq!(b.id(), "dectalk");
    assert_eq!(b.engine(), Some("fake"));
    assert_eq!(b.sample_rate(), Some(11_025));
    assert_eq!(b.host_path(), Some(test_host().as_path()));
    let info = textweaver_dectalk::backend_info();
    assert_eq!(info.id, "dectalk");
    assert_eq!(info.priority, 300);
    assert!(!info.opt_in);
}

#[test]
fn the_factory_builds_a_working_backend() {
    let f = textweaver_dectalk::factory(config(8.0));
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
fn without_a_library_the_backend_says_what_to_do() {
    let dir = tempfile::tempdir().unwrap();
    let e = DectalkBackend::new(DectalkConfig {
        library: Some(dir.path().join("DECtalk.dll")),
        host: Some(test_host()),
        output: AudioOutput::Null { speed: 1.0 },
        ..DectalkConfig::default()
    });
    // A machine with a DECtalk installed would find it after the missing
    // option; this development machine has none.
    if textweaver_dectalk::library_path().is_none() {
        let e = e.unwrap_err();
        assert!(
            matches!(&e, SpeechError::Unavailable("dectalk", m) if m.contains("TEXTWEAVER_DECTALK_LIBRARY")),
            "{e}"
        );
    }
}

#[test]
fn a_host_stuck_in_synthesis_exits_when_its_input_closes() {
    // Before: the host of an engine stuck in synthesis lived on after
    // textweaver closed its input (textweaver exited or crashed).
    use textweaver_dectalk::protocol::{Piece, Reply, Request};
    use textweaver_enginehost::{HostMsg, HostProcess};
    let mut h = HostProcess::<Reply>::spawn(&test_host(), ["--engine", "fake"], "dectalk-test")
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

/// Crashes the backend's host, so the next request starts a new one.
fn crash(b: &mut DectalkBackend, rec: &mut Rec) {
    let bad = utt("__crash__", 90, 0);
    b.speak(&bad, rec).unwrap();
    assert!(pump(b, rec, Duration::from_secs(10), finished(bad.id)));
}

#[test]
fn a_restart_starts_in_poll_and_stop_and_pause_work_meanwhile() {
    // Every start of this host takes two seconds; the first one waits.
    let mut b = DectalkBackend::new(DectalkConfig {
        host_args: vec!["--start-delay-ms".into(), "2000".into()],
        ..config(8.0)
    })
    .unwrap();
    let mut rec = Rec::default();
    crash(&mut b, &mut rec);
    // `speak` returns at once; the request waits for the host in `poll`.
    let u = utt("stopped while starting", 91, 0);
    // These checks use the order of events, not the clock, so a loaded
    // machine cannot fail them (before, they asserted `speak` took under
    // a second and `stop` under half a second): the host is set only when
    // its start completes in `poll`, two seconds after it began, so a
    // `speak` or a `stop` that waited for the start would find it up.
    b.speak(&u, &mut rec).unwrap();
    assert!(
        b.host_path().is_none(),
        "speak returned before the host was ready"
    );
    b.stop();
    b.poll(&mut rec);
    assert!(
        b.host_path().is_none(),
        "stop took effect before the host was ready"
    );
    assert_eq!(rec.of(u.id), [RawEvent::Cancelled]);
    // Pause while starting: silent until resumed.
    let p = utt("paused while starting", 92, 0);
    b.speak(&p, &mut rec).unwrap();
    b.pause().unwrap();
    // Wait for the host (a hang check, not a speed check), then give it
    // time to synthesize: the paused utterance stays silent.
    let hang = Instant::now() + Duration::from_secs(20);
    while b.host_path().is_none() && Instant::now() < hang {
        b.poll(&mut rec);
        std::thread::sleep(Duration::from_millis(3));
    }
    assert!(b.host_path().is_some(), "the host came up");
    assert!(!pump(&mut b, &mut rec, Duration::from_millis(1000), |r| r
        .has(p.id, |e| *e == RawEvent::Started)));
    b.resume().unwrap();
    assert!(pump(
        &mut b,
        &mut rec,
        Duration::from_secs(10),
        finished(p.id)
    ));
    assert_eq!(rec.of(p.id).last(), Some(&RawEvent::Finished));
    assert_eq!(rec.words(p.id).len(), 3);
    assert_eq!(rec.of(u.id).len(), 1, "the stopped one said nothing more");
}

#[test]
fn long_utterances_are_spoken_whole_across_sentences() {
    // Synthesized a sentence at a time in the host; every word is marked
    // once, in order, with rising offsets.
    let mut b = backend(8.0);
    let mut rec = Rec::default();
    let text = "First sentence here. Second one! Third? And the last one.";
    let u = utt(text, 95, 0);
    b.speak(&u, &mut rec).unwrap();
    assert!(pump(
        &mut b,
        &mut rec,
        Duration::from_secs(10),
        finished(u.id)
    ));
    let words = rec.words(u.id);
    let spoken: Vec<&str> = words
        .iter()
        .map(|(r, _)| &text[r.start as usize..r.end as usize])
        .collect();
    assert_eq!(
        spoken,
        [
            "First", "sentence", "here", "Second", "one", "Third", "And", "the", "last", "one"
        ]
    );
    assert!(words.windows(2).all(|w| w[0].1 < w[1].1), "{words:?}");
}
