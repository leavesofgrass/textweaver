use std::sync::Mutex;

use textweaver_core::CharPos;

use super::*;
use crate::backends::{RecordingBackend, RecordingMode};

/// Collects events.
#[derive(Default)]
struct Sink(Vec<(UtteranceId, RawEvent)>);

impl EventSink for Sink {
    fn emit(&mut self, id: UtteranceId, event: RawEvent) {
        self.0.push((id, event));
    }
    fn is_current(&self, _: UtteranceId) -> bool {
        true
    }
}

/// A player that plays nothing: each utterance starts, says its first
/// word and finishes on the next poll.
struct FakePlayer {
    log: Arc<Mutex<Vec<String>>>,
    queued: Vec<(UtteranceId, u32)>,
    paused: bool,
}

impl ClipPlayer for FakePlayer {
    fn play(&mut self, id: UtteranceId, text: &str, clips: &[RecordedClip]) {
        let file = clips[0].file.display().to_string();
        self.log
            .lock()
            .unwrap()
            .push(format!("play {text} from {file}"));
        self.queued
            .push((id, u32::try_from(text.len()).unwrap_or(0)));
    }
    fn poll(&mut self, sink: &mut dyn EventSink) {
        if self.paused {
            return;
        }
        for (id, len) in self.queued.drain(..) {
            sink.emit(id, RawEvent::Started);
            sink.emit(
                id,
                RawEvent::Word {
                    byte_range: 0..len,
                    audio_ms: Some(0),
                },
            );
            sink.emit(id, RawEvent::Finished);
        }
    }
    fn stop(&mut self) {
        self.queued.clear();
    }
    fn pause(&mut self) {
        self.paused = true;
    }
    fn resume(&mut self) {
        self.paused = false;
    }
    fn set_volume(&mut self, _: Volume) {}
}

fn plan(log: &Arc<Mutex<Vec<String>>>) -> Arc<RecordedPlan> {
    let log = Arc::clone(log);
    Arc::new(RecordedPlan {
        pars: vec![RecordedPar {
            range: CharRange::new(0, 10),
            clips: vec![RecordedClip {
                file: PathBuf::from("a.mp3"),
                begin: Duration::ZERO,
                end: Some(Duration::from_secs(1)),
            }],
        }],
        player: Arc::new(move || {
            Ok(Box::new(FakePlayer {
                log: Arc::clone(&log),
                queued: Vec::new(),
                paused: false,
            }) as Box<dyn ClipPlayer>)
        }),
    })
}

fn utterance(text: &str, at: usize, chunk: u32) -> Utterance {
    let mut u = Utterance::literal(text, CharPos(at));
    u.id = UtteranceId {
        generation: 1,
        chunk,
    };
    u
}

#[test]
fn phrases_with_audio_play_and_speech_waits_its_turn() {
    let (engine, handle) = RecordingBackend::with(RecordingMode::Manual, Caps::WORD_EVENTS);
    let mut m = Mixed::new(Box::new(engine));
    assert!(!m.capabilities().contains(Caps::PAUSE));
    let log = Arc::new(Mutex::new(Vec::new()));
    m.set_recorded(Some(plan(&log)));
    assert!(m.capabilities().contains(Caps::PAUSE));
    let mut sink = Sink::default();
    // A recorded phrase, then text without audio, then a phrase again
    // (inside the same par, so recorded).
    m.speak(&utterance("First one", 0, 0), &mut sink).unwrap();
    m.speak(&utterance("Spoken.", 12, 1), &mut sink).unwrap();
    assert!(
        handle.spoken_texts().is_empty(),
        "speech waits for the clip"
    );
    m.poll(&mut sink);
    assert_eq!(*log.lock().unwrap(), ["play First one from a.mp3"]);
    assert_eq!(handle.spoken_texts(), ["Spoken."]);
    let finished: Vec<u32> = sink
        .0
        .iter()
        .filter(|(_, e)| *e == RawEvent::Finished)
        .map(|(id, _)| id.chunk)
        .collect();
    assert_eq!(finished, [0]);
    // A recorded phrase now waits for speech to end.
    m.speak(&utterance("one", 6, 2), &mut sink).unwrap();
    m.poll(&mut sink);
    assert_eq!(log.lock().unwrap().len(), 1);
    handle.finish(UtteranceId {
        generation: 1,
        chunk: 1,
    });
    m.poll(&mut sink);
    m.poll(&mut sink);
    assert_eq!(log.lock().unwrap().len(), 2);
}

#[test]
fn without_a_plan_everything_is_spoken_and_stop_cancels_the_waiting() {
    let (engine, handle) = RecordingBackend::with(RecordingMode::Manual, Caps::WORD_EVENTS);
    let mut m = Mixed::new(Box::new(engine));
    let mut sink = Sink::default();
    m.speak(&utterance("First one", 0, 0), &mut sink).unwrap();
    assert_eq!(handle.spoken_texts(), ["First one"]);
    let log = Arc::new(Mutex::new(Vec::new()));
    m.set_recorded(Some(plan(&log)));
    m.speak(&utterance("Second", 0, 1), &mut sink).unwrap();
    m.stop();
    m.poll(&mut sink);
    assert!(sink.0.contains(&(
        UtteranceId {
            generation: 1,
            chunk: 1
        },
        RawEvent::Cancelled
    )));
    assert!(log.lock().unwrap().is_empty());
}
