//! Pauses written as markup in a document's text (`<break time="1s"/>`):
//! planned as pauses of their written length, recorded as silences by the
//! test double, and never spoken or shown.

use std::time::Duration;

use textweaver_app::core::{CharPos, CharRange, Utterance, UtteranceKind};
use textweaver_app::formats::{Registry, Source};
use textweaver_app::speech::backends::recording::Call;
use textweaver_app::speech::{
    PauseAt, PauseKind, RecordingBackend, ServiceConfig, SpeechService, SpeechStatus,
};
use textweaver_app::store::Settings;
use textweaver_app::text::Document;

const THREE: &str =
    "Ready.<break time=\"500ms\"/> Set. <break time=\"1s\"/> Go <break strength=\"strong\"/>now.";

fn load(text: &str, settings: &Settings) -> Document {
    let src = Source::Bytes {
        data: text.as_bytes().to_vec().into(),
        hint: "txt".into(),
    };
    Registry::with_builtins()
        .load(&src, &textweaver_app::load_options(settings))
        .expect("loads")
}

fn plan(doc: &Document) -> (Vec<Utterance>, Vec<PauseAt>) {
    textweaver_app::plan_with_written_pauses(
        doc,
        CharRange::new(0, doc.len_chars()),
        &textweaver_app::narration_policy(&Settings::default()),
        &[],
    )
}

#[test]
fn three_breaks_plan_three_pauses_of_their_lengths() {
    let doc = load(THREE, &Settings::default());
    // The screen text (the canonical text) has no markup.
    assert_eq!(doc.text().to_string(), "Ready. Set. Go now.");
    let (us, pauses) = plan(&doc);
    let spoken: Vec<&str> = us.iter().map(|u| u.text.as_str()).collect();
    assert!(
        spoken
            .iter()
            .all(|t| !t.contains('<') && !t.contains("break")),
        "{spoken:?}"
    );
    let lengths: Vec<u32> = pauses
        .iter()
        .map(|p| match p.kind {
            PauseKind::Written { ms } => ms,
            other => panic!("not a written pause: {other:?}"),
        })
        .collect();
    assert_eq!(lengths, [500, 1000, 1000]);
    // Each pause follows the utterance that ends at its break.
    for p in &pauses {
        assert!(
            us.iter()
                .any(|u| u.source_range().is_some_and(|r| r.end == p.after)),
            "no utterance ends at {:?}: {spoken:?}",
            p.after
        );
    }
    // Offsets stay exact across a break: "now." is where it is on screen.
    let now = us
        .iter()
        .find(|u| u.text.contains("now"))
        .expect("now is read");
    let at = u32::try_from(now.text.find("now").expect("now")).expect("short");
    let r = now.source_for(at..at + 3).expect("maps to the source");
    let text = doc.text().to_string();
    let chars: String = text.chars().skip(r.start.0).take(r.len()).collect();
    assert_eq!(chars, "now");
    assert_eq!(r.start, CharPos(15));
    // Chunk numbers run on across the cuts.
    for (i, u) in us.iter().enumerate() {
        assert_eq!(u.id.chunk as usize, i);
        assert_eq!(u.kind, UtteranceKind::Text);
    }
}

#[test]
fn the_recording_double_records_the_silences() {
    let doc = load(THREE, &Settings::default());
    let (us, pauses) = plan(&doc);
    let (backend, rec) = RecordingBackend::new();
    let service =
        SpeechService::spawn(backend.into_factory(), ServiceConfig::default()).expect("spawn");
    let reading = service.read_with_pauses(us, pauses);
    loop {
        let status = service
            .statuses()
            .recv_timeout(Duration::from_secs(10))
            .expect("the service answers");
        if matches!(status, SpeechStatus::Finished { generation } if generation == reading) {
            break;
        }
    }
    service.shutdown();
    let calls = rec.calls();
    let silences: Vec<u32> = calls
        .iter()
        .filter_map(|c| match c {
            Call::Silence { ms, .. } => Some(*ms),
            _ => None,
        })
        .collect();
    assert_eq!(silences, [500, 1000, 1000]);
    for c in &calls {
        if let Call::Speak(u) = c {
            assert!(!u.text.contains("break"), "spoken: {}", u.text);
        }
    }
}

#[test]
fn the_setting_off_reads_the_markup_as_text() {
    let mut settings = Settings::default();
    settings.speech.markup_pauses = false;
    let doc = load(THREE, &settings);
    assert_eq!(doc.text().to_string(), THREE);
    let (us, pauses) = plan(&doc);
    assert!(pauses.is_empty());
    assert!(us.iter().any(|u| u.text.contains("break")));
}
