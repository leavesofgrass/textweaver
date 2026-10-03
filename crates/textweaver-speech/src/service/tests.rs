//! Service tests with the recording backend and a fake clock (ADR-0003):
//! queue order, generations and chunk cancellation, stale events, every
//! pause/resume edge case, pacing, audio-clock scheduling, say modes,
//! characters, tones, and normalization.

use std::time::Duration;

use textweaver_core::{CharPos, CharRange, SpokenBuilder, Utterance, UtteranceKind};

use super::*;
use crate::backends::{Call, RecordingBackend, RecordingHandle, RecordingMode};
use crate::pacing::FakeClock;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

#[test]
fn word_lateness_is_summarized_in_words_and_starts_again() {
    let mut l = Lateness::default();
    l.step(ms(0));
    for i in 0..9 {
        l.step(ms(10 * (i + 1)));
        l.word(ms(i));
    }
    // An idle gap between readings is not a step.
    l.step(ms(10_000));
    let line = l.summary();
    assert!(
        line.starts_with("speech timing: 9 scheduled words late by median 4.0 ms"),
        "{line}"
    );
    assert!(line.contains("timer steps median 10.0 ms"), "{line}");
    assert!(l.late_ms.is_empty() && l.step_ms.is_empty());
}

/// A config that leaves plain words untouched.
fn plain() -> ServiceConfig {
    ServiceConfig {
        normalize: NormalizeConfig::none(),
        ..ServiceConfig::default()
    }
}

struct Rig {
    core: ServiceCore,
    rec: RecordingHandle,
    clock: FakeClock,
}

impl Rig {
    fn new(mode: RecordingMode, caps: Caps, config: ServiceConfig) -> Self {
        let clock = FakeClock::new();
        let (b, rec) = RecordingBackend::with(mode, caps);
        let b = b.with_clock(Box::new(clock.clone()));
        let core = ServiceCore::new(Box::new(b), config, Box::new(clock.clone()));
        Rig { core, rec, clock }
    }

    fn manual() -> Self {
        Self::new(
            RecordingMode::Manual,
            RecordingBackend::DEFAULT_CAPS,
            plain(),
        )
    }

    fn instant() -> Self {
        Self::new(
            RecordingMode::Instant,
            RecordingBackend::DEFAULT_CAPS,
            plain(),
        )
    }

    /// Steps the core and returns the statuses produced so far.
    fn step(&mut self) -> Vec<SpeechStatus> {
        self.core.step();
        self.core.take_statuses()
    }

    fn advance(&mut self, d: Duration) -> Vec<SpeechStatus> {
        self.clock.advance(d);
        self.step()
    }

    fn spoken(&self) -> Vec<Utterance> {
        self.rec.spoken()
    }

    fn last_spoken(&self) -> Utterance {
        self.rec.spoken().pop().expect("something was spoken")
    }
}

/// Literal utterances for consecutive sentences separated by one space,
/// starting at `start`.
fn doc(start: usize, sentences: &[&str]) -> Vec<Utterance> {
    let mut pos = start;
    sentences
        .iter()
        .map(|s| {
            let u = Utterance::literal(*s, CharPos(pos));
            pos += s.chars().count() + 1;
            u
        })
        .collect()
}

fn positions(statuses: &[SpeechStatus]) -> Vec<Option<CharRange>> {
    statuses
        .iter()
        .filter_map(|s| match s {
            SpeechStatus::Position { source_range, .. } => Some(*source_range),
            _ => None,
        })
        .collect()
}

fn r(a: usize, b: usize) -> Option<CharRange> {
    Some(CharRange::new(a, b))
}

fn count(statuses: &[SpeechStatus], want: &SpeechStatus) -> usize {
    statuses.iter().filter(|s| *s == want).count()
}

/// `Finished` for the first reading of a rig.
const FIN: SpeechStatus = SpeechStatus::Finished { generation: 1 };
/// `Stopped` while the first reading of a rig is the latest.
const STOPPED: SpeechStatus = SpeechStatus::Stopped { generation: 1 };

fn generations(statuses: &[SpeechStatus]) -> Vec<ReadingGeneration> {
    statuses
        .iter()
        .filter_map(SpeechStatus::generation)
        .collect()
}

// ---- queue order, lookahead, generations ---------------------------------

#[test]
fn reads_in_order_with_word_positions_then_finished_once() {
    let mut rig = Rig::instant();
    rig.core.read(doc(0, &["One two.", "Three four."]));
    let st = rig.step();
    assert_eq!(
        positions(&st),
        [r(0, 3), r(4, 7), r(9, 14), r(15, 19)],
        "{st:?}"
    );
    assert_eq!(count(&st, &FIN), 1);
    assert_eq!(st.last(), Some(&FIN));
    assert_eq!(
        rig.rec.spoken_texts(),
        ["One two.", "Three four."],
        "queue order"
    );
    let ids: Vec<UtteranceId> = rig.spoken().iter().map(|u| u.id).collect();
    assert_eq!(ids[0].generation, ids[1].generation);
    assert_eq!((ids[0].chunk, ids[1].chunk), (0, 1));
}

#[test]
fn empty_read_finishes_at_once() {
    let mut rig = Rig::manual();
    rig.core.read(Vec::new());
    assert_eq!(rig.step(), [FIN]);
}

#[test]
fn engine_holds_two_chunks_of_lookahead() {
    let mut rig = Rig::manual();
    rig.core
        .read(doc(0, &["A a.", "B b.", "C c.", "D d.", "E e."]));
    rig.step();
    assert_eq!(rig.spoken().len(), 3, "playing + two lookahead");
    let first = rig.spoken()[0].id;
    rig.rec.start(first);
    rig.rec.finish(first);
    rig.step();
    assert_eq!(rig.rec.spoken_texts(), ["A a.", "B b.", "C c.", "D d."]);
}

#[test]
fn a_new_read_makes_old_events_stale() {
    let mut rig = Rig::manual();
    rig.core.read(doc(0, &["Old words here."]));
    rig.step();
    let old = rig.spoken()[0].id;
    rig.core.read(doc(100, &["New words."]));
    assert!(rig.rec.calls().contains(&Call::Stop));
    // The old engine copy reports late.
    rig.rec.start(old);
    assert!(rig.rec.word(old, 1, None));
    rig.rec.finish(old);
    let st = rig.step();
    assert!(positions(&st).is_empty(), "{st:?}");
    assert!(!st.contains(&FIN));
    let new = rig.last_spoken();
    assert!(new.id.generation > old.generation);
    rig.rec.start(new.id);
    let st = rig.step();
    assert_eq!(positions(&st), [r(100, 103)]);
    assert_eq!(
        generations(&st),
        [2],
        "the new reading's statuses carry its generation"
    );
}

// ---- reading generations (Wave 1 request from the app) ----------------------

#[test]
fn read_returns_rising_generations_that_statuses_carry() {
    let mut rig = Rig::instant();
    assert_eq!(rig.core.reading_generation(), 0);
    let g1 = rig.core.read(doc(0, &["One two."]));
    let st = rig.step();
    assert_eq!(g1, 1);
    assert!(!st.is_empty());
    assert!(generations(&st).iter().all(|&g| g == g1), "{st:?}");
    assert_eq!(st.last(), Some(&SpeechStatus::Finished { generation: g1 }));
    let g2 = rig.core.read(Vec::new());
    assert_eq!(g2, 2);
    assert_eq!(rig.step(), [SpeechStatus::Finished { generation: 2 }]);
}

#[test]
fn a_reading_keeps_its_generation_across_pause_resume_and_announcements() {
    let mut rig = Rig::manual();
    let g = rig.core.read(doc(0, &["One two three.", "Four five."]));
    rig.step();
    let first = rig.spoken()[0].id;
    rig.rec.start(first);
    rig.rec.word(first, 1, None);
    rig.step();
    rig.core.pause();
    let paused = rig.core.take_statuses();
    assert!(matches!(paused[..], [SpeechStatus::Paused { generation, .. }] if generation == g));
    rig.core.resume();
    rig.step();
    let resumed = rig.last_spoken();
    assert!(
        resumed.id.generation > first.generation,
        "engine generation moved on"
    );
    rig.rec.start(resumed.id);
    rig.rec.word(resumed.id, 0, None);
    rig.core.say("Note", SayMode::Announce);
    let st = rig.step();
    assert!(generations(&st).iter().all(|&x| x == g), "{st:?}");
    // Finish everything still pending: the reading reports its generation.
    for _ in 0..6 {
        for id in rig.rec.pending() {
            rig.rec.start(id);
            rig.rec.finish(id);
        }
        let st = rig.step();
        if st.contains(&SpeechStatus::Finished { generation: g }) {
            return;
        }
    }
    panic!("the reading never finished");
}

#[test]
fn stop_and_interrupt_carry_the_latest_generation() {
    let mut rig = Rig::manual();
    rig.core.read(doc(0, &["Alpha."]));
    let g = rig.core.read(doc(10, &["Beta gamma."]));
    rig.step();
    rig.core.take_statuses();
    rig.core.say("Hello", SayMode::Interrupt);
    assert_eq!(
        rig.core.take_statuses(),
        [SpeechStatus::Stopped { generation: g }]
    );
    rig.core.stop();
    assert_eq!(
        rig.core.take_statuses(),
        [SpeechStatus::Stopped { generation: g }]
    );
}

#[test]
fn threaded_read_returns_the_generation_statuses_carry() {
    let (b, _rec) = RecordingBackend::new();
    let s = SpeechService::spawn(b.into_factory(), plain()).unwrap();
    let g1 = s.read(doc(0, &["One."]));
    let g2 = s.read(doc(5, &["Two three."]));
    assert_eq!((g1, g2), (1, 2));
    let mut got = Vec::new();
    while let Ok(st) = s.statuses().recv_timeout(Duration::from_secs(5)) {
        let done = st == SpeechStatus::Finished { generation: g2 };
        got.push(st);
        if done {
            break;
        }
    }
    let latest: Vec<Option<CharRange>> = got
        .iter()
        .filter_map(|st| match st {
            SpeechStatus::Position {
                generation,
                source_range,
                ..
            } if *generation == g2 => Some(*source_range),
            _ => None,
        })
        .collect();
    assert_eq!(latest, [r(5, 8), r(9, 14)]);
}

// ---- capabilities follow the voice (Wave 1 request from the SAPI backend) --

/// A backend whose voice "quiet" has no word events, like some SAPI voices.
struct VoiceCaps {
    inner: RecordingBackend,
    caps: Caps,
}

impl SpeechBackend for VoiceCaps {
    fn id(&self) -> BackendId {
        "voice-caps"
    }
    fn capabilities(&self) -> Caps {
        self.caps
    }
    fn voices(&self) -> Result<Vec<crate::Voice>, SpeechError> {
        self.inner.voices()
    }
    fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError> {
        self.caps = if params.voice.as_deref() == Some("quiet") {
            Caps::PITCH | Caps::NATIVE_NORMALIZATION
        } else {
            RecordingBackend::DEFAULT_CAPS
        };
        self.inner.set_params(params)
    }
    fn effective_wpm(&self) -> u16 {
        self.inner.effective_wpm()
    }
    fn speak(&mut self, u: &Utterance, sink: &mut dyn EventSink) -> Result<(), SpeechError> {
        self.inner.speak(u, sink)
    }
    fn poll(&mut self, sink: &mut dyn EventSink) {
        self.inner.poll(sink);
    }
    fn stop(&mut self) {
        self.inner.stop();
    }
}

#[test]
fn capabilities_are_re_read_after_set_params() {
    let (inner, _rec) = RecordingBackend::new();
    let backend = VoiceCaps {
        inner,
        caps: RecordingBackend::DEFAULT_CAPS,
    };
    let mut core = ServiceCore::new(
        Box::new(backend),
        ServiceConfig::default(),
        Box::new(FakeClock::new()),
    );
    assert!(core.take_statuses().is_empty(), "no status for the start");
    assert!(core.pipeline().names().contains(&"numbers"));
    core.set_voice(Some("quiet".into()));
    let caps = Caps::PITCH | Caps::NATIVE_NORMALIZATION;
    assert_eq!(core.take_statuses(), [SpeechStatus::Capabilities { caps }]);
    assert_eq!(core.capabilities(), caps);
    assert!(
        !core.pipeline().names().contains(&"numbers"),
        "native normalization now skips numbers"
    );
    core.set_rate(Rate::Wpm(300));
    assert!(
        core.take_statuses().is_empty(),
        "unchanged caps are not reported"
    );
    core.set_voice(None);
    assert_eq!(
        core.take_statuses(),
        [SpeechStatus::Capabilities {
            caps: RecordingBackend::DEFAULT_CAPS
        }]
    );
}

#[test]
fn the_handle_follows_capability_changes() {
    let (inner, _rec) = RecordingBackend::new();
    let backend = VoiceCaps {
        inner,
        caps: RecordingBackend::DEFAULT_CAPS,
    };
    let s = SpeechService::spawn(
        Box::new(move || Ok(Box::new(backend) as Box<dyn SpeechBackend>)),
        plain(),
    )
    .unwrap();
    assert!(s.capabilities().contains(Caps::WORD_EVENTS));
    s.set_voice(Some("quiet".into()));
    let st = s.statuses().recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(st, SpeechStatus::Capabilities { .. }), "{st:?}");
    assert!(!s.capabilities().contains(Caps::WORD_EVENTS));
}

#[test]
fn stop_reports_stopped_and_drops_late_events() {
    let mut rig = Rig::manual();
    rig.core.read(doc(0, &["Alpha beta."]));
    rig.step();
    let id = rig.spoken()[0].id;
    rig.core.stop();
    assert_eq!(rig.core.take_statuses(), [STOPPED]);
    rig.rec.start(id);
    rig.rec.word(id, 1, None);
    rig.rec.finish(id);
    assert!(rig.step().is_empty());
}

#[test]
fn skip_cancels_the_playing_chunk_by_id() {
    let mut rig = Rig::manual();
    rig.core
        .read(doc(0, &["First one.", "Second one.", "Third one."]));
    rig.step();
    let first = rig.spoken()[0].id;
    rig.rec.start(first);
    rig.step();
    rig.core.skip();
    let calls = rig.rec.calls();
    assert!(calls.contains(&Call::Stop));
    let resubmitted: Vec<String> = rig.spoken()[3..].iter().map(|u| u.text.clone()).collect();
    assert_eq!(resubmitted, ["Second one.", "Third one."]);
    // Late events from the skipped chunk are stale.
    rig.rec.word(first, 1, None);
    assert!(positions(&rig.step()).is_empty());
}

#[test]
fn backend_error_is_reported_and_reading_goes_on() {
    let mut rig = Rig::instant();
    rig.rec.fail_next_speak("synth failed");
    rig.core.read(doc(0, &["Bad one.", "Good one."]));
    let st = rig.step();
    assert!(st.contains(&SpeechStatus::BackendError(
        "engine error: synth failed".into()
    )));
    assert_eq!(positions(&st), [r(9, 13), r(14, 17)]);
    assert_eq!(st.last(), Some(&FIN));
}

#[test]
fn an_engine_that_always_fails_stops_the_reading_once() {
    // Before: every remaining sentence was tried in one pump, each
    // reporting its own error (a whole book's worth at once).
    let mut rig = Rig::instant();
    rig.rec.fail_every_speak(Some("host did not start".into()));
    let sentences: Vec<String> = (0..200).map(|i| format!("Sentence {i}.")).collect();
    let refs: Vec<&str> = sentences.iter().map(String::as_str).collect();
    rig.core.read(doc(0, &refs));
    let st = rig.step();
    let speaks = rig
        .rec
        .calls()
        .iter()
        .filter(|c| matches!(c, Call::Speak(_)))
        .count();
    assert_eq!(speaks, MAX_CONSECUTIVE_FAILURES as usize);
    let errors: Vec<&SpeechStatus> = st
        .iter()
        .filter(|s| matches!(s, SpeechStatus::BackendError(_)))
        .collect();
    assert_eq!(
        errors,
        [
            &SpeechStatus::BackendError("engine error: host did not start".into()),
            &SpeechStatus::BackendError(
                "the speech engine failed 3 times in a row, so speech stopped".into()
            ),
        ]
    );
    assert_eq!(st.last(), Some(&STOPPED));
    assert!(!rig.core.is_active());
}

#[test]
fn a_repeated_error_is_not_reported_again_within_the_window() {
    // A frontend that voices "Speech error" through the failing engine gets
    // no new error back, so it cannot loop.
    let mut rig = Rig::instant();
    rig.rec.fail_every_speak(Some("no audio device".into()));
    rig.core.read(doc(0, &["One.", "Two.", "Three.", "Four."]));
    let first = rig.step();
    assert!(
        first
            .iter()
            .any(|s| matches!(s, SpeechStatus::BackendError(_)))
    );
    for _ in 0..5 {
        rig.core
            .say("Speech error: no audio device", SayMode::Announce);
        let st = rig.step();
        assert!(
            !st.iter()
                .any(|s| matches!(s, SpeechStatus::BackendError(_))),
            "{st:?}"
        );
    }
    // Later, the same failure is reported again.
    rig.clock.advance(ERROR_REPEAT_WINDOW + ms(1));
    rig.core.say("Hello.", SayMode::Interrupt);
    let st = rig.step();
    assert!(st.contains(&SpeechStatus::BackendError(
        "engine error: no audio device".into()
    )));
    // A working engine resets the count: one failure no longer stops.
    rig.rec.fail_every_speak(None);
    rig.clock.advance(ERROR_REPEAT_WINDOW + ms(1));
    rig.core.say("Works.", SayMode::Interrupt);
    rig.step();
    rig.rec.fail_next_speak("glitch");
    rig.core.read(doc(0, &["Bad one.", "Good one."]));
    let st = rig.step();
    assert_eq!(positions(&st), [r(9, 13), r(14, 17)]);
    assert_eq!(st.last(), Some(&SpeechStatus::Finished { generation: 2 }));
}

fn restarted(statuses: &[SpeechStatus]) -> Vec<String> {
    statuses
        .iter()
        .filter_map(|s| match s {
            SpeechStatus::Restarted { generation, reason } => {
                assert_eq!(*generation, 1);
                Some(reason.clone())
            }
            _ => None,
        })
        .collect()
}

fn errors(statuses: &[SpeechStatus]) -> Vec<String> {
    statuses
        .iter()
        .filter_map(|s| match s {
            SpeechStatus::BackendError(e) => Some(e.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_host_crash_resumes_from_the_last_confirmed_word() {
    // Before: every utterance the dead host owed failed, and the reading
    // skipped the rest of the sentence and the two lookahead sentences
    // (audit finding R4).
    let mut rig = Rig::manual();
    rig.core.read(doc(
        0,
        &["One two three four.", "Five six.", "Seven eight."],
    ));
    rig.step();
    let first = rig.spoken()[0].id;
    rig.rec.start(first);
    rig.rec.word(first, 0, None);
    rig.rec.word(first, 1, None);
    rig.step();
    let before = rig.spoken().len();
    for u in rig.spoken() {
        rig.rec
            .emit(u.id, RawEvent::Error("engine host exited".into()));
    }
    let st = rig.step();
    assert_eq!(restarted(&st), ["engine host exited"], "{st:?}");
    assert!(errors(&st).is_empty(), "{st:?}");
    assert!(!st.contains(&FIN));
    // The engine was reset and read on from "two".
    assert!(rig.rec.calls().contains(&Call::Stop));
    let again: Vec<String> = rig.spoken()[before..]
        .iter()
        .map(|u| u.text.clone())
        .collect();
    assert_eq!(again, ["two three four.", "Five six.", "Seven eight."]);
    // The highlight goes on in the same reading, in document order.
    let id = rig.spoken()[before].id;
    rig.rec.start(id);
    rig.rec.word(id, 0, None);
    let st = rig.step();
    assert_eq!(positions(&st), [r(4, 7)]);
    assert!(st.iter().all(|s| s.generation().is_none_or(|g| g == 1)));
}

#[test]
fn a_second_crash_without_progress_reports_and_moves_on() {
    let mut rig = Rig::manual();
    rig.core.read(doc(0, &["Only one.", "Then two."]));
    rig.step();
    let id = rig.spoken()[0].id;
    rig.rec.emit(id, RawEvent::Error("device lost".into()));
    let st = rig.step();
    assert_eq!(restarted(&st), ["device lost"]);
    // It fails again at once: reported, and the reading goes on without it.
    let id = rig.spoken()[2].id;
    assert_eq!(rig.spoken()[2].text, "Only one.");
    rig.rec.emit(id, RawEvent::Error("device lost".into()));
    let st = rig.step();
    assert_eq!(errors(&st), ["device lost"]);
    assert!(restarted(&st).is_empty());
    let next = rig.rec.pending();
    assert!(!next.is_empty());
    for id in next {
        rig.rec.start(id);
        rig.rec.word(id, 0, None);
        rig.rec.finish(id);
    }
    let st = rig.step();
    assert_eq!(st.last(), Some(&FIN));
}

/// Crashes every utterance the engine holds (a host that died owes them
/// all).
fn crash_all(rig: &Rig, why: &str) {
    for id in rig.rec.pending() {
        rig.rec.emit(id, RawEvent::Error(why.into()));
    }
}

/// The first utterance spoken after `before` calls: the restarted reading.
fn restarted_front(rig: &Rig, before: usize) -> Utterance {
    rig.spoken()[before..]
        .first()
        .cloned()
        .expect("the reading was restarted")
}

#[test]
fn an_engine_that_crashes_on_the_same_word_is_not_restarted_forever() {
    // Before: the repeated resume word counted as progress, so an engine
    // that crashed on the same text was restarted forever.
    let mut rig = Rig::manual();
    rig.core.read(doc(0, &["One two three four.", "Five six."]));
    rig.step();
    let first = rig.spoken()[0].id;
    rig.rec.start(first);
    rig.rec.word(first, 0, None);
    rig.rec.word(first, 1, None);
    rig.step();
    let mut restarts = 0;
    for _ in 0..10 {
        let before = rig.spoken().len();
        crash_all(&rig, "engine host exited");
        let st = rig.step();
        restarts += restarted(&st).len();
        if !rig.core.is_active() || rig.spoken().len() == before {
            break;
        }
        // The new engine says the resume word again, then crashes on the
        // same word as before.
        let again = restarted_front(&rig, before);
        if again.text != "two three four." {
            break;
        }
        rig.rec.start(again.id);
        rig.rec.word(again.id, 0, None);
        rig.step();
    }
    assert_eq!(restarts, 1, "restarted once, not in a loop");
}

#[test]
fn progress_past_the_resume_point_allows_another_restart() {
    let mut rig = Rig::manual();
    rig.core.read(doc(0, &["One two three four five.", "Six."]));
    rig.step();
    let first = rig.spoken()[0].id;
    rig.rec.start(first);
    rig.rec.word(first, 0, None);
    rig.rec.word(first, 1, None);
    rig.step();
    let before = rig.spoken().len();
    crash_all(&rig, "engine host exited");
    assert_eq!(restarted(&rig.step()), ["engine host exited"]);
    // The new engine gets past "two": a later crash is a new one.
    let again = restarted_front(&rig, before);
    assert_eq!(again.text, "two three four five.");
    rig.rec.start(again.id);
    rig.rec.word(again.id, 0, None);
    rig.rec.word(again.id, 1, None);
    rig.step();
    let before = rig.spoken().len();
    crash_all(&rig, "engine host exited");
    let st = rig.step();
    assert_eq!(restarted(&st), ["engine host exited"], "{st:?}");
    assert_eq!(
        restarted_front(&rig, before).text,
        "three four five.",
        "reads on from the last confirmed word"
    );
}

#[test]
fn restarts_in_one_sentence_are_capped_then_the_reading_stops() {
    let words: Vec<String> = (0..20).map(|i| format!("w{i}")).collect();
    let sentence = format!("{}.", words.join(" "));
    let mut rig = Rig::manual();
    rig.core.read(doc(0, &[&sentence, "After."]));
    rig.step();
    let mut id = rig.spoken()[0].id;
    let mut all = Vec::new();
    for _ in 0..=MAX_RESTARTS_PER_UTTERANCE {
        // Some progress each time, then a crash.
        rig.rec.start(id);
        rig.rec.word(id, 0, None);
        rig.rec.word(id, 1, None);
        rig.rec.word(id, 2, None);
        all.extend(rig.step());
        let before = rig.spoken().len();
        crash_all(&rig, "engine host exited");
        all.extend(rig.step());
        if rig.spoken().len() == before {
            break;
        }
        id = restarted_front(&rig, before).id;
    }
    assert_eq!(
        restarted(&all).len(),
        MAX_RESTARTS_PER_UTTERANCE as usize,
        "{all:?}"
    );
    assert_eq!(
        errors(&all),
        [
            "the speech engine stopped 4 times in the same sentence (engine host exited), so reading stopped"
        ]
    );
    assert_eq!(all.last(), Some(&STOPPED));
    assert!(!rig.core.is_active());
    // The next reading starts with a fresh count.
    rig.core.read(doc(0, &["New reading."]));
    rig.step();
    let id = rig.rec.pending()[0];
    rig.rec.start(id);
    rig.rec.word(id, 0, None);
    rig.step();
    crash_all(&rig, "engine host exited");
    let st = rig.step();
    assert_eq!(
        st.iter()
            .filter(|s| matches!(s, SpeechStatus::Restarted { .. }))
            .count(),
        1,
        "{st:?}"
    );
}

#[test]
fn a_stall_after_only_the_resume_word_stops_the_reading() {
    // Before: the resume word reset the count, so a device that played one
    // word and stalled again was restarted every 12 seconds forever.
    let mut rig = Rig::manual();
    rig.core.read(doc(0, &["One two three.", "Four five."]));
    rig.step();
    let first = rig.spoken()[0].id;
    rig.rec.start(first);
    rig.rec.word(first, 0, None);
    rig.rec.word(first, 1, None);
    rig.step();
    let before = rig.spoken().len();
    let st = rig.advance(STALL_TIMEOUT + ms(10));
    assert_eq!(restarted(&st), ["no speech for 12 seconds"]);
    let again = restarted_front(&rig, before);
    rig.rec.start(again.id);
    rig.rec.word(again.id, 0, None);
    rig.step();
    let st = rig.advance(STALL_TIMEOUT + ms(10));
    assert!(restarted(&st).is_empty(), "{st:?}");
    assert_eq!(st.last(), Some(&STOPPED));
}

#[test]
fn a_stalled_reading_is_restarted_once_then_stopped() {
    let mut rig = Rig::manual();
    rig.core.read(doc(0, &["One two.", "Three four."]));
    rig.step();
    let first = rig.spoken()[0].id;
    rig.rec.start(first);
    rig.rec.word(first, 0, None);
    rig.step();
    // Not yet stalled.
    let st = rig.advance(STALL_TIMEOUT - ms(100));
    assert!(restarted(&st).is_empty(), "{st:?}");
    // No word for STALL_TIMEOUT: reset and read on from "One".
    let before = rig.spoken().len();
    let st = rig.advance(ms(200));
    assert_eq!(restarted(&st), ["no speech for 12 seconds"], "{st:?}");
    assert_eq!(rig.spoken()[before].text, "One two.");
    // Still nothing: the reading stops with a message.
    let st = rig.advance(STALL_TIMEOUT + ms(10));
    assert!(restarted(&st).is_empty());
    assert_eq!(
        errors(&st),
        [
            "no speech for 12 seconds, even after restarting the voice, so reading stopped. Check the audio device"
        ]
    );
    assert_eq!(st.last(), Some(&STOPPED));
    assert!(!rig.core.is_active());
}

#[test]
fn progress_keeps_the_watchdog_quiet_and_paused_readings_never_stall() {
    let mut rig = Rig::manual();
    rig.core.read(doc(0, &["One two three.", "Four five."]));
    rig.step();
    let first = rig.spoken()[0].id;
    rig.rec.start(first);
    // A word every few seconds: never a stall.
    for n in 0..3 {
        rig.rec.word(first, n, None);
        let st = rig.advance(STALL_TIMEOUT / 2);
        assert!(restarted(&st).is_empty(), "{st:?}");
    }
    // Paused for a long time: not a stall.
    rig.core.pause();
    let st = rig.advance(STALL_TIMEOUT * 3);
    assert!(restarted(&st).is_empty(), "{st:?}");
    assert!(errors(&st).is_empty(), "{st:?}");
}

// ---- normalization and mapping ----------------------------------------------

#[test]
fn word_events_are_mapped_through_the_normalized_offset_map() {
    let mut rig = Rig::new(
        RecordingMode::Instant,
        RecordingBackend::DEFAULT_CAPS,
        ServiceConfig::default(),
    );
    rig.core
        .read(vec![Utterance::literal("Dr. Smith paid $5.", CharPos(40))]);
    let st = rig.step();
    assert_eq!(rig.rec.spoken_texts(), ["Doctor Smith paid five dollars."]);
    // "five" and "dollars" both highlight "$5"; reported once.
    assert_eq!(positions(&st), [r(40, 43), r(44, 49), r(50, 54), r(55, 57)]);
    for u in rig.spoken() {
        u.offset_map.check_invariants(&u.text).unwrap();
    }
}

#[test]
fn native_normalization_engines_get_raw_numbers() {
    let mut rig = Rig::new(
        RecordingMode::Instant,
        RecordingBackend::DEFAULT_CAPS | Caps::NATIVE_NORMALIZATION,
        ServiceConfig::default(),
    );
    rig.core
        .read(vec![Utterance::literal("Dr. Smith paid $5.", CharPos(0))]);
    rig.step();
    assert_eq!(rig.rec.spoken_texts(), ["Dr. Smith paid $5."]);
}

#[test]
fn inserted_speech_reports_no_range() {
    let mut rig = Rig::instant();
    let mut b = SpokenBuilder::new();
    b.push_inserted("heading level 2, ", CharPos(50));
    b.push_literal("Intro", CharPos(50));
    let (text, map) = b.finish();
    rig.core.read(vec![Utterance::with_map(text, map)]);
    let st = rig.step();
    assert_eq!(positions(&st), [None, r(50, 55)]);
}

#[test]
fn punctuation_and_split_caps_settings_apply_to_later_reads() {
    let mut rig = Rig::instant();
    rig.core.set_split_caps(true);
    rig.core.set_punctuation(PunctuationLevel::All);
    rig.core
        .read(vec![Utterance::literal("camelCase, ok", CharPos(0))]);
    rig.step();
    assert_eq!(rig.rec.spoken_texts(), ["camel Case comma ok"]);
    let mut n = NormalizeConfig::none();
    n.pronunciations = [("ok".to_owned(), "okay".to_owned())].into();
    n.use_pronunciations = true;
    rig.core.set_normalization(n);
    rig.core.set_split_caps(false);
    rig.core.set_punctuation(PunctuationLevel::Some);
    rig.core
        .read(vec![Utterance::literal("camelCase, ok", CharPos(0))]);
    rig.step();
    assert_eq!(rig.rec.spoken_texts()[1], "camelCase, okay");
}

#[test]
fn spoken_math_highlights_the_formula_word_by_word() {
    let mut rig = Rig::instant();
    let mut n = NormalizeConfig::none();
    n.math = true;
    n.numbers = true;
    rig.core.set_normalization(n);
    // "Area $x^2$ costs $5." at document char 100.
    rig.core.read(vec![Utterance::literal(
        "Area $x^2$ costs $5.",
        CharPos(100),
    )]);
    let st = rig.step();
    assert_eq!(
        rig.rec.spoken_texts(),
        ["Area x squared costs five dollars."]
    );
    assert_eq!(
        positions(&st),
        [
            r(100, 104), // Area
            r(106, 107), // x
            r(107, 109), // squared: ^2
            r(111, 116), // costs
            r(117, 119), // five dollars: $5 (one position for both words)
        ]
    );
}

// ---- pause and resume -------------------------------------------------------

#[test]
fn pause_before_the_first_word_resumes_at_the_utterance_start() {
    let mut rig = Rig::manual();
    rig.core.read(doc(20, &["One two three.", "Four five."]));
    rig.step();
    rig.core.pause();
    assert_eq!(
        rig.core.take_statuses(),
        [SpeechStatus::Paused {
            generation: 1,
            resume_at: Some(CharPos(20))
        }]
    );
    assert!(
        rig.rec.calls().contains(&Call::Stop),
        "emulated pause stops"
    );
    let before = rig.spoken().len();
    rig.core.resume();
    let again: Vec<String> = rig.spoken()[before..]
        .iter()
        .map(|u| u.text.clone())
        .collect();
    assert_eq!(again, ["One two three.", "Four five."]);
}

#[test]
fn pause_resumes_from_the_last_confirmed_word() {
    let mut rig = Rig::manual();
    rig.core
        .read(doc(10, &["Alpha beta gamma delta.", "Next one."]));
    rig.step();
    let id = rig.spoken()[0].id;
    rig.rec.start(id);
    for w in 0..3 {
        rig.rec.word(id, w, None);
    }
    assert_eq!(positions(&rig.step()), [r(10, 15), r(16, 20), r(21, 26)]);
    rig.core.pause();
    assert_eq!(
        rig.core.take_statuses(),
        [SpeechStatus::Paused {
            generation: 1,
            resume_at: Some(CharPos(21))
        }]
    );
    // A late word event after the pause is stale.
    rig.rec.word(id, 3, None);
    assert!(rig.step().is_empty());
    assert!(rig.core.is_paused());
    let before = rig.spoken().len();
    rig.core.resume();
    let resumed = &rig.spoken()[before];
    assert_eq!(resumed.text, "gamma delta.");
    resumed.offset_map.check_invariants(&resumed.text).unwrap();
    assert!(resumed.id.generation > id.generation);
    rig.rec.start(resumed.id);
    rig.rec.word(resumed.id, 1, None);
    assert_eq!(positions(&rig.step()), [r(21, 26), r(27, 32)]);
}

#[test]
fn resume_from_a_moved_cursor() {
    let mut rig = Rig::manual();
    rig.core
        .read(doc(0, &["Alpha beta gamma.", "Delta epsilon zeta."]));
    rig.step();
    rig.core.pause();
    rig.core.take_statuses();
    let before = rig.spoken().len();
    // Cursor moved into the second sentence.
    rig.core.resume_at(CharPos(24));
    let texts: Vec<String> = rig.spoken()[before..]
        .iter()
        .map(|u| u.text.clone())
        .collect();
    assert_eq!(texts, ["epsilon zeta."]);
    // Cursor inside the first sentence, mid-word: resume from that char.
    rig.core.pause();
    rig.core.take_statuses();
    let before = rig.spoken().len();
    rig.core.resume_at(CharPos(27));
    assert_eq!(rig.spoken()[before].text, "ilon zeta.");
}

#[test]
fn pause_inside_inserted_speech_repeats_it_and_reports_its_anchor() {
    let mut rig = Rig::manual();
    let mut b = SpokenBuilder::new();
    b.push_inserted("heading level 2, ", CharPos(50));
    b.push_literal("Intro text", CharPos(50));
    let (text, map) = b.finish();
    rig.core.read(vec![Utterance::with_map(text, map)]);
    rig.step();
    let id = rig.spoken()[0].id;
    rig.rec.start(id);
    rig.rec.word(id, 1, None); // "level"
    assert_eq!(positions(&rig.step()), [None]);
    rig.core.pause();
    assert_eq!(
        rig.core.take_statuses(),
        [SpeechStatus::Paused {
            generation: 1,
            resume_at: Some(CharPos(50))
        }]
    );
    rig.core.resume();
    assert_eq!(rig.last_spoken().text, "heading level 2, Intro text");
}

#[test]
fn pause_inside_an_expansion_repeats_the_whole_expansion() {
    let mut rig = Rig::new(
        RecordingMode::Manual,
        RecordingBackend::DEFAULT_CAPS,
        ServiceConfig::default(),
    );
    rig.core
        .read(vec![Utterance::literal("Born in 1984 here.", CharPos(0))]);
    rig.step();
    let u = rig.spoken()[0].clone();
    assert_eq!(u.text, "Born in nineteen eighty-four here.");
    rig.rec.start(u.id);
    rig.rec.word(u.id, 3, None); // "eighty-four"
    rig.step();
    rig.core.pause();
    assert_eq!(
        rig.core.take_statuses(),
        [SpeechStatus::Paused {
            generation: 1,
            resume_at: Some(CharPos(8))
        }]
    );
    rig.core.resume();
    assert_eq!(rig.last_spoken().text, "nineteen eighty-four here.");
}

#[test]
fn a_long_reading_is_normalized_as_it_is_reached() {
    let mut rig = Rig::new(
        RecordingMode::Manual,
        RecordingBackend::DEFAULT_CAPS,
        ServiceConfig::default(),
    );
    let sentences: Vec<String> = (0..10).map(|i| format!("Item {i} costs $5.")).collect();
    let refs: Vec<&str> = sentences.iter().map(String::as_str).collect();
    rig.core.read(doc(0, &refs));
    rig.step();
    assert_eq!(rig.spoken().len(), 3, "only the lookahead is handed over");
    let mut done = 0;
    while done < rig.spoken().len() {
        let u = rig.spoken()[done].clone();
        assert_eq!(u.text, format!("Item {done} costs five dollars."));
        assert_eq!(u.id.chunk, u32::try_from(done).unwrap());
        rig.rec.finish(u.id);
        let st = rig.step();
        done += 1;
        assert_eq!(st.contains(&FIN), done == 10, "{done}");
    }
    assert_eq!(done, 10);
}

#[test]
fn resume_at_a_cursor_beyond_the_normalized_window() {
    let mut rig = Rig::manual();
    let sentences: Vec<String> = (0..10).map(|i| format!("Sentence number {i}.")).collect();
    let refs: Vec<&str> = sentences.iter().map(String::as_str).collect();
    let utts = doc(0, &refs);
    let eighth = utts[8].source_range().unwrap();
    rig.core.read(utts);
    rig.step();
    rig.core.pause();
    rig.core.take_statuses();
    let before = rig.spoken().len();
    // The cursor moved to "number" in sentence 8 (still in the backlog).
    rig.core.resume_at(eighth.start.saturating_add(9));
    let texts: Vec<String> = rig.spoken()[before..]
        .iter()
        .map(|u| u.text.clone())
        .collect();
    assert_eq!(texts, ["number 8.", "Sentence number 9."]);
    let resumed = &rig.spoken()[before];
    assert_eq!(
        resumed.source_for(0..6),
        Some(CharRange::new(eighth.start.0 + 9, eighth.start.0 + 15))
    );
}

#[test]
fn pause_when_idle_does_nothing() {
    let mut rig = Rig::manual();
    rig.core.pause();
    assert!(rig.core.take_statuses().is_empty());
    rig.core.resume();
    assert!(rig.spoken().is_empty());
}

#[test]
fn native_pause_freezes_the_playback_clock() {
    let caps = RecordingBackend::DEFAULT_CAPS | Caps::AUDIO_CLOCK | Caps::PAUSE;
    let mut rig = Rig::new(RecordingMode::Timed { ms_per_word: 200 }, caps, plain());
    rig.core.read(doc(0, &["one two three."]));
    assert_eq!(positions(&rig.step()), [r(0, 3)]);
    assert_eq!(positions(&rig.advance(ms(330))), [r(4, 7)]);
    rig.core.pause();
    assert_eq!(
        rig.core.take_statuses(),
        [SpeechStatus::Paused {
            generation: 1,
            resume_at: Some(CharPos(4))
        }]
    );
    assert!(rig.rec.calls().contains(&Call::Pause));
    assert!(!rig.rec.calls().contains(&Call::Stop), "native pause");
    assert!(
        rig.advance(ms(1000)).is_empty(),
        "no highlight while paused"
    );
    rig.core.resume();
    assert!(rig.rec.calls().contains(&Call::Resume));
    assert!(positions(&rig.advance(ms(150))).is_empty());
    assert_eq!(positions(&rig.advance(ms(40))), [r(8, 13)]);
    let st = rig.advance(ms(100));
    assert_eq!(st, [FIN]);
}

// ---- pacing ---------------------------------------------------------------

#[test]
fn timer_paces_an_engine_without_word_events() {
    let mut cfg = plain();
    cfg.params.rate = Rate::Wpm(600); // 100 ms per word
    let mut rig = Rig::new(RecordingMode::Manual, Caps::empty(), cfg);
    rig.core.read(doc(0, &["a b c d e."]));
    rig.step();
    let id = rig.spoken()[0].id;
    rig.rec.start(id);
    assert_eq!(positions(&rig.step()), [r(0, 1)], "start word at once");
    assert!(positions(&rig.advance(ms(99))).is_empty());
    assert_eq!(positions(&rig.advance(ms(1))), [r(2, 3)]);
    assert_eq!(
        positions(&rig.advance(ms(300))),
        [r(4, 5), r(6, 7), r(8, 9)]
    );
    assert!(
        positions(&rig.advance(ms(500))).is_empty(),
        "stops at the end"
    );
    assert_eq!(rig.core.next_wakeup(), Some(POLL_INTERVAL));
    rig.rec.finish(id);
    assert_eq!(rig.step(), [FIN]);
    assert_eq!(rig.core.next_wakeup(), None);
}

#[test]
fn stalled_word_events_let_the_timer_run_at_most_four_ahead() {
    let mut cfg = plain();
    cfg.params.rate = Rate::Wpm(600);
    let mut rig = Rig::new(RecordingMode::Manual, Caps::WORD_EVENTS, cfg);
    let words: Vec<String> = (0..20).map(|i| format!("w{i}")).collect();
    let text = words.join(" ");
    rig.core
        .read(vec![Utterance::literal(text.clone(), CharPos(0))]);
    rig.step();
    let id = rig.spoken()[0].id;
    rig.rec.start(id);
    rig.rec.word(id, 2, None);
    let mut painted = positions(&rig.step());
    let mut t = 0;
    while t < 3000 {
        t += 50;
        let got = positions(&rig.advance(ms(50)));
        if t < 1500 {
            assert!(got.is_empty(), "fresh callback: no lead at {t} ms: {got:?}");
        }
        painted.extend(got);
    }
    let last = painted.last().copied().flatten().unwrap();
    // Word 6 ("w6") is confirmed word 2 + MAX_AHEAD 4.
    let w6 = text.find("w6").unwrap();
    assert_eq!(last, CharRange::new(w6, w6 + 2));
}

#[test]
fn audio_clock_words_are_scheduled_not_fired_on_arrival() {
    let caps = RecordingBackend::DEFAULT_CAPS | Caps::AUDIO_CLOCK;
    let mut rig = Rig::new(RecordingMode::Timed { ms_per_word: 200 }, caps, plain());
    rig.core.read(doc(0, &["one two three."]));
    // All word events arrive now; only the first word is shown.
    assert_eq!(positions(&rig.step()), [r(0, 3)]);
    // Word 1 sounds at 200 ms; highlighted at 200 + 120 ms latency.
    assert!(positions(&rig.advance(ms(319))).is_empty());
    assert_eq!(positions(&rig.advance(ms(1))), [r(4, 7)]);
    assert_eq!(positions(&rig.advance(ms(200))), [r(8, 13)]);
    assert_eq!(rig.advance(ms(80)), [FIN]);
}

#[test]
fn latency_offset_comes_from_the_pacing_config() {
    let caps = RecordingBackend::DEFAULT_CAPS | Caps::AUDIO_CLOCK;
    let mut rig = Rig::new(RecordingMode::Timed { ms_per_word: 200 }, caps, plain());
    rig.core.set_pacing(PacingConfig {
        latency_offset: ms(0),
        ..PacingConfig::default()
    });
    rig.core.read(doc(0, &["one two."]));
    rig.step();
    assert_eq!(positions(&rig.advance(ms(200))), [r(4, 7)]);
}

#[test]
fn a_rate_change_while_reading_is_heard_from_the_current_word() {
    // Engines without LIVE_RATE synthesize ahead: the new rate used to
    // start two or three sentences later.
    let mut rig = Rig::manual();
    rig.core.read(doc(
        10,
        &["Alpha beta gamma delta.", "Next one.", "Last one."],
    ));
    rig.step();
    let id = rig.spoken()[0].id;
    rig.rec.start(id);
    for w in 0..3 {
        rig.rec.word(id, w, None);
    }
    rig.step();
    let before = rig.spoken().len();
    rig.core.set_rate(Rate::Wpm(400));
    assert_eq!(rig.rec.params().rate, Rate::Wpm(400));
    let again = &rig.spoken()[before..];
    assert_eq!(again[0].text, "gamma delta.");
    assert!(again[0].id.generation > id.generation);
    assert_eq!(again[1].text, "Next one.");
    // The reading goes on as one reading.
    rig.rec.start(again[0].id);
    rig.rec.word(again[0].id, 1, None);
    assert_eq!(positions(&rig.step()), [r(21, 26), r(27, 32)]);
    // Setting the same rate again restarts nothing.
    let n = rig.spoken().len();
    rig.core.set_rate(Rate::Wpm(400));
    assert_eq!(rig.spoken().len(), n);
}

#[test]
fn parameter_changes_while_idle_or_paused_restart_nothing() {
    let mut rig = Rig::manual();
    rig.core.set_pitch(Pitch::Semitones(2));
    assert!(rig.spoken().is_empty());
    rig.core.read(doc(0, &["One two.", "Three."]));
    rig.step();
    rig.core.pause();
    let n = rig.spoken().len();
    rig.core.set_volume(Volume::new(50));
    rig.core.set_rate(Rate::Wpm(200));
    assert_eq!(rig.spoken().len(), n);
    assert!(rig.core.is_paused());
    // A timer-paced engine (no word events) is not restarted either.
    let mut rig = Rig::new(RecordingMode::Manual, Caps::PITCH, plain());
    rig.core.read(doc(0, &["One two three.", "Four."]));
    rig.step();
    let n = rig.spoken().len();
    rig.core.set_rate(Rate::Wpm(400));
    assert_eq!(rig.spoken().len(), n);
}

#[test]
fn live_rate_change_updates_the_timer() {
    let mut cfg = plain();
    cfg.params.rate = Rate::Wpm(600);
    let mut rig = Rig::new(RecordingMode::Manual, Caps::LIVE_RATE, cfg);
    rig.core.read(doc(0, &["a b c d e f."]));
    rig.step();
    let id = rig.spoken()[0].id;
    rig.rec.start(id);
    rig.step();
    assert_eq!(positions(&rig.advance(ms(100))), [r(2, 3)]);
    rig.core.set_rate(Rate::Wpm(300));
    // The tick already scheduled at 200 ms runs, then every 200 ms.
    assert_eq!(positions(&rig.advance(ms(150))), [r(4, 5)]);
    assert!(positions(&rig.advance(ms(149))).is_empty());
    assert_eq!(positions(&rig.advance(ms(1))), [r(6, 7)]);
}

// ---- say modes --------------------------------------------------------------

#[test]
fn say_interrupt_stops_the_reading() {
    let mut rig = Rig::manual();
    rig.core.read(doc(0, &["Reading this."]));
    rig.step();
    rig.core.say("Hello", SayMode::Interrupt);
    let st = rig.core.take_statuses();
    assert_eq!(st, [STOPPED]);
    let last = rig.last_spoken();
    assert_eq!(last.text, "Hello");
    assert_eq!(last.kind, UtteranceKind::Announcement);
    rig.rec.start(last.id);
    rig.rec.finish(last.id);
    assert!(rig.step().is_empty(), "announcements report no Finished");
}

#[test]
fn say_queue_speaks_after_the_reading() {
    let mut rig = Rig::manual();
    rig.core.read(doc(0, &["Reading this."]));
    rig.core.say("Done", SayMode::Queue);
    rig.step();
    assert_eq!(rig.rec.spoken_texts(), ["Reading this.", "Done"]);
    let ids: Vec<UtteranceId> = rig.spoken().iter().map(|u| u.id).collect();
    rig.rec.finish(ids[0]);
    assert!(rig.step().is_empty());
    rig.rec.finish(ids[1]);
    assert_eq!(rig.step(), [FIN]);
}

#[test]
fn say_announce_interrupts_and_the_reading_continues() {
    let mut rig = Rig::manual();
    rig.core.read(doc(0, &["Alpha beta gamma.", "Second."]));
    rig.step();
    let id = rig.spoken()[0].id;
    rig.rec.start(id);
    rig.rec.word(id, 1, None);
    rig.step();
    rig.core.say("Rate 300", SayMode::Announce);
    assert!(rig.core.take_statuses().is_empty(), "no Stopped");
    let texts: Vec<String> = rig.spoken()[2..].iter().map(|u| u.text.clone()).collect();
    assert_eq!(texts, ["Rate 300", "beta gamma.", "Second."]);
    // A second announcement replaces the first, the reading still follows.
    rig.core.say("Rate 320", SayMode::Announce);
    let n = rig.spoken().len();
    let texts: Vec<String> = rig.spoken()[n - 3..]
        .iter()
        .map(|u| u.text.clone())
        .collect();
    assert_eq!(texts, ["Rate 320", "beta gamma.", "Second."]);
}

#[test]
fn say_announce_while_paused_stays_paused() {
    let mut rig = Rig::manual();
    rig.core.read(doc(0, &["Alpha beta."]));
    rig.step();
    rig.core.pause();
    rig.core.take_statuses();
    rig.core.say("Paused", SayMode::Announce);
    let ann = rig.last_spoken();
    assert_eq!(ann.text, "Paused");
    rig.rec.start(ann.id);
    rig.rec.finish(ann.id);
    assert!(rig.step().is_empty());
    assert!(rig.core.is_paused());
    assert_eq!(rig.spoken().len(), 2);
    rig.core.resume();
    assert_eq!(rig.last_spoken().text, "Alpha beta.");
}

#[test]
fn say_interrupt_while_paused_keeps_the_pause() {
    let mut rig = Rig::manual();
    rig.core.read(doc(0, &["Alpha beta."]));
    rig.step();
    rig.core.pause();
    rig.core.take_statuses();
    rig.core.say("Hello", SayMode::Interrupt);
    assert!(rig.core.take_statuses().is_empty());
    assert!(rig.core.is_paused());
    rig.core.resume();
    assert_eq!(rig.last_spoken().text, "Alpha beta.");
}

// ---- characters, tones, earcons, voices ------------------------------------

#[test]
fn speak_char_raises_pitch_for_capitals_and_restores_it() {
    let mut rig = Rig::manual();
    let normal = rig.rec.params();
    rig.core.speak_char('A', Some(CharPos(5)));
    let calls = rig.rec.calls();
    let Call::SetParams(p) = &calls[calls.len() - 2] else {
        panic!("{calls:?}")
    };
    assert_eq!(p.pitch, Pitch::Semitones(4));
    assert_eq!(p.rate, Rate::Wpm(318), "265 × 1.2");
    let u = rig.last_spoken();
    assert_eq!((u.text.as_str(), u.kind), ("A", UtteranceKind::Character));
    assert_eq!(u.source_range(), Some(CharRange::new(5, 6)));
    rig.rec.finish(u.id);
    rig.step();
    assert_eq!(rig.rec.params(), normal);
}

/// The voice manager's preview: one utterance in another voice, set just
/// before it is spoken, then the voice in use again (W7v).
#[test]
fn preview_speaks_once_in_another_voice_then_restores_the_voice() {
    let mut rig = Rig::manual();
    rig.core.set_voice(Some("zira".into()));
    let normal = rig.rec.params();
    rig.core.preview("david", "David. The quick brown fox.");
    let calls = rig.rec.calls();
    let Call::SetParams(p) = &calls[calls.len() - 2] else {
        panic!("{calls:?}")
    };
    assert_eq!(p.voice.as_deref(), Some("david"));
    assert_eq!(
        (p.rate, p.pitch),
        (normal.rate, normal.pitch),
        "only the voice"
    );
    let u = rig.last_spoken();
    assert_eq!(u.text, "David. The quick brown fox.");
    assert_eq!(u.kind, UtteranceKind::Announcement);
    // A rate change while it plays waits, and comes with the voice back.
    rig.core.set_rate(Rate::Wpm(300));
    assert_eq!(rig.rec.params().voice.as_deref(), Some("david"));
    rig.rec.finish(u.id);
    rig.step();
    let after = rig.rec.params();
    assert_eq!(after.voice.as_deref(), Some("zira"));
    assert_eq!(after.rate, Rate::Wpm(300));
}

/// A preview interrupts what is speaking; stopping it restores the voice.
#[test]
fn a_stopped_preview_restores_the_voice() {
    let mut rig = Rig::manual();
    rig.core.set_voice(Some("zira".into()));
    rig.core.say("Voice manager.", SayMode::Queue);
    rig.core.preview("david", "David.");
    let texts = rig.rec.spoken_texts();
    assert_eq!(texts.last().map(String::as_str), Some("David."));
    assert!(rig.rec.calls().iter().any(|c| matches!(c, Call::Stop)));
    rig.core.stop();
    assert_eq!(rig.rec.params().voice.as_deref(), Some("zira"));
    // The voice in use needs no change: nothing is set.
    let before = rig.rec.calls().len();
    rig.core.preview("zira", "Zira.");
    let set = rig.rec.calls()[before..]
        .iter()
        .filter(|c| matches!(c, Call::SetParams(_)))
        .count();
    assert_eq!(set, 0);
}

#[test]
fn speak_char_names_punctuation_and_says_cap_without_pitch_or_tones() {
    let mut rig = Rig::new(RecordingMode::Instant, Caps::WORD_EVENTS, plain());
    rig.core.speak_char('B', None);
    rig.core.speak_char(',', Some(CharPos(3)));
    rig.core.speak_char(' ', None);
    assert_eq!(rig.rec.spoken_texts(), ["cap B", "comma", "space"]);
    assert!(
        positions(&rig.step()).is_empty(),
        "characters move no highlight"
    );
}

#[test]
fn speak_char_tone_indication() {
    let cfg = ServiceConfig {
        caps: CapsIndication::Tone,
        ..plain()
    };
    let mut rig = Rig::new(RecordingMode::Instant, RecordingBackend::DEFAULT_CAPS, cfg);
    rig.core.speak_char('Q', None);
    let calls = rig.rec.calls();
    let tone = calls
        .iter()
        .position(|c| matches!(c, Call::Tone { .. }))
        .unwrap();
    let speak = calls
        .iter()
        .position(|c| matches!(c, Call::Speak(_)))
        .unwrap();
    assert!(tone < speak);
    assert_eq!(rig.rec.spoken_texts(), ["Q"]);
}

#[test]
fn earcons_play_tones_only_when_the_engine_can() {
    let mut rig = Rig::manual();
    rig.core.earcon(Earcon::Wrap);
    let tones = rig
        .rec
        .calls()
        .into_iter()
        .filter(|c| matches!(c, Call::Tone { .. }))
        .count();
    assert_eq!(tones, 2);
    let mut quiet = Rig::new(RecordingMode::Manual, Caps::WORD_EVENTS, plain());
    quiet.core.earcon(Earcon::Error);
    quiet.core.tone(440.0, 100);
    assert!(
        !quiet
            .rec
            .calls()
            .iter()
            .any(|c| matches!(c, Call::Tone { .. }))
    );
}

#[test]
fn preferred_voice_is_resolved_when_no_voice_is_set() {
    let cfg = ServiceConfig {
        prefer_voice: Some("recording".into()),
        ..plain()
    };
    let rig = Rig::new(RecordingMode::Manual, RecordingBackend::DEFAULT_CAPS, cfg);
    assert_eq!(rig.rec.params().voice.as_deref(), Some("rec-en-us"));
    let cfg = ServiceConfig {
        prefer_voice: Some("recording".into()),
        params: VoiceParams {
            voice: Some("rec-en-gb".into()),
            ..VoiceParams::default()
        },
        ..plain()
    };
    let rig = Rig::new(RecordingMode::Manual, RecordingBackend::DEFAULT_CAPS, cfg);
    assert_eq!(rig.rec.params().voice.as_deref(), Some("rec-en-gb"));
}

#[test]
fn voice_rate_pitch_volume_reach_the_backend() {
    let mut rig = Rig::manual();
    rig.core.set_rate(Rate::Wpm(2000));
    rig.core.set_pitch(Pitch::Semitones(-30));
    rig.core.set_volume(Volume::new(40));
    rig.core.set_voice(Some("rec-en-gb".into()));
    let p = rig.rec.params();
    assert_eq!(p.rate, Rate::Wpm(Rate::MAX_WPM));
    assert_eq!(p.pitch, Pitch::Semitones(-12));
    assert_eq!(p.volume, Volume::new(40));
    assert_eq!(p.voice.as_deref(), Some("rec-en-gb"));
}

// ---- helpers ---------------------------------------------------------------

#[test]
fn trimming_keeps_a_valid_map() {
    let mut b = SpokenBuilder::new();
    b.push_literal("Hello ", CharPos(10));
    b.push_expanded("Doctor", CharRange::new(16, 19));
    b.push_elided(CharRange::new(19, 21));
    b.push_literal(" Who", CharPos(21));
    let (text, map) = b.finish();
    let u = Utterance::with_map(text, map);
    for byte in 0..u.text.len() as u32 {
        let byte = snap_to_span(&u, byte);
        if let Some(t) = trim_utterance(&u, byte) {
            t.offset_map.check_invariants(&t.text).unwrap();
            assert_eq!(t.text, u.text[byte as usize..]);
        }
    }
    let t = trim_utterance(&u, 3).unwrap();
    assert_eq!(t.source_for(0..3), Some(CharRange::new(13, 16)));
    assert!(trim_utterance(&u, u.text.len() as u32).is_none());
}

// ---- the threaded service ------------------------------------------------------

#[test]
fn threaded_service_with_the_recording_backend() {
    let (b, rec) = RecordingBackend::new();
    let s = SpeechService::spawn(b.into_factory(), plain()).unwrap();
    assert_eq!(s.backend_id(), "recording");
    assert!(s.capabilities().contains(Caps::WORD_EVENTS));
    s.read(doc(0, &["One two.", "Three."]));
    let mut got = Vec::new();
    while let Ok(st) = s.statuses().recv_timeout(Duration::from_secs(5)) {
        let done = st == FIN;
        got.push(st);
        if done {
            break;
        }
    }
    assert_eq!(positions(&got), [r(0, 3), r(4, 7), r(9, 14)]);
    s.say("Bye", SayMode::Queue);
    s.shutdown();
    assert_eq!(rec.spoken_texts().last().map(String::as_str), Some("Bye"));
}

#[test]
fn null_service_reports_positions_then_finished() {
    let s = SpeechService::null();
    s.read(vec![
        Utterance::literal("One.", CharPos(0)),
        Utterance::literal("Two.", CharPos(5)),
    ]);
    let mut got = Vec::new();
    while let Ok(st) = s.statuses().recv_timeout(Duration::from_secs(2)) {
        let done = st == FIN;
        got.push(st);
        if done {
            break;
        }
    }
    assert_eq!(positions(&got), [r(0, 3), r(5, 8)]);
}

/// The waker is called after statuses are sent, so a frontend can sleep
/// until then; a cleared waker is not called again.
#[test]
fn the_waker_rings_after_statuses_arrive() {
    let s = SpeechService::null();
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let tx = std::sync::Mutex::new(tx);
    s.set_waker(Some(Arc::new(move || {
        let _ = tx.lock().map(|t| t.send(()));
    })));
    s.read(vec![Utterance::literal("One.", CharPos(0))]);
    // Woken, and the status is there to take by then.
    rx.recv_timeout(Duration::from_secs(5)).expect("woken");
    assert!(s.try_status().is_some());
    while s
        .statuses()
        .recv_timeout(Duration::from_millis(200))
        .is_ok()
    {}
    while rx.try_recv().is_ok() {}
    s.set_waker(None);
    s.read(vec![Utterance::literal("Two.", CharPos(0))]);
    let mut got = Vec::new();
    while let Ok(st) = s.statuses().recv_timeout(Duration::from_secs(2)) {
        let done = st == FIN;
        got.push(st);
        if done {
            break;
        }
    }
    assert!(!got.is_empty());
    assert!(rx.try_recv().is_err(), "a cleared waker stays quiet");
}

#[test]
fn the_voice_list_is_asked_on_the_speech_thread() {
    let (b, rec) = RecordingBackend::with(RecordingMode::Instant, RecordingBackend::DEFAULT_CAPS);
    rec.set_voices(vec![
        Voice {
            id: "a".into(),
            name: "Alpha".into(),
            ..Voice::default()
        },
        Voice {
            id: "b".into(),
            name: "Beta".into(),
            ..Voice::default()
        },
    ]);
    let service = SpeechService::spawn(b.into_factory(), plain()).unwrap();
    let names: Vec<String> = service
        .voices()
        .unwrap()
        .into_iter()
        .map(|v| v.name)
        .collect();
    assert_eq!(names, ["Alpha", "Beta"]);
    service.shutdown();
}

// ---- voices listed in the background --------------------------------------------

/// A backend whose voice list arrives later (as SAPI's does).
struct SlowVoices(RecordingBackend, crate::voices::VoiceCache);

impl SpeechBackend for SlowVoices {
    fn id(&self) -> BackendId {
        "slow-voices"
    }
    fn capabilities(&self) -> Caps {
        self.0.capabilities()
    }
    fn voices(&self) -> Result<Vec<crate::Voice>, SpeechError> {
        panic!("the service must not list voices on the speech thread");
    }
    fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError> {
        self.0.set_params(params)
    }
    fn effective_wpm(&self) -> u16 {
        self.0.effective_wpm()
    }
    fn speak(&mut self, u: &Utterance, sink: &mut dyn EventSink) -> Result<(), SpeechError> {
        self.0.speak(u, sink)
    }
    fn poll(&mut self, sink: &mut dyn EventSink) {
        self.0.poll(sink);
    }
    fn stop(&mut self) {
        self.0.stop();
    }
    fn voice_cache(&self) -> Option<crate::voices::VoiceCache> {
        Some(self.1.clone())
    }
}

#[test]
fn voices_never_wait_and_a_name_asked_while_loading_resolves_when_they_arrive() {
    let (b, rec) = RecordingBackend::with(RecordingMode::Instant, RecordingBackend::DEFAULT_CAPS);
    let cache = crate::voices::VoiceCache::loading();
    let backend_cache = cache.clone();
    let service = SpeechService::spawn(
        Box::new(move || Ok(Box::new(SlowVoices(b, backend_cache)) as Box<dyn SpeechBackend>)),
        plain(),
    )
    .unwrap();
    // No waiting: the list is loading, and says so.
    let t = std::time::Instant::now();
    assert_eq!(service.voice_list(), crate::VoiceList::Loading);
    assert!(
        service
            .voices()
            .unwrap_err()
            .to_string()
            .contains("still loading")
    );
    assert!(t.elapsed() < Duration::from_millis(100));
    // A voice asked for by name goes through as typed for now.
    service.set_voice(Some("Beta".into()));
    assert!(service.sync());
    assert_eq!(rec.params().voice.as_deref(), Some("Beta"));
    // The list arrives: the name resolves to the voice's id.
    cache.set(Ok(vec![
        Voice {
            id: "a".into(),
            name: "Alpha".into(),
            ..Voice::default()
        },
        Voice {
            id: "b".into(),
            name: "Beta".into(),
            ..Voice::default()
        },
    ]));
    assert!(service.sync());
    assert_eq!(rec.params().voice.as_deref(), Some("b"));
    assert_eq!(service.voices().unwrap().len(), 2);
    assert!(
        rec.calls()
            .iter()
            .filter(|c| matches!(c, Call::SetParams(_)))
            .count()
            >= 2
    );
    service.shutdown();
}

#[test]
fn a_failed_listing_is_kept() {
    let (b, _rec) = RecordingBackend::with(RecordingMode::Instant, RecordingBackend::DEFAULT_CAPS);
    let cache = crate::voices::VoiceCache::ready(Err(SpeechError::Engine("no registry".into())));
    let service = SpeechService::spawn(
        Box::new(move || Ok(Box::new(SlowVoices(b, cache)) as Box<dyn SpeechBackend>)),
        plain(),
    )
    .unwrap();
    assert_eq!(
        service.voice_list(),
        crate::VoiceList::Failed(SpeechError::Engine("no registry".into()))
    );
    service.shutdown();
}

// ---- a panic on the speech thread ---------------------------------------------

/// A backend with a bug: `speak` panics on the text "boom".
struct Buggy(RecordingBackend);

impl SpeechBackend for Buggy {
    fn id(&self) -> BackendId {
        "buggy"
    }
    fn capabilities(&self) -> Caps {
        self.0.capabilities()
    }
    fn voices(&self) -> Result<Vec<crate::Voice>, SpeechError> {
        self.0.voices()
    }
    fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError> {
        self.0.set_params(params)
    }
    fn effective_wpm(&self) -> u16 {
        self.0.effective_wpm()
    }
    fn speak(&mut self, u: &Utterance, sink: &mut dyn EventSink) -> Result<(), SpeechError> {
        assert!(!u.text.contains("boom"), "the buggy backend blew up");
        self.0.speak(u, sink)
    }
    fn poll(&mut self, sink: &mut dyn EventSink) {
        self.0.poll(sink);
    }
    fn stop(&mut self) {
        self.0.stop();
    }
}

fn buggy_service() -> SpeechService {
    let (b, _rec) = RecordingBackend::with(RecordingMode::Instant, RecordingBackend::DEFAULT_CAPS);
    SpeechService::spawn(
        Box::new(move || Ok(Box::new(Buggy(b)) as Box<dyn SpeechBackend>)),
        plain(),
    )
    .unwrap()
}

#[test]
fn a_panic_on_the_speech_thread_is_reported_and_the_service_is_dead() {
    // Before: the thread died silently; statuses just stopped coming, and
    // every command vanished.
    let s = buggy_service();
    s.say("Fine.", SayMode::Interrupt);
    assert!(s.is_alive());
    s.say("boom", SayMode::Interrupt);
    let mut last = None;
    loop {
        match s.statuses().recv_timeout(Duration::from_secs(10)) {
            Ok(st) => last = Some(st),
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => panic!("the speech thread neither spoke nor ended"),
        }
    }
    let Some(SpeechStatus::BackendError(message)) = last else {
        panic!("the last status says what happened: {last:?}");
    };
    assert!(
        message.starts_with("speech stopped after an internal error (the buggy backend blew up)"),
        "{message}"
    );
    assert!(!s.is_alive());
    assert_eq!(s.failure().as_deref(), Some("the buggy backend blew up"));
    // A dead thread is not an empty queue.
    assert_eq!(s.poll_status(), Err(SpeechError::ServiceStopped));
    assert_eq!(s.try_status(), None);
    assert_eq!(s.voices(), Err(SpeechError::ServiceStopped));
    // Commands are ignored, not a crash of the caller; a new service works.
    s.say("Anyone there?", SayMode::Interrupt);
    s.stop();
    drop(s);
    let again = buggy_service();
    assert!(again.is_alive());
    assert_eq!(again.poll_status(), Ok(None), "alive, nothing waiting");
    again.shutdown();
}

#[test]
fn a_panic_while_the_backend_starts_is_an_error_from_spawn() {
    let err = SpeechService::spawn(
        Box::new(|| -> Result<Box<dyn SpeechBackend>, SpeechError> { panic!("no engine today") }),
        plain(),
    )
    .unwrap_err();
    assert_eq!(
        err,
        SpeechError::Engine(
            "speech stopped after an internal error (no engine today); restart speech to go on"
                .into()
        )
    );
}

// ---- structural pauses ------------------------------------------------------

/// "Title." then a paragraph of two sentences, with the title's end (6) a
/// heading's end.
fn titled() -> (Vec<Utterance>, Vec<PauseAt>) {
    let us = doc(0, &["Title.", "Body one.", "Body two."]);
    let pauses = vec![PauseAt {
        after: CharPos(6),
        kind: crate::pauses::PauseKind::Heading,
    }];
    (us, pauses)
}

/// The engine calls other than parameter changes, as short strings.
fn engine_calls(rec: &RecordingHandle) -> Vec<String> {
    rec.calls()
        .into_iter()
        .filter_map(|c| match c {
            Call::Speak(u) => Some(format!("speak {}", u.text)),
            Call::Silence { id, ms } => Some(format!("silence {ms} after {}", id.chunk)),
            Call::Stop => Some("stop".into()),
            _ => None,
        })
        .collect()
}

fn silences(rec: &RecordingHandle) -> usize {
    rec.calls()
        .iter()
        .filter(|c| matches!(c, Call::Silence { .. }))
        .count()
}

#[test]
fn engines_that_play_silence_get_the_pause_after_the_right_utterance() {
    let mut rig = Rig::manual();
    let (us, pauses) = titled();
    rig.core.read_with_pauses(us, pauses);
    rig.step();
    assert_eq!(
        engine_calls(&rig.rec),
        [
            "speak Title.",
            "silence 400 after 0",
            "speak Body one.",
            "speak Body two."
        ],
        "the lookahead is untouched: the next sentence is synthesized ahead"
    );
}

#[test]
fn a_zero_pause_is_no_pause() {
    let mut rig = Rig::manual();
    rig.core.set_pauses(PauseConfig {
        heading_ms: 0,
        ..PauseConfig::default()
    });
    let (us, pauses) = titled();
    rig.core.read_with_pauses(us, pauses);
    rig.step();
    assert_eq!(silences(&rig.rec), 0);
    // A plain read has no pauses either.
    let mut rig = Rig::manual();
    rig.core.read(titled().0);
    rig.step();
    assert_eq!(silences(&rig.rec), 0);
}

#[test]
fn pauses_shorten_at_high_rates() {
    let mut rig = Rig::manual();
    rig.core.set_rate(Rate::Wpm(530));
    let (us, pauses) = titled();
    rig.core.read_with_pauses(us, pauses);
    rig.step();
    assert!(
        engine_calls(&rig.rec).contains(&"silence 200 after 0".to_owned()),
        "{:?}",
        engine_calls(&rig.rec)
    );
}

#[test]
fn a_resumed_sentence_keeps_its_pause() {
    let mut rig = Rig::manual();
    let (us, pauses) = titled();
    rig.core.read_with_pauses(us, pauses);
    rig.step();
    let title = rig.spoken()[0].id;
    rig.rec.start(title);
    rig.step();
    rig.core.pause();
    rig.rec.clear_calls();
    rig.core.resume();
    rig.step();
    let chunk = rig.spoken()[0].id.chunk;
    assert_eq!(
        engine_calls(&rig.rec)[..2],
        [
            "speak Title.".to_owned(),
            format!("silence 400 after {chunk}")
        ]
    );
}

/// A rig whose engine plays its own audio: pauses are gaps in the queue.
fn gap_rig() -> Rig {
    Rig::new(
        RecordingMode::Manual,
        RecordingBackend::DEFAULT_CAPS - Caps::SILENCE,
        plain(),
    )
}

/// `rig` read [`titled`] and the title has finished: the pause runs.
fn in_the_gap(rig: &mut Rig) {
    let (us, pauses) = titled();
    rig.core.read_with_pauses(us, pauses);
    rig.step();
    assert_eq!(rig.rec.spoken_texts(), ["Title."], "held for the pause");
    let title = rig.spoken()[0].id;
    rig.rec.start(title);
    rig.rec.finish(title);
    rig.step();
    assert_eq!(rig.rec.spoken_texts(), ["Title."], "the pause runs");
}

#[test]
fn engines_that_play_their_own_audio_get_a_timed_gap() {
    let mut rig = gap_rig();
    in_the_gap(&mut rig);
    assert!(
        rig.core.next_wakeup().is_some_and(|w| w <= ms(400)),
        "the service wakes for the end of the pause"
    );
    rig.advance(ms(399));
    assert_eq!(rig.rec.spoken_texts(), ["Title."]);
    let st = rig.advance(ms(1));
    assert_eq!(rig.rec.spoken_texts(), ["Title.", "Body one.", "Body two."]);
    assert!(!st.contains(&FIN));
    assert_eq!(silences(&rig.rec), 0);
}

#[test]
fn the_next_word_is_highlighted_as_soon_as_it_sounds() {
    let mut rig = gap_rig();
    in_the_gap(&mut rig);
    rig.advance(ms(400));
    let body = rig.spoken()[1].id;
    rig.rec.start(body);
    assert!(rig.rec.word(body, 0, None));
    let st = rig.step();
    assert_eq!(positions(&st), [r(7, 11)], "{st:?}");
}

#[test]
fn stop_during_a_gap_is_immediate() {
    let mut rig = gap_rig();
    in_the_gap(&mut rig);
    rig.core.stop();
    let st = rig.step();
    assert!(st.contains(&STOPPED), "{st:?}");
    // A new reading starts at once, with no pause left over.
    rig.core.read(doc(100, &["Fresh."]));
    rig.step();
    assert_eq!(
        rig.rec.spoken_texts().last().map(String::as_str),
        Some("Fresh.")
    );
}

#[test]
fn skipping_during_a_gap_cuts_the_pause_not_a_sentence() {
    let mut rig = gap_rig();
    in_the_gap(&mut rig);
    rig.core.skip();
    rig.step();
    assert_eq!(rig.rec.spoken_texts(), ["Title.", "Body one.", "Body two."]);
}

#[test]
fn pausing_during_a_gap_resumes_with_the_next_sentence_at_once() {
    let mut rig = gap_rig();
    in_the_gap(&mut rig);
    rig.core.pause();
    let st = rig.step();
    assert!(
        st.iter().any(|s| matches!(s, SpeechStatus::Paused { .. })),
        "{st:?}"
    );
    rig.advance(ms(5000));
    assert_eq!(rig.rec.spoken_texts(), ["Title."], "nothing while paused");
    rig.core.resume();
    rig.step();
    assert_eq!(rig.rec.spoken_texts(), ["Title.", "Body one.", "Body two."]);
}

#[test]
fn a_played_silence_delays_the_next_sentence_by_its_length() {
    // An audio-clock engine that plays the silence: 100 ms a word.
    let config = ServiceConfig {
        pacing: PacingConfig {
            latency_offset: Duration::ZERO,
            ..PacingConfig::default()
        },
        ..plain()
    };
    let mut rig = Rig::new(
        RecordingMode::Timed { ms_per_word: 100 },
        RecordingBackend::DEFAULT_CAPS | Caps::AUDIO_CLOCK,
        config,
    );
    let (us, pauses) = titled();
    rig.core.read_with_pauses(us, pauses);
    // "Title." is 100 ms, then 400 ms of silence: "Body" sounds at 500.
    let mut seen = Vec::new();
    for t in (0..=600).step_by(10) {
        if t > 0 {
            rig.clock.advance(ms(10));
        }
        for p in positions(&rig.step()) {
            seen.push((t, p));
        }
    }
    assert_eq!(seen.first(), Some(&(0, r(0, 5))));
    let body = seen.iter().find(|(_, p)| *p == r(7, 11)).copied();
    assert_eq!(body, Some((500, r(7, 11))), "{seen:?}");
}
