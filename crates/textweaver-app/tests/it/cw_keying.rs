//! CW keying: Morse code from a key adapter typed at the cursor in edit
//! mode. The keyboard adapters' keys are given with their times, so the
//! keyer and the decoder see exact timing; the test waits only for the
//! keying thread's clock to pass the last silence. The speech double has
//! no audio device, so no sidetone is started: nothing is played.

use std::time::{Duration, Instant};

use textweaver_app::formats::{LoadOptions, Registry, Source};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::{CwKeyer, DocKey, Settings};
use textweaver_app::testing::recording_service;
use textweaver_app::{App, AppConfig, Command, CwAdapterKey};

fn editing(settings: Settings) -> App {
    let doc = Registry::with_builtins()
        .load(
            &Source::Bytes {
                data: b"Log:\n".to_vec(),
                hint: "md".into(),
            },
            &LoadOptions::default(),
        )
        .unwrap();
    let (speech, _log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        self_voicing: true,
        backend_name: "test-recording".into(),
        settings,
        ..AppConfig::for_tests()
    });
    app.open_document(doc, DocKey::untitled(1), "T".into());
    app.dispatch(Command::Action(ActionId::ToggleEditMode));
    assert!(app.is_editing());
    app
}

fn act(app: &mut App, a: ActionId) {
    app.dispatch(Command::Action(a));
}

fn text(app: &App) -> String {
    app.edit_session()
        .unwrap()
        .editor()
        .unwrap()
        .text()
        .to_string()
}

/// Ticks until the editor's text starts with `want`: keying types at the
/// caret, which edit mode puts at the start (at most ten seconds).
fn wait_for(app: &mut App, want: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !text(app).starts_with(want) {
        assert!(Instant::now() < deadline, "got {:?}", text(app));
        app.tick(Instant::now());
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// One unit at 20 words per minute, in milliseconds.
const U: u64 = 60;

/// How far ahead of the clock a test's keying starts, in milliseconds:
/// every contact is queued before the keying thread's clock reaches the
/// first, even on a busy machine.
const LEAD: u64 = 400;

/// Keys `code` (dots and dashes) by hand on `key` from `at`, with exact
/// spacing; returns the end of the last element.
fn hand_key(app: &mut App, key: CwAdapterKey, code: &str, mut at: Instant) -> Instant {
    for (i, e) in code.chars().enumerate() {
        if i > 0 {
            at += Duration::from_millis(U);
        }
        assert!(app.cw_adapter_key_at(key, true, at));
        at += Duration::from_millis(if e == '-' { 3 * U } else { U });
        assert!(app.cw_adapter_key_at(key, false, at));
    }
    at
}

fn straight() -> Settings {
    let mut s = Settings::default();
    s.speech.cw.keyer = CwKeyer::Straight;
    s
}

#[test]
fn keying_needs_edit_mode_and_is_a_toggle() {
    let mut app = editing(Settings::default());
    act(&mut app, ActionId::ToggleEditMode);
    assert!(!app.is_editing());
    act(&mut app, ActionId::CwKeying);
    assert!(!app.cw_keying_active());
    assert!(
        app.status_text()
            .starts_with("CW keying types in edit mode."),
        "{}",
        app.status_text()
    );
    // Not keying: the adapter keys are ordinary keys.
    assert!(!app.cw_adapter_key(CwAdapterKey::LeftBracket, true));
    act(&mut app, ActionId::ToggleEditMode);
    act(&mut app, ActionId::CwKeying);
    assert!(app.cw_keying_active());
    assert!(
        app.status_text().starts_with("CW keying on."),
        "{}",
        app.status_text()
    );
    act(&mut app, ActionId::CwKeying);
    assert!(!app.cw_keying_active());
    assert_eq!(app.status_text(), "CW keying off.");
}

#[test]
fn a_straight_key_types_words_with_spaces() {
    let mut app = editing(straight());
    act(&mut app, ActionId::CwKeying);
    let ms = Duration::from_millis;
    let start = Instant::now() + ms(LEAD);
    // C, Q, a word gap, then D, E.
    let mut at = hand_key(&mut app, CwAdapterKey::LeftControl, "-.-.", start);
    at = hand_key(&mut app, CwAdapterKey::LeftControl, "--.-", at + ms(3 * U));
    at = hand_key(&mut app, CwAdapterKey::RightBracket, "-..", at + ms(7 * U));
    hand_key(&mut app, CwAdapterKey::RightBracket, ".", at + ms(3 * U));
    wait_for(&mut app, "cq de ");
    assert_eq!(text(&app), "cq de Log:\n");
    // The status line carries what was keyed, for the Braille display.
    assert_eq!(app.status_text(), "CW: cq de ");
    // RSVP, refused in edit mode otherwise, shows the word keyed.
    assert!(app.rsvp().is_none());
    act(&mut app, ActionId::RsvpToggle);
    let word = app
        .rsvp()
        .and_then(|r| r.frame())
        .map(|f| f.text.to_owned());
    assert_eq!(word.as_deref(), Some("de"));
    assert_eq!(app.status_text(), "RSVP shows the words you key.");
    act(&mut app, ActionId::RsvpToggle);
    assert!(app.rsvp().is_none());
}

#[test]
fn the_iambic_keyer_in_mode_b_adds_the_squeezed_element() {
    let mut app = editing(Settings::default());
    act(&mut app, ActionId::CwKeying);
    let ms = Duration::from_millis;
    let t0 = Instant::now() + ms(LEAD);
    // Squeeze, dit first; both released during the dah (units 2 to 5):
    // di-dah-dit, an R, in mode B.
    app.cw_adapter_key_at(CwAdapterKey::LeftControl, true, t0);
    app.cw_adapter_key_at(CwAdapterKey::RightControl, true, t0 + ms(10));
    // A repeat of a held key is ignored.
    app.cw_adapter_key_at(CwAdapterKey::RightControl, true, t0 + ms(20));
    app.cw_adapter_key_at(CwAdapterKey::LeftControl, false, t0 + ms(3 * U));
    app.cw_adapter_key_at(CwAdapterKey::RightControl, false, t0 + ms(3 * U));
    wait_for(&mut app, "r ");
}

#[test]
fn paddles_swap_and_prosigns_have_their_text_forms() {
    let mut s = Settings::default();
    s.speech.cw.keyer = CwKeyer::IambicA;
    s.speech.cw.swap_paddles = true;
    let mut app = editing(s);
    act(&mut app, ActionId::CwKeying);
    let ms = Duration::from_millis;
    let t0 = Instant::now() + ms(LEAD);
    // Swapped: `]` is dit. A tap gives one element: E.
    app.cw_adapter_key_at(CwAdapterKey::RightBracket, true, t0);
    app.cw_adapter_key_at(CwAdapterKey::RightBracket, false, t0 + ms(U / 2));
    wait_for(&mut app, "e ");
    act(&mut app, ActionId::CwKeying);
    // AR in straight keying: typed as <AR>.
    let mut app = editing(straight());
    act(&mut app, ActionId::CwKeying);
    hand_key(
        &mut app,
        CwAdapterKey::LeftControl,
        ".-.-.",
        Instant::now() + ms(LEAD),
    );
    wait_for(&mut app, "<AR> ");
}

#[test]
fn the_error_sign_erases_the_word_keyed() {
    let mut app = editing(straight());
    act(&mut app, ActionId::CwKeying);
    let ms = Duration::from_millis;
    let mut at = hand_key(
        &mut app,
        CwAdapterKey::LeftControl,
        "-",
        Instant::now() + ms(LEAD),
    );
    at = hand_key(&mut app, CwAdapterKey::LeftControl, "--", at + ms(3 * U));
    hand_key(&mut app, CwAdapterKey::LeftControl, "....", at + ms(7 * U));
    wait_for(&mut app, "tm h");
    hand_key(
        &mut app,
        CwAdapterKey::LeftControl,
        "........",
        Instant::now() + ms(LEAD),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while text(&app) != "tm Log:\n" {
        assert!(Instant::now() < deadline, "got {:?}", text(&app));
        app.tick(Instant::now());
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        app.status_text().starts_with("CW: tm"),
        "{}",
        app.status_text()
    );
}

#[test]
fn leaving_edit_mode_ends_keying() {
    let mut app = editing(Settings::default());
    act(&mut app, ActionId::CwKeying);
    assert!(app.cw_keying_active());
    act(&mut app, ActionId::ToggleEditMode);
    if !app.is_editing() {
        app.tick(Instant::now());
        assert!(!app.cw_keying_active());
    }
}
