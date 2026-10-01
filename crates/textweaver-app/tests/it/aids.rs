//! Reading aids in the app (Agent D3, ADR-0022): RSVP on the app's clock
//! and following speech, navigation while RSVP shows, bionic reading, the
//! reading ruler, and the reading level.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use textweaver_app::a11y::{Announcer, Priority};
use textweaver_app::aids::RulerMode;
use textweaver_app::core::{CharPos, CharRange};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::{DocKey, Settings};
use textweaver_app::testing::recording_service;
use textweaver_app::text::{Document, GoTo};
use textweaver_app::{App, AppConfig, Command, Effect, Playback};

#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Said {
    fn all(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
    fn last(&self) -> String {
        self.all().last().cloned().unwrap_or_default()
    }
}

impl Announcer for Said {
    fn announce(&mut self, text: &str, _priority: Priority) {
        self.0.lock().unwrap().push(text.to_owned());
    }
}

const PROSE: &str = "Alpha beta gamma. Delta epsilon zeta.\n\nEta theta iota kappa.";

fn rig_with(text: &str, settings: Settings) -> (App, Said) {
    let said = Said::default();
    let (speech, _log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        settings,
        speech,
        announcer: Box::new(said.clone()),
        ..AppConfig::for_tests()
    });
    app.open_document(
        Document::from_plain_text(text),
        DocKey::untitled(1),
        "Test".into(),
    );
    (app, said)
}

fn rig(text: &str) -> (App, Said) {
    rig_with(text, Settings::default())
}

fn act(app: &mut App, a: ActionId) -> Vec<Effect> {
    app.dispatch(Command::Action(a))
}

fn at(text: &str, needle: &str) -> CharPos {
    CharPos(text.find(needle).unwrap())
}

fn word(app: &App) -> String {
    app.rsvp().and_then(|r| r.frame()).unwrap().text.to_owned()
}

#[test]
fn rsvp_shows_the_word_at_the_cursor_and_steps_on_the_clock() {
    let (mut app, said) = rig(PROSE);
    app.dispatch(Command::GoTo(GoTo::Char(at(PROSE, "beta"))));
    act(&mut app, ActionId::RsvpToggle);
    assert!(
        said.last().starts_with("RSVP on. Word 2 of 10"),
        "{}",
        said.last()
    );
    assert_eq!(word(&app), "beta");
    assert_eq!(app.rsvp_wait(Instant::now()), None, "paused: no timer");

    act(&mut app, ActionId::RsvpPlayPause);
    assert_eq!(said.last(), "RSVP playing.");
    let wait = app
        .rsvp_wait(Instant::now())
        .expect("a deadline while playing");
    assert!(
        wait <= Duration::from_millis(200),
        "300 wpm is 200 ms: {wait:?}"
    );
    // Before the deadline nothing moves; after it, one word per tick.
    assert!(app.tick(Instant::now()).is_empty());
    let later = Instant::now() + Duration::from_millis(250);
    assert_eq!(app.tick(later), vec![Effect::Redraw]);
    assert_eq!(word(&app), "gamma.");
    assert_eq!(
        app.session().unwrap().cursor,
        at(PROSE, "gamma"),
        "the cursor follows the word"
    );
    // A very late tick still shows only the next word.
    app.tick(Instant::now() + Duration::from_secs(60));
    assert_eq!(word(&app), "Delta");

    act(&mut app, ActionId::RsvpPlayPause);
    assert_eq!(said.last(), "RSVP paused.");
    act(&mut app, ActionId::RsvpToggle);
    assert_eq!(said.last(), "RSVP off.");
    assert!(app.rsvp().is_none());
}

#[test]
fn navigation_moves_the_rsvp_word_and_stop_closes_it() {
    let (mut app, said) = rig(PROSE);
    act(&mut app, ActionId::RsvpToggle);
    act(&mut app, ActionId::NextSentence);
    assert_eq!(word(&app), "Delta");
    act(&mut app, ActionId::CaretNextWord);
    assert_eq!(word(&app), "epsilon");
    act(&mut app, ActionId::NextParagraph);
    assert_eq!(word(&app), "Eta");
    assert_eq!(app.session().unwrap().cursor, at(PROSE, "Eta"));
    act(&mut app, ActionId::PreviousParagraph);
    assert_eq!(word(&app), "Alpha");
    act(&mut app, ActionId::CaretPreviousWord);
    assert_eq!(said.last(), "Start of text.");
    // Space starts it when nothing is read aloud.
    act(&mut app, ActionId::PlayPause);
    assert_eq!(said.last(), "RSVP playing.");
    assert_eq!(app.playback(), Playback::Idle, "no speech");
    act(&mut app, ActionId::Stop);
    assert_eq!(said.last(), "RSVP off.");
    assert!(app.rsvp().is_none());
}

#[test]
fn rsvp_rate_and_position_are_saved_settings() {
    let (mut app, said) = rig(PROSE);
    act(&mut app, ActionId::RsvpFaster);
    assert_eq!(said.last(), "RSVP 325 words per minute.");
    assert_eq!(app.settings().reading_aids.rsvp.wpm, 325);
    act(&mut app, ActionId::RsvpSlower);
    act(&mut app, ActionId::RsvpSlower);
    assert_eq!(app.settings().reading_aids.rsvp.wpm, 275);
    act(&mut app, ActionId::RsvpPositionNext);
    assert_eq!(said.last(), "RSVP at the top right.");
}

#[test]
fn rsvp_continues_into_the_next_window_of_a_long_document() {
    let long: String = (0..20_000).map(|i| format!("w{i} ")).collect();
    let (mut app, _said) = rig(&long);
    act(&mut app, ActionId::RsvpToggle);
    let first = app.rsvp().unwrap().track().range();
    assert!(first.end.0 < long.len(), "a window, not the whole text");
    // Jump to the last word of the window and step past it.
    let last = app.rsvp().unwrap().len() - 1;
    let mut app2 = app;
    for _ in 0..last {
        act(&mut app2, ActionId::CaretNextWord);
    }
    let before = word(&app2);
    act(&mut app2, ActionId::CaretNextWord);
    let after = word(&app2);
    assert_ne!(before, after);
    assert!(app2.rsvp().unwrap().track().range().start > first.start);
}

#[test]
fn speech_drives_the_rsvp_word_while_reading() {
    let (mut app, _said) = rig(PROSE);
    act(&mut app, ActionId::RsvpToggle);
    act(&mut app, ActionId::ReadFromCursor);
    let deadline = Instant::now() + Duration::from_secs(30);
    while app.playback() != Playback::Idle {
        app.poll_speech();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    // The last word spoken is the RSVP word.
    assert_eq!(word(&app), "kappa.");
}

#[test]
fn bionic_ruler_and_reading_level() {
    let (mut app, said) = rig(PROSE);
    let all = CharRange::new(0, PROSE.len());
    assert!(app.bionic_ranges(all).is_empty(), "off by default");
    act(&mut app, ActionId::BionicToggle);
    assert_eq!(said.last(), "Bionic reading on.");
    let bold = app.bionic_ranges(all);
    // "Alpha": 2 of 5 letters.
    assert_eq!(bold[0], CharRange::new(0, 2));
    act(&mut app, ActionId::BionicToggle);
    assert_eq!(said.last(), "Bionic reading off.");

    assert_eq!(app.ruler().mode, RulerMode::Off);
    act(&mut app, ActionId::RulerCycle);
    assert_eq!(said.last(), "Current line marked.");
    act(&mut app, ActionId::RulerCycle);
    assert_eq!(said.last(), "Reading ruler on.");
    assert_eq!(app.ruler().mode, RulerMode::Ruler);
    act(&mut app, ActionId::RulerCycle);
    assert_eq!(said.last(), "Reading ruler off.");

    act(&mut app, ActionId::ReadingLevel);
    assert!(
        said.last().starts_with("Document: Grade"),
        "{}",
        said.last()
    );
    app.dispatch(Command::Select(CharRange::new(0, 17)));
    act(&mut app, ActionId::ReadingLevel);
    assert!(
        said.last().starts_with("Selection: Grade"),
        "{}",
        said.last()
    );
}

#[test]
fn terminal_spacing_comes_from_the_settings() {
    let (app, _) = rig(PROSE);
    let t = app.terminal_spacing();
    assert_eq!((t.rows_between_lines, t.extra_word_spaces), (0, 0));
    let mut s = Settings::default();
    s.reading_aids.spacing.line_height = 2.0;
    s.reading_aids.spacing.word_spacing = 0.5;
    let (app, _) = rig_with(PROSE, s);
    let t = app.terminal_spacing();
    assert_eq!((t.rows_between_lines, t.extra_word_spaces), (1, 2));
}
