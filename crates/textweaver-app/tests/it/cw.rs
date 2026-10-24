//! CW (Morse code) as the engine: structure sent as prosigns, nothing
//! spelled out in words, and the rate keys changing the character speed.
//! The recording double stands in for the engine under the id `cw`;
//! nothing is played.

use std::time::{Duration, Instant};

use textweaver_app::formats::{LoadOptions, Registry, Source};
use textweaver_app::keymap::ActionId;
use textweaver_app::speech::Caps;
use textweaver_app::store::{DocKey, Settings};
use textweaver_app::testing::{SpeechLog, recording_service_with};
use textweaver_app::{App, AppConfig, Command, Playback};

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
    let (speech, log) = recording_service_with("cw", Caps::RAW_TEXT).unwrap();
    let mut app = App::new(AppConfig {
        speech,
        self_voicing: true,
        backend_name: "CW (Morse code)".into(),
        settings,
        ..AppConfig::for_tests()
    });
    app.open_document(doc, DocKey::untitled(1), "T".into());
    (app, log)
}

fn wait_idle(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.playback() != Playback::Idle && Instant::now() < deadline {
        app.poll_speech();
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(app.playback(), Playback::Idle, "still reading");
}

const DOC: &str = "# Alpha\n\nOne fish.\n\nTwo fish.\n";

#[test]
fn structure_goes_as_prosigns_not_words() {
    let (mut app, log) = app_with(DOC, Settings::default());
    app.dispatch(Command::Action(ActionId::ReadFromCursor));
    wait_idle(&mut app);
    let texts = log.texts();
    // The text exactly as written: no normalizer turned <BT> into words.
    assert_eq!(
        texts,
        [
            "Opened T.",
            "Alpha",
            "<BT>",
            "One fish.",
            "<BT>",
            "Two fish.",
            "<AR>"
        ],
        "{texts:?}"
    );
    assert!(!texts.iter().any(|t| t.contains("heading")));
}

#[test]
fn prosigns_off_sends_the_text_alone() {
    let mut settings = Settings::default();
    settings.speech.cw.prosigns = false;
    let (mut app, log) = app_with(DOC, settings);
    app.dispatch(Command::Action(ActionId::ReadFromCursor));
    wait_idle(&mut app);
    assert_eq!(
        log.texts(),
        ["Opened T.", "Alpha", "One fish.", "Two fish."]
    );
}

#[test]
fn rate_keys_change_the_character_speed_and_farnsworth_with_it() {
    let (mut app, _log) = app_with(DOC, Settings::default());
    let rate = app.settings().speech.rate;
    app.dispatch(Command::Action(ActionId::RateUp));
    let cw = &app.settings().speech.cw;
    assert_eq!((cw.wpm, cw.effective_wpm), (21, 11));
    assert_eq!(app.settings().speech.rate, rate, "the speech rate stays");
    assert_eq!(
        app.status_text(),
        "Character speed 21, effective 11 words per minute."
    );
    app.dispatch(Command::Action(ActionId::RateDown));
    app.dispatch(Command::Action(ActionId::RateDown));
    let cw = &app.settings().speech.cw;
    assert_eq!((cw.wpm, cw.effective_wpm), (19, 10));
}

/// The rate part of the title line, as drawn and as "say status" says it.
fn rate_parts(app: &App) -> String {
    app.title_parts(app.title_position().as_deref())
        .into_iter()
        .filter(|p| p.starts_with("CW ") && p.contains("wpm"))
        .collect::<Vec<_>>()
        .join(" | ")
}

#[test]
fn the_title_line_shows_the_cw_speeds_not_the_speech_rate() {
    let (mut app, _log) = app_with(DOC, Settings::default());
    let parts = app.title_parts(app.title_position().as_deref());
    assert!(
        !parts.iter().any(|p| p == "265 wpm"),
        "the speech rate is not shown: {parts:?}"
    );
    assert_eq!(rate_parts(&app), "CW 20 wpm, effective 10");
    // The speeds come before the engine's name, and the line starts with
    // the place, so a 40-cell Braille line shows the character speed.
    let line = parts.join(", ");
    assert!(
        line.ends_with("CW 20 wpm, effective 10, CW (Morse code)"),
        "{line}"
    );
    assert!(line.starts_with("Line 1 of"), "{line}");
    // The rate keys' change shows at once.
    app.dispatch(Command::Action(ActionId::RateUp));
    assert_eq!(rate_parts(&app), "CW 21 wpm, effective 11");
    // Said in words.
    app.dispatch(Command::Action(ActionId::SayStatus));
    let said = app.status_text().to_owned();
    assert!(
        said.contains(", CW at 21 words per minute, effective 11, CW (Morse code)."),
        "{said}"
    );
}

#[test]
fn without_farnsworth_spacing_only_the_character_speed_shows() {
    let mut settings = Settings::default();
    settings.speech.cw.effective_wpm = 20;
    let (mut app, _log) = app_with(DOC, settings);
    assert_eq!(rate_parts(&app), "CW 20 wpm");
    app.dispatch(Command::Action(ActionId::SayStatus));
    let said = app.status_text().to_owned();
    assert!(said.contains(", CW at 20 words per minute, CW"), "{said}");
    assert!(!said.contains("effective"), "{said}");
    // An effective speed above the character speed is plain spacing too.
    app.set_setting("speech.cw.effective_wpm", serde_json::json!(30))
        .unwrap();
    assert_eq!(rate_parts(&app), "CW 20 wpm");
    // Farnsworth back on, from the settings: shown at once.
    app.set_setting("speech.cw.effective_wpm", serde_json::json!(12))
        .unwrap();
    assert_eq!(rate_parts(&app), "CW 20 wpm, effective 12");
}

#[test]
fn another_engine_shows_the_speech_rate() {
    let (speech, _log) = textweaver_app::testing::recording_service().unwrap();
    let app = App::new(AppConfig {
        speech,
        self_voicing: true,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    let parts = app.title_parts(None);
    assert!(parts.iter().any(|p| p == "265 wpm"), "{parts:?}");
    assert!(!parts.iter().any(|p| p.starts_with("CW ")), "{parts:?}");
}
