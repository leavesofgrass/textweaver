//! CW practice: Koch rounds, the score in words, the next character added
//! at the pass mark, progress kept, and groups sent through a CW engine.
//! Recording doubles stand in for the engines; nothing is played.

use std::time::{Duration, Instant};

use textweaver_app::keymap::ActionId;
use textweaver_app::speech::Caps;
use textweaver_app::store::cw_practice::{CwPractice, Drill};
use textweaver_app::store::{Paths, Settings};
use textweaver_app::testing::{SpeechLog, recording_service_as, recording_service_with};
use textweaver_app::{App, AppConfig, Command, Mode};

/// Two groups of five a round.
fn practice_settings() -> Settings {
    let mut s = Settings::default();
    s.speech.cw.practice_groups = 2;
    s.speech.cw.practice_group = 5;
    s
}

/// An app whose speech engine is CW (the recording double under `cw`).
fn cw_app(paths: Option<Paths>) -> (App, SpeechLog) {
    let (speech, log) = recording_service_with("cw", Caps::RAW_TEXT).unwrap();
    let mut app = App::new(AppConfig {
        speech,
        self_voicing: true,
        backend_name: "CW (Morse code)".into(),
        settings: practice_settings(),
        paths,
        ..AppConfig::for_tests()
    });
    app.seed_cw_practice(11);
    (app, log)
}

/// Waits until `log` has `n` utterances; returns their texts.
fn texts_after(log: &SpeechLog, n: usize) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while log.texts().len() < n && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    let texts = log.texts();
    assert!(texts.len() >= n, "{texts:?}");
    texts
}

/// The group just sent: the last utterance, once there are `n`.
fn group(log: &SpeechLog, n: usize) -> String {
    texts_after(log, n)[n - 1].clone()
}

/// `g` with its first character copied wrong.
fn one_wrong(g: &str) -> (char, char, String) {
    let first = g.chars().next().unwrap();
    let typed = if first == 'K' { 'R' } else { 'K' };
    (first, typed, format!("{typed}{}", &g[1..]))
}

#[test]
fn a_perfect_koch_round_adds_the_next_character_and_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, log) = cw_app(Some(Paths::under(dir.path())));
    assert_eq!(app.cw_koch_lesson(), 1);
    app.dispatch(Command::Action(ActionId::CwPracticeKoch));
    assert_eq!(app.mode(), Mode::Prompt);
    assert_eq!(
        app.status_text(),
        "Koch lesson 1 of 40, 2 characters, newest M. Group 1 of 2."
    );
    // The message, then the group queued after it on the CW engine.
    let first = group(&log, 2);
    assert_eq!(first.chars().count(), 5);
    assert!(first.chars().all(|c| c == 'K' || c == 'M'), "{first}");
    // Nothing shows the group before the answer.
    assert!(!app.status_text().contains(&first));

    app.dispatch(Command::Answer(first.to_lowercase()));
    assert_eq!(app.status_text(), "5 of 5, all correct. Group 2 of 2.");
    let second = group(&log, 4);
    app.dispatch(Command::Answer(second));
    assert_eq!(
        app.status_text(),
        "5 of 5, all correct. Round: 10 of 10, 100 percent. Lesson 2: added U."
    );
    assert_ne!(app.mode(), Mode::Prompt);
    assert_eq!(app.cw_koch_lesson(), 2);

    app.wait_for_writes();
    let kept = CwPractice::load(&Paths::under(dir.path())).unwrap();
    assert_eq!(kept.koch_lesson, 2);
    assert_eq!(kept.recent.len(), 1);
    let s = &kept.recent[0];
    assert_eq!(
        (s.drill, s.lesson, s.correct, s.total),
        (Drill::Koch, Some(1), 10, 10)
    );
}

#[test]
fn a_miss_is_said_in_words_and_below_the_mark_no_character_is_added() {
    let (mut app, log) = cw_app(None);
    app.dispatch(Command::Action(ActionId::CwPracticeKoch));
    let first = group(&log, 2);
    let (sent, typed, answer) = one_wrong(&first);
    app.dispatch(Command::Answer(answer));
    let spaced: Vec<String> = first.chars().map(String::from).collect();
    assert_eq!(
        app.status_text(),
        format!(
            "4 of 5. Missed: {sent} as {typed}. Sent: {}. Group 2 of 2.",
            spaced.join(" ")
        )
    );
    let second = group(&log, 4);
    // One character not copied at all.
    app.dispatch(Command::Answer(second[1..].to_owned()));
    let status = app.status_text();
    assert!(
        status.starts_with(&format!("4 of 5. Missed: {} not copied.", &second[..1])),
        "{status}"
    );
    assert!(
        status.ends_with("Round: 8 of 10, 80 percent. 90 percent adds the next character."),
        "{status}"
    );
    assert_eq!(app.cw_koch_lesson(), 1);
}

#[test]
fn an_empty_answer_sends_the_group_again_and_escape_stops() {
    let (mut app, log) = cw_app(None);
    app.dispatch(Command::Action(ActionId::CwPracticeKoch));
    let first = group(&log, 2);
    app.dispatch(Command::Answer(String::new()));
    assert_eq!(app.status_text(), "Again, group 1 of 2.");
    assert_eq!(group(&log, 4), first);
    assert_eq!(app.mode(), Mode::Prompt);
    app.dispatch(Command::Cancel);
    assert_eq!(app.status_text(), "Practice stopped.");
    assert_ne!(app.mode(), Mode::Prompt);
}

#[test]
fn with_another_engine_the_group_goes_to_practices_own_cw_engine_later() {
    let (speech, main) = recording_service_as("test-recording").unwrap();
    let mut app = App::new(AppConfig {
        speech,
        self_voicing: true,
        settings: practice_settings(),
        ..AppConfig::for_tests()
    });
    app.seed_cw_practice(5);
    let (cw, sent) = recording_service_with("cw", Caps::RAW_TEXT).unwrap();
    app.set_cw_practice_speech(cw);
    app.dispatch(Command::Action(ActionId::CwPracticeCharacters));
    assert_eq!(
        app.status_text(),
        "Random characters, 2 groups. Group 1 of 2."
    );
    // Not before the message has had time to be said.
    app.tick(Instant::now());
    let said = texts_after(&main, 1);
    assert_eq!(said, ["Random characters, 2 groups. Group 1 of 2."]);
    assert!(sent.texts().is_empty(), "{:?}", sent.texts());
    app.tick(Instant::now() + Duration::from_secs(60));
    let g = group(&sent, 1);
    assert_eq!(g.chars().count(), 5);
    // Never on the reader's engine.
    assert!(!main.texts().contains(&g));
    app.dispatch(Command::Answer(g));
    assert_eq!(app.status_text(), "5 of 5, all correct. Group 2 of 2.");
}

#[test]
fn without_a_cw_engine_practice_says_so() {
    let (speech, _main) = recording_service_as("test-recording").unwrap();
    let mut app = App::new(AppConfig {
        speech,
        settings: practice_settings(),
        ..AppConfig::for_tests()
    });
    app.dispatch(Command::Action(ActionId::CwPracticeWords));
    assert_eq!(
        app.status_text(),
        "Error: The CW engine did not start, so practice cannot send."
    );
    assert_ne!(app.mode(), Mode::Prompt);
}

#[test]
fn progress_names_the_lesson_and_the_latest_rounds() {
    let (mut app, log) = cw_app(None);
    app.dispatch(Command::Action(ActionId::CwPracticeProgress));
    assert_eq!(
        app.status_text(),
        "Koch lesson 1 of 40, newest M. No rounds yet."
    );
    let n = texts_after(&log, 1).len() + 2;
    app.dispatch(Command::Action(ActionId::CwPracticeKoch));
    let first = group(&log, n);
    app.dispatch(Command::Answer(first));
    let second = group(&log, n + 2);
    app.dispatch(Command::Answer(second));
    app.dispatch(Command::Action(ActionId::CwPracticeProgress));
    assert_eq!(
        app.status_text(),
        "Koch lesson 2 of 40, newest U. Latest rounds: 100 percent."
    );
}
