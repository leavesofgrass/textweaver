//! The backend against the real host binary running its fake engine: no
//! SAPI, no audio device (a silent output consumes samples on a timer).

#![cfg(windows)]

use std::ops::Range;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use textweaver_core::{Rate, Utterance, UtteranceId, Volume};
use textweaver_sapi::voices::Arch;
use textweaver_sapi::{AudioOutput, SapiBackend, SapiConfig};
use textweaver_speech::{Caps, EventSink, RawEvent, SpeechBackend, SpeechError, VoiceParams};

fn host() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_textweaver-sapi-host"))
}

fn config(speed: f32) -> SapiConfig {
    SapiConfig {
        host_x64: Some(host()),
        host_x86: Some(host()),
        onecore: false,
        output: AudioOutput::Null { speed },
        fake_engine: true,
        host_args: Vec::new(),
    }
}

/// A backend at SAPI rate 0, where the fake engine speaks 200 wpm: 300 ms
/// (6,615 samples) per word.
fn backend(speed: f32) -> SapiBackend {
    let mut b = SapiBackend::new(config(speed)).expect("fake host starts");
    b.set_params(&rate0()).unwrap();
    assert_eq!(b.sapi_rate(), 0);
    b
}

fn rate0() -> VoiceParams {
    VoiceParams {
        rate: Rate::Wpm(textweaver_sapi::calibration::MICROSOFT.wpm_at(0)),
        ..VoiceParams::default()
    }
}

/// Records events; utterances of generations below `current` are stale.
#[derive(Default)]
struct Rec {
    events: Vec<(UtteranceId, RawEvent)>,
    current: u64,
}

impl EventSink for Rec {
    fn emit(&mut self, id: UtteranceId, event: RawEvent) {
        self.events.push((id, event));
    }
    fn is_current(&self, id: UtteranceId) -> bool {
        id.generation >= self.current
    }
}

impl Rec {
    fn of(&self, id: UtteranceId) -> Vec<&RawEvent> {
        self.events
            .iter()
            .filter(|(i, _)| *i == id)
            .map(|(_, e)| e)
            .collect()
    }

    fn words(&self, id: UtteranceId) -> Vec<(Range<u32>, u32)> {
        self.of(id)
            .into_iter()
            .filter_map(|e| match e {
                RawEvent::Word {
                    byte_range,
                    audio_ms,
                } => Some((byte_range.clone(), audio_ms.expect("audio clock"))),
                _ => None,
            })
            .collect()
    }

    fn ended(&self, id: UtteranceId) -> bool {
        self.of(id)
            .iter()
            .any(|e| matches!(e, RawEvent::Finished | RawEvent::Cancelled))
    }
}

fn utt(text: &str, generation: u64, chunk: u32) -> Utterance {
    let mut u = Utterance::literal(text, textweaver_core::CharPos(0));
    u.id = UtteranceId { generation, chunk };
    u
}

/// Polls until `done` or the timeout.
fn pump(b: &mut SapiBackend, rec: &mut Rec, timeout: Duration, done: impl Fn(&Rec) -> bool) {
    let t0 = Instant::now();
    while !done(rec) && t0.elapsed() < timeout {
        b.poll(rec);
        std::thread::sleep(Duration::from_millis(3));
    }
}

fn slices<'a>(text: &'a str, words: &[(Range<u32>, u32)]) -> Vec<&'a str> {
    words
        .iter()
        .map(|(r, _)| &text[r.start as usize..r.end as usize])
        .collect()
}

#[test]
fn words_arrive_in_order_with_rising_audio_ms() {
    let mut b = backend(8.0);
    let mut rec = Rec::default();
    let text = "Café crème, naïve 😀 résumé end.";
    let u = utt(text, 1, 0);
    b.speak(&u, &mut rec).unwrap();
    pump(&mut b, &mut rec, Duration::from_secs(10), |r| r.ended(u.id));
    let events = rec.of(u.id);
    assert_eq!(events.first(), Some(&&RawEvent::Started), "{events:?}");
    assert_eq!(events.last(), Some(&&RawEvent::Finished), "{events:?}");
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, RawEvent::Finished | RawEvent::Cancelled))
            .count(),
        1
    );
    let words = rec.words(u.id);
    assert_eq!(
        slices(text, &words),
        ["Café", "crème,", "naïve", "😀", "résumé", "end."]
    );
    assert_eq!(words[0].1, 0);
    assert!(words.windows(2).all(|w| w[0].1 < w[1].1), "{words:?}");
    // 200 wpm at SAPI rate 0 in the fake engine: 300 ms per word.
    assert!((words[1].1 as i64 - 300).abs() <= 1, "{words:?}");
}

#[test]
fn queued_utterances_play_back_to_back_in_order() {
    let mut b = backend(8.0);
    let mut rec = Rec::default();
    let a = utt("alpha beta", 1, 0);
    let c = utt("gamma delta", 1, 1);
    b.speak(&a, &mut rec).unwrap();
    b.speak(&c, &mut rec).unwrap();
    pump(&mut b, &mut rec, Duration::from_secs(10), |r| r.ended(c.id));
    let order: Vec<(u32, String)> = rec
        .events
        .iter()
        .map(|(id, e)| (id.chunk, format!("{e:?}").chars().take(8).collect()))
        .collect();
    let first_c = rec.events.iter().position(|(id, _)| *id == c.id).unwrap();
    let last_a = rec.events.iter().rposition(|(id, _)| *id == a.id).unwrap();
    assert!(last_a < first_c, "{order:?}");
    assert_eq!(rec.of(a.id).last(), Some(&&RawEvent::Finished));
    assert_eq!(slices("gamma delta", &rec.words(c.id)), ["gamma", "delta"]);
}

#[test]
fn stop_mid_utterance_cancels_and_nothing_follows() {
    let mut b = backend(4.0);
    let mut rec = Rec::default();
    let long = "one two three four five six seven eight nine ten eleven twelve";
    let a = utt(long, 1, 0);
    let queued = utt("never heard", 1, 1);
    b.speak(&a, &mut rec).unwrap();
    b.speak(&queued, &mut rec).unwrap();
    pump(&mut b, &mut rec, Duration::from_secs(10), |r| {
        r.words(a.id).len() >= 2
    });
    assert!(rec.words(a.id).len() >= 2);
    b.stop();
    rec.current = 2;
    let before = rec.events.len();
    pump(&mut b, &mut rec, Duration::from_millis(400), |_| false);
    let after: Vec<_> = rec.events[before..].to_vec();
    assert_eq!(
        after,
        [
            (a.id, RawEvent::Cancelled),
            (queued.id, RawEvent::Cancelled)
        ]
    );
    assert!(rec.words(a.id).len() < 12);
    // The host keeps working after a stop.
    let next = utt("after stop", 2, 0);
    b.speak(&next, &mut rec).unwrap();
    pump(&mut b, &mut rec, Duration::from_secs(10), |r| {
        r.ended(next.id)
    });
    assert_eq!(rec.of(next.id).last(), Some(&&RawEvent::Finished));
    assert_eq!(slices("after stop", &rec.words(next.id)), ["after", "stop"]);
    // And nothing more ever arrives for the stopped utterances.
    assert_eq!(rec.of(a.id).last(), Some(&&RawEvent::Cancelled));
    assert_eq!(rec.of(queued.id), [&RawEvent::Cancelled]);
}

#[test]
fn pause_holds_the_clock_and_resume_continues() {
    // Real time: the utterance lasts about 1.8 s. At four times real time
    // (0.45 s) a loaded machine starved the silent output's thread, which
    // then consumed everything it owed at once, so the whole utterance
    // could play between two polls (1 in 40 runs under load).
    let mut b = backend(1.0);
    let mut rec = Rec::default();
    let text = "one two three four five six";
    // The pause must land while most of the utterance is still to come.
    // If the machine held the test off until it had mostly played, the
    // attempt says nothing about pausing, and the test tries again with a
    // new utterance rather than fail on the machine's load.
    let mut attempt = 0;
    let u = loop {
        attempt += 1;
        let u = utt(text, attempt, 0);
        b.speak(&u, &mut rec).unwrap();
        pump(&mut b, &mut rec, Duration::from_secs(10), |r| {
            r.words(u.id).len() >= 2
        });
        b.pause().unwrap();
        let early = rec.words(u.id).len() <= 3 && !rec.ended(u.id);
        if early || attempt == 5 {
            assert!(early, "five attempts all ended before the pause");
            break u;
        }
        b.resume().unwrap();
        b.stop();
        pump(&mut b, &mut rec, Duration::from_secs(10), |r| r.ended(u.id));
    };
    // Anything already due is delivered; after that the clock holds.
    b.poll(&mut rec);
    let held = rec.words(u.id).len();
    pump(&mut b, &mut rec, Duration::from_millis(500), |_| false);
    assert_eq!(rec.words(u.id).len(), held, "no words while paused");
    assert!(!rec.ended(u.id));
    b.resume().unwrap();
    pump(&mut b, &mut rec, Duration::from_secs(10), |r| r.ended(u.id));
    let words = rec.words(u.id);
    assert_eq!(slices(text, &words), text.split(' ').collect::<Vec<_>>());
    // audio_ms counts audio only: the pause does not appear in it.
    assert!(
        words
            .windows(2)
            .all(|w| (w[1].1 - w[0].1).abs_diff(300) <= 1)
    );
    assert_eq!(rec.of(u.id).last(), Some(&&RawEvent::Finished));
}

#[test]
fn voices_come_from_both_hosts_with_x64_first() {
    let b = backend(8.0);
    let details = b.voice_details().unwrap();
    let names: Vec<&str> = details.iter().map(|v| v.voice.name.as_str()).collect();
    assert_eq!(names, ["Fake Alpha", "Fake Wide", "Fake Narrow (32-bit)"]);
    assert_eq!(details[2].arch, Arch::X86);
    assert!(details[2].voice.id.starts_with("x86:"));
    let plain = b.voices().unwrap();
    assert_eq!(plain.len(), 3);
    assert_eq!(plain[0].languages, ["en-US"]);
}

#[test]
fn a_32_bit_voice_runs_in_the_x86_host() {
    let mut b = backend(8.0);
    let narrow = b
        .voice_details()
        .unwrap()
        .into_iter()
        .find(|v| v.arch == Arch::X86)
        .unwrap();
    b.set_params(&VoiceParams {
        voice: Some(narrow.voice.id.clone()),
        ..rate0()
    })
    .unwrap();
    assert!(b.host_path(Arch::X86).is_none(), "started on first use");
    let mut rec = Rec::default();
    let u = utt("narrow voice", 1, 0);
    // The 32-bit host starts without holding up `speak`; the utterance
    // waits for it in `poll`.
    b.speak(&u, &mut rec).unwrap();
    pump(&mut b, &mut rec, Duration::from_secs(10), |r| r.ended(u.id));
    assert!(b.host_path(Arch::X86).is_some());
    assert_eq!(
        slices("narrow voice", &rec.words(u.id)),
        ["narrow", "voice"]
    );
}

#[test]
fn a_host_starting_in_poll_takes_stop_and_pause_meanwhile() {
    // Every host start takes two seconds; the first (64-bit) one waits in
    // `new`, the 32-bit one starts on first use without waiting.
    let mut b = SapiBackend::new(SapiConfig {
        host_args: vec!["--start-delay-ms".into(), "2000".into()],
        ..config(8.0)
    })
    .unwrap();
    b.set_params(&rate0()).unwrap();
    let narrow = b
        .voice_details()
        .unwrap()
        .into_iter()
        .find(|v| v.arch == Arch::X86)
        .unwrap();
    b.set_params(&VoiceParams {
        voice: Some(narrow.voice.id.clone()),
        ..rate0()
    })
    .unwrap();
    let mut rec = Rec::default();
    let u = utt("stopped while starting", 1, 0);
    // These checks use the order of events, not the clock, so a loaded
    // machine cannot fail them: the host is set only when its start
    // completes in `poll`, two seconds after it began, so a `speak` or a
    // `stop` that waited for the start would find it up.
    b.speak(&u, &mut rec).unwrap();
    assert!(
        b.host_path(Arch::X86).is_none(),
        "speak returned before the host was ready"
    );
    b.stop();
    b.poll(&mut rec);
    assert!(
        b.host_path(Arch::X86).is_none(),
        "stop took effect before the host was ready"
    );
    // Cancelled, and never Started.
    assert_eq!(rec.of(u.id), [&RawEvent::Cancelled]);
    let p = utt("paused while starting", 2, 0);
    b.speak(&p, &mut rec).unwrap();
    b.pause().unwrap();
    // Wait for the host (a hang check, not a speed check), then give it
    // time to synthesize: the paused utterance stays silent.
    let hang = Instant::now() + Duration::from_secs(20);
    while b.host_path(Arch::X86).is_none() && Instant::now() < hang {
        b.poll(&mut rec);
        std::thread::sleep(Duration::from_millis(3));
    }
    assert!(b.host_path(Arch::X86).is_some(), "the host came up");
    pump(&mut b, &mut rec, Duration::from_millis(1000), |r| {
        r.of(p.id).contains(&&RawEvent::Started)
    });
    assert!(
        !rec.of(p.id).contains(&&RawEvent::Started),
        "paused: silent"
    );
    assert!(
        !rec.of(u.id).contains(&&RawEvent::Started),
        "the stopped one never started"
    );
    b.resume().unwrap();
    pump(&mut b, &mut rec, Duration::from_secs(10), |r| r.ended(p.id));
    assert_eq!(
        slices("paused while starting", &rec.words(p.id)),
        ["paused", "while", "starting"]
    );
}

#[test]
fn unknown_voices_are_refused() {
    let mut b = backend(8.0);
    // Once the background listing is in (while it loads, an id is taken as
    // given and settled when the list arrives).
    b.voice_details().unwrap();
    let e = b
        .set_params(&VoiceParams {
            voice: Some("x64:HKEY_LOCAL_MACHINE\\nowhere".into()),
            ..VoiceParams::default()
        })
        .unwrap_err();
    assert!(matches!(e, SpeechError::UnknownVoice(_)), "{e:?}");
}

#[test]
fn a_crashed_host_fails_its_utterance_and_restarts() {
    let mut b = backend(8.0);
    let mut rec = Rec::default();
    let bad = utt("__crash__", 1, 0);
    b.speak(&bad, &mut rec).unwrap();
    pump(&mut b, &mut rec, Duration::from_secs(10), |r| {
        r.ended(bad.id)
    });
    let ev = rec.of(bad.id);
    assert!(
        matches!(
            ev.as_slice(),
            [RawEvent::Started, RawEvent::Error(_), RawEvent::Finished]
        ),
        "{ev:?}"
    );
    let good = utt("still here", 1, 1);
    b.speak(&good, &mut rec).unwrap();
    pump(&mut b, &mut rec, Duration::from_secs(10), |r| {
        r.ended(good.id)
    });
    assert_eq!(slices("still here", &rec.words(good.id)), ["still", "here"]);
}

#[test]
fn an_engine_failure_is_reported_on_its_utterance() {
    let mut b = backend(8.0);
    let mut rec = Rec::default();
    let bad = utt("__fail__", 1, 0);
    b.speak(&bad, &mut rec).unwrap();
    pump(&mut b, &mut rec, Duration::from_secs(10), |r| {
        r.ended(bad.id)
    });
    let ev = rec.of(bad.id);
    assert!(
        matches!(ev.as_slice(), [RawEvent::Started, RawEvent::Error(m), RawEvent::Finished] if m.contains("failed on request")),
        "{ev:?}"
    );
}

#[test]
fn synthesize_to_file_writes_a_wav_with_volume_applied() {
    let mut b = backend(8.0);
    let dir = tempfile::tempdir().unwrap();
    let full = b.synthesize("one two").unwrap();
    assert_eq!(full.sample_rate, 22050);
    assert_eq!(full.samples.len(), 2 * 6615);
    assert_eq!(
        full.words
            .iter()
            .map(|(r, s)| (r.clone(), *s))
            .collect::<Vec<_>>(),
        [(0..3, 0), (4..7, 6615)]
    );
    b.set_params(&VoiceParams {
        volume: Volume::new(50),
        ..rate0()
    })
    .unwrap();
    let path = dir.path().join("out.wav");
    b.synthesize_to_file("one two", &path).unwrap();
    let wav = std::fs::read(&path).unwrap();
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(wav.len(), 44 + 2 * 2 * 6615);
    let peak = |s: &[i16]| s.iter().map(|v| v.unsigned_abs()).max().unwrap();
    let half: Vec<i16> = wav[44..]
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect();
    assert_eq!(peak(&half), peak(&full.samples) / 2);
}

#[test]
fn synthesize_utterance_reports_word_timings_on_the_files_clock() {
    let mut b = backend(8.0);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("utt.wav");
    let text = "one naïve three";
    let fs = b.synthesize_utterance(&utt(text, 1, 0), &path).unwrap();
    let wav = std::fs::read(&path).unwrap();
    // Three words of 6,615 samples at 22,050 Hz: 300 ms each.
    assert_eq!(wav.len(), 44 + 2 * 3 * 6615);
    let got: Vec<(&str, u32)> = fs
        .words
        .iter()
        .map(|w| {
            (
                &text[w.byte_range.start as usize..w.byte_range.end as usize],
                w.audio_ms,
            )
        })
        .collect();
    assert_eq!(got, [("one", 0), ("naïve", 300), ("three", 600)]);
}

#[test]
fn rate_maps_through_the_calibration_and_reaches_the_host() {
    let mut b = backend(8.0);
    assert!(
        b.capabilities()
            .contains(Caps::WORD_EVENTS | Caps::AUDIO_CLOCK | Caps::PAUSE)
    );
    b.set_params(&VoiceParams {
        rate: Rate::Wpm(900),
        ..rate0()
    })
    .unwrap();
    assert_eq!(b.sapi_rate(), 10);
    assert_eq!(
        b.effective_wpm(),
        textweaver_sapi::calibration::MICROSOFT.wpm_at(10)
    );
    // The fake engine speaks 3x faster at +10: 100 ms per word.
    let s = b.synthesize("one two").unwrap();
    assert_eq!(s.words[1].1, 2205);
}

#[test]
fn capabilities_declare_playback_events_and_follow_the_output() {
    let b = backend(8.0);
    let caps = b.capabilities();
    // The backend plays the audio, so word events arrive as words are heard.
    assert!(caps.contains(Caps::PLAYBACK_EVENTS | Caps::SYNTH_TO_FILE | Caps::VOLUME));
    // Microsoft voices keep textweaver's own normalization.
    assert!(!caps.contains(Caps::NATIVE_NORMALIZATION));
    // A silent output cannot play tones.
    assert!(!caps.contains(Caps::TONES));
}

#[test]
fn a_host_stuck_in_synthesis_exits_when_its_input_closes() {
    // Before: the host of a voice stuck in synthesis lived on after
    // textweaver closed its input (textweaver exited or crashed).
    use textweaver_enginehost::{HostMsg, HostProcess};
    use textweaver_sapi::protocol::{Reply, Request};
    let mut h = HostProcess::<Reply>::spawn(&host(), ["--engine", "fake"], "sapi-test")
        .expect("the fake host starts");
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut next = || h.recv_until(deadline).expect("the host answers in time");
    while !matches!(next(), HostMsg::Reply(Reply::Ready { .. })) {}
    h.send(&Request::Speak {
        token: 1,
        text: "one __hang__".into(),
        pitch: 0,
    })
    .unwrap();
    // The first word says synthesis has started.
    loop {
        match h.recv_until(deadline) {
            Some(HostMsg::Reply(Reply::Word { token: 1, .. })) => break,
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
