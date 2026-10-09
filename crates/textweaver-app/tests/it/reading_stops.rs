//! Stop at the end of a section (`[reading] stop_at`), repeat slower, and
//! the reading timer (`[reading] stop_after_minutes`), B1-s2. Driven
//! through `App::dispatch` with the recording speech double; nothing is
//! played.

use std::time::{Duration, Instant};

use textweaver_app::core::{CharPos, Rate};
use textweaver_app::formats::{LoadOptions, Registry, Source};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::{DocKey, Settings, StopAt};
use textweaver_app::testing::{SpeechLog, recording_service};
use textweaver_app::{App, AppConfig, Command, Playback};

/// A self-voicing app reading Markdown `text` with `settings`.
fn app_with(text: &str, settings: Settings) -> (App, SpeechLog) {
    let doc = Registry::with_builtins()
        .load(
            &Source::Bytes {
                data: text.as_bytes().to_vec(),
                hint: "md".into(),
            },
            &LoadOptions::default(),
        )
        .unwrap();
    let (speech, log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        self_voicing: true,
        backend_name: "test-recording".into(),
        settings,
        ..AppConfig::for_tests()
    });
    app.open_document(doc, DocKey::untitled(1), "T".into());
    (app, log)
}

fn act(app: &mut App, a: ActionId) {
    app.dispatch(Command::Action(a));
}

fn wait_idle(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.playback() != Playback::Idle && Instant::now() < deadline {
        app.poll_speech();
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(app.playback(), Playback::Idle, "still reading");
}

fn cursor(app: &App) -> CharPos {
    app.session().unwrap().cursor
}

/// Where `needle` starts in the open document's text.
fn at(app: &App, needle: &str) -> CharPos {
    let text = app.session().unwrap().doc.text().to_string();
    let byte = text.find(needle).unwrap();
    CharPos(text[..byte].chars().count())
}

fn said(log: &SpeechLog, needle: &str) -> bool {
    log.texts().iter().any(|t| t.contains(needle))
}

const SECTIONS: &str =
    "# Alpha\n\nOne fish. Two fish.\n\n## Beta\n\nRed fish.\n\n# Gamma\n\nBlue fish.\n";

#[test]
fn reading_stops_at_the_next_heading_and_goes_on_from_it() {
    let mut settings = Settings::default();
    settings.reading.stop_at = StopAt::Heading;
    let (mut app, log) = app_with(SECTIONS, settings);
    act(&mut app, ActionId::ReadFromCursor);
    wait_idle(&mut app);
    assert!(said(&log, "Two fish."), "{:?}", log.texts());
    assert!(
        !said(&log, "Beta") && !said(&log, "Red fish."),
        "{:?}",
        log.texts()
    );
    let msg = app.status_text().to_owned();
    assert!(msg.starts_with("End of section. "), "{msg}");
    assert!(msg.ends_with(" to go on."), "{msg}");
    assert_eq!(cursor(&app), at(&app, "Beta"));
    // The read key goes on with the next section, and stops again.
    log.clear();
    act(&mut app, ActionId::ReadFromCursor);
    wait_idle(&mut app);
    assert!(said(&log, "Red fish."), "{:?}", log.texts());
    assert!(!said(&log, "Blue fish."), "{:?}", log.texts());
    assert_eq!(cursor(&app), at(&app, "Gamma"));
}

#[test]
fn chapter_stops_only_at_a_level_one_heading() {
    let mut settings = Settings::default();
    settings.reading.stop_at = StopAt::Chapter;
    let (mut app, log) = app_with(SECTIONS, settings);
    act(&mut app, ActionId::ReadFromCursor);
    wait_idle(&mut app);
    assert!(said(&log, "Red fish."), "{:?}", log.texts());
    assert!(!said(&log, "Blue fish."), "{:?}", log.texts());
    assert_eq!(cursor(&app), at(&app, "Gamma"));
    assert!(app.status_text().starts_with("End of section."));
}

#[test]
fn a_recall_prompt_names_the_section_and_stops_at_the_next_heading() {
    let mut settings = Settings::default();
    settings.reading.recall_prompts = true;
    let (mut app, log) = app_with(SECTIONS, settings);
    act(&mut app, ActionId::ReadFromCursor);
    wait_idle(&mut app);
    assert!(said(&log, "Two fish."), "{:?}", log.texts());
    assert!(!said(&log, "Red fish."), "{:?}", log.texts());
    let msg = app.status_text().to_owned();
    assert!(
        msg.starts_with("Say what you remember from Alpha. "),
        "{msg}"
    );
    assert!(msg.ends_with(" to go on."), "{msg}");
    assert_eq!(cursor(&app), at(&app, "Beta"));
    act(&mut app, ActionId::ReadFromCursor);
    wait_idle(&mut app);
    assert!(
        app.status_text()
            .starts_with("Say what you remember from Beta. ")
    );
}

#[test]
fn off_reads_to_the_end() {
    let (mut app, log) = app_with(SECTIONS, Settings::default());
    act(&mut app, ActionId::ReadFromCursor);
    wait_idle(&mut app);
    assert!(said(&log, "Blue fish."), "{:?}", log.texts());
    assert!(!app.status_text().starts_with("End of section."));
}

#[test]
fn repeat_slower_says_the_sentence_slower_then_the_rate_comes_back() {
    let (mut app, log) = app_with(
        "One fish two fish. Red fish blue fish.\n",
        Settings::default(),
    );
    let usual = app.settings().speech.rate;
    act(&mut app, ActionId::RepeatSentenceSlower);
    wait_idle(&mut app);
    app.wait_for_speech_thread();
    let texts = log.texts();
    assert!(said(&log, "One fish two fish."), "{texts:?}");
    assert!(!said(&log, "Red fish"), "only the sentence: {texts:?}");
    let rates: Vec<Rate> = log.all_params().iter().map(|p| p.rate).collect();
    assert!(rates.contains(&usual.step(-60)), "{rates:?}");
    assert_eq!(log.params().map(|p| p.rate), Some(usual), "{rates:?}");
    assert_eq!(
        app.settings().speech.rate,
        usual,
        "the setting is untouched"
    );
    assert_eq!(cursor(&app), CharPos(0));
}

#[test]
fn the_reading_timer_stops_at_the_end_of_the_sentence() {
    let mut settings = Settings::default();
    settings.reading.stop_after_minutes = 1;
    let (mut app, log) = app_with(
        "One fish two fish. Red fish blue fish. Old fish new fish.\n",
        settings,
    );
    act(&mut app, ActionId::ReadFromCursor);
    // A minute of reading has passed before the first word is reported.
    app.tick(Instant::now() + Duration::from_secs(61));
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.playback() != Playback::Idle && Instant::now() < deadline {
        if app.poll_speech_step().is_none() {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    let msg = app.status_text().to_owned();
    assert!(msg.starts_with("Time is up after 1 minute. "), "{msg}");
    assert_eq!(cursor(&app), at(&app, "Red fish"));
    // The highlight never reached the next sentence.
    let red = at(&app, "Red fish");
    assert!(
        app.spoken_log().iter().all(|r| r.start < red),
        "{:?}",
        log.texts()
    );
    // Stop starts the clock over: reading goes on without stopping.
    act(&mut app, ActionId::Stop);
    act(&mut app, ActionId::ReadFromCursor);
    app.tick(Instant::now() + Duration::from_secs(30));
    wait_idle(&mut app);
    assert!(said(&log, "Old fish new fish."), "{:?}", log.texts());
}

#[test]
fn the_timer_is_off_at_zero() {
    let (mut app, log) = app_with("One fish. Two fish.\n", Settings::default());
    act(&mut app, ActionId::ReadFromCursor);
    app.tick(Instant::now() + Duration::from_secs(24 * 3600));
    wait_idle(&mut app);
    assert!(said(&log, "Two fish."), "{:?}", log.texts());
}
