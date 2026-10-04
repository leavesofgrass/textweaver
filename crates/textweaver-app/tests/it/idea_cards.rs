//! The Wave 9 idea cards (W9x-i), driven through `App::dispatch` without a
//! terminal: a picture with no description is said and reachable by the
//! graphic key, the document overview, and the reading passes.

use std::time::{Duration, Instant};

use textweaver_app::keymap::ActionId;
use textweaver_app::store::DocKey;
use textweaver_app::testing::{SpeechLog, recording_service};
use textweaver_app::text::Document;
use textweaver_app::{App, AppConfig, Command};

/// A silent app with an HTML page `html` open from a file.
fn app_with_html(html: &str) -> (App, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("page.html");
    std::fs::write(&file, html).unwrap();
    let mut app = App::new(AppConfig::for_tests());
    app.open(&file).unwrap();
    (app, dir)
}

fn act(app: &mut App, a: ActionId) {
    app.dispatch(Command::Action(a));
}

#[test]
fn the_graphic_key_lands_on_a_picture_with_no_description() {
    let (mut app, _dir) = app_with_html(
        "<html><head><title>Cells</title></head><body><p>Intro.</p><img src=deco.png alt=\"\"><img src=fig1.png><p>After the figure.</p></body></html>",
    );
    act(&mut app, ActionId::NextGraphic);
    let said = app.status_text();
    assert!(said.contains("no description"), "{said}");
    assert!(said.starts_with("Graphic"), "{said}");
}

#[test]
fn the_overview_says_the_title_and_counts_first() {
    let (mut app, _dir) = app_with_html(
        "<html><head><title>Cells</title></head><body><h1>Cells</h1><p>One.</p><h2>Parts</h2><table><tr><th>A</th></tr><tr><td>1</td></tr></table><img src=a.png alt=\"A cell\"><img src=b.png></body></html>",
    );
    act(&mut app, ActionId::DocumentOverview);
    assert_eq!(
        app.status_text(),
        "Cells. Headings: 2, tables: 1, pictures: 2, footnotes: 0. Less than a minute left."
    );
}

/// A self-voicing app recording what it says, with `text` open.
fn voiced_app(text: &str) -> (App, SpeechLog) {
    let (speech, log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        self_voicing: true,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    app.open_document(
        Document::from_plain_text(text),
        DocKey::untitled(1),
        "T".into(),
    );
    (app, log)
}

fn wait_for_speech(log: &SpeechLog, needle: &str) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let texts = log.texts();
        if texts.iter().any(|t| t.contains(needle)) || Instant::now() > deadline {
            return texts;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_reading_pass_is_named_and_skims_continuous_reading() {
    let (mut app, log) = voiced_app("One. Two. Three.\n\nFour. Five.\n");
    act(&mut app, ActionId::ReadingPass);
    assert_eq!(app.status_text(), "Pass: first sentences.");
    log.clear();
    act(&mut app, ActionId::ReadFromCursor);
    let said = wait_for_speech(&log, "Four.");
    let at = |needle: &str| said.iter().position(|t| t.contains(needle));
    let lead = at("Pass: first sentences.").expect("the pass is named");
    assert!(lead < at("One.").unwrap(), "{said:?}");
    assert!(at("Two.").is_none() && at("Five.").is_none(), "{said:?}");
    act(&mut app, ActionId::Stop);
    // Headings, then back to the full text.
    act(&mut app, ActionId::ReadingPass);
    assert_eq!(app.status_text(), "Pass: headings.");
    act(&mut app, ActionId::ReadingPass);
    assert_eq!(app.status_text(), "Pass: full text.");
}

#[test]
fn the_time_left_changes_by_the_minute_never_by_the_word() {
    // W9b-r: the window's status bar ends with the time left. Over 200
    // one-word moves it changes only when a whole minute passes, and the
    // position only when a whole percent does.
    let text = "word ".repeat(6000);
    let mut app = App::new(AppConfig::for_tests());
    app.open_document(
        Document::from_plain_text(&text),
        DocKey::untitled(1),
        "T".into(),
    );
    let wpm = app.settings().speech.rate.wpm().max(1) as usize;
    assert_eq!(app.minutes_left(), Some(6000 / wpm));
    let (mut minutes, mut position) = (app.minutes_left(), app.title_position());
    let (mut minute_changes, mut position_changes) = (0, 0);
    for k in 1..=200 {
        app.dispatch(Command::SetCursor(textweaver_app::core::CharPos(k * 5)));
        let m = app.minutes_left();
        assert!(m <= minutes, "the time left never grows going forward");
        minute_changes += usize::from(m != minutes);
        minutes = m;
        let p = app.title_position();
        position_changes += usize::from(p != position);
        position = p;
    }
    // 200 words: at most one minute more than 200 / wpm, and 1,000 of
    // 30,000 chars: at most four whole percents.
    assert!(minute_changes <= 200 / wpm + 1, "{minute_changes}");
    assert!(position_changes <= 4, "{position_changes}");
    let empty = App::new(AppConfig::for_tests());
    assert_eq!(empty.minutes_left(), None);
}
