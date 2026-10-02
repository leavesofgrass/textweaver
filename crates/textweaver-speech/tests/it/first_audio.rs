//! The stop-to-first-audio stamp (W8b-i): the speech thread records when a
//! reading first sounds, once per reading, and a stop before it sounds
//! records nothing.

use std::time::{Duration, Instant};

use textweaver_speech::core::{CharPos, Utterance};
use textweaver_speech::{RecordingBackend, RecordingMode, ServiceConfig, SpeechService};

fn sentence(text: &str) -> Vec<Utterance> {
    vec![Utterance::literal(text, CharPos(0))]
}

#[test]
fn each_reading_is_stamped_when_it_first_sounds() {
    let (backend, _handle) = RecordingBackend::new();
    let service =
        SpeechService::spawn(backend.into_factory(), ServiceConfig::default()).expect("spawn");
    let probe = service.first_audio();
    assert_eq!(probe.latest(), None, "nothing has sounded yet");

    let before = Instant::now();
    let first = service.read(sentence("Hello brave world."));
    let stamp = probe
        .wait_for(first, Duration::from_secs(5))
        .expect("the first reading sounded");
    assert_eq!(stamp.generation, first);
    assert!(stamp.requested >= before && stamp.started >= stamp.requested);
    assert!(stamp.latency() < Duration::from_secs(5));

    service.stop();
    let second = service.read(sentence("Again from the cursor."));
    assert!(second > first);
    let again = probe
        .wait_for(second, Duration::from_secs(5))
        .expect("the restarted reading sounded");
    assert_eq!(again.generation, second);
    assert!(again.started >= stamp.started);
}

#[test]
fn a_reading_stopped_before_it_sounds_is_not_stamped() {
    // Manual mode: the backend says nothing until the test makes it.
    let (backend, _handle) =
        RecordingBackend::with(RecordingMode::Manual, RecordingBackend::DEFAULT_CAPS);
    let service =
        SpeechService::spawn(backend.into_factory(), ServiceConfig::default()).expect("spawn");
    let probe = service.first_audio();
    let generation = service.read(sentence("Never heard."));
    service.stop();
    assert!(service.sync());
    assert_eq!(
        probe.wait_for(generation, Duration::from_millis(100)),
        None,
        "no audio, no stamp"
    );
}
