//! `App` behavior without a terminal, driven through `dispatch` with a
//! recording speech backend.
//!
//! Expectations about unit boundaries use texts every sane segmentation
//! agrees on (plain words, sentences ending in ". "), so these tests keep
//! passing when Agent A replaces the Phase 0 unit rules.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ropey::Rope;
use textweaver_app::a11y::{Announcer, Priority, Verbosity};
use textweaver_app::core::{CharPos, CharRange, MarkerKind, UtteranceKind};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::{DocKey, Paths};
use textweaver_app::testing::{SpeechLog, recording_service};
use textweaver_app::text::{Document, DocumentMeta, GoTo, Marker};
use textweaver_app::{App, AppConfig, Command, Confirm, Effect, Mode, Playback, PromptPurpose};

/// Collects announcements for assertions.
#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Said {
    fn all(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
    fn last(&self) -> String {
        self.all().last().cloned().unwrap_or_default()
    }
    fn any(&self, needle: &str) -> bool {
        self.all().iter().any(|s| s.contains(needle))
    }
}

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

fn make_config(said: &Said) -> (AppConfig, SpeechLog) {
    let (speech, log) = recording_service().unwrap();
    let config = AppConfig {
        speech,
        announcer: Box::new(said.clone()),
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    };
    (config, log)
}

fn rig_with_doc(doc: Document) -> Rig {
    let said = Said::default();
    let (config, log) = make_config(&said);
    let mut app = App::new(config);
    app.open_document(doc, DocKey::untitled(1), "Test".into());
    Rig { app, log, said }
}

fn rig(text: &str) -> Rig {
    rig_with_doc(Document::from_plain_text(text))
}

impl Rig {
    fn act(&mut self, a: ActionId) -> Vec<Effect> {
        self.app.dispatch(Command::Action(a))
    }
    /// Quit, answering yes to "Quit textweaver? y or n".
    fn quit(&mut self) -> Vec<Effect> {
        assert_eq!(self.act(ActionId::Quit), vec![Effect::Redraw]);
        assert_eq!(self.app.pending_confirmation(), Some(ActionId::Quit));
        self.app.dispatch(Command::Confirm(Confirm::Yes))
    }
    fn cursor(&self) -> CharPos {
        self.app.session().unwrap().cursor
    }
    fn go(&mut self, pos: CharPos) {
        self.app.dispatch(Command::GoTo(GoTo::Char(pos)));
    }
    fn wait_idle(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            self.app.poll_speech();
            if self.app.playback() == Playback::Idle {
                return;
            }
            assert!(Instant::now() < deadline, "speech never finished");
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    /// Waits until the speech thread has spoken an utterance satisfying `f`.
    fn wait_spoken(&self, f: impl Fn(&textweaver_app::core::Utterance) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(30);
        while !self.log.utterances().iter().any(&f) {
            assert!(Instant::now() < deadline, "utterance never spoken");
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

/// Char offset of the first occurrence of `needle` (ASCII texts).
fn at(text: &str, needle: &str) -> CharPos {
    CharPos(text.find(needle).expect("needle in text"))
}

const PROSE: &str = "Alpha beta gamma delta epsilon zeta. Eta theta iota. Kappa lambda mu nu xi omicron.\n\nPi rho sigma. Tau upsilon phi.\n\nChi psi omega.";

#[test]
fn open_navigate_quit() {
    let mut r = rig("One two. Three four. Five.");
    r.act(ActionId::NextSentence);
    assert_eq!(r.cursor(), CharPos(9));
    assert!(r.said.any("Opened Test"));
    assert_eq!(r.quit(), vec![Effect::Quit]);
}

#[test]
fn next_sentence_does_not_wrap_and_says_so() {
    let mut r = rig(PROSE);
    r.go(at(PROSE, "Chi"));
    r.act(ActionId::NextSentence);
    assert_eq!(r.cursor(), at(PROSE, "Chi"));
    assert_eq!(r.said.last(), "No next sentence.");
}

#[test]
fn previous_sentence_three_word_rule() {
    let mut r = rig(PROSE);
    // "xi" has four words before it in its sentence: rewind to its start.
    r.go(at(PROSE, "xi"));
    r.act(ActionId::PreviousSentence);
    assert_eq!(r.cursor(), at(PROSE, "Kappa"));
    // "nu" has three: go to the previous sentence.
    r.go(at(PROSE, "nu"));
    r.act(ActionId::PreviousSentence);
    assert_eq!(r.cursor(), at(PROSE, "Eta"));
    // At a sentence start: the previous sentence.
    r.act(ActionId::PreviousSentence);
    assert_eq!(r.cursor(), CharPos(0));
    // In the first sentence, near its start: its start; then nothing.
    r.go(at(PROSE, "gamma"));
    r.act(ActionId::PreviousSentence);
    assert_eq!(r.cursor(), CharPos(0));
    r.act(ActionId::PreviousSentence);
    assert_eq!(r.said.last(), "No previous sentence.");
}

#[test]
fn replay_sentence_always_reads_from_its_start() {
    let mut r = rig(PROSE);
    r.go(at(PROSE, "theta"));
    r.act(ActionId::ReplaySentence);
    assert_eq!(r.app.playback(), Playback::Reading);
    r.wait_idle();
    let first = r.log.spoken_ranges()[0];
    assert_eq!(first.start, at(PROSE, "Eta"));
}

#[test]
fn reads_each_unit_in_place() {
    let text = "First line here.\n\nSecond line. Another sentence here.";
    let mut r = rig(text);
    r.go(at(text, "line here"));

    r.act(ActionId::ReadCurrentWord);
    r.wait_idle();
    let word = CharRange::new(
        at(text, "line here"),
        at(text, "line here").saturating_add(4),
    );
    assert_eq!(r.log.spoken_ranges().last().copied(), Some(word));
    assert_eq!(
        r.cursor(),
        at(text, "line here"),
        "reading in place does not move"
    );

    r.act(ActionId::ReadCurrentSentence);
    r.wait_idle();
    assert_eq!(
        r.log.text_utterances().last().unwrap().text,
        "First line here."
    );

    r.act(ActionId::ReadCurrentLine);
    r.wait_idle();
    assert_eq!(
        r.log.text_utterances().last().unwrap().text,
        "First line here."
    );

    r.act(ActionId::ReadCurrentCharacter);
    r.wait_spoken(|u| u.kind == UtteranceKind::Character && u.text == "l");

    // An empty line says "blank".
    r.go(CharPos(17));
    r.act(ActionId::ReadCurrentLine);
    r.wait_spoken(|u| u.text == "blank");

    // No selection yet, then a selection.
    r.act(ActionId::ReadSelection);
    assert_eq!(r.said.last(), "No selection.");
    let sel = CharRange::new(at(text, "Another"), at(text, "Another").saturating_add(16));
    r.app.dispatch(Command::Select(sel));
    r.act(ActionId::ReadSelection);
    r.wait_idle();
    assert_eq!(r.log.spoken_ranges().last().copied(), Some(sel));
}

#[test]
fn reading_the_document_highlights_what_is_spoken() {
    let mut r = rig(PROSE);
    r.act(ActionId::ReadFromCursor);
    assert_eq!(r.app.playback(), Playback::Reading);
    r.wait_idle();
    let spoken = r.log.spoken_ranges();
    let highlights = r.app.spoken_log().to_vec();
    assert!(!spoken.is_empty() && !highlights.is_empty());
    for h in &highlights {
        assert!(
            spoken.iter().any(|s| s.contains_range(*h)),
            "highlight {h} outside every spoken range {spoken:?}"
        );
    }
    for s in &spoken {
        assert!(
            highlights.iter().any(|h| s.contains_range(*h)),
            "spoken range {s} never highlighted"
        );
    }
    // Highlights move forward only, and the cursor followed them.
    assert!(highlights.windows(2).all(|w| w[0].start <= w[1].start));
    assert_eq!(r.cursor(), highlights.last().unwrap().start);
    // The whole rest of the document was read.
    assert_eq!(spoken.last().unwrap().end.0, PROSE.len());
}

#[test]
fn restarting_ignores_the_stale_reading() {
    let mut r = rig(PROSE);
    r.act(ActionId::ReadFromCursor);
    // Restart immediately, before any status is polled.
    r.act(ActionId::NextSentence);
    r.wait_idle();
    let second = at(PROSE, "Eta");
    assert!(!r.app.spoken_log().is_empty());
    for h in r.app.spoken_log() {
        assert!(h.start >= second, "stale highlight {h}");
    }
}

#[test]
fn pause_then_move_resumes_from_the_new_place() {
    let mut r = rig(PROSE);
    r.act(ActionId::PlayPause);
    assert_eq!(r.app.playback(), Playback::Reading);
    r.act(ActionId::PlayPause);
    assert!(matches!(r.app.playback(), Playback::Paused { .. }));
    assert!(r.said.any("Paused."));
    r.go(at(PROSE, "Pi"));
    assert_eq!(
        r.app.playback(),
        Playback::Paused {
            resume_at: Some(at(PROSE, "Pi"))
        }
    );
    r.act(ActionId::PlayPause);
    r.wait_idle();
    // The first reading may still be finishing on the speech thread; the
    // highlight only follows the resumed one.
    assert_eq!(r.app.spoken_log()[0].start, at(PROSE, "Pi"));
    assert!(
        r.log
            .spoken_ranges()
            .iter()
            .any(|s| s.start == at(PROSE, "Pi"))
    );
}

#[test]
fn stop_announces_and_clears_find() {
    let mut r = rig(PROSE);
    r.act(ActionId::ReadFromCursor);
    r.act(ActionId::Stop);
    assert_eq!(r.app.playback(), Playback::Idle);
    assert_eq!(r.said.last(), "Stopped.");
    r.app.dispatch(Command::Find("beta".into()));
    assert!(r.app.session().unwrap().find.is_some());
    r.act(ActionId::Stop);
    assert!(r.app.session().unwrap().find.is_none());
}

#[test]
fn paragraph_navigation() {
    let mut r = rig(PROSE);
    r.act(ActionId::NextParagraph);
    assert_eq!(r.cursor(), at(PROSE, "Pi"));
    r.act(ActionId::NextParagraph);
    assert_eq!(r.cursor(), at(PROSE, "Chi"));
    r.act(ActionId::NextParagraph);
    assert_eq!(r.said.last(), "No next paragraph.");
    r.go(at(PROSE, "rho"));
    r.act(ActionId::PreviousParagraph);
    assert_eq!(r.cursor(), at(PROSE, "Pi"));
    r.act(ActionId::PreviousParagraph);
    assert_eq!(r.cursor(), CharPos(0));
    r.go(at(PROSE, "Tau"));
    r.act(ActionId::ReplayParagraph);
    r.wait_idle();
    assert_eq!(r.log.spoken_ranges()[0].start, at(PROSE, "Pi"));
}

const STRUCT: &str = "Title\n\nIntro one. Intro two.\n\nSection A\n\nFirst item\nSecond item\n\nName Role\nAda Engineer\n\nSee the site now.\n\nSection B\n\nThe end.";

fn structured() -> Document {
    let t = STRUCT;
    let r = |needle: &str| {
        let s = at(t, needle);
        CharRange::new(s, s.saturating_add(needle.len()))
    };
    let heading = |needle: &str, level| Marker {
        level,
        ..Marker::new(MarkerKind::Heading, r(needle))
    };
    let item = |needle: &str| Marker {
        level: 1,
        ..Marker::new(MarkerKind::ListItem, r(needle))
    };
    let markers = vec![
        heading("Title", 1),
        heading("Section A", 2),
        Marker::new(MarkerKind::List, r("First item\nSecond item")),
        item("First item"),
        item("Second item"),
        Marker::new(MarkerKind::Table, r("Name Role\nAda Engineer")),
        Marker::new(MarkerKind::TableRow, r("Name Role")),
        Marker::new(MarkerKind::TableRow, r("Ada Engineer")),
        Marker {
            reference: Some("https://example.org".into()),
            ..Marker::new(MarkerKind::Link, r("site"))
        },
        heading("Section B", 1),
    ];
    Document::new(
        DocumentMeta {
            format: "test".into(),
            ..DocumentMeta::default()
        },
        Rope::from_str(t),
        markers,
    )
}

#[test]
fn structure_navigation() {
    let mut r = rig_with_doc(structured());
    // Move-only heading: announced, no reading.
    r.act(ActionId::SkipNextHeading);
    assert_eq!(r.cursor(), at(STRUCT, "Section A"));
    assert_eq!(r.app.playback(), Playback::Idle);
    assert_eq!(r.said.last(), "Heading level 2: Section A");
    // Reading heading: always reads from it.
    r.act(ActionId::NextHeading);
    assert_eq!(r.cursor(), at(STRUCT, "Section B"));
    assert_eq!(r.app.playback(), Playback::Reading);
    r.wait_idle();
    assert_eq!(r.log.spoken_ranges()[0].start, at(STRUCT, "Section B"));
    r.act(ActionId::NextHeading);
    assert_eq!(r.said.last(), "No next heading.");
    r.act(ActionId::SkipPreviousHeading);
    assert_eq!(r.cursor(), at(STRUCT, "Section B"));
    r.act(ActionId::SkipPreviousHeading);
    assert_eq!(r.cursor(), at(STRUCT, "Section A"));

    r.go(CharPos(0));
    r.act(ActionId::NextList);
    assert_eq!(r.cursor(), at(STRUCT, "First item"));
    assert_eq!(r.said.last(), "List, 2 items: First item Second item");
    r.act(ActionId::NextListItem);
    assert_eq!(r.cursor(), at(STRUCT, "Second item"));
    assert_eq!(r.said.last(), "List item: Second item");
    r.act(ActionId::PreviousListItem);
    assert_eq!(r.cursor(), at(STRUCT, "First item"));
    r.act(ActionId::NextTable);
    assert_eq!(r.cursor(), at(STRUCT, "Name"));
    assert!(r.said.last().starts_with("Table, 2 rows: Name Role"));
    r.act(ActionId::NextTable);
    assert_eq!(r.said.last(), "No next table.");
    r.act(ActionId::NextLink);
    assert_eq!(r.cursor(), at(STRUCT, "site"));
    assert_eq!(r.said.last(), "Link: site");
    r.act(ActionId::PreviousTable);
    assert_eq!(r.cursor(), at(STRUCT, "Name"));
}

#[test]
fn chapters_use_level_one_headings() {
    let mut r = rig_with_doc(structured());
    r.act(ActionId::NextChapter);
    assert_eq!(r.cursor(), at(STRUCT, "Section B"));
    assert_eq!(r.said.last(), "Chapter: Section B");
    r.act(ActionId::NextChapter);
    assert_eq!(r.said.last(), "No next chapter.");
    // Near the chapter start: previous chapter.
    r.act(ActionId::PreviousChapter);
    assert_eq!(r.cursor(), CharPos(0));
    // More than five words in: back to this chapter's start.
    r.go(at(STRUCT, "site"));
    r.act(ActionId::PreviousChapter);
    assert_eq!(r.cursor(), CharPos(0));
    let mut plain = rig(PROSE);
    plain.act(ActionId::NextChapter);
    assert_eq!(plain.said.last(), "This document has no chapters.");
}

#[test]
fn verbosity_shapes_messages() {
    let mut r = rig_with_doc(structured());
    r.app.dispatch(Command::Action(ActionId::SkipNextHeading));
    assert_eq!(r.said.last(), "Heading level 2: Section A");
    let said = Said::default();
    let (mut config, _log) = make_config(&said);
    config.settings.speech.verbosity = Verbosity::Low;
    let mut app = App::new(config);
    app.open_document(structured(), DocKey::untitled(2), "Low".into());
    app.dispatch(Command::Action(ActionId::SkipNextHeading));
    assert_eq!(said.last(), "Section A");
    // Normal-level notes are dropped at Low.
    app.dispatch(Command::Action(ActionId::ReadFromCursor));
    app.dispatch(Command::Action(ActionId::PlayPause));
    assert!(!said.any("Paused."));
}

#[test]
fn speech_cursor_reads_lines_and_says_blank() {
    let text = "Line one\n\nLine three\nLast line";
    let mut r = rig(text);
    r.act(ActionId::SpeechCursorToggle);
    assert_eq!(r.app.mode(), Mode::SpeechCursor);
    r.wait_idle();
    assert_eq!(r.log.text_utterances().last().unwrap().text, "Line one");
    r.act(ActionId::SpeechCursorNextLine);
    r.wait_spoken(|u| u.text == "blank");
    r.act(ActionId::SpeechCursorNextLine);
    r.wait_idle();
    assert_eq!(r.log.text_utterances().last().unwrap().text, "Line three");
    r.act(ActionId::SpeechCursorNextLine);
    r.act(ActionId::SpeechCursorNextLine);
    assert_eq!(r.said.last(), "End of document.");
    assert_eq!(
        r.app.session().unwrap().speech_cursor_line,
        Some(3),
        "clamped, no wrap"
    );
    r.wait_idle();
    r.act(ActionId::SpeechCursorRereadLine);
    r.wait_idle();
    assert_eq!(r.log.text_utterances().last().unwrap().text, "Last line");
    // Other navigation moves the Speech Cursor to the target's line.
    r.act(ActionId::DocumentStart);
    assert_eq!(r.app.session().unwrap().speech_cursor_line, Some(0));
    r.wait_idle();
    r.act(ActionId::SpeechCursorPreviousLine);
    assert_eq!(r.said.last(), "Top of document.");
    // Enter leaves and reads on from the line.
    r.act(ActionId::SpeechCursorNextLine);
    r.act(ActionId::SpeechCursorNextLine);
    r.wait_idle();
    r.log.clear();
    r.act(ActionId::SpeechCursorExitAndRead);
    assert_eq!(r.app.mode(), Mode::Browse);
    r.wait_idle();
    assert_eq!(r.log.spoken_ranges()[0].start, at(text, "Line three"));
    // Tab toggles off and stops.
    r.act(ActionId::SpeechCursorToggle);
    r.act(ActionId::SpeechCursorToggle);
    assert_eq!(r.app.mode(), Mode::Browse);
    assert!(r.said.any("Speech Cursor off."));
}

#[test]
fn find_prompt_next_previous_and_wrap() {
    let text = "A cat sat. The dog ran. A cat ate. The cat slept.";
    let mut r = rig(text);
    let effects = r.act(ActionId::Find);
    assert_eq!(
        effects,
        vec![Effect::Prompt {
            label: "Find".into(),
            purpose: PromptPurpose::Find
        }]
    );
    assert_eq!(r.app.mode(), Mode::Find);
    r.app.dispatch(Command::Answer("CAT".into()));
    assert_eq!(r.app.mode(), Mode::Browse);
    assert_eq!(r.cursor(), CharPos(2));
    assert!(r.said.last().starts_with("Match 1 of 3"));
    r.act(ActionId::FindNext);
    assert_eq!(r.cursor(), CharPos(26));
    r.act(ActionId::FindNext);
    assert_eq!(r.cursor(), CharPos(39));
    r.act(ActionId::FindNext);
    assert_eq!(r.cursor(), CharPos(2));
    assert!(r.said.last().starts_with("Wrapped to top."));
    r.act(ActionId::FindPrevious);
    assert_eq!(r.cursor(), CharPos(39));
    assert!(r.said.last().starts_with("Wrapped to bottom."));
    // Each find jump recorded history once.
    let entries = r.app.session().unwrap().history.entries().len();
    assert!(entries >= 4, "{entries}");
    r.app.dispatch(Command::Find("zebra".into()));
    assert_eq!(r.said.last(), "No matches for zebra.");
    r.app.dispatch(Command::Find("/d.g/".into()));
    assert_eq!(r.cursor(), CharPos(15));
    // Find next with no search opens the prompt.
    let mut fresh = rig(text);
    assert!(matches!(
        fresh.act(ActionId::FindNext).as_slice(),
        [Effect::Prompt {
            purpose: PromptPurpose::Find,
            ..
        }]
    ));
    fresh.app.dispatch(Command::Cancel);
    assert_eq!(fresh.app.mode(), Mode::Browse);
    assert_eq!(fresh.said.last(), "Cancelled.");
}

#[test]
fn history_records_each_jump_once() {
    let mut r = rig(PROSE);
    r.act(ActionId::NextSentence);
    r.act(ActionId::NextSentence);
    let third = at(PROSE, "Kappa");
    assert_eq!(r.cursor(), third);
    assert_eq!(r.app.session().unwrap().history.entries().len(), 2);
    // Caret moves do not record.
    r.act(ActionId::CaretNextWord);
    r.act(ActionId::CaretPreviousWord);
    assert_eq!(r.app.session().unwrap().history.entries().len(), 2);
    r.act(ActionId::HistoryBack);
    assert_eq!(r.cursor(), at(PROSE, "Eta"));
    r.act(ActionId::HistoryBack);
    assert_eq!(r.cursor(), CharPos(0));
    r.act(ActionId::HistoryBack);
    assert_eq!(r.said.last(), "No earlier history.");
    r.act(ActionId::HistoryForward);
    assert_eq!(r.cursor(), at(PROSE, "Eta"));
    r.act(ActionId::HistoryForward);
    assert_eq!(r.cursor(), third, "forward returns to where we were");
    r.act(ActionId::HistoryForward);
    assert_eq!(r.said.last(), "No forward history.");
}

#[test]
fn go_to_prompt_and_targets() {
    let text = "one\ntwo\nthree\nfour";
    let mut r = rig(text);
    assert!(matches!(
        r.act(ActionId::GoTo).as_slice(),
        [Effect::Prompt {
            purpose: PromptPurpose::GoTo,
            ..
        }]
    ));
    assert_eq!(r.app.mode(), Mode::GoTo);
    r.app.dispatch(Command::Answer("3".into()));
    assert_eq!(r.cursor(), at(text, "three"));
    r.act(ActionId::GoTo);
    r.app.dispatch(Command::Answer("tomorrow".into()));
    assert!(r.said.last().starts_with("Not a go-to target"));
    r.act(ActionId::DocumentEnd);
    assert_eq!(r.cursor(), at(text, "four"));
    assert!(r.said.last().starts_with("End of document."));
    r.act(ActionId::DocumentStart);
    assert_eq!(r.cursor(), CharPos(0));
}

#[test]
fn caret_moves_speak_and_stop_reading() {
    let text = "alpha beta\n\ngamma delta\nepsilon";
    let mut r = rig(text);
    r.act(ActionId::ReadFromCursor);
    r.act(ActionId::CaretNextWord);
    assert_eq!(r.app.playback(), Playback::Idle);
    assert_eq!(r.cursor(), at(text, "beta"));
    r.wait_spoken(|u| u.text == "beta");
    // Line moves skip blank lines and keep the column.
    r.act(ActionId::CaretNextLine);
    assert_eq!(r.cursor(), at(text, "delta"));
    r.wait_spoken(|u| u.text == "gamma delta");
    r.act(ActionId::CaretNextLine);
    assert_eq!(r.cursor(), at(text, "epsilon"));
    r.act(ActionId::CaretNextLine);
    assert_eq!(r.said.last(), "End of document.");
    r.act(ActionId::CaretPreviousLine);
    assert_eq!(r.cursor(), at(text, "delta"), "goal column is sticky");
}

#[test]
fn voice_changes_are_announced_and_applied() {
    let mut r = rig(PROSE);
    r.act(ActionId::RateUp);
    assert_eq!(r.said.last(), "285 words per minute.");
    assert_eq!(r.app.settings().speech.rate.wpm(), 285);
    let deadline = Instant::now() + Duration::from_secs(30);
    while r.log.params().map(|p| p.rate.wpm()) != Some(285) {
        assert!(Instant::now() < deadline, "rate never reached the backend");
        std::thread::sleep(Duration::from_millis(2));
    }
    r.act(ActionId::PitchDown);
    assert_eq!(r.said.last(), "Pitch minus 1.");
    r.act(ActionId::VolumeDown);
    assert_eq!(r.said.last(), "Volume 90 percent.");
    // Presets, fastest first: skim 350, normal 265, study 200, slow 150.
    // From 285 wpm (no preset) the next slower preset comes first.
    r.act(ActionId::CycleSpeedPreset);
    assert_eq!(r.said.last(), "Normal speed, 265 words per minute.");
    r.act(ActionId::CycleSpeedPreset);
    assert_eq!(r.said.last(), "Study speed, 200 words per minute.");
    r.act(ActionId::CycleSpeedPreset);
    r.act(ActionId::CycleSpeedPreset);
    assert_eq!(r.said.last(), "Skim speed, 350 words per minute.");
    r.act(ActionId::NextTheme);
    // Galaxy is the default; the cycle follows the theme registry.
    assert_eq!(r.said.last(), "Theme Galaxy Light.");
    assert_eq!(r.app.settings().display.theme, "galaxy-light");
    assert!(r.app.settings().display.theme_explicit);
    r.act(ActionId::ToggleLineNumbers);
    assert_eq!(r.said.last(), "Line numbers on.");
}

#[test]
fn keyboard_help_and_palette_come_from_the_keymap() {
    let mut r = rig(PROSE);
    let effects = r.act(ActionId::KeyboardHelp);
    let [Effect::ShowList { title, items }] = effects.as_slice() else {
        panic!("{effects:?}");
    };
    assert_eq!(title, "Keyboard shortcuts");
    assert_eq!(items.len(), ActionId::ALL.len());
    let i = items
        .iter()
        .position(|s| s.contains(ActionId::NextSentence.help()))
        .unwrap();
    assert!(items[i].contains("Alt+."), "{}", items[i]);
    r.app.dispatch(Command::Choose(i));
    assert_eq!(r.cursor(), at(PROSE, "Eta"));

    r.act(ActionId::CommandPalette);
    assert_eq!(r.app.mode(), Mode::Command);
    r.app.dispatch(Command::Answer("next sentence".into()));
    assert_eq!(r.cursor(), at(PROSE, "Kappa"));
    r.act(ActionId::CommandPalette);
    r.app.dispatch(Command::Answer("frobnicate".into()));
    assert_eq!(r.said.last(), "Unknown command: frobnicate.");
    assert!(
        r.app
            .palette_candidates("bookmark")
            .iter()
            .any(|(a, _)| *a == ActionId::AddBookmark)
    );
}

#[test]
fn bookmarks_add_list_step() {
    let mut r = rig(PROSE);
    r.go(at(PROSE, "Eta"));
    r.act(ActionId::AddBookmark);
    assert!(r.said.last().starts_with("Bookmark mark1 set at"));
    r.act(ActionId::AddBookmark);
    assert_eq!(r.said.last(), "Bookmark mark1 is already here.");
    r.go(at(PROSE, "Tau"));
    r.act(ActionId::AddBookmark);
    r.go(CharPos(0));
    r.act(ActionId::NextBookmark);
    assert_eq!(r.cursor(), at(PROSE, "Eta"));
    r.act(ActionId::NextBookmark);
    assert_eq!(r.cursor(), at(PROSE, "Tau"));
    r.act(ActionId::NextBookmark);
    assert_eq!(r.cursor(), at(PROSE, "Eta"));
    assert!(r.said.last().starts_with("Wrapped."));
    r.act(ActionId::PreviousBookmark);
    assert_eq!(r.cursor(), at(PROSE, "Tau"));
    let effects = r.act(ActionId::ListBookmarks);
    let [Effect::ShowList { items, .. }] = effects.as_slice() else {
        panic!("{effects:?}");
    };
    assert_eq!(items.len(), 2);
    assert!(items[0].starts_with("mark1, line 1"));
    r.app.dispatch(Command::Choose(0));
    assert_eq!(r.cursor(), at(PROSE, "Eta"));
}

#[test]
fn selection_extends_by_word() {
    let text = "one two three";
    let mut r = rig(text);
    r.app.dispatch(Command::ExtendSelection(
        textweaver_app::core::Unit::Word,
        textweaver_app::core::Direction::Forward,
    ));
    assert_eq!(r.said.last(), "one selected");
    r.app.dispatch(Command::ExtendSelection(
        textweaver_app::core::Unit::Word,
        textweaver_app::core::Direction::Forward,
    ));
    assert_eq!(
        r.app.session().unwrap().selection,
        Some(CharRange::new(0, 7))
    );
    r.app.dispatch(Command::ExtendSelection(
        textweaver_app::core::Unit::Word,
        textweaver_app::core::Direction::Backward,
    ));
    assert_eq!(r.said.last(), "two unselected");
}

#[test]
fn no_document_is_explained() {
    let said = Said::default();
    let (config, _log) = make_config(&said);
    let mut app = App::new(config);
    app.dispatch(Command::Action(ActionId::NextSentence));
    assert!(said.last().starts_with("No document is open."));
    app.dispatch(Command::Open("definitely/missing/file.txt".into()));
    assert!(said.last().starts_with("Could not open"));
}

#[test]
fn position_bookmarks_and_settings_persist() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("prose.txt");
    std::fs::write(&file, PROSE).unwrap();
    let paths = Paths::under(&dir.path().join("home"));

    let said = Said::default();
    let (mut config, _log) = make_config(&said);
    config.paths = Some(paths.clone());
    let mut app = App::new(config);
    app.open(&file).unwrap();
    app.dispatch(Command::GoTo(GoTo::Char(at(PROSE, "Kappa"))));
    app.dispatch(Command::Action(ActionId::AddBookmark));
    app.dispatch(Command::GoTo(GoTo::Char(at(PROSE, "rho"))));
    app.dispatch(Command::Action(ActionId::RateUp));
    app.dispatch(Command::Action(ActionId::Quit));
    assert_eq!(
        app.dispatch(Command::Confirm(Confirm::Yes)),
        vec![Effect::Quit]
    );
    drop(app);
    assert!(paths.settings_file().exists());
    assert!(paths.recent_file().exists());

    // Relaunch: settings and position come back.
    let said = Said::default();
    let (mut config, _log) = make_config(&said);
    config.paths = Some(paths.clone());
    config.settings = textweaver_app::store::SettingsStore::new(paths.clone())
        .load()
        .0;
    assert_eq!(config.settings.speech.rate.wpm(), 285);
    let mut app = App::new(config);
    app.open(&file).unwrap();
    let s = app.session().unwrap();
    assert_eq!(s.cursor, at(PROSE, "rho"));
    assert_eq!(s.bookmarks.len(), 1);
    assert_eq!(s.bookmarks[0].pos, at(PROSE, "Kappa"));
    assert!(said.last().contains("Resumed at"), "{}", said.last());

    // Without auto-resume the document opens at the start.
    let said = Said::default();
    let (mut config, _log) = make_config(&said);
    config.paths = Some(paths);
    config.settings.reading.auto_resume = false;
    let mut app = App::new(config);
    app.open(&file).unwrap();
    assert_eq!(app.session().unwrap().cursor, CharPos(0));
    assert_eq!(said.last(), "Opened prose.txt.");
}

#[test]
fn switching_documents_saves_the_first() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.txt");
    let b = dir.path().join("b.txt");
    std::fs::write(&a, PROSE).unwrap();
    std::fs::write(&b, "Other text here.").unwrap();
    let paths = Paths::under(&dir.path().join("home"));
    let said = Said::default();
    let (mut config, _log) = make_config(&said);
    config.paths = Some(paths);
    let mut app = App::new(config);
    app.open(&a).unwrap();
    app.dispatch(Command::Action(ActionId::NextParagraph));
    app.dispatch(Command::Open(b.clone()));
    assert_eq!(app.session().unwrap().title, "b.txt");
    app.dispatch(Command::Open(a));
    assert_eq!(app.session().unwrap().cursor, at(PROSE, "Pi"));
}

#[test]
fn say_position_reports_line_and_percent() {
    let mut r = rig_with_doc(structured());
    r.go(at(STRUCT, "First item"));
    r.act(ActionId::SayPosition);
    let line = STRUCT[..STRUCT.find("First item").unwrap()]
        .matches('\n')
        .count()
        + 1;
    assert!(
        r.said.last().starts_with(&format!("Line {line} of 17")),
        "{}",
        r.said.last()
    );
    assert!(r.said.last().contains("Under heading Section A."));
}

#[test]
fn highlight_granularity_word_sentence_both() {
    use textweaver_app::HighlightKind;
    use textweaver_app::core::HighlightGranularity;
    for g in [
        HighlightGranularity::Word,
        HighlightGranularity::Sentence,
        HighlightGranularity::Both,
    ] {
        let said = Said::default();
        let (mut config, _log) = make_config(&said);
        config.settings.highlight.granularity = g;
        let mut app = App::new(config);
        app.open_document(
            Document::from_plain_text(PROSE),
            DocKey::untitled(3),
            "G".into(),
        );
        app.dispatch(Command::Action(ActionId::ReadFromCursor));
        // Step until the first highlight arrives, then inspect it.
        let deadline = Instant::now() + Duration::from_secs(30);
        let spoken = loop {
            app.poll_speech_step();
            if let Some(r) = app.session().unwrap().spoken {
                break r;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        };
        let all = CharRange::new(0, PROSE.len());
        let kinds: Vec<(HighlightKind, CharRange)> = app
            .highlights(all)
            .into_iter()
            .map(|h| (h.kind, h.range))
            .collect();
        let word = (HighlightKind::SpokenWord, spoken);
        let sentence = kinds
            .iter()
            .find(|(k, _)| *k == HighlightKind::SpokenSentence)
            .map(|(_, r)| *r);
        match g {
            HighlightGranularity::Word => assert_eq!(kinds, vec![word]),
            HighlightGranularity::Sentence => {
                assert!(!kinds.contains(&word));
                let first = CharRange::new(0, at(PROSE, " Eta").0);
                assert!(sentence.unwrap().contains_range(first));
            }
            HighlightGranularity::Both => {
                assert!(kinds.contains(&word));
                assert!(sentence.unwrap().contains_range(spoken));
            }
        }
        app.dispatch(Command::Action(ActionId::Stop));
    }
}

#[test]
fn stop_leaves_speech_cursor() {
    let mut r = rig(PROSE);
    r.act(ActionId::SpeechCursorToggle);
    assert_eq!(r.app.mode(), Mode::SpeechCursor);
    r.act(ActionId::Stop);
    assert_eq!(r.app.mode(), Mode::Browse);
    assert_eq!(r.app.playback(), Playback::Idle);
    assert_eq!(r.said.last(), "Stopped. Speech Cursor off.");
}

// Ported from the September 2026 audit's patches (docs/audit-2026-09/) by
// Agent D4: S2 (single keys and missing actions), S6 (table mode), S7
// (ordered list items).

#[test]
fn every_action_with_a_default_key_has_a_handler() {
    // Shift+arrows (select), Shift+S (read paragraph), and F9 (single-key
    // shortcuts) said "... is not available yet."
    use textweaver_app::keymap::{Frontend, Keymap, Platform};
    let keymap = Keymap::defaults(Platform::current(), Frontend::Terminal);
    let mut r = rig(PROSE);
    for &a in ActionId::ALL {
        // Choose voice waits for a voice list from the speech service.
        let pending = a == ActionId::ChooseVoice;
        if pending || a.confirmation_prompt().is_some() || keymap.chords_for(a).is_empty() {
            continue;
        }
        let before = r.said.all().len();
        r.act(a);
        r.app.dispatch(Command::Cancel);
        let new = &r.said.all()[before..];
        assert!(
            !new.iter().any(|m| m.contains("is not available yet")),
            "{a:?} has a key but no handler: {new:?}"
        );
        // Leave edit mode where an action entered it.
        if r.app.mode() == Mode::Edit {
            r.act(ActionId::ToggleEditMode);
        }
        r.app.dispatch(Command::Action(ActionId::Stop));
    }
}

#[test]
fn single_key_shortcuts_follow_the_setting_and_f9() {
    use std::str::FromStr;
    let mut r = rig(PROSE);
    let t = textweaver_app::keymap::KeyChord::from_str("t").unwrap();
    let layer = Mode::Browse.layer();
    assert!(r.app.keymap().lookup(&t, layer).is_some());
    r.act(ActionId::ToggleCharacterKeys);
    assert!(r.said.any("Single-key shortcuts off"), "{:?}", r.said.all());
    assert!(r.app.keymap().lookup(&t, layer).is_none());
    assert!(!r.app.settings().keyboard.character_keys);
    r.act(ActionId::ToggleCharacterKeys);
    assert!(r.app.keymap().lookup(&t, layer).is_some());
    // The setting applies at startup.
    let said = Said::default();
    let (mut config, _log) = make_config(&said);
    config.settings.keyboard.character_keys = false;
    let app = App::new(config);
    assert!(app.keymap().lookup(&t, layer).is_none());
}

#[test]
fn shift_arrows_select_in_browse_mode() {
    let mut r = rig(PROSE);
    r.act(ActionId::SelectNextWord);
    r.act(ActionId::SelectNextWord);
    let sel = r.app.session().unwrap().selection.expect("a selection");
    assert_eq!(r.app.session().unwrap().doc.slice(sel).trim(), "Alpha beta");
}

fn load_md(md: &str) -> Document {
    use textweaver_app::formats::{LoadOptions, Registry, Source};
    Registry::with_builtins()
        .load(
            &Source::Bytes {
                data: md.as_bytes().to_vec(),
                hint: "md".into(),
            },
            &LoadOptions::default(),
        )
        .unwrap()
}

#[test]
fn table_mode_setting_changes_how_tables_are_read() {
    use textweaver_app::store::TableMode;
    let doc = load_md("Intro.\n\n| Name | Role |\n|---|---|\n| Ada | Engineer |\n\nAfter.\n");
    for (mode, expect) in [
        (TableMode::Flat, "Ada, Engineer"),
        (TableMode::Skip, "skipped"),
    ] {
        let said = Said::default();
        let (mut config, log) = make_config(&said);
        config.settings.normalization.table_mode = mode;
        let mut app = App::new(config);
        app.open_document(doc.clone(), DocKey::untitled(1), "T".into());
        app.dispatch(Command::Action(ActionId::ReadFromCursor));
        let deadline = Instant::now() + Duration::from_secs(10);
        while app.playback() != Playback::Idle && Instant::now() < deadline {
            app.poll_speech();
            std::thread::sleep(Duration::from_millis(2));
        }
        let texts = log.texts().join(" | ");
        assert!(texts.contains(expect), "{mode:?}: {texts}");
    }
}

#[test]
fn footnote_mode_setting_decides_where_footnotes_are_read() {
    use textweaver_app::store::FootnoteMode;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("notes.md");
    std::fs::write(
        &file,
        "Text with a note[^1] here.\n\n[^1]: The note body.\n",
    )
    .unwrap();
    let text_for = |mode: FootnoteMode| {
        let said = Said::default();
        let (mut config, _log) = make_config(&said);
        config.settings.normalization.footnote_mode = mode;
        let mut app = App::new(config);
        app.open(&file).unwrap();
        app.session().unwrap().doc.text().to_string()
    };
    let inline = text_for(FootnoteMode::Inline);
    let skip = text_for(FootnoteMode::Skip);
    let deferred = text_for(FootnoteMode::Deferred);
    assert!(inline.contains("footnote: The note body"), "{inline}");
    assert!(!skip.contains("The note body"), "{skip}");
    assert!(deferred.contains("Footnotes"), "{deferred}");
    assert_ne!(inline, deferred);
}

#[test]
fn an_ordered_list_item_is_announced_with_its_text() {
    let doc = load_md("Intro.\n\n1. Buy milk\n2. Walk the dog\n");
    let mut r = rig_with_doc(doc);
    r.act(ActionId::NextListItem);
    r.act(ActionId::NextListItem);
    assert_eq!(r.said.last(), "List item: 2. Walk the dog");
}
