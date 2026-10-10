//! Offline documentation and Search help, on a fake docs folder: the
//! index, the cache, search results, a missing guide's warning, and a
//! link followed between guides and back.

use std::sync::{Arc, Mutex};

use textweaver_a11y::{Announcer, Priority};
use textweaver_core::MarkerKind;
use textweaver_store::Paths;

use super::*;
use crate::AppConfig;
use crate::command::Command;

/// What the app announced.
#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Said {
    fn any(&self, needle: &str) -> bool {
        self.0.lock().unwrap().iter().any(|s| s.contains(needle))
    }
}

impl Announcer for Said {
    fn announce(&mut self, text: &str, _priority: Priority) {
        self.0.lock().unwrap().push(text.to_owned());
    }
}

const INDEX: &str = "# Documentation

Intro [not a guide](elsewhere.md).

## For users

- [Reading](reading.md): reading aloud.
- [Audio](audio.md#top) and [Reading again](reading.md).
- [Scripts](../scripts/README.md) and [the site](https://example.org/x.md).
- [Gone](gone.md): missing from this package.

## For contributors

- [Design](design.md)
";

const READING: &str = "# Reading and moving around

Before any section.

## Opening a document

Press the open key. See [audio](audio.md#exporting-audio).

## Quokka mode

A *quokka* reads slowly.
";

const AUDIO: &str = "# Audio export

## Exporting audio

Use `tw export-audio` to make an audiobook.

## Subtitles

Subtitles come with each chapter.
";

/// A docs folder with the index, two guides, and the scripts guide one
/// folder up; `gone.md` is listed but absent.
fn fake_docs(root: &Path) -> PathBuf {
    let docs = root.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::create_dir_all(root.join("scripts")).unwrap();
    std::fs::write(docs.join("README.md"), INDEX).unwrap();
    std::fs::write(docs.join("reading.md"), READING).unwrap();
    std::fs::write(docs.join("audio.md"), AUDIO).unwrap();
    let scripts = "# Scripts\n\n## Doctor\n\nChecks.\n";
    std::fs::write(root.join("scripts").join("README.md"), scripts).unwrap();
    docs
}

/// An app on the fake docs folder, keeping its files under `home`.
fn app_on(docs: &Path, home: &Path) -> (App, Said) {
    let said = Said::default();
    let mut app = App::new(AppConfig {
        announcer: Box::new(said.clone()),
        paths: Some(Paths::under(home)),
        ..AppConfig::for_tests()
    });
    app.docs_override = Some(docs.to_owned());
    (app, said)
}

fn open_path(app: &App) -> Option<PathBuf> {
    app.session.as_ref().and_then(|s| s.doc.meta.path.clone())
}

#[test]
fn the_user_guides_are_the_local_links_under_for_users() {
    assert_eq!(
        user_guides(INDEX),
        ["reading.md", "audio.md", "../scripts/README.md", "gone.md"]
    );
}

#[test]
fn a_guide_splits_into_sections_under_its_title() {
    let s = sections_of("reading.md", READING);
    let headings: Vec<&str> = s.iter().map(|x| x.heading.as_str()).collect();
    assert_eq!(
        headings,
        [
            "Reading and moving around",
            "Opening a document",
            "Quokka mode"
        ]
    );
    assert!(s.iter().all(|x| x.guide == "Reading and moving around"));
    assert_eq!(s[2].text, "A quokka reads slowly.");
    assert!(
        s[1].text.contains("Press the open key. See audio."),
        "{:?}",
        s[1]
    );
}

#[test]
fn the_index_is_built_from_the_docs_folder_and_kept_in_the_cache() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = fake_docs(tmp.path());
    let cache = tmp.path().join("cache");
    let index = build_index(Some(docs.clone()), Some(&cache));
    assert_eq!(index.missing, ["gone.md"]);
    assert_eq!(index.sections.len(), 8, "{:?}", index.sections);
    assert!(cache.join(CACHE_FILE).is_file());

    // A second build reads the cache: the same sections.
    let again = build_index(Some(docs.clone()), Some(&cache));
    assert_eq!(again.sections, index.sections);

    // A changed guide builds the index again.
    let changed_audio = "# Audio export\n\n## Chapters\n\nMarks, and more marks.\n";
    std::fs::write(docs.join("audio.md"), changed_audio).unwrap();
    let changed = build_index(Some(docs.clone()), Some(&cache));
    assert!(changed.sections.iter().any(|s| s.heading == "Chapters"));
    assert!(!changed.sections.iter().any(|s| s.heading == "Subtitles"));

    // No index file, no docs: Help offers the online documentation.
    assert!(build_index(Some(cache), None).docs.is_none());
}

#[test]
fn a_question_finds_the_guide_section_and_says_how_many_match() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = fake_docs(tmp.path());
    let (mut app, said) = app_on(&docs, &tmp.path().join("home"));
    app.dispatch(Command::Action(ActionId::SearchHelp));
    assert!(matches!(app.list, Some(ListKind::HelpSearch(_))));
    assert_eq!(app.list_filter(), Some(""));

    app.dispatch(Command::FilterList("quokka".into()));
    assert!(said.any("1 match."));
    let Some(ListKind::HelpSearch(topics)) = app.list.clone() else {
        panic!("the search list is shown");
    };
    assert_eq!(topics.len(), 1);
    let row = app.list_model().unwrap().items[0].clone();
    assert_eq!(row, "Quokka mode, in Reading and moving around");

    // Question words are dropped; commands come before guide sections,
    // and sections whose heading matches come first.
    let found = app.help_topics("how do I export audio");
    assert_eq!(found[0], HelpTopic::Command(ActionId::ExportAudio));
    let index = app.help_index.clone().unwrap();
    let guides: Vec<&str> = found
        .iter()
        .filter_map(|t| match t {
            HelpTopic::Guide(i) => Some(index.sections[*i].heading.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(guides, ["Audio export", "Exporting audio"]);

    // Settings are found by their help.
    let rate = app.help_topics("speech rate");
    assert!(rate.contains(&HelpTopic::Setting("speech.rate".into())));

    app.dispatch(Command::FilterList("zzzz nothing".into()));
    assert!(said.any("No help matches zzzz nothing."));
}

#[test]
fn typing_in_f1_help_starts_the_search() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = fake_docs(tmp.path());
    let (mut app, _) = app_on(&docs, &tmp.path().join("home"));
    app.dispatch(Command::Action(ActionId::Help));
    assert_eq!(app.list, Some(ListKind::Help));
    app.dispatch(Command::ListKey(ListKey::Char('q')));
    assert!(matches!(app.list, Some(ListKind::HelpSearch(_))));
    assert_eq!(app.list_filter(), Some("q"));
}

#[test]
fn choosing_a_guide_section_opens_the_guide_at_its_heading() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = fake_docs(tmp.path());
    let (mut app, said) = app_on(&docs, &tmp.path().join("home"));
    app.dispatch(Command::Action(ActionId::SearchHelp));
    app.dispatch(Command::FilterList("subtitles chapter".into()));
    let Some(ListKind::HelpSearch(topics)) = app.list.clone() else {
        panic!("the search list is shown");
    };
    let row = topics
        .iter()
        .position(|t| matches!(t, HelpTopic::Guide(_)))
        .unwrap();
    app.dispatch(Command::ListFocus(row));
    app.dispatch(Command::ListKey(ListKey::Enter));
    assert_eq!(open_path(&app), Some(docs.join("audio.md")));
    let s = app.session.as_ref().unwrap();
    let rest: String = s.doc.text().chars().skip(s.cursor.0).take(9).collect();
    assert_eq!(rest, "Subtitles");
    assert!(said.any("At Subtitles."));
}

#[test]
fn documentation_warns_of_a_missing_guide_and_follows_links_and_back() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = fake_docs(tmp.path());
    let (mut app, said) = app_on(&docs, &tmp.path().join("home"));
    app.dispatch(Command::Action(ActionId::Documentation));
    assert_eq!(open_path(&app), Some(docs.join("README.md")));
    assert!(said.any("Guide missing: gone.md."), "a missing guide warns");

    // Follow the link under "For users" to the reading guide.
    let link = {
        let s = app.session.as_ref().unwrap();
        s.doc
            .marker_index()
            .iter(MarkerKind::Link, None)
            .find(|m| m.reference.as_deref() == Some("reading.md"))
            .map(|m| m.range.start)
            .unwrap()
    };
    app.session.as_mut().unwrap().cursor = link;
    app.dispatch(Command::Action(ActionId::FollowLink));
    assert_eq!(open_path(&app), Some(docs.join("reading.md")));

    // Back (Backspace or Alt+Left) returns to the index, at the link.
    app.dispatch(Command::Action(ActionId::HistoryBack));
    assert_eq!(open_path(&app), Some(docs.join("README.md")));
    assert_eq!(app.session.as_ref().unwrap().cursor, link);
}

#[test]
fn without_docs_documentation_offers_the_online_version() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut app, said) = app_on(&tmp.path().join("none"), &tmp.path().join("home"));
    app.dispatch(Command::Action(ActionId::Documentation));
    assert!(said.any("Documentation not found beside textweaver."));
    assert!(app.confirmation_pending());
    assert!(app.session.is_none());
}
