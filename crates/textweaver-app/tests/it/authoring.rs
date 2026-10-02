//! Phase 2 authoring and navigation (Agent P2b), driven through
//! `App::dispatch` without a terminal: structure while editing, the
//! outline, editing basics, citations, export and preview, spell checking,
//! tables, links and footnotes, run-time settings, notes, find and replace,
//! and templates.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use textweaver_app::a11y::{Announcer, Priority};
use textweaver_app::core::{CharPos, CharRange};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::Paths;
use textweaver_app::{App, AppConfig, Command, Confirm, Effect, Mode};

#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Said {
    fn all(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
    fn any(&self, needle: &str) -> bool {
        self.all().iter().any(|s| s.contains(needle))
    }
    fn clear(&self) {
        self.0.lock().unwrap().clear();
    }
}

impl Announcer for Said {
    fn announce(&mut self, text: &str, _priority: Priority) {
        self.0.lock().unwrap().push(text.to_owned());
    }
}

/// A silent app with persistence under a temporary folder, a file written
/// there, and a launcher that records what it was asked to open.
struct Rig {
    app: App,
    said: Said,
    opened: Arc<Mutex<Vec<String>>>,
    dir: PathBuf,
    paths: Paths,
    _tmp: tempfile::TempDir,
}

impl Rig {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_owned();
        let paths = Paths::under(&dir.join("home"));
        let said = Said::default();
        let mut app = App::new(AppConfig {
            paths: Some(paths.clone()),
            announcer: Box::new(said.clone()),
            ..AppConfig::for_tests()
        });
        let opened = Arc::new(Mutex::new(Vec::new()));
        let sink = opened.clone();
        app.set_launcher(Arc::new(move |target: &str| {
            sink.lock().unwrap().push(target.to_owned());
            Ok(())
        }));
        Rig {
            app,
            said,
            opened,
            dir,
            paths,
            _tmp: tmp,
        }
    }

    /// Writes `name` with `text` and opens it.
    fn open(&mut self, name: &str, text: &str) -> PathBuf {
        let path = self.dir.join(name);
        std::fs::write(&path, text).unwrap();
        self.app.open(&path).unwrap();
        path
    }

    /// Dispatches, then waits for the background writer (saves, bookmarks)
    /// and applies its results, as the event loop's next tick would.
    fn send(&mut self, cmd: Command) -> Vec<Effect> {
        let effects = self.app.dispatch(cmd);
        self.app.wait_for_writes();
        effects
    }

    fn act(&mut self, a: ActionId) -> Vec<Effect> {
        self.send(Command::Action(a))
    }

    fn status(&self) -> String {
        self.app.status_text().to_owned()
    }

    fn cursor(&self) -> CharPos {
        self.app.session().unwrap().cursor
    }

    /// The text from the cursor, `n` chars.
    fn at_cursor(&self, n: usize) -> String {
        let s = self.app.session().unwrap();
        s.doc
            .slice(CharRange::new(s.cursor, s.cursor.saturating_add(n)))
    }

    fn text(&self) -> String {
        self.app.session().unwrap().doc.text().to_string()
    }

    fn type_text(&mut self, text: &str) {
        for c in text.chars() {
            self.send(Command::Insert(c.to_string()));
        }
    }

    /// A tick after typing has paused.
    fn pause(&mut self) {
        self.app.tick(Instant::now() + Duration::from_millis(400));
    }
}

const ESSAY: &str = "# Essay\n\nIntro paragraph with **bold** words.\n\n## Methods\n\nWe measured [things](https://example.org) carefully.\n\n- first item\n- second item\n\n## Results\n\n| Name | Age |\n|---|---|\n| Ada | 36 |\n| Bob | 41 |\n";

// Structure while editing.

#[test]
fn heading_navigation_works_in_edit_mode_on_the_source() {
    let mut r = Rig::new();
    r.open("essay.md", ESSAY);
    r.act(ActionId::ToggleEditMode);
    assert_eq!(r.app.mode(), Mode::Edit);
    // Alt+H in the terminal: the next heading, at its text.
    r.act(ActionId::SkipNextHeading);
    assert_eq!(r.at_cursor(7), "Methods");
    assert!(r.status().contains("Methods"), "{}", r.status());
    r.act(ActionId::SkipNextHeading);
    assert_eq!(r.at_cursor(7), "Results");
    r.act(ActionId::SkipPreviousHeading);
    assert_eq!(r.at_cursor(7), "Methods");
    // The editor's caret followed.
    let head = r
        .app
        .edit_session()
        .unwrap()
        .editor()
        .unwrap()
        .selection()
        .head;
    assert_eq!(head, r.cursor());
    // Lists, links, and tables too.
    r.act(ActionId::NextLink);
    assert_eq!(r.at_cursor(6), "things");
    r.act(ActionId::NextListItem);
    assert_eq!(r.at_cursor(10), "first item");
    r.act(ActionId::NextTable);
    assert!(r.at_cursor(6).starts_with("| Name"), "{:?}", r.at_cursor(6));
    // Say position names the heading above, without its # marks.
    r.act(ActionId::SayPosition);
    assert!(
        r.status().contains("Under heading Results."),
        "{}",
        r.status()
    );
}

#[test]
fn typing_a_heading_is_found_after_a_pause() {
    let mut r = Rig::new();
    r.open("essay.md", ESSAY);
    r.act(ActionId::ToggleEditMode);
    r.act(ActionId::DocumentEnd);
    r.send(Command::MoveCaret {
        by: textweaver_app::CaretMove::DocumentEdge,
        direction: textweaver_app::core::Direction::Forward,
        extend: false,
    });
    r.type_text("\n## Discussion\n\nMore words.\n");
    r.pause();
    // From the top: Essay, Methods, Results, Discussion.
    r.act(ActionId::DocumentStart);
    for _ in 0..4 {
        r.act(ActionId::SkipNextHeading);
    }
    assert_eq!(r.at_cursor(10), "Discussion");
    // The outline has it too, as a level-2 heading.
    let effects = r.act(ActionId::Outline);
    let items = list_items(&effects);
    assert_eq!(
        items,
        [
            "Essay, level 1",
            "Methods, level 2",
            "Results, level 2",
            "Discussion, level 2"
        ]
    );
}

#[test]
fn positions_carry_into_the_source_and_back() {
    let mut r = Rig::new();
    r.open("essay.md", ESSAY);
    // Reading: the cursor on "carefully".
    let reading = r.text();
    let at = reading.find("carefully").unwrap();
    r.send(Command::GoTo(textweaver_app::text::GoTo::Char(CharPos(at))));
    assert_eq!(r.at_cursor(9), "carefully");
    r.act(ActionId::ToggleEditMode);
    assert_eq!(r.at_cursor(9), "carefully");
    // A bookmark in the table survives the round trip.
    r.act(ActionId::ToggleEditMode);
    assert_eq!(r.app.mode(), Mode::Browse);
    assert_eq!(r.at_cursor(9), "carefully");
    let t = r.text();
    let bob = t.find("Bob").unwrap();
    r.send(Command::GoTo(textweaver_app::text::GoTo::Char(CharPos(
        bob,
    ))));
    r.act(ActionId::AddBookmark);
    r.act(ActionId::ToggleEditMode);
    let b = r.app.bookmark_position(0).unwrap();
    let s = r.app.session().unwrap();
    assert_eq!(s.doc.slice(CharRange::new(b, b.saturating_add(3))), "Bob");
    // Edit above it, save, and leave: the bookmark stays on Bob.
    let intro = r.text().find("Intro").unwrap();
    r.send(Command::GoTo(textweaver_app::text::GoTo::Char(CharPos(
        intro,
    ))));
    r.type_text("Preface. ");
    r.act(ActionId::Save);
    r.act(ActionId::ToggleEditMode);
    assert_eq!(r.app.mode(), Mode::Browse);
    let b = r.app.bookmark_position(0).unwrap();
    let s = r.app.session().unwrap();
    assert_eq!(s.doc.slice(CharRange::new(b, b.saturating_add(3))), "Bob");
    assert!(
        s.doc
            .text()
            .to_string()
            .contains("Preface. Intro paragraph"),
        "{}",
        s.doc.text()
    );
}

/// The items of the list an effect shows.
fn list_items(effects: &[Effect]) -> Vec<String> {
    effects
        .iter()
        .find_map(|e| match e {
            Effect::ShowList { items, .. } => Some(items.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// The title of the list an effect shows.
fn list_title(effects: &[Effect]) -> String {
    effects
        .iter()
        .find_map(|e| match e {
            Effect::ShowList { title, .. } => Some(title.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

impl Rig {
    /// Moves the cursor to the first `needle` in the document.
    fn go(&mut self, needle: &str) {
        let t = self.text();
        let at = t
            .find(needle)
            .unwrap_or_else(|| panic!("{needle:?} in {t:?}"));
        let pos = CharPos(t[..at].chars().count());
        self.send(Command::GoTo(textweaver_app::text::GoTo::Char(pos)));
    }

    fn opened(&self) -> Vec<String> {
        self.opened.lock().unwrap().clone()
    }

    fn wait(&mut self) {
        assert!(self.app.wait_for_background(Duration::from_secs(60)));
    }

    fn fixtures() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/p")
    }

    /// Imports the citation fixtures into the user library.
    fn with_library(&mut self) {
        let bib = Self::fixtures().join("sample.bib");
        self.act(ActionId::ImportReferences);
        self.send(Command::Answer(bib.display().to_string()));
        assert!(self.said.any("Imported"), "{:?}", self.said.all());
    }
}

// The outline.

#[test]
fn the_outline_filters_as_you_type_and_jumps() {
    let mut r = Rig::new();
    r.open("essay.md", ESSAY);
    r.go("carefully");
    let effects = r.act(ActionId::Outline);
    assert_eq!(list_title(&effects), "Outline, 3 headings");
    assert!(r.said.any("You are under Methods."), "{:?}", r.said.all());
    assert_eq!(r.app.list_filter(), Some(""));
    let effects = r.send(Command::FilterList("res".into()));
    assert_eq!(list_items(&effects), ["Results, level 2"]);
    assert_eq!(list_title(&effects), "Outline, 1 of 3 match res");
    assert_eq!(r.app.list_filter(), Some("res"));
    let effects = r.send(Command::FilterList("zzz".into()));
    assert!(list_items(&effects).is_empty());
    assert!(
        r.status().contains("No headings match zzz"),
        "{}",
        r.status()
    );
    r.send(Command::FilterList("res".into()));
    r.send(Command::Choose(0));
    assert_eq!(r.at_cursor(7), "Results");
    assert!(r.status().contains("Results"), "{}", r.status());
    assert_eq!(r.app.list_filter(), None);
    // A jump: Back returns.
    r.act(ActionId::HistoryBack);
    assert_eq!(r.at_cursor(9), "carefully");
    // Escape closes it.
    r.act(ActionId::Outline);
    r.send(Command::Cancel);
    assert_eq!(r.app.list_filter(), None);
}

#[test]
fn a_document_without_headings_says_so() {
    let mut r = Rig::new();
    r.open("plain.txt", "Just words.\n");
    let effects = r.act(ActionId::Outline);
    assert!(list_items(&effects).is_empty());
    assert_eq!(r.status(), "This document has no headings.");
}

// Editing basics.

#[test]
fn select_all_delete_words_copy_and_paste() {
    let mut r = Rig::new();
    r.open("words.md", "one two three\n");
    r.act(ActionId::ToggleEditMode);
    r.act(ActionId::SelectAll);
    let sel = r.app.edit_session().unwrap().editor().unwrap().selection();
    assert_eq!(sel.range(), CharRange::new(0, 14));
    assert_eq!(r.status(), "Selected all, 3 words.");
    // Word before the caret, from the end of "three" (Left collapses the
    // selection to its start, End goes to the end of the line).
    for (by, direction) in [
        (
            textweaver_app::CaretMove::Char,
            textweaver_app::core::Direction::Backward,
        ),
        (
            textweaver_app::CaretMove::LineEdge,
            textweaver_app::core::Direction::Forward,
        ),
    ] {
        r.send(Command::MoveCaret {
            by,
            direction,
            extend: false,
        });
    }
    r.act(ActionId::DeleteWordBefore);
    assert_eq!(r.text(), "one two \n");
    assert_eq!(r.status(), "three deleted.");
    // Word after the caret, from the start: "one " goes.
    r.act(ActionId::DocumentStart);
    r.act(ActionId::DeleteWordAfter);
    assert_eq!(r.text(), "two \n");
    // Each is one undo step.
    r.act(ActionId::Undo);
    assert_eq!(r.text(), "one two \n");
    // Copy a word, paste it at the end.
    r.send(Command::Select(CharRange::new(4, 7)));
    r.act(ActionId::Copy);
    assert_eq!(r.app.take_clipboard().as_deref(), Some("two"));
    r.send(Command::MoveCaret {
        by: textweaver_app::CaretMove::LineEdge,
        direction: textweaver_app::core::Direction::Forward,
        extend: false,
    });
    r.act(ActionId::Paste);
    assert_eq!(r.text(), "one two two\n");
    // Outside edit mode Paste says how to start editing.
    r.act(ActionId::Save);
    r.act(ActionId::ToggleEditMode);
    r.act(ActionId::Paste);
    assert!(
        r.status().starts_with("Turn on edit mode"),
        "{}",
        r.status()
    );
}

// Citations.

#[test]
fn citations_are_picked_inserted_checked_and_listed() {
    let mut r = Rig::new();
    r.open("paper.md", "# Paper\n\nAs shown.\n\n## References\n\n");
    r.with_library();
    r.act(ActionId::ToggleEditMode);
    r.go(".\n\n##");
    let effects = r.act(ActionId::InsertCitation);
    assert!(
        list_title(&effects).starts_with("Insert citation, "),
        "{effects:?}"
    );
    assert!(r.app.list_filter().is_some());
    let effects = r.send(Command::FilterList("thermometry".into()));
    let items = list_items(&effects);
    assert_eq!(items.len(), 1, "{items:?}");
    assert!(items[0].contains("Key kucsko2013"), "{items:?}");
    let effects = r.send(Command::Choose(0));
    assert!(matches!(
        effects.first(),
        Some(Effect::Prompt {
            purpose: textweaver_app::PromptPurpose::CitationLocator,
            ..
        })
    ));
    // A locator it cannot read asks again.
    let effects = r.send(Command::Answer("the intro".into()));
    assert!(r.said.any("Could not read the locator the intro"));
    assert!(matches!(effects.first(), Some(Effect::Prompt { .. })));
    r.send(Command::Answer("54".into()));
    assert!(
        r.text().contains("As shown [@kucsko2013, p. 54]."),
        "{}",
        r.text()
    );
    assert!(
        r.status().starts_with("Inserted citation of Kucsko"),
        "{}",
        r.status()
    );
    assert!(r.status().ends_with("page 54."), "{}", r.status());
    // Inside that citation a second key joins it.
    r.go("kucsko2013");
    r.act(ActionId::InsertCitation);
    r.send(Command::FilterList("fox".into()));
    r.send(Command::Choose(0));
    r.send(Command::Answer(String::new()));
    assert!(
        r.text().contains("[@kucsko2013, p. 54; @dahl1988]"),
        "{}",
        r.text()
    );
    // Check citations.
    r.act(ActionId::CheckCitations);
    assert_eq!(r.status(), "1 citation found. Every key is in the library.");
    // The bibliography, at the caret under References.
    r.act(ActionId::DocumentEnd);
    r.send(Command::MoveCaret {
        by: textweaver_app::CaretMove::DocumentEdge,
        direction: textweaver_app::core::Direction::Forward,
        extend: false,
    });
    r.act(ActionId::InsertBibliography);
    assert!(
        r.status()
            .starts_with("Inserted the bibliography, 2 entries, apa style."),
        "{}",
        r.status()
    );
    let text = r.text();
    assert!(text.contains("Dahl, R. (1988)"), "{text}");
    assert!(text.contains("Kucsko, G."), "{text}");
    // Reading: a word move onto the citation says it in words.
    r.act(ActionId::Save);
    r.act(ActionId::ToggleEditMode);
    r.go("As shown");
    r.act(ActionId::CaretNextWord);
    r.act(ActionId::CaretNextWord);
    assert!(r.status().starts_with("Citation: Kucsko"), "{}", r.status());
    assert!(r.status().contains("page 54"), "{}", r.status());
    assert!(r.status().contains("Dahl, 1988"), "{}", r.status());
}

#[test]
fn a_reference_is_added_by_doi_off_the_ui_thread() {
    let mut r = Rig::new();
    r.open("paper.md", "# Paper\n");
    let body =
        std::fs::read_to_string(Rig::fixtures().join("recorded/doi-nature12373.json")).unwrap();
    r.app.set_citation_client(Arc::new(move || {
        Box::new(textweaver_app::cite::RecordedClient::new().with(
            "https://doi.org/10.1038/nature12373",
            200,
            &body,
        ))
    }));
    let effects = r.act(ActionId::AddReference);
    assert!(matches!(
        effects.first(),
        Some(Effect::Prompt {
            purpose: textweaver_app::PromptPurpose::ReferenceIdentifier,
            ..
        })
    ));
    r.send(Command::Answer(
        "https://doi.org/10.1038/nature12373".into(),
    ));
    assert!(r.status().starts_with("Looking up"), "{}", r.status());
    r.wait();
    assert!(r.status().starts_with("Added reference "), "{}", r.status());
    let lib = std::fs::read_to_string(r.paths.data_dir.join("references.json")).unwrap();
    assert!(lib.contains("Nanometre-scale thermometry"), "{lib}");
    // A malformed identifier is refused at once.
    r.act(ActionId::AddReference);
    r.send(Command::Answer("not an id".into()));
    assert!(r.app.wait_for_background(Duration::from_secs(1)));
}

// Export and preview.

/// Wave 5, W5x: `y` and `n` answer "Open it? y or n" while a list is
/// shown, through the app's list model (the window and JSON-RPC send list
/// keys there), as the save list's letters choose at once. The list stays
/// open, and other letters still move in it.
#[test]
fn y_and_n_answer_open_it_while_a_list_is_shown() {
    let mut r = Rig::new();
    let path = r.open("essay.md", ESSAY);
    r.act(ActionId::ExportHtml);
    r.wait();
    assert!(r.app.confirmation_pending());
    r.act(ActionId::Outline);
    assert!(r.app.list_model().is_some());
    r.send(Command::ListKey(textweaver_app::ListKey::Char('n')));
    assert!(!r.app.confirmation_pending());
    assert!(r.opened().is_empty());
    assert!(r.app.list_model().is_some(), "the list stays open");
    r.send(Command::ListKey(textweaver_app::ListKey::Escape));

    r.act(ActionId::ExportHtml);
    r.wait();
    r.act(ActionId::Outline);
    // Another letter moves in the list; the question still waits.
    r.send(Command::ListKey(textweaver_app::ListKey::Char('m')));
    assert!(r.app.confirmation_pending());
    r.send(Command::ListKey(textweaver_app::ListKey::Char('Y')));
    assert!(!r.app.confirmation_pending());
    assert_eq!(
        r.opened(),
        [path.with_extension("html").display().to_string()]
    );
    assert!(r.app.list_model().is_some(), "the list stays open");
}

#[test]
fn exports_go_next_to_the_document_and_offer_to_open() {
    let mut r = Rig::new();
    let path = r.open("essay.md", ESSAY);
    for (action, ext) in [
        (ActionId::ExportHtml, "html"),
        (ActionId::ExportDocx, "docx"),
        (ActionId::ExportEpub, "epub"),
        (ActionId::ExportBrf, "brf"),
    ] {
        r.act(action);
        r.wait();
        let out = path.with_extension(ext);
        assert!(out.is_file(), "{}", out.display());
        // The question before the folder, for a 40-cell Braille display
        // (Wave 5, the Braille pass).
        assert!(
            r.status()
                .starts_with(&format!("Exported essay.{ext}. Open it? y or n. ")),
            "{}",
            r.status()
        );
        assert!(r.app.confirmation_pending());
        r.send(Command::Confirm(Confirm::No));
    }
    // Yes opens it with the default program.
    r.act(ActionId::ExportHtml);
    r.wait();
    r.send(Command::Confirm(Confirm::Yes));
    assert_eq!(
        r.opened(),
        [path.with_extension("html").display().to_string()]
    );
    // In edit mode the live text is exported, saved or not.
    r.act(ActionId::ToggleEditMode);
    r.go("Intro");
    r.type_text("Unsaved words. ");
    r.act(ActionId::ExportHtml);
    r.wait();
    r.send(Command::Confirm(Confirm::No));
    let html = std::fs::read_to_string(path.with_extension("html")).unwrap();
    assert!(html.contains("Unsaved words."), "{html}");
    assert!(r.app.is_dirty());
}

#[test]
fn preview_opens_the_browser_and_saving_rewrites_it() {
    let mut r = Rig::new();
    r.open(
        "math.md",
        "# Math\n\nThe area is $\\pi r^2$.\n\n![A cat](cat.png)\n",
    );
    r.act(ActionId::PreviewInBrowser);
    r.wait();
    let opened = r.opened();
    assert_eq!(opened.len(), 1, "{opened:?}");
    assert!(opened[0].ends_with("math.html"), "{opened:?}");
    let page = std::fs::read_to_string(&opened[0]).unwrap();
    assert!(page.contains("<math"), "MathML: {page}");
    assert!(page.contains("<base href=\"file://"), "{page}");
    // Saving in edit mode writes it again, without opening another tab.
    r.act(ActionId::ToggleEditMode);
    r.go("The area");
    r.type_text("Note: ");
    r.act(ActionId::Save);
    r.wait();
    assert_eq!(r.opened().len(), 1);
    let page = std::fs::read_to_string(&opened[0]).unwrap();
    assert!(page.contains("Note: The area"), "{page}");
    // The owner's decision: say it, and do not reload by itself.
    assert!(
        r.said.any("Preview updated. Press F5 in the browser."),
        "{:?}",
        r.said.all()
    );
}

/// `[preview] auto_reload`: the page comes from a server on 127.0.0.1 with
/// a secret path, and a save sends a reload naming the heading nearest the
/// caret; turning it off stops the server.
#[test]
fn preview_auto_reload_serves_the_page_and_reloads_after_saves() {
    use std::io::{BufRead, BufReader, Read, Write};
    let mut r = Rig::new();
    r.open(
        "live.md",
        "# Intro\n\nFirst part.\n\n## Methods\n\nWe measured.\n\n## Results\n\nIt worked.\n",
    );
    r.act(ActionId::TogglePreviewAutoReload);
    assert!(r.app.settings().preview.auto_reload);
    assert!(
        r.status().starts_with("Automatic preview reloading on"),
        "{}",
        r.status()
    );
    r.act(ActionId::PreviewInBrowser);
    r.wait();
    let opened = r.opened();
    assert_eq!(opened.len(), 1, "{opened:?}");
    let url = opened[0].clone();
    assert!(url.starts_with("http://127.0.0.1:"), "{url}");
    let rest = url.trim_start_matches("http://");
    let (host, path) = rest.split_once('/').unwrap();
    let addr: std::net::SocketAddr = host.parse().unwrap();
    let get = |p: &str| {
        let mut c = std::net::TcpStream::connect(addr).unwrap();
        c.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        write!(c, "GET {p} HTTP/1.1\r\n\r\n").unwrap();
        let mut out = String::new();
        let _ = c.read_to_string(&mut out);
        out
    };
    let page = get(&format!("/{path}"));
    assert!(page.contains("<h2 id=\"methods\""), "{page}");
    assert!(get("/nottheright/").starts_with("HTTP/1.1 404"));
    // A page listens for reloads.
    let mut events = std::net::TcpStream::connect(addr).unwrap();
    events
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    write!(events, "GET /{path}events HTTP/1.1\r\n\r\n").unwrap();
    let mut reader = BufReader::new(events);
    let mut line = String::new();
    while !line.starts_with("retry:") {
        line.clear();
        reader.read_line(&mut line).unwrap();
    }
    // Edit under Methods and save: the page is told to reload there.
    r.act(ActionId::ToggleEditMode);
    r.go("We measured");
    r.type_text("Then ");
    r.act(ActionId::Save);
    r.wait();
    let mut event = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).unwrap() == 0 {
            break;
        }
        if line.trim().is_empty() {
            if event.is_empty() {
                continue;
            }
            break;
        }
        event.push_str(&line);
    }
    assert_eq!(
        event,
        "event: reload\ndata: methods\n",
        "{:?}",
        r.said.all()
    );
    assert!(r.said.any("Preview updated."));
    assert!(!r.said.any("Press F5"), "{:?}", r.said.all());
    // Off: the server stops.
    r.act(ActionId::TogglePreviewAutoReload);
    assert!(!r.app.settings().preview.auto_reload);
    let refused = std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(300))
        .and_then(|mut c| {
            write!(c, "GET /{path} HTTP/1.1\r\n\r\n")?;
            c.set_read_timeout(Some(Duration::from_millis(500)))?;
            let mut b = [0u8; 1];
            c.read(&mut b)
        })
        .map_or(true, |n| n == 0);
    assert!(refused, "the server still answers");
}

// Spelling.

#[test]
fn misspellings_are_found_spelled_suggested_and_learned() {
    let mut r = Rig::new();
    r.open(
        "spell.md",
        "This is a tset of the sytem. `fnord` and https://exmaple.org stay.\n\nQwertyson wrote it.\n",
    );
    r.act(ActionId::ToggleEditMode);
    r.act(ActionId::DocumentStart);
    r.act(ActionId::NextMisspelling);
    assert_eq!(r.status(), "tset. t s e t.");
    let sel = r.app.edit_session().unwrap().editor().unwrap().selection();
    assert_eq!(
        r.text()
            .chars()
            .skip(sel.range().start.0)
            .take(4)
            .collect::<String>(),
        "tset"
    );
    let effects = r.act(ActionId::SpellingSuggestions);
    let items = list_items(&effects);
    assert!(items.contains(&"test".to_owned()), "{items:?}");
    assert_eq!(items.last().map(String::as_str), Some("Leave it as it is"));
    let i = items.iter().position(|x| x == "test").unwrap();
    r.send(Command::Choose(i));
    assert!(
        r.text().starts_with("This is a test of the sytem."),
        "{}",
        r.text()
    );
    r.act(ActionId::NextMisspelling);
    assert_eq!(r.status(), "sytem. s y t e m.");
    r.act(ActionId::NextMisspelling);
    assert!(r.status().starts_with("Qwertyson."), "{}", r.status());
    // Add it to the word list: it is not found again, and the list is saved.
    let effects = r.act(ActionId::SpellingSuggestions);
    let items = list_items(&effects);
    let add = items
        .iter()
        .position(|x| x.starts_with("Add Qwertyson"))
        .unwrap();
    r.send(Command::Choose(add));
    let words = std::fs::read_to_string(r.paths.data_dir.join("words.txt")).unwrap();
    assert_eq!(words, "qwertyson\n");
    r.act(ActionId::PreviousMisspelling);
    assert_eq!(r.status(), "sytem. s y t e m.");
    // Saving counts what is left, on a helper thread (Wave 3).
    r.act(ActionId::Save);
    assert!(
        r.app
            .wait_for_spell_count(std::time::Duration::from_secs(30))
    );
    assert!(r.said.any("1 possible misspelling."), "{:?}", r.said.all());
    // Reading mode finds them too, and skips the code span.
    r.act(ActionId::ToggleEditMode);
    r.act(ActionId::DocumentStart);
    r.act(ActionId::NextMisspelling);
    assert_eq!(r.status(), "sytem. s y t e m.");
    r.act(ActionId::NextMisspelling);
    assert!(
        r.status().starts_with("No more misspelling"),
        "{}",
        r.status()
    );
}

// Tables.

#[test]
fn tables_move_by_row_and_cell_with_their_headers() {
    let mut r = Rig::new();
    r.open("essay.md", ESSAY);
    r.act(ActionId::NextTable);
    r.act(ActionId::TableNextRow);
    assert_eq!(r.status(), "Row 2, Name: Ada");
    r.act(ActionId::TableNextColumn);
    assert_eq!(r.status(), "Age: 36");
    r.act(ActionId::TableNextColumn);
    assert_eq!(r.status(), "End of row.");
    r.act(ActionId::TableNextRow);
    assert_eq!(r.status(), "Row 3, Age: 41");
    r.act(ActionId::TableNextRow);
    assert_eq!(r.status(), "End of table.");
    r.act(ActionId::SayPosition);
    assert!(
        r.status().contains("Table, row 3 of 3, column 2 of 2."),
        "{}",
        r.status()
    );
    r.act(ActionId::TablePreviousRow);
    r.act(ActionId::TablePreviousRow);
    assert_eq!(r.status(), "Header row, Age");
    r.act(ActionId::TablePreviousColumn);
    assert_eq!(r.status(), "Name");
    // The same keys work on the source table while editing.
    r.act(ActionId::ToggleEditMode);
    r.go("| Ada");
    r.act(ActionId::TableNextColumn);
    assert_eq!(r.status(), "Age: 36");
    r.act(ActionId::DocumentStart);
    r.act(ActionId::TableNextRow);
    assert_eq!(r.status(), "Not in a table.");
}

// Links and footnotes.

#[test]
fn links_open_local_files_and_come_back() {
    let mut r = Rig::new();
    std::fs::write(
        r.dir.join("b.md"),
        "# B\n\nFirst.\n\n## Part two\n\nSecond part.\n",
    )
    .unwrap();
    r.open(
        "a.md",
        "# A\n\nSee [the other](b.md#part-two) and [up](#a) and [web](https://example.org) and [[b]].\n",
    );
    r.go("the other");
    r.act(ActionId::FollowLink);
    assert!(
        r.status().starts_with("Followed the link to b.md."),
        "{}",
        r.status()
    );
    assert_eq!(r.app.session().unwrap().title, "B");
    assert_eq!(r.at_cursor(8), "Part two");
    r.act(ActionId::HistoryBack);
    assert!(r.status().starts_with("Back in a.md"), "{}", r.status());
    assert_eq!(r.at_cursor(9), "the other");
    // A link to a heading here.
    r.go("up");
    r.act(ActionId::FollowLink);
    assert_eq!(r.at_cursor(1), "A");
    // A web link asks first.
    r.go("web");
    r.act(ActionId::FollowLink);
    assert_eq!(r.status(), "Open web link? y or n. https://example.org");
    r.send(Command::Confirm(Confirm::Yes));
    assert_eq!(r.opened(), ["https://example.org"]);
    // A wiki link finds b.md by name.
    r.go("b.");
    r.act(ActionId::FollowLink);
    assert_eq!(r.app.session().unwrap().title, "B");
    // Nothing here.
    r.act(ActionId::DocumentStart);
    r.act(ActionId::FollowLink);
    assert_eq!(r.status(), "No link or footnote at the cursor.");
}

/// Opening links safely (W8a, the R4 review's first finding): a web
/// address with `&` in it reaches the opener exactly as written (no shell
/// reads it), while a link with another scheme (`ms-msdt:`) is refused in
/// words, without the question, and never reaches the opener.
#[test]
fn links_open_only_web_and_mail_addresses_without_a_shell() {
    let mut r = Rig::new();
    r.open(
        "a.md",
        "# A\n\nSee [plain](https://example.org/page) and [amp](https://example.org/?q=1&calc) and [msdt](ms-msdt:-id) and [js](javascript:alert) here.\n",
    );
    r.go("plain");
    r.act(ActionId::FollowLink);
    r.send(Command::Confirm(Confirm::Yes));
    r.go("amp");
    r.act(ActionId::FollowLink);
    assert_eq!(
        r.status(),
        "Open web link? y or n. https://example.org/?q=1&calc"
    );
    r.send(Command::Confirm(Confirm::Yes));
    assert_eq!(
        r.opened(),
        ["https://example.org/page", "https://example.org/?q=1&calc"]
    );
    for (word, scheme) in [("msdt", "ms-msdt"), ("js", "javascript")] {
        r.said.clear();
        r.go(word);
        r.act(ActionId::FollowLink);
        let said = format!("Not opened: {scheme} link blocked.");
        assert!(r.said.any(&said), "{:?}", r.said.all());
        assert!(said.chars().count() <= 40, "{said}");
        assert!(!r.app.confirmation_pending(), "no question for {scheme}");
    }
    assert_eq!(r.opened().len(), 2, "nothing else reached the opener");
}

#[test]
fn footnotes_go_to_their_note_and_back() {
    let mut r = Rig::new();
    // Read in place (the default), a footnote is said where it is.
    r.open(
        "inline.md",
        "Text.[^1] More words.\n\n[^1]: The note itself.\n",
    );
    r.go("(footnote");
    r.act(ActionId::FollowLink);
    assert_eq!(r.status(), "Footnote 1: (footnote: The note itself.)");
    // Deferred to the end, the reference goes to the note and back.
    r.app
        .update_settings(|s| {
            s.normalization.footnote_mode = textweaver_app::store::FootnoteMode::Deferred;
        })
        .unwrap();
    r.open(
        "notes.md",
        "Text.[^1] More words.\n\n[^1]: The note itself.\n",
    );
    r.go("[1]");
    r.act(ActionId::FollowLink);
    assert!(
        r.status().contains("Footnote 1: [1] The note itself."),
        "{}",
        r.status()
    );
    r.act(ActionId::FollowLink);
    assert!(
        r.status().starts_with("Back to footnote reference 1"),
        "{}",
        r.status()
    );
    assert_eq!(r.at_cursor(3), "[1]");
    // In the source too.
    r.act(ActionId::ToggleEditMode);
    r.go("[^1] More");
    r.act(ActionId::FollowLink);
    assert_eq!(r.at_cursor(5), "[^1]:");
    r.act(ActionId::FollowLink);
    assert_eq!(r.at_cursor(9), "[^1] More");
}

// Settings at run time.

#[test]
fn verbosity_and_punctuation_cycle_and_are_saved() {
    let mut r = Rig::new();
    r.open("a.md", "# A\n");
    r.act(ActionId::CycleVerbosity);
    assert_eq!(r.status(), "Verbosity: high.");
    r.act(ActionId::CycleVerbosity);
    assert_eq!(r.status(), "Verbosity: low.");
    r.act(ActionId::CyclePunctuation);
    assert_eq!(r.status(), "Punctuation: all.");
    let saved = std::fs::read_to_string(r.paths.settings_file()).unwrap();
    assert!(saved.contains("verbosity = \"low\""), "{saved}");
    assert!(saved.contains("punctuation = \"all\""), "{saved}");
}

// Notes.

#[test]
fn notes_export_as_a_study_sheet_grouped_by_heading() {
    let mut r = Rig::new();
    let path = r.open("essay.md", ESSAY);
    r.go("We measured");
    r.act(ActionId::AddNote);
    r.send(Command::Answer("Check the method #exam".into()));
    r.go("Intro");
    r.act(ActionId::HighlightSelection);
    r.act(ActionId::ExportStudySheet);
    let out = path.with_file_name("essay-study-sheet.md");
    let sheet = std::fs::read_to_string(&out).unwrap();
    assert!(sheet.starts_with("# Study sheet: Essay\n"), "{sheet}");
    let essay = sheet.find("## Essay").unwrap();
    let methods = sheet.find("### Methods").unwrap();
    assert!(essay < methods, "{sheet}");
    assert!(
        sheet.contains("- > Intro paragraph with bold words."),
        "{sheet}"
    );
    assert!(
        sheet
            .contains("- > We measured things carefully.\n\n  Check the method #exam (tags: exam)"),
        "{sheet}"
    );
    assert!(
        r.status()
            .contains("Study sheet with 1 note and 1 highlight"),
        "{}",
        r.status()
    );
    r.send(Command::Confirm(Confirm::Yes));
    assert_eq!(r.opened(), [out.display().to_string()]);
}

#[test]
fn a_note_is_signalled_while_reading_and_on_word_moves() {
    let tmp = tempfile::tempdir().unwrap();
    let said = Said::default();
    let (speech, _log) = textweaver_app::testing::recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        announcer: Box::new(said.clone()),
        self_voicing: true,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    app.open_document(
        textweaver_app::text::Document::from_plain_text(
            "First sentence here. Second one has a note. Third.",
        ),
        textweaver_app::store::DocKey::untitled(1),
        "T".into(),
    );
    app.dispatch(Command::GoTo(textweaver_app::text::GoTo::Char(CharPos(21))));
    app.dispatch(Command::Action(ActionId::AddNote));
    app.dispatch(Command::Answer("Remember this".into()));
    app.dispatch(Command::Action(ActionId::DocumentStart));
    said.clear();
    app.dispatch(Command::Action(ActionId::ReadFromCursor));
    let deadline = Instant::now() + Duration::from_secs(10);
    while !said.any("Note: Remember this") && Instant::now() < deadline {
        app.wait_for_speech_thread();
        app.poll_speech();
    }
    assert!(said.any("Note: Remember this"), "{:?}", said.all());
    // Once per note.
    let n = said
        .all()
        .iter()
        .filter(|s| s.contains("Note: Remember"))
        .count();
    assert_eq!(n, 1);
    // A word move into the passage says so.
    app.dispatch(Command::Action(ActionId::Stop));
    app.dispatch(Command::Action(ActionId::DocumentStart));
    for _ in 0..3 {
        app.dispatch(Command::Action(ActionId::CaretNextWord));
    }
    assert_eq!(app.status_text(), "Second. Has a note: Remember this");
    app.dispatch(Command::Action(ActionId::CaretNextWord));
    assert_eq!(app.status_text(), "one");
    drop(tmp);
}

// Listening to the rendered text.

#[test]
fn listening_to_the_rendered_text_reads_no_markup_and_highlights_the_source() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("r.md");
    std::fs::write(&file, "# Title\n\nSome **bold** and [a link](x.md) here.\n").unwrap();
    let (speech, log) = textweaver_app::testing::recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        paths: Some(Paths::under(&tmp.path().join("home"))),
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    app.open(&file).unwrap();
    app.dispatch(Command::Action(ActionId::ToggleEditMode));
    app.dispatch(Command::Action(ActionId::DocumentStart));
    app.dispatch(Command::Action(ActionId::ListenRendered));
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        app.wait_for_speech_thread();
        app.poll_speech();
        if app.playback() == textweaver_app::Playback::Idle || Instant::now() > deadline {
            break;
        }
    }
    let texts = log.texts().join(" ");
    assert!(texts.contains("Some bold and a link here"), "{texts}");
    assert!(!texts.contains("**") && !texts.contains("x.md"), "{texts}");
    // The highlight followed in the source: every range is on words there.
    let s = app.session().unwrap();
    let words: Vec<String> = app.spoken_log().iter().map(|r| s.doc.slice(*r)).collect();
    assert!(words.contains(&"bold".to_owned()), "{words:?}");
    assert!(words.contains(&"link".to_owned()), "{words:?}");
    assert_eq!(app.mode(), Mode::Edit);
}

// Find and replace, one at a time.

#[test]
fn replace_one_at_a_time_with_skip_case_and_all() {
    let mut r = Rig::new();
    r.open("cats.md", "cat Cat cat cat dog\n");
    r.act(ActionId::ToggleEditMode);
    r.act(ActionId::Replace);
    r.send(Command::Answer("cat".into()));
    let effects = r.send(Command::Answer("bird".into()));
    assert_eq!(
        list_title(&effects),
        "Match 1 of 4, line 1: cat Cat cat cat dog"
    );
    assert_eq!(
        list_items(&effects),
        [
            "Replace this one",
            "Skip this one",
            "Replace all the rest",
            "Match case: off",
            "Whole words only: off"
        ]
    );
    // r: replace this one.
    r.send(Command::Choose(0));
    assert_eq!(r.text(), "bird Cat cat cat dog\n");
    // c: match case; "Cat" no longer matches.
    let effects = r.send(Command::Choose(3));
    assert!(
        r.said.any("Match case on. 2 matches."),
        "{:?}",
        r.said.all()
    );
    assert!(
        list_title(&effects).starts_with("Match 1 of 2"),
        "{effects:?}"
    );
    // s: skip, then a: all the rest.
    r.send(Command::Choose(1));
    r.send(Command::Choose(2));
    assert_eq!(r.text(), "bird Cat cat bird dog\n");
    assert_eq!(r.status(), "Replaced 2, skipped 1.");
    // Undo takes back the last step only.
    r.act(ActionId::Undo);
    assert_eq!(r.text(), "bird Cat cat cat dog\n");
    // Escape stops with the counts.
    r.act(ActionId::Replace);
    r.send(Command::Answer("dog".into()));
    r.send(Command::Answer("cow".into()));
    r.send(Command::Cancel);
    assert_eq!(r.status(), "Stopped. Replaced 0, skipped 0.");
}

// Templates.

#[test]
fn a_new_document_from_a_template_has_front_matter_and_references() {
    let mut r = Rig::new();
    let dir = r.paths.config_dir.join("templates");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("Lab report.md"),
        "# {{title}}\n\nBy {{author}}, {{date}}.\n",
    )
    .unwrap();
    let effects = r.act(ActionId::NewFromTemplate);
    assert_eq!(
        list_items(&effects),
        ["Essay", "Report", "Notes", "Lab report, your template"]
    );
    let effects = r.send(Command::Choose(0));
    assert!(matches!(
        effects.first(),
        Some(Effect::Prompt {
            purpose: textweaver_app::PromptPurpose::TemplateTitle,
            ..
        })
    ));
    r.send(Command::Answer("On Bees".into()));
    assert_eq!(r.app.mode(), Mode::Edit);
    assert!(r.app.is_dirty());
    let date = textweaver_app::local_date();
    let text = r.text();
    assert!(
        text.starts_with(&format!(
            "---\ntitle: \"On Bees\"\nauthor: \"\"\ndate: {date}\n---\n\n# On Bees\n\n## Introduction\n"
        )),
        "{text}"
    );
    assert!(text.ends_with("## References\n"), "{text}");
    assert!(
        r.status().contains(&format!("Dated {date}.")),
        "{}",
        r.status()
    );
    // The caret is where the writing starts: under Introduction.
    let head = r
        .app
        .edit_session()
        .unwrap()
        .editor()
        .unwrap()
        .selection()
        .head;
    let before: String = text.chars().take(head.0).collect();
    assert!(before.ends_with("## Introduction\n\n"), "{before:?}");
    // The outline sees its headings at once.
    let effects = r.act(ActionId::Outline);
    assert_eq!(list_items(&effects).len(), 5);
    // A user template, while editing: asks about the unsaved one first.
    r.send(Command::Cancel);
    let effects = r.act(ActionId::NewFromTemplate);
    assert!(
        list_title(&effects).starts_with("Save changes to"),
        "{effects:?}"
    );
    r.send(Command::Choose(1));
    let effects = r.send(Command::Choose(3));
    assert!(matches!(effects.first(), Some(Effect::Prompt { .. })));
    r.send(Command::Answer(String::new()));
    assert!(
        r.text().starts_with(&format!("# Untitled\n\nBy , {date}.")),
        "{}",
        r.text()
    );
    let _ = Path::new("");
}

// Large documents: the structure comes from a background parse at open,
// typing drops the markers, and commands that need them parse again.

#[test]
fn large_markdown_is_parsed_in_the_background_and_after_typing() {
    let mut r = Rig::new();
    let mut text = String::new();
    let mut n = 0;
    while text.len() < 400 * 1024 {
        n += 1;
        text.push_str(&format!(
            "## Section {n}\n\nSome *words* here and [a link](x{n}.md) in paragraph {n}.\n\n"
        ));
    }
    r.open("big.md", &text);
    r.go("Section 500\n");
    r.act(ActionId::ToggleEditMode);
    assert_eq!(r.at_cursor(11), "Section 500");
    r.act(ActionId::SkipNextHeading);
    assert_eq!(r.at_cursor(11), "Section 501");
    // Typing a heading: found at once by the next heading command.
    r.send(Command::MoveCaret {
        by: textweaver_app::CaretMove::DocumentEdge,
        direction: textweaver_app::core::Direction::Backward,
        extend: false,
    });
    r.type_text("## Preface\n\n");
    r.act(ActionId::DocumentStart);
    r.act(ActionId::SkipNextHeading);
    assert_eq!(r.at_cursor(7), "Preface");
    r.act(ActionId::SkipNextHeading);
    assert_eq!(r.at_cursor(9), "Section 1");
    // After a pause the background parse lands; the outline has the edit.
    r.type_text("New ");
    r.pause();
    assert!(r.app.wait_for_structure(Duration::from_secs(30)));
    let effects = r.act(ActionId::Outline);
    let items = list_items(&effects);
    assert_eq!(items[..2], ["Preface, level 2", "New Section 1, level 2"]);
}
