//! Phase 1 safety and authoring quick wins (Agent P1b), driven through
//! `App::dispatch` without a terminal.

use std::time::{Duration, Instant};

use textweaver_app::core::{CharPos, Direction};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::{DocKey, Paths, StateStore};
use textweaver_app::testing::{SpeechLog, recording_service};
use textweaver_app::text::{Document, GoTo};
use textweaver_app::{App, AppConfig, CaretMove, Command, Effect};

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

/// Waits (up to a deadline) until the speech log has a text containing
/// `needle`; returns every text.
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

/// Selecting or deleting more than about 200 characters says how many and
/// where they start and end, not the whole text.
#[test]
fn big_selections_and_deletions_are_summarized() {
    let long = format!(
        "Opening words here {}closing words here.",
        "and more ".repeat(40)
    );
    let text = format!("{long}\nNext line.\n");
    let mut app = app_with(&text);
    act(&mut app, ActionId::SelectNextLine);
    let n = long.chars().count();
    let expected = format!(
        "{} characters selected, from Opening words here and to more closing words here.",
        textweaver_app::editor::echo::thousands(n)
    );
    assert_eq!(app.status_text(), expected);
    // Short changes are still read as they are.
    act(&mut app, ActionId::SelectNextLine);
    assert_eq!(app.status_text(), "Next line. selected");

    let (mut app, log) = voiced_app(&text);
    act(&mut app, ActionId::ToggleEditMode);
    // Entering edit mode reads the line; only what follows matters here.
    wait_for_speech(&log, "Edit mode on");
    log.clear();
    for _ in 0..n {
        app.dispatch(Command::MoveCaret {
            by: CaretMove::Char,
            direction: Direction::Forward,
            extend: true,
        });
    }
    app.dispatch(Command::DeleteBack);
    let said = wait_for_speech(&log, "characters deleted");
    assert!(
        said.iter()
            .any(|t| t.contains("characters deleted, from Opening words here and to")),
        "{said:?}"
    );
    assert!(
        !said.iter().any(|t| t.contains(&long)),
        "the whole text was read"
    );
}

/// A silent app (no self-voicing, as with `--no-speech`) with `text` open.
fn app_with(text: &str) -> App {
    let mut app = App::new(AppConfig::for_tests());
    app.open_document(
        Document::from_plain_text(text),
        DocKey::untitled(1),
        "T".into(),
    );
    app
}

fn act(app: &mut App, a: ActionId) -> Vec<Effect> {
    app.dispatch(Command::Action(a))
}

/// The saved position and every bookmark carry the text found there, for
/// finding them again after the file changes outside textweaver.
#[test]
fn positions_and_bookmarks_are_saved_with_anchors() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(&dir.path().join("home"));
    let file = dir.path().join("a.txt");
    let text = "Alpha beta gamma. Delta epsilon zeta eta theta iota kappa lambda.\n";
    std::fs::write(&file, text).unwrap();
    let mut app = App::new(AppConfig {
        paths: Some(paths.clone()),
        ..AppConfig::for_tests()
    });
    app.open(&file).unwrap();
    app.dispatch(Command::GoTo(GoTo::Char(CharPos(18))));
    act(&mut app, ActionId::AddBookmark);
    app.dispatch(Command::GoTo(GoTo::Char(CharPos(6))));
    app.save_position().unwrap();
    // Written by the background writer.
    app.wait_for_writes();
    let state = StateStore::new(paths.state_dir())
        .load(&DocKey::for_path(&file))
        .unwrap();
    let anchor = state.anchor.expect("position anchor");
    assert!(anchor.context.starts_with("beta gamma."), "{anchor:?}");
    assert_eq!(anchor.context.chars().count(), 40);
    let b = state.bookmarks[0].anchor.as_ref().expect("bookmark anchor");
    assert!(b.context.starts_with("Delta epsilon"), "{b:?}");
}

/// The hook for a dead speech thread (wired once Agent P1a's detection is
/// on main): reading stops, the error is shown, and the app keeps working
/// silently.
#[test]
fn a_dead_speech_thread_is_reported_and_the_app_goes_on() {
    let (mut app, _log) = voiced_app("One. Two. Three.\n");
    act(&mut app, ActionId::ReadFromCursor);
    app.speech_thread_died("the engine crashed");
    assert_eq!(app.playback(), textweaver_app::Playback::Idle);
    assert!(
        app.status_text()
            .starts_with("Speech stopped working (the engine crashed)."),
        "{}",
        app.status_text()
    );
    assert_eq!(app.backend_name(), "silent");
    act(&mut app, ActionId::NextSentence);
    assert!(app.status_text().contains("Two."), "{}", app.status_text());
}

/// A backend with a bug: its `speak` panics, which kills the speech
/// thread.
struct Panics;

impl textweaver_app::speech::SpeechBackend for Panics {
    fn id(&self) -> textweaver_app::speech::BackendId {
        "panics"
    }
    fn capabilities(&self) -> textweaver_app::speech::Caps {
        textweaver_app::speech::Caps::WORD_EVENTS
    }
    fn voices(
        &self,
    ) -> Result<Vec<textweaver_app::speech::Voice>, textweaver_app::speech::SpeechError> {
        Ok(Vec::new())
    }
    fn set_params(
        &mut self,
        _: &textweaver_app::speech::VoiceParams,
    ) -> Result<(), textweaver_app::speech::SpeechError> {
        Ok(())
    }
    fn effective_wpm(&self) -> u16 {
        180
    }
    fn speak(
        &mut self,
        _: &textweaver_app::core::Utterance,
        _: &mut dyn textweaver_app::speech::EventSink,
    ) -> Result<(), textweaver_app::speech::SpeechError> {
        panic!("the engine had a bug")
    }
    fn stop(&mut self) {}
}

/// The speech thread really dies: the app notices it while polling,
/// reports it, and goes on silently.
#[test]
fn the_app_notices_a_dead_speech_thread_while_polling() {
    let speech = textweaver_app::speech::SpeechService::spawn(
        Box::new(|| Ok(Box::new(Panics) as _)),
        textweaver_app::speech::ServiceConfig::default(),
    )
    .unwrap();
    let mut app = App::new(AppConfig {
        speech,
        self_voicing: true,
        backend_name: "panics".into(),
        ..AppConfig::for_tests()
    });
    app.open_document(
        Document::from_plain_text("One. Two.\n"),
        DocKey::untitled(1),
        "T".into(),
    );
    act(&mut app, ActionId::ReadFromCursor);
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.backend_name() != "silent" {
        assert!(Instant::now() < deadline, "the dead thread was not noticed");
        app.poll_speech();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(app.playback(), textweaver_app::Playback::Idle);
    assert!(
        app.status_text()
            .starts_with("Speech stopped working (the engine had a bug)."),
        "{}",
        app.status_text()
    );
    act(&mut app, ActionId::NextSentence);
    assert!(app.status_text().contains("Two."), "{}", app.status_text());
}

/// With `--no-speech` a screen reader reads the status line, so every
/// line the Speech Cursor reads is put there.
#[test]
fn speech_cursor_lines_are_on_the_status_line() {
    let mut app = app_with("First line.\n\nThird line here.\n");
    act(&mut app, ActionId::SpeechCursorToggle);
    assert!(
        app.status_text().contains("First line."),
        "{}",
        app.status_text()
    );
    act(&mut app, ActionId::SpeechCursorNextLine);
    assert_eq!(app.status_text(), "blank");
    act(&mut app, ActionId::SpeechCursorNextLine);
    assert_eq!(app.status_text(), "Third line here.");
    act(&mut app, ActionId::SpeechCursorRereadLine);
    assert_eq!(app.status_text(), "Third line here.");
}

/// A Markdown file opened in an app recording its speech (voiced when
/// `voiced`), so the document has its structure markers.
fn markdown(text: &str, voiced: bool) -> (App, SpeechLog, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.md");
    std::fs::write(&file, text).unwrap();
    let (speech, log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        self_voicing: voiced,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    app.open(&file).unwrap();
    (app, log, dir)
}

fn cursor(app: &App) -> CharPos {
    app.session().unwrap().cursor
}

fn source(app: &App) -> String {
    app.session().unwrap().doc.text().to_string()
}

fn type_str(app: &mut App, s: &str) {
    for c in s.chars() {
        app.dispatch(Command::Insert(c.to_string()));
    }
}

const STRUCTURED: &str = "# Title\n\nIntro text.\n\n## Methods\n\nWe measured.\n\n### Detail\n\n- milk\n- eggs\n\n## Results\n\nIt worked, see [the site](https://example.org/a).\n";

/// Keys 1 to 6 go to the next heading of that level, Shift with them to
/// the previous one, and the palette has the same commands.
#[test]
fn headings_by_level() {
    let (mut app, _log, _dir) = markdown(STRUCTURED, false);
    let doc = source(&app);
    act(&mut app, ActionId::NextHeadingLevel2);
    assert_eq!(cursor(&app).0, doc.find("Methods").unwrap());
    assert!(
        app.status_text().contains("Heading level 2"),
        "{}",
        app.status_text()
    );
    act(&mut app, ActionId::NextHeadingLevel2);
    assert_eq!(cursor(&app).0, doc.find("Results").unwrap());
    act(&mut app, ActionId::PreviousHeadingLevel3);
    assert_eq!(cursor(&app).0, doc.find("Detail").unwrap());
    act(&mut app, ActionId::NextHeadingLevel4);
    assert_eq!(app.status_text(), "No next heading at level 4.");
    act(&mut app, ActionId::PreviousHeadingLevel1);
    assert_eq!(cursor(&app).0, doc.find("Title").unwrap());
    // The palette finds them by name.
    app.dispatch(Command::Action(ActionId::CommandPalette));
    app.dispatch(Command::Answer("next heading level 3".into()));
    assert_eq!(cursor(&app).0, doc.find("Detail").unwrap());
}

/// Word count for the document or the selection, and in "say position".
#[test]
fn word_count_and_position() {
    let mut app = app_with("One two three.\nFour five, six seven.\n");
    act(&mut app, ActionId::WordCount);
    assert_eq!(app.status_text(), "7 words in the document.");
    app.dispatch(Command::Select(textweaver_app::core::CharRange::new(0, 7)));
    act(&mut app, ActionId::WordCount);
    assert_eq!(app.status_text(), "2 words in the selection.");
    app.dispatch(Command::Select(textweaver_app::core::CharRange::new(0, 0)));
    app.dispatch(Command::GoTo(GoTo::Char(CharPos(20))));
    act(&mut app, ActionId::SayPosition);
    assert!(
        app.status_text().contains("Word 5 of 7."),
        "{}",
        app.status_text()
    );
}

/// The link address at the cursor, in reading and in edit mode.
#[test]
fn link_address_at_the_cursor() {
    let (mut app, _log, _dir) = markdown(STRUCTURED, false);
    act(&mut app, ActionId::LinkAddress);
    assert_eq!(app.status_text(), "No link at the cursor.");
    let doc = source(&app);
    app.dispatch(Command::GoTo(GoTo::Char(CharPos(
        doc.find("the site").unwrap() + 4,
    ))));
    act(&mut app, ActionId::LinkAddress);
    assert_eq!(
        app.status_text(),
        "Link the site, address: https://example.org/a"
    );
    act(&mut app, ActionId::ToggleEditMode);
    let src = source(&app);
    app.dispatch(Command::GoTo(GoTo::Char(CharPos(
        src.find("the site").unwrap(),
    ))));
    act(&mut app, ActionId::LinkAddress);
    assert_eq!(
        app.status_text(),
        "Link the site, address: https://example.org/a"
    );
}

/// Enter in a list item continues the list; Enter on an empty item ends
/// it; numbers count up and task items get an empty box.
#[test]
fn enter_continues_and_ends_lists() {
    let mut app = app_with("");
    act(&mut app, ActionId::ToggleEditMode);
    type_str(&mut app, "- milk\n");
    assert_eq!(source(&app), "- milk\n- ");
    assert_eq!(app.status_text(), "Bullet");
    type_str(&mut app, "eggs\n\n");
    assert_eq!(source(&app), "- milk\n- eggs\n");
    assert_eq!(app.status_text(), "List ended.");
    type_str(&mut app, "\n3. three\n");
    assert!(
        source(&app).ends_with("3. three\n4. "),
        "{:?}",
        source(&app)
    );
    assert_eq!(app.status_text(), "Item 4");
    type_str(&mut app, "\n- [x] done\n");
    assert!(
        source(&app).ends_with("- [x] done\n- [ ] "),
        "{:?}",
        source(&app)
    );
    // One undo takes the continuation back.
    act(&mut app, ActionId::Undo);
    assert!(source(&app).ends_with("- [x] done"), "{:?}", source(&app));
}

/// Tab and Shift+Tab move between table cells in edit mode and say the
/// column header; outside a table Tab types a tab.
#[test]
fn tab_moves_between_table_cells() {
    let text = "| Name | Age |\n| --- | --- |\n| Ada | 36 |\n| Alan |  |\n";
    let mut app = app_with(text);
    act(&mut app, ActionId::ToggleEditMode);
    // From the start of the line, into the first cell.
    act(&mut app, ActionId::NextTableCell);
    assert_eq!(app.status_text(), "Name: Name");
    act(&mut app, ActionId::NextTableCell);
    assert_eq!(app.status_text(), "Age: Age");
    act(&mut app, ActionId::NextTableCell);
    assert_eq!(app.status_text(), "Row 2. Name: Ada");
    act(&mut app, ActionId::NextTableCell);
    assert_eq!(app.status_text(), "Age: 36");
    act(&mut app, ActionId::NextTableCell);
    act(&mut app, ActionId::NextTableCell);
    assert_eq!(app.status_text(), "Age: blank");
    act(&mut app, ActionId::NextTableCell);
    assert_eq!(app.status_text(), "End of table.");
    act(&mut app, ActionId::PreviousTableCell);
    assert_eq!(app.status_text(), "Name: Alan");
    // The cell's text is selected, so typing replaces it.
    type_str(&mut app, "Grace");
    assert!(source(&app).contains("| Grace |"), "{}", source(&app));
    // Outside a table: a tab.
    let mut app = app_with("plain\n");
    act(&mut app, ActionId::ToggleEditMode);
    act(&mut app, ActionId::NextTableCell);
    assert_eq!(source(&app), "\tplain\n");
    act(&mut app, ActionId::PreviousTableCell);
    assert_eq!(app.status_text(), "Not in a table.");
}

/// Caret and Speech Cursor moves say the structure first.
#[test]
fn structure_is_said_before_the_text() {
    let (mut app, log, _dir) = markdown(STRUCTURED, true);
    // Browse caret: down onto the level 2 heading.
    let doc = source(&app);
    app.dispatch(Command::GoTo(GoTo::Char(CharPos(
        doc.find("Intro").unwrap(),
    ))));
    act(&mut app, ActionId::CaretNextLine);
    assert_eq!(app.status_text(), "heading level 2, Methods");
    // Speech Cursor on the second list item.
    app.dispatch(Command::GoTo(GoTo::Char(CharPos(
        doc.find("eggs").unwrap(),
    ))));
    act(&mut app, ActionId::SpeechCursorToggle);
    log.clear();
    act(&mut app, ActionId::SpeechCursorRereadLine);
    assert_eq!(app.status_text(), "list item, eggs");
    // "list item" is spoken just before the line.
    let deadline = Instant::now() + Duration::from_secs(10);
    let said = loop {
        let said = log.texts();
        let found = said
            .windows(2)
            .any(|w| w[0] == "list item" && w[1] == "eggs");
        if found || Instant::now() > deadline {
            break said;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    assert!(
        said.windows(2)
            .any(|w| w[0] == "list item" && w[1] == "eggs"),
        "{said:?}"
    );
    act(&mut app, ActionId::SpeechCursorToggle);
    // Edit mode: the source line with its structure, without the markup.
    act(&mut app, ActionId::ToggleEditMode);
    let src = source(&app);
    app.dispatch(Command::GoTo(GoTo::Char(CharPos(
        src.find("We measured").unwrap(),
    ))));
    log.clear();
    app.dispatch(Command::MoveCaret {
        by: CaretMove::Line,
        direction: Direction::Backward,
        extend: false,
    });
    app.dispatch(Command::MoveCaret {
        by: CaretMove::Line,
        direction: Direction::Backward,
        extend: false,
    });
    let said = wait_for_speech(&log, "heading level 2, Methods");
    assert!(
        said.iter().any(|t| t == "heading level 2, Methods"),
        "{said:?}"
    );
}

/// Markdown typed in edit mode is said as what it means.
#[test]
fn markdown_is_echoed_as_structure() {
    let (mut app, log) = voiced_app("");
    act(&mut app, ActionId::ToggleEditMode);
    type_str(&mut app, "## ");
    let said = wait_for_speech(&log, "Heading level 2");
    assert!(said.iter().any(|t| t == "Heading level 2"), "{said:?}");
    type_str(&mut app, "Methods ");
    let said = wait_for_speech(&log, "Heading level 2, Methods");
    assert!(
        said.iter().any(|t| t == "Heading level 2, Methods"),
        "{said:?}"
    );
    type_str(&mut app, "\n\n- ");
    let said = wait_for_speech(&log, "Bullet");
    assert!(said.iter().any(|t| t == "Bullet"), "{said:?}");
}

/// The typing echo command cycles and saves.
#[test]
fn typing_echo_cycles_and_is_saved() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    let mut app = App::new(AppConfig {
        paths: Some(paths.clone()),
        ..AppConfig::for_tests()
    });
    let mut heard = Vec::new();
    for _ in 0..4 {
        act(&mut app, ActionId::CycleTypingEcho);
        heard.push(app.status_text().to_owned());
    }
    assert_eq!(
        heard,
        [
            "Typing echo: characters.",
            "Typing echo: words.",
            "Typing echo: none.",
            "Typing echo: characters and words."
        ]
    );
    act(&mut app, ActionId::CycleTypingEcho);
    // Settings are written by the writer thread (Wave 3).
    app.wait_for_writes();
    let saved = textweaver_app::store::SettingsStore::new(paths).load().0;
    assert!(saved.editing.echo_characters && !saved.editing.echo_words);
}

/// Alt+N adds a note in edit mode too.
#[test]
fn add_note_works_in_edit_mode() {
    let mut app = app_with("A sentence to annotate. Another one.\n");
    act(&mut app, ActionId::ToggleEditMode);
    let effects = act(&mut app, ActionId::AddNote);
    assert!(
        matches!(effects.first(), Some(Effect::Prompt { .. })),
        "{effects:?}"
    );
    app.dispatch(Command::Answer("check this".into()));
    assert_eq!(app.session().unwrap().notes.len(), 1);
    assert!(app.is_editing());
}

/// A paste says how it starts, not only how long it is.
#[test]
fn paste_says_the_first_words() {
    let mut app = app_with("");
    act(&mut app, ActionId::ToggleEditMode);
    app.dispatch(Command::Insert(
        "The quick brown fox jumps over the lazy dog.".into(),
    ));
    assert_eq!(
        app.status_text(),
        "Pasted 44 characters: The quick brown fox jumps over…"
    );
}

/// Copy takes the selection (or the sentence), Cut removes it too.
#[test]
fn copy_and_cut_fill_the_clipboard() {
    let mut app = app_with("First sentence here. Second one.\n");
    act(&mut app, ActionId::Copy);
    assert_eq!(
        app.take_clipboard().as_deref(),
        Some("First sentence here.")
    );
    assert!(
        app.status_text().starts_with("Copied the sentence:"),
        "{}",
        app.status_text()
    );
    assert_eq!(app.take_clipboard(), None);
    act(&mut app, ActionId::ToggleEditMode);
    app.dispatch(Command::GoTo(GoTo::Char(CharPos(21))));
    for _ in 0..6 {
        app.dispatch(Command::MoveCaret {
            by: CaretMove::Char,
            direction: Direction::Forward,
            extend: true,
        });
    }
    act(&mut app, ActionId::Cut);
    assert_eq!(app.take_clipboard().as_deref(), Some("Second"));
    assert_eq!(source(&app), "First sentence here.  one.\n");
    assert_eq!(app.status_text(), "Cut: Second");
    act(&mut app, ActionId::Cut);
    assert_eq!(app.status_text(), "Nothing selected to cut.");
    assert_eq!(textweaver_app::osc52("Second"), "\u{1b}]52;c;U2Vjb25k\u{7}");
}
