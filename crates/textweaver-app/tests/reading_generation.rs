//! The app follows the reading generation `SpeechService::read` returns,
//! and plans continuous reading in windows (docs/history/audit-2026-09.md, findings
//! R1 and P1, patches S1 and S3; ported by Agent D4).

use std::time::{Duration, Instant};

use textweaver_app::core::CharPos;
use textweaver_app::formats::{LoadOptions, Registry, Source};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::DocKey;
use textweaver_app::testing::recording_service;
use textweaver_app::text::GoTo;
use textweaver_app::{App, AppConfig, Command, Playback};

fn markdown(text: &str) -> textweaver_app::text::Document {
    Registry::with_builtins()
        .load(
            &Source::Bytes {
                data: text.as_bytes().to_vec(),
                hint: "md".into(),
            },
            &LoadOptions::default(),
        )
        .unwrap()
}

fn wait_idle(app: &mut App, limit: Duration) {
    let deadline = Instant::now() + limit;
    while app.playback() != Playback::Idle && Instant::now() < deadline {
        app.poll_speech();
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn a_reading_that_starts_with_a_skipped_code_block_is_followed() {
    // The first utterance is "code block skipped" (skip_code is on by
    // default), anchored at the code block; the wave 1 guess from positions
    // rejected the whole reading, so the highlight never moved and the app
    // stayed in Reading after speech ended.
    let text = "Intro words here.\n\n```\ncode line\n```\n\nAfter the code we keep reading. And more words.\n";
    let doc = markdown(text);
    let canonical = doc.text().to_string();
    let (speech, log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        ..AppConfig::for_tests()
    });
    app.open_document(doc, DocKey::untitled(1), "T".into());
    let blank = canonical.find("\n\n").unwrap() + 1;
    app.dispatch(Command::GoTo(GoTo::Char(CharPos(blank))));
    app.dispatch(Command::Action(ActionId::ReadFromCursor));
    wait_idle(&mut app, Duration::from_secs(10));
    assert!(log.texts().iter().any(|t| t == "code block skipped"));
    assert!(!app.spoken_log().is_empty(), "the highlight never moved");
    assert_eq!(app.playback(), Playback::Idle, "stuck in Reading");
}

fn long_text() -> (String, usize) {
    // 200,000 chars: several reading windows.
    let mut text = String::new();
    let mut n = 0;
    while text.len() < 200_000 {
        text.push_str(&format!("Sentence number {n} is here. "));
        n += 1;
        if n % 7 == 0 {
            text.push_str("\n\n");
        }
    }
    (text, n)
}

#[test]
fn continuous_reading_plans_in_windows_and_reaches_the_end() {
    let (text, n) = long_text();
    let doc = markdown(&text);
    let canonical = doc.text().to_string();
    let (speech, log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        ..AppConfig::for_tests()
    });
    app.open_document(doc, DocKey::untitled(1), "T".into());
    app.dispatch(Command::Action(ActionId::ReadFromCursor));
    wait_idle(&mut app, Duration::from_secs(120));
    assert_eq!(app.playback(), Playback::Idle);
    let ranges = log.spoken_ranges();
    assert!(
        ranges.windows(2).all(|w| w[0].end <= w[1].start),
        "no sentence twice, none out of order"
    );
    let texts = log.texts();
    assert_eq!(
        texts
            .iter()
            .filter(|t| t.starts_with("Sentence number"))
            .count(),
        n,
        "every sentence read once"
    );
    let end = canonical.trim_end().chars().count();
    assert_eq!(
        ranges.last().map(|r| r.end.0),
        Some(end),
        "the reading reached the end"
    );
}

#[test]
fn reading_hands_the_service_only_a_window_at_a_time() {
    // Planning the whole rest of the document on every Read, jump, and
    // resume took a quarter of a second per key on 10 MB. The first
    // reading must plan well under the whole document.
    let (text, _) = long_text();
    let doc = markdown(&text);
    let (speech, log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        ..AppConfig::for_tests()
    });
    app.open_document(doc, DocKey::untitled(1), "T".into());
    app.dispatch(Command::Action(ActionId::ReadFromCursor));
    assert_eq!(app.playback(), Playback::Reading);
    let planned = app.planned_reading_end().map_or(0, |p| p.0);
    assert!(
        planned > 0 && planned < 100_000,
        "the first window ends at char {planned} of {}",
        text.len()
    );
    app.dispatch(Command::Action(ActionId::Stop));
    drop(log);
}
