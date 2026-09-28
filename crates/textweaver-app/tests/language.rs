//! The interface language changes while textweaver runs (Wave 4, W4d;
//! ADR-0030): the change is said in the new language, followed by the
//! title line; the voice follows when the engine has one for the language,
//! and when it has none the current voice stays and says so, never going
//! silent.

use std::sync::{Arc, Mutex};

use serde_json::Value;
use textweaver_app::a11y::{Announcer, Priority};
use textweaver_app::lexicon::args;
use textweaver_app::lexicon::i18n::Catalog;
use textweaver_app::speech::Voice;
use textweaver_app::store::DocKey;
use textweaver_app::testing::recording_service;
use textweaver_app::text::Document;
use textweaver_app::{App, AppConfig, Command, Effect};

#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Announcer for Said {
    fn announce(&mut self, text: &str, _priority: Priority) {
        self.0.lock().unwrap().push(text.to_owned());
    }
}

fn app() -> (App, Said) {
    let said = Said::default();
    let (speech, _log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        announcer: Box::new(said.clone()),
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    app.open_document(
        Document::from_plain_text("One. Two.\n"),
        DocKey::untitled(1),
        "Essay".into(),
    );
    (app, said)
}

/// The recording engine has only English voices: Spanish keeps the voice
/// and says so, in Spanish, then the title line in Spanish.
#[test]
fn a_language_without_a_voice_keeps_the_voice_and_says_so() {
    let (mut app, _) = app();
    let said = app
        .set_setting("interface.language", Value::String("es".into()))
        .unwrap();
    let es = Catalog::builtin("es").unwrap();
    assert_eq!(app.catalog().lang(), "es");
    assert!(
        said.starts_with(&es.tr("setting-interface-language")),
        "{said}"
    );
    let kept = es.fmt(
        "language-voice-kept",
        &args!["voice" => "test-recording", "language" => "Español"],
    );
    let loading = es.tr("language-voices-loading");
    assert!(said.contains(&kept) || said.contains(&loading), "{said}");
    assert!(said.contains(&es.tr("state-ready")), "{said}");
    assert!(said.contains("Essay"), "{said}");
    // The voice did not change.
    assert_eq!(app.settings().speech.voice, None);
}

/// A language the engine has a voice for picks it (the chooser the app
/// uses; the recording engine here has English voices only).
#[test]
fn a_language_with_a_voice_changes_to_it() {
    let voices = vec![
        Voice {
            id: "en".into(),
            name: "Reed".into(),
            languages: vec!["en-US".into()],
            ..Voice::default()
        },
        Voice {
            id: "fr".into(),
            name: "Hortense".into(),
            languages: vec!["fr-FR".into()],
            ..Voice::default()
        },
    ];
    let settings = textweaver_app::store::Settings::default();
    let v = textweaver_app::engines::voice_for_language(&voices, "fr", &settings, Some("en"));
    assert_eq!(v.map(|v| v.name.as_str()), Some("Hortense"));
}

/// The language list: the languages by their own names, and Enter changes
/// the language at once.
#[test]
fn the_language_list_changes_the_language() {
    let (mut app, _) = app();
    let effects = app.language_list();
    let Some(Effect::ShowList { items, .. }) = effects.first() else {
        panic!("{effects:?}")
    };
    let fr = items.iter().position(|i| i == "Français").unwrap();
    assert!(items.contains(&"العربية".to_owned()));
    app.dispatch(Command::Choose(fr));
    assert_eq!(app.catalog().lang(), "fr");
    assert_eq!(app.settings().interface.language, "fr");
    // Messages from now on are in French.
    let c = Catalog::builtin("fr").unwrap();
    app.dispatch(Command::Action(textweaver_app::keymap::ActionId::SayStatus));
    assert!(
        app.status_text().contains(&c.tr("state-ready")),
        "{}",
        app.status_text()
    );
}
