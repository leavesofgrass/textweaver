use std::time::{Duration, Instant};

use textweaver_a11y::Verbosity;
use textweaver_core::CharRange;
use textweaver_store::DocKey;
use textweaver_text::Document;

use crate::{App, AppConfig};

fn app_with(doc: Document) -> App {
    let mut app = App::new(AppConfig::for_tests());
    app.open_document(doc, DocKey::untitled(1), "Notes".into());
    app
}

/// With `[reading_aids] difficult_definitions` on, a word move onto a
/// difficult word at high verbosity also says its first definition, once
/// the dictionary has opened quietly.
#[test]
fn difficult_words_say_their_first_definition_when_asked() {
    let text = "The students read about mitochondria today.";
    let mut app = app_with(Document::from_plain_text(text));
    app.settings.speech.verbosity = Verbosity::High;
    app.settings.reading_aids.difficult_words = true;
    let start = text.find("mitochondria").unwrap_or(0);
    let word = CharRange::new(start, start + "mitochondria".len());
    // Off by default: "difficult word" alone, and no dictionary opens.
    assert_eq!(
        app.difficult_word_note(word).as_deref(),
        Some(", difficult word")
    );
    assert!(!app.lexicon_loading());
    // A common word gets nothing.
    assert_eq!(app.difficult_word_note(CharRange::new(4, 12)), None);
    // On: the first move opens the dictionary quietly.
    app.last_message = None;
    app.settings.reading_aids.difficult_definitions = true;
    assert_eq!(
        app.difficult_word_note(word).as_deref(),
        Some(", difficult word")
    );
    assert_eq!(app.last_message, None, "opened quietly");
    let deadline = Instant::now() + Duration::from_secs(30);
    while app.lexicon_loading() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
        app.tick(Instant::now());
    }
    assert!(!app.lexicon_loading(), "the dictionary never opened");
    assert!(app.list.is_none(), "no definition list after a quiet open");
    let said = app.difficult_word_note(word).unwrap_or_default();
    assert!(
        said.starts_with(", difficult word: ") && said.len() > 30,
        "{said}"
    );
    assert!(!said.contains(';'), "first clause only: {said}");
    // Low verbosity: nothing, as before.
    app.settings.speech.verbosity = Verbosity::Low;
    assert_eq!(app.difficult_word_note(word), None);
}
