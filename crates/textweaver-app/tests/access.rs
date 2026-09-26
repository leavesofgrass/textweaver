//! Screen reader coexistence (Agent P2c): the accessibility modes decide
//! what textweaver speaks and what goes to the status line for a screen
//! reader, so nothing is heard twice.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use textweaver_app::a11y::detect::Detected;
use textweaver_app::a11y::{AccessMode, Announcer, CursorPlacement, Priority};
use textweaver_app::core::CharPos;
use textweaver_app::keymap::ActionId;
use textweaver_app::store::{AccessMode as ModeSetting, CursorPlacement as CursorSetting, SayAll};
use textweaver_app::store::{DocKey, Settings};
use textweaver_app::testing::{SpeechLog, recording_service};
use textweaver_app::text::Document;
use textweaver_app::{App, AppConfig, Command, Confirm, Playback};

const TEXT: &str = "First sentence here. Second sentence follows now. Third and last one.";

#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Announcer for Said {
    fn announce(&mut self, text: &str, _priority: Priority) {
        self.0.lock().unwrap().push(text.to_owned());
    }
}

struct Rig {
    app: App,
    log: SpeechLog,
    said: Said,
}

fn rig_with(text: &str, change: impl FnOnce(&mut Settings)) -> Rig {
    let said = Said::default();
    let (speech, log) = recording_service().unwrap();
    let mut settings = Settings::default();
    change(&mut settings);
    let config = AppConfig {
        settings,
        speech,
        announcer: Box::new(said.clone()),
        self_voicing: true,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    };
    let mut app = App::new(config);
    app.open_document(
        Document::from_plain_text(text),
        DocKey::untitled(1),
        "Test".into(),
    );
    app.wait_for_speech_thread();
    log.clear();
    Rig { app, log, said }
}

fn rig(mode: AccessMode) -> Rig {
    rig_with(TEXT, |s| {
        s.accessibility.mode = textweaver_app::access_mode_setting(mode);
    })
}

impl Rig {
    fn act(&mut self, a: ActionId) {
        self.app.dispatch(Command::Action(a));
    }

    /// Everything the speech engine was asked to say, after waiting for it.
    fn spoken(&mut self) -> Vec<String> {
        self.app.wait_for_speech_thread();
        self.log.texts()
    }

    fn status(&self) -> String {
        self.app.status_text().to_owned()
    }
}

/// Self-voicing is the behaviour from before the setting: messages are
/// spoken and shown.
#[test]
fn self_voicing_speaks_and_shows_messages() {
    let mut r = rig(AccessMode::SelfVoicing);
    r.act(ActionId::NextSentence);
    assert!(r.status().contains("Second sentence"), "{}", r.status());
    let spoken = r.spoken();
    assert!(
        spoken.iter().any(|t| t.contains("Second sentence")),
        "{spoken:?}"
    );
    r.app.echo("x");
    assert!(!r.spoken().is_empty());
}

/// Screen-reader mode: textweaver says nothing at all, even with a working
/// voice; the status line carries messages, caret moves, and read text.
#[test]
fn screen_reader_mode_never_speaks() {
    let mut r = rig(AccessMode::ScreenReader);
    r.act(ActionId::NextSentence);
    assert!(r.status().contains("Second sentence"), "{}", r.status());
    r.act(ActionId::ReadCurrentSentence);
    assert_eq!(r.status(), "Second sentence follows now.");
    r.act(ActionId::ReadCurrentWord);
    assert_eq!(r.status(), "Second");
    r.act(ActionId::CaretNextWord);
    assert_eq!(r.status(), "sentence");
    r.act(ActionId::ReadCurrentCharacter);
    assert_eq!(r.app.playback(), Playback::Idle);
    r.act(ActionId::ReadCurrentLine);
    r.act(ActionId::RateUp);
    assert!(r.status().contains("words per minute"), "{}", r.status());
    r.app.echo("a");
    r.act(ActionId::SpeechCursorToggle);
    assert!(
        r.status().starts_with("Speech Cursor on, line 1: "),
        "{}",
        r.status()
    );
    r.act(ActionId::SpeechCursorToggle);
    assert!(r.spoken().is_empty(), "{:?}", r.log.texts());
    // The announcer (log, JSON-RPC client) still hears every message.
    assert!(!r.said.0.lock().unwrap().is_empty());
}

/// Math on the status line is in words for a screen reader, as textweaver
/// would say it; self-voicing shows the source.
#[test]
fn screen_reader_status_line_says_math_in_words() {
    let text = "The area is $x^2$ here. Next.";
    let mut r = rig_with(text, |s| s.accessibility.mode = ModeSetting::ScreenReader);
    r.act(ActionId::ReadCurrentSentence);
    let st = r.status();
    assert!(st.contains("squared") && !st.contains('$'), "{st}");
    let mut sv = rig_with(text, |s| s.accessibility.mode = ModeSetting::SelfVoicing);
    sv.app.announce("Formula $x^2$", Priority::Polite);
    assert_eq!(sv.status(), "Formula $x^2$");
}

/// Hybrid: textweaver reads text aloud and leaves messages, echo, and caret
/// moves to the screen reader; nothing goes to both.
#[test]
fn hybrid_reads_aloud_and_leaves_messages_to_the_screen_reader() {
    let mut r = rig(AccessMode::Hybrid);
    r.act(ActionId::NextSentence);
    assert!(r.status().contains("Second sentence"), "{}", r.status());
    r.act(ActionId::CaretNextWord);
    r.app.echo("a");
    r.act(ActionId::RateUp);
    assert!(r.spoken().is_empty(), "{:?}", r.log.texts());
    let before = r.status();
    r.act(ActionId::ReadCurrentSentence);
    let spoken = r.spoken();
    assert!(
        spoken.iter().any(|t| t.contains("Second sentence follows")),
        "{spoken:?}"
    );
    assert_eq!(
        r.status(),
        before,
        "read text is not copied to the status line"
    );
    // Continuous reading uses the voice and adds nothing to the status line.
    r.log.clear();
    let before = r.status();
    r.act(ActionId::ReadFromCursor);
    assert_eq!(r.app.playback(), Playback::Reading);
    assert!(!r.app.screen_say_all_running());
    assert!(!r.spoken().is_empty());
    assert_eq!(r.status(), before);
}

/// Screen-reader mode's say all: a sentence at a time on the status line,
/// paced by the rate, with pause and resume, and silent.
#[test]
fn screen_reader_say_all_moves_through_the_text_on_the_status_line() {
    let mut r = rig(AccessMode::ScreenReader);
    r.act(ActionId::PlayPause);
    assert!(r.app.screen_say_all_running());
    assert_eq!(r.app.playback(), Playback::Reading);
    assert_eq!(r.status(), "First sentence here.");
    let later = |n: u64| Instant::now() + Duration::from_secs(n);
    // Not yet due: nothing changes.
    r.app.tick(Instant::now());
    assert_eq!(r.status(), "First sentence here.");
    r.app.tick(later(10));
    assert_eq!(r.status(), "Second sentence follows now.");
    let second = r.app.session().unwrap().cursor;
    assert!(second > CharPos(0));
    r.act(ActionId::PlayPause);
    assert!(!r.app.screen_say_all_running());
    assert!(matches!(r.app.playback(), Playback::Paused { .. }));
    r.act(ActionId::PlayPause);
    assert_eq!(r.status(), "Second sentence follows now.");
    r.app.tick(later(20));
    assert_eq!(r.status(), "Third and last one.");
    r.app.tick(later(30));
    assert_eq!(r.app.playback(), Playback::Idle);
    assert_eq!(r.status(), "End of document.");
    assert!(r.spoken().is_empty(), "{:?}", r.log.texts());
    // Stop ends it at once.
    r.act(ActionId::DocumentStart);
    r.act(ActionId::PlayPause);
    r.act(ActionId::Stop);
    assert!(!r.app.screen_say_all_running());
}

/// `say_all = "voice"`: in screen-reader mode, continuous reading uses
/// textweaver's voice, the one thing it says there.
#[test]
fn screen_reader_say_all_can_use_the_voice() {
    let mut r = rig_with(TEXT, |s| {
        s.accessibility.mode = ModeSetting::ScreenReader;
        s.accessibility.say_all = SayAll::Voice;
    });
    r.act(ActionId::PlayPause);
    assert!(!r.app.screen_say_all_running());
    let spoken = r.spoken();
    assert!(
        spoken.iter().any(|t| t.contains("First sentence")),
        "{spoken:?}"
    );
}

/// Hybrid without an engine falls back to the status line.
#[test]
fn hybrid_without_a_voice_reads_on_the_status_line() {
    let said = Said::default();
    let mut settings = Settings::default();
    settings.accessibility.mode = ModeSetting::Hybrid;
    let mut app = App::new(AppConfig {
        settings,
        announcer: Box::new(said),
        ..AppConfig::for_tests()
    });
    app.open_document(
        Document::from_plain_text(TEXT),
        DocKey::untitled(1),
        "Test".into(),
    );
    app.dispatch(Command::Action(ActionId::ReadCurrentSentence));
    assert_eq!(app.status_text(), "First sentence here.");
    app.dispatch(Command::Action(ActionId::PlayPause));
    assert!(app.screen_say_all_running());
}

/// The mode cycles self-voicing, hybrid, screen reader; the change is
/// saved and said the way the old mode spoke.
#[test]
fn cycling_the_mode_saves_it_and_says_it() {
    let mut r = rig(AccessMode::SelfVoicing);
    r.act(ActionId::CycleAccessMode);
    assert_eq!(r.app.access_mode(), AccessMode::Hybrid);
    assert_eq!(r.app.settings().accessibility.mode, ModeSetting::Hybrid);
    assert!(r.status().starts_with("Hybrid mode."), "{}", r.status());
    // Said by textweaver: the old mode was self-voicing.
    assert!(r.spoken().iter().any(|t| t.starts_with("Hybrid mode.")));
    r.log.clear();
    r.act(ActionId::CycleAccessMode);
    assert_eq!(r.app.access_mode(), AccessMode::ScreenReader);
    assert!(
        r.status().starts_with("Screen reader mode."),
        "{}",
        r.status()
    );
    assert!(
        r.spoken().is_empty(),
        "hybrid leaves messages to the screen reader"
    );
    r.act(ActionId::CycleAccessMode);
    assert_eq!(r.app.access_mode(), AccessMode::SelfVoicing);
    // A mode for this run only is not written to the settings.
    r.app.set_access_mode_for_run(AccessMode::ScreenReader);
    assert_eq!(
        r.app.settings().accessibility.mode,
        ModeSetting::SelfVoicing
    );
}

/// First run with a screen reader: one question, y switches to hybrid, and
/// it is never asked again either way.
#[test]
fn the_first_run_offers_hybrid_once() {
    let nvda = Detected {
        name: Some("NVDA".into()),
    };
    let mut r = rig(AccessMode::SelfVoicing);
    assert!(r.app.hybrid_offer_due());
    assert!(r.app.offer_hybrid(&nvda));
    assert!(r.app.confirmation_pending());
    let q = r.app.pending_question().unwrap();
    assert!(q.starts_with("NVDA is running. Use hybrid mode"), "{q}");
    assert_eq!(r.status(), q);
    // Another key repeats the question.
    r.app.dispatch(Command::Confirm(Confirm::Repeat));
    assert_eq!(r.status(), q);
    r.app.dispatch(Command::Confirm(Confirm::Yes));
    assert!(!r.app.confirmation_pending());
    assert_eq!(r.app.access_mode(), AccessMode::Hybrid);
    let a = &r.app.settings().accessibility;
    assert!(a.hybrid_offered);
    assert_eq!(a.mode, ModeSetting::Hybrid);
    assert!(r.status().contains("Alt+Shift+A"), "{}", r.status());
    assert!(!r.app.hybrid_offer_due());
    assert!(!r.app.offer_hybrid(&nvda));

    let mut no = rig(AccessMode::SelfVoicing);
    assert!(no.app.offer_hybrid(&Detected { name: None }));
    assert!(
        no.app
            .pending_question()
            .unwrap()
            .starts_with("A screen reader is running.")
    );
    no.app.dispatch(Command::Cancel);
    assert_eq!(no.app.access_mode(), AccessMode::SelfVoicing);
    assert!(no.app.settings().accessibility.hybrid_offered);
    assert!(no.status().starts_with("Staying in self-voicing mode."));
    assert!(!no.app.offer_hybrid(&nvda));

    // A mode chosen already (in the settings or for this run) asks nothing.
    let mut chosen = rig(AccessMode::ScreenReader);
    assert!(!chosen.app.offer_hybrid(&nvda));
    let mut run = rig(AccessMode::SelfVoicing);
    run.app.set_access_mode_for_run(AccessMode::Hybrid);
    assert!(!run.app.offer_hybrid(&nvda));
}

/// Quiet screen: while reading aloud, the text read is not copied to the
/// status line and the "reading at" note is left out.
#[test]
fn quiet_screen_keeps_read_text_off_the_status_line() {
    let mut r = rig_with(TEXT, |s| s.accessibility.quiet_screen = true);
    r.act(ActionId::ReadFromCursor);
    assert_eq!(r.app.playback(), Playback::Reading);
    assert!(r.app.quiet_screen_active());
    assert!(!r.status().contains("words per minute"), "{}", r.status());
    r.act(ActionId::Stop);
    assert!(!r.app.quiet_screen_active());
    // Without it, the note is there (today's behaviour).
    let mut loud = rig(AccessMode::SelfVoicing);
    loud.act(ActionId::ReadFromCursor);
    assert!(!loud.app.quiet_screen_active());
    assert!(
        loud.status().contains("words per minute"),
        "{}",
        loud.status()
    );
}

#[test]
fn cursor_placement_comes_from_the_settings() {
    let r = rig(AccessMode::SelfVoicing);
    assert_eq!(r.app.cursor_placement(), CursorPlacement::Follow);
    let r = rig_with(TEXT, |s| s.accessibility.cursor = CursorSetting::Status);
    assert_eq!(r.app.cursor_placement(), CursorPlacement::Status);
}
