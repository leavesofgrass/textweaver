//! A speech engine that crashes in the middle of a reading is restarted by
//! the service, and the app keeps following the reading from the last word
//! (docs/audit-2026-09.md, finding R4; Agent D4).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use textweaver_app::a11y::{Announcer, Priority};
use textweaver_app::core::{Utterance, UtteranceKind};
use textweaver_app::keymap::ActionId;
use textweaver_app::speech::{
    Caps, EventSink, RawEvent, ServiceConfig, SpeechBackend, SpeechError, SpeechService,
};
use textweaver_app::store::DocKey;
use textweaver_app::text::Document;
use textweaver_app::{App, AppConfig, Command, Playback};

#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Announcer for Said {
    fn announce(&mut self, text: &str, _priority: Priority) {
        self.0.lock().unwrap().push(text.to_owned());
    }
}

/// Speaks every utterance at once, except that the first time it meets the
/// word "Crash" it says the first word, then fails like a host that died.
struct CrashOnce {
    crashed: bool,
    spoken: Arc<Mutex<Vec<String>>>,
}

impl SpeechBackend for CrashOnce {
    fn id(&self) -> &'static str {
        "crash-once"
    }

    fn capabilities(&self) -> Caps {
        Caps::WORD_EVENTS
    }

    fn voices(&self) -> Result<Vec<textweaver_app::speech::Voice>, SpeechError> {
        Ok(Vec::new())
    }

    fn set_params(
        &mut self,
        _params: &textweaver_app::speech::VoiceParams,
    ) -> Result<(), SpeechError> {
        Ok(())
    }

    fn effective_wpm(&self) -> u16 {
        180
    }

    fn speak(&mut self, u: &Utterance, sink: &mut dyn EventSink) -> Result<(), SpeechError> {
        self.spoken.lock().unwrap().push(u.text.clone());
        sink.emit(u.id, RawEvent::Started);
        let words: Vec<(usize, &str)> = u
            .text
            .split(' ')
            .scan(0usize, |at, w| {
                let start = *at;
                *at += w.len() + 1;
                Some((start, w))
            })
            .filter(|(_, w)| !w.is_empty())
            .collect();
        let crash = !self.crashed && u.kind == UtteranceKind::Text && u.text.contains("Crash");
        for (i, (start, w)) in words.iter().enumerate() {
            let end = start + w.trim_end_matches('.').len();
            sink.emit(
                u.id,
                RawEvent::Word {
                    byte_range: u32::try_from(*start).unwrap()..u32::try_from(end).unwrap(),
                    audio_ms: None,
                },
            );
            if crash && i == 1 {
                self.crashed = true;
                sink.emit(
                    u.id,
                    RawEvent::Error("the voice stopped unexpectedly".into()),
                );
                return Ok(());
            }
        }
        sink.emit(u.id, RawEvent::Finished);
        Ok(())
    }

    fn stop(&mut self) {}
}

#[test]
fn a_crash_mid_reading_restarts_and_the_reading_goes_on() {
    let spoken = Arc::new(Mutex::new(Vec::new()));
    let backend_spoken = spoken.clone();
    let speech = SpeechService::spawn(
        Box::new(move || {
            Ok(Box::new(CrashOnce {
                crashed: false,
                spoken: backend_spoken.clone(),
            }) as _)
        }),
        ServiceConfig::default(),
    )
    .unwrap();
    let said = Said::default();
    let mut app = App::new(AppConfig {
        speech,
        announcer: Box::new(said.clone()),
        self_voicing: false,
        backend_name: "crash-once".into(),
        ..AppConfig::for_tests()
    });
    let text = "Before the trouble. Crash right here in this sentence. After it all.";
    app.open_document(
        Document::from_plain_text(text),
        DocKey::untitled(1),
        "T".into(),
    );
    app.dispatch(Command::Action(ActionId::ReadFromCursor));
    let deadline = Instant::now() + Duration::from_secs(20);
    while app.playback() != Playback::Idle && Instant::now() < deadline {
        app.poll_speech();
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(app.playback(), Playback::Idle, "the reading never ended");
    let texts = spoken.lock().unwrap().clone();
    // Read on from the last word heard ("right"), then the rest.
    let crash = texts.iter().position(|t| t.starts_with("Crash")).unwrap();
    assert!(
        texts[crash + 1..]
            .iter()
            .any(|t| t.starts_with("right here")),
        "{texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.starts_with("After it all")),
        "{texts:?}"
    );
    let messages = said.0.lock().unwrap().clone();
    assert!(
        messages.iter().any(|m| m
            == "Speech restarted: the voice stopped unexpectedly. Reading on from the last word."),
        "{messages:?}"
    );
    assert!(
        !messages.iter().any(|m| m.starts_with("Speech error")),
        "{messages:?}"
    );
    // The highlight followed to the end.
    let last = app.spoken_log().last().copied().unwrap();
    assert_eq!(&text[last.start.0..last.end.0], "all");
}
