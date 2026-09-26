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
    assert_eq!(count(&st, &SpeechStatus::Finished), 1);
    assert_eq!(st.last(), Some(&SpeechStatus::Finished));
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
    assert_eq!(rig.step(), [SpeechStatus::Finished]);
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
    assert!(!st.contains(&SpeechStatus::Finished));
    let new = rig.last_spoken();
    assert!(new.id.generation > old.generation);
    rig.rec.start(new.id);
    assert_eq!(positions(&rig.step()), [r(100, 103)]);
}

#[test]
fn stop_reports_stopped_and_drops_late_events() {
    let mut rig = Rig::manual();
    rig.core.read(doc(0, &["Alpha beta."]));
    rig.step();
    let id = rig.spoken()[0].id;
    rig.core.stop();
    assert_eq!(rig.core.take_statuses(), [SpeechStatus::Stopped]);
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
    assert_eq!(st.last(), Some(&SpeechStatus::Finished));
}

#[test]
fn engine_error_event_is_reported() {
    let mut rig = Rig::manual();
    rig.core.read(doc(0, &["Only one."]));
    rig.step();
    let id = rig.spoken()[0].id;
    rig.rec.emit(id, RawEvent::Error("device lost".into()));
    let st = rig.step();
    assert_eq!(
        st,
        [
            SpeechStatus::BackendError("device lost".into()),
            SpeechStatus::Finished
        ]
    );
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
            resume_at: Some(CharPos(8))
        }]
    );
    rig.core.resume();
    assert_eq!(rig.last_spoken().text, "nineteen eighty-four here.");
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
    assert_eq!(st, [SpeechStatus::Finished]);
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
    assert_eq!(rig.step(), [SpeechStatus::Finished]);
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
    assert_eq!(rig.advance(ms(80)), [SpeechStatus::Finished]);
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
    assert_eq!(st, [SpeechStatus::Stopped]);
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
    assert_eq!(rig.step(), [SpeechStatus::Finished]);
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
        let done = st == SpeechStatus::Finished;
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
        let done = st == SpeechStatus::Finished;
        got.push(st);
        if done {
            break;
        }
    }
    assert_eq!(positions(&got), [r(0, 3), r(5, 8)]);
}
