//! Citations in continuous reading (Jon's decision, 2026-09-26): skipped
//! by default, said in words when turned on (Alt+Shift+Q), with the
//! highlight exact both ways; word moves say them in words either way.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use textweaver_app::core::{CharPos, CharRange};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::{CitationReading, Settings};
use textweaver_app::testing::{SpeechLog, recording_service};
use textweaver_app::{App, AppConfig, Command, Playback};

const REFS: &str = r#"[
  {"id": "doe2020", "type": "book", "title": "On reading",
   "author": [{"family": "Doe", "given": "Jane"}, {"family": "Roe", "given": "Rick"}],
   "issued": {"date-parts": [[2020]]}}
]"#;

struct Rig {
    app: App,
    log: SpeechLog,
    _tmp: tempfile::TempDir,
    text: String,
}

fn rig(text: &str, mode: CitationReading) -> Rig {
    let tmp = tempfile::tempdir().unwrap();
    let dir: PathBuf = tmp.path().to_owned();
    std::fs::write(dir.join("references.json"), REFS).unwrap();
    let file = dir.join("essay.md");
    std::fs::write(&file, text).unwrap();
    let (speech, log) = recording_service().unwrap();
    let mut settings = Settings::default();
    settings.reading.citations = mode;
    let mut app = App::new(AppConfig {
        settings,
        speech,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    app.open(&file).unwrap();
    Rig {
        app,
        log,
        _tmp: tmp,
        text: text.to_owned(),
    }
}

impl Rig {
    fn read_all(&mut self) {
        self.log.clear();
        self.app.set_cursor(CharPos(0));
        self.app.dispatch(Command::Action(ActionId::ReadFromCursor));
        let deadline = Instant::now() + Duration::from_secs(30);
        while self.app.playback() != Playback::Idle {
            assert!(Instant::now() < deadline, "reading never finished");
            if self.app.poll_speech_step().is_none() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
    fn spoken(&self) -> String {
        self.log
            .text_utterances()
            .iter()
            .map(|u| u.text.clone())
            .collect::<Vec<_>>()
            .join(" | ")
    }
    fn range_of(&self, word: &str) -> CharRange {
        let byte = self.text.find(word).unwrap();
        let start = self.text[..byte].chars().count();
        CharRange::new(start, start + word.chars().count())
    }
    /// Each highlight, as the document text it covers.
    fn highlighted(&self) -> Vec<(CharRange, String)> {
        let doc = &self.app.session().unwrap().doc;
        let mut out: Vec<(CharRange, String)> = Vec::new();
        for r in self.app.spoken_log() {
            if out.last().is_none_or(|(l, _)| l != r) {
                out.push((*r, doc.slice(*r)));
            }
        }
        out
    }
}

const TEXT: &str = "Reading helps [@doe2020, p. 12] a lot. Next one is here.\n";

#[test]
fn citations_are_skipped_by_default_and_highlights_stay_exact() {
    assert_eq!(Settings::default().reading.citations, CitationReading::Off);
    let mut r = rig(TEXT, CitationReading::Off);
    r.read_all();
    let spoken = r.spoken();
    assert!(!spoken.contains("doe2020"), "{spoken}");
    assert!(!spoken.contains("Doe"), "{spoken}");
    assert!(spoken.starts_with("Reading helps a lot."), "{spoken}");
    // Every highlight is exactly a word of the source, in order, and the
    // words after the skipped citation are where they are in the text.
    let words: Vec<String> = r.highlighted().into_iter().map(|(_, w)| w).collect();
    assert_eq!(
        words,
        ["Reading", "helps", "a", "lot", "Next", "one", "is", "here"]
    );
    let hl: Vec<CharRange> = r.highlighted().into_iter().map(|(h, _)| h).collect();
    assert!(hl.contains(&r.range_of("lot")));
    assert!(hl.contains(&r.range_of("Next")));
    let cite = r.range_of("[@doe2020, p. 12]");
    assert!(hl.iter().all(|h| !cite.contains_range(*h)));
}

#[test]
fn citations_on_are_said_in_words_over_the_whole_citation() {
    let mut r = rig(TEXT, CitationReading::Words);
    r.read_all();
    let spoken = r.spoken();
    assert!(
        spoken.starts_with("Reading helps Doe and Roe, twenty twenty, page 12 a lot."),
        "{spoken}"
    );
    let cite = r.range_of("[@doe2020, p. 12]");
    let hl = r.highlighted();
    // The words said for the citation highlight the citation; the rest are
    // exact words.
    assert!(hl.iter().any(|(h, _)| *h == cite), "{hl:?}");
    for (h, w) in &hl {
        assert!(*h == cite || !w.contains(' '), "{h}: {w}");
    }
    let words: Vec<&str> = hl
        .iter()
        .filter(|(h, _)| *h != cite)
        .map(|(_, w)| w.as_str())
        .collect();
    assert_eq!(
        words,
        ["Reading", "helps", "a", "lot", "Next", "one", "is", "here"]
    );
    assert!(hl.iter().any(|(h, _)| *h == r.range_of("lot")));
}

#[test]
fn in_text_citations_and_unknown_keys() {
    let text =
        "@doe2020 argues this. See [@doe2020; @nobody, p. 4] too. Mail jon@example.com now.\n";
    let mut r = rig(text, CitationReading::Off);
    r.read_all();
    let spoken = r.spoken();
    // Off: an in-text citation keeps its authors, the sentence's subject.
    assert!(spoken.starts_with("Doe and Roe argues this."), "{spoken}");
    assert!(spoken.contains("See too."), "{spoken}");
    assert!(spoken.contains("jon at example.com"), "{spoken}");
    let mut r = rig(text, CitationReading::Words);
    r.read_all();
    let spoken = r.spoken();
    assert!(
        spoken.contains("Doe and Roe, twenty twenty argues"),
        "{spoken}"
    );
    // An unknown key is read as the key.
    assert!(
        spoken.contains("See Doe and Roe, twenty twenty; nobody, page 4 too."),
        "{spoken}"
    );
    // Not a citation when no key is known (the renderer's rule), and never
    // inside code.
    let text = "Code `[@doe2020]` and [@smith1999] stay.\n";
    let mut r = rig(text, CitationReading::Off);
    r.read_all();
    let spoken = r.spoken();
    assert!(spoken.contains("smith1999"), "{spoken}");
    assert!(spoken.contains("doe2020"), "{spoken}");
}

#[test]
fn the_key_toggles_and_says_so() {
    let mut r = rig(TEXT, CitationReading::Off);
    r.app.dispatch(Command::Action(ActionId::ToggleCitations));
    assert_eq!(r.app.status_text(), "Citations on.");
    assert_eq!(r.app.settings().reading.citations, CitationReading::Words);
    r.app.dispatch(Command::Action(ActionId::ToggleCitations));
    assert_eq!(r.app.status_text(), "Citations off.");
    // During continuous reading it goes on from the word being read, after
    // saying the change.
    r.log.clear();
    r.app.dispatch(Command::Action(ActionId::ReadFromCursor));
    r.app.dispatch(Command::Action(ActionId::ToggleCitations));
    let deadline = Instant::now() + Duration::from_secs(30);
    while r.app.playback() != Playback::Idle {
        assert!(Instant::now() < deadline);
        if r.app.poll_speech_step().is_none() {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    let texts = r.log.texts();
    assert!(texts.iter().any(|t| t == "Citations on."), "{texts:?}");
    assert!(
        texts
            .iter()
            .any(|t| t.contains("Doe and Roe, twenty twenty, page 12")),
        "{texts:?}"
    );
}

#[test]
fn word_moves_say_citations_in_words_whatever_the_setting() {
    let mut r = rig(TEXT, CitationReading::Off);
    r.app.set_cursor(r.range_of("helps").start);
    r.app.dispatch(Command::Action(ActionId::CaretNextWord));
    assert!(
        r.app
            .status_text()
            .starts_with("Citation: Doe and Roe, 2020"),
        "{}",
        r.app.status_text()
    );
}
