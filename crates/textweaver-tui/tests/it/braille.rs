//! The 40-cell test (Wave 5, W5x, the Braille pass): every status line,
//! title line, list line, and prompt line a reader meets on the way
//! through a document is read as a 40-cell Braille line, the owner's
//! HumanWare Mantis Q40, and its key fact must sit in the first 40 cells.
//!
//! The reader runs in screen-reader mode with `[accessibility] cursor =
//! "status"`, as the owner reads with the display, on an 80-column
//! terminal. The prompt labels of all six languages are checked in the
//! reader's own unit tests (`ui.rs`).

use std::path::{Path, PathBuf};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::text::Span;
use textweaver_app::core::{CharPos, CharRange, MarkerKind};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::{AccessMode as ModeSetting, CursorPlacement, DocKey, Settings};
use textweaver_app::testing::recording_service;
use textweaver_app::text::{Document, DocumentMeta, GoTo, Marker};
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig, Command};
use textweaver_tui::Tui;
use textweaver_tui::ui::BRAILLE_CELLS;

const WIDTH: u16 = 80;
const HEIGHT: u16 = 24;

fn fixture(parts: &[&str]) -> PathBuf {
    let mut p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    for part in parts {
        p = p.join(part);
    }
    p
}

struct Harness {
    tui: Tui,
    term: Terminal<TestBackend>,
    /// Every line checked, as "what: line", for the failure message.
    checked: Vec<String>,
}

fn launch() -> Harness {
    let (speech, _log) = recording_service().unwrap();
    let mut settings = Settings::default();
    settings.accessibility.mode = ModeSetting::ScreenReader;
    settings.accessibility.cursor = CursorPlacement::Status;
    let app = App::new(AppConfig {
        settings,
        speech,
        self_voicing: true,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    let mut h = Harness {
        tui: Tui::with_color_support(app, ColorSupport::NoColor),
        term: Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap(),
        checked: Vec::new(),
    };
    h.draw();
    h
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

impl Harness {
    fn draw(&mut self) {
        let tui = &mut self.tui;
        self.term.draw(|f| tui.draw(f)).unwrap();
    }

    fn press(&mut self, k: KeyEvent) {
        self.tui.handle_key(k);
        self.draw();
    }

    fn act(&mut self, a: ActionId) {
        self.tui.dispatch(Command::Action(a));
        self.draw();
    }

    fn typed(&mut self, s: &str) {
        for c in s.chars() {
            self.press(key(KeyCode::Char(c)));
        }
    }

    fn open(&mut self, path: &Path) {
        self.tui.app_mut().open(path).unwrap();
        self.draw();
    }

    fn row(&self, y: u16) -> String {
        let buf = self.term.backend().buffer();
        (0..WIDTH).map(|x| buf[(x, y)].symbol()).collect()
    }

    /// The first line of the status area, as a screen reader's Braille
    /// display shows it with the cursor parked there. Drawn once more a
    /// moment later, since a repeated message is blank at first
    /// ([`textweaver_tui::ui::REPEAT_BLANK`]).
    fn status_row(&mut self) -> String {
        let later = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let tui = &mut self.tui;
        self.term.draw(|f| tui.draw_at(f, later)).unwrap();
        let size = self.term.backend().buffer().area;
        self.row(self.tui.areas(size).status.y)
    }

    fn bottom_row(&self) -> String {
        self.row(HEIGHT - 1)
    }

    /// The first line of the list, just under the title line.
    fn list_title_row(&self) -> String {
        self.row(1)
    }

    /// Asserts that `line` holds `key` and that it ends within the first
    /// 40 cells, with no border or padding before the meaning.
    fn forty(&mut self, what: &str, line: &str, key: &str) {
        self.checked.push(format!("{what}: {}", line.trim_end()));
        let Some(at) = line.find(key) else {
            panic!("{what}: {key:?} is not in {line:?}");
        };
        let end = Span::raw(&line[..at + key.len()]).width();
        assert!(
            end <= BRAILLE_CELLS,
            "{what}: {key:?} ends at cell {end}, past {BRAILLE_CELLS}: {line:?}"
        );
        let first = line.chars().next().unwrap_or(' ');
        assert!(
            !first.is_whitespace() && !('\u{2500}'..='\u{257f}').contains(&first),
            "{what}: the line starts with {first:?}, not its meaning: {line:?}"
        );
    }
}

/// A plain document: opening, the title line, the position report, the
/// outline, list moves and filtering, prompts, and a question.
#[test]
fn a_document_reads_meaning_first_in_forty_cells() {
    let mut h = launch();
    h.open(&fixture(&["x", "essay.md"]));

    // Opening: "Opened" and the title.
    let s = h.status_row();
    h.forty(
        "status after opening",
        &s,
        "Opened Reading for the Braille pass",
    );
    // The title line: the position, then the reading state.
    let t = h.row(0);
    h.forty("title line", &t, "Line 1 of ");
    h.forty("title line", &t, ", Ready");

    // Two-digit lines: still first.
    h.tui.dispatch(Command::GoTo(GoTo::Line(12)));
    h.draw();
    let t = h.row(0);
    h.forty("title line at line 12", &t, "Line 12 of ");
    h.act(ActionId::SayPosition);
    let s = h.status_row();
    h.forty("say position", &s, "Line 12 of ");

    // The outline: the list's first line is its place and title.
    h.act(ActionId::Outline);
    let l = h.list_title_row();
    h.forty("outline title line", &l, " of 6, Outline, 6 headings");
    let s = h.status_row();
    h.forty("outline introduction", &s, "Outline, 6 headings.");
    // Moving says the place first, then the heading.
    h.tui
        .dispatch(Command::ListKey(textweaver_app::ListKey::Home));
    h.press(key(KeyCode::Down));
    let s = h.status_row();
    h.forty("outline item", &s, "2 of 6, Chapter one, level 2");
    for _ in 0..3 {
        h.press(key(KeyCode::Down));
    }
    let s = h.status_row();
    h.forty("long outline item", &s, "5 of 6, A heading with a much");
    let l = h.list_title_row();
    h.forty("outline title line after moving", &l, "5 of 6, Outline");
    // The focused item's own line starts at the edge, with no border.
    let size = h.term.backend().buffer().area;
    let body = h.tui.areas(size).body;
    let focused = (body.y..body.y + body.height)
        .map(|y| h.row(y))
        .find(|r| r.starts_with("A heading with"))
        .expect("the focused item is drawn at the left edge");
    h.forty("focused list line", &focused, "A heading with");
    // Filtering: the count first.
    h.typed("two");
    let s = h.status_row();
    h.forty("outline filter", &s, "1 heading match.");
    let l = h.list_title_row();
    h.forty(
        "filtered outline title line",
        &l,
        "1 of 1, Outline, 1 of 6 match two",
    );
    h.press(key(KeyCode::Esc));

    // Keyboard help, a long list.
    h.act(ActionId::KeyboardHelp);
    let l = h.list_title_row();
    h.forty("keyboard help title line", &l, "Keyboard shortcuts");
    h.press(key(KeyCode::Down));
    let s = h.status_row();
    h.forty("keyboard help item", &s, "2 of ");
    h.press(key(KeyCode::Esc));

    // Prompts: the label, then what is typed.
    h.act(ActionId::Find);
    h.typed("chapter");
    let b = h.bottom_row();
    h.forty("find prompt", &b, "Find: chapter");
    h.press(key(KeyCode::Esc));
    h.act(ActionId::GoTo);
    h.typed("12");
    let b = h.bottom_row();
    h.forty("go-to prompt", &b, "Go to line: 12");
    h.press(key(KeyCode::Esc));
    h.act(ActionId::ExportSettings);
    h.typed("mine.json");
    let b = h.bottom_row();
    h.forty("long prompt", &b, "Export settings to file: mine.json");
    // The whole label is on the status line, said when it opened.
    let s = h.status_row();
    h.forty("long prompt's label", &s, "Export settings to file");
    h.press(key(KeyCode::Esc));

    assert!(h.checked.len() >= 18, "{:#?}", h.checked);
}

/// A web link asks first: the question before the address.
#[test]
fn a_question_comes_before_what_it_is_about() {
    let mut h = launch();
    let text =
        "See [the site](https://example.org/a/rather/long/address/for/a/braille/line) now.\n";
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("links.md");
    std::fs::write(&path, text).unwrap();
    h.open(&path);
    let at = h
        .tui
        .app()
        .session()
        .unwrap()
        .doc
        .text()
        .to_string()
        .find("the site")
        .unwrap();
    h.tui.dispatch(Command::GoTo(GoTo::Char(CharPos(at))));
    h.act(ActionId::FollowLink);
    let s = h.status_row();
    h.forty("open link question", &s, "Open web link? y or n.");
    h.press(key(KeyCode::Char('n')));
}

/// An export offers to open the file: the question before the folder.
#[cfg(feature = "publish")]
#[test]
fn an_export_asks_before_naming_the_folder() {
    let mut h = launch();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("essay.md");
    std::fs::copy(fixture(&["x", "essay.md"]), &path).unwrap();
    h.open(&path);
    h.act(ActionId::ExportHtml);
    // "Theme for the HTML page?": Enter keeps the reading theme.
    h.press(key(KeyCode::Enter));
    // The Export as prompt; Enter takes the name offered (essay.html).
    h.draw();
    let b = h.bottom_row();
    h.forty("export as prompt", &b, "Export as:");
    h.press(key(KeyCode::Enter));
    assert!(
        h.tui
            .app_mut()
            .wait_for_background(std::time::Duration::from_secs(30))
    );
    h.draw();
    let s = h.status_row();
    h.forty(
        "export question",
        &s,
        "Exported essay.html. Open it? y or n.",
    );
    h.press(key(KeyCode::Char('n')));
}

/// A PDF with printed page labels i, 1, 2: the title line, go to page,
/// and the position report name the page first.
#[test]
fn a_pdf_names_its_page_first() {
    let mut h = launch();
    h.open(&fixture(&["a", "running.pdf"]));
    let t = h.row(0);
    h.forty("PDF title line", &t, "Page i, 1 of 3, 0%");

    // A bare number is a page in a PDF: the page printed "2", the third.
    h.act(ActionId::GoTo);
    let b = h.bottom_row();
    h.forty("PDF go-to prompt", &b, "Go to page: ");
    h.typed("2");
    h.press(key(KeyCode::Enter));
    let s = h.status_row();
    h.forty("go to page", &s, "Page 2, line ");
    let t = h.row(0);
    h.forty("PDF title line on page 2", &t, "Page 2, 3 of 3, ");
    h.act(ActionId::SayPosition);
    let s = h.status_row();
    h.forty("PDF position report", &s, "Page 2, 3 of 3. Line ");

    // "p i" goes back to the first page; "line 1" is still a line.
    h.act(ActionId::GoTo);
    h.typed("p i");
    h.press(key(KeyCode::Enter));
    let t = h.row(0);
    h.forty("PDF title line on page i", &t, "Page i, 1 of 3, ");
    h.act(ActionId::GoTo);
    h.typed("page 9");
    h.press(key(KeyCode::Enter));
    let s = h.status_row();
    h.forty("no such page", &s, "No page 9.");
}

/// A paged document without headings: the outline lists its pages.
#[test]
fn the_outline_lists_pages_when_there_are_no_headings() {
    let mut h = launch();
    let text = "Preface words here.\nFirst page text.\nSecond page text.\n";
    let doc = Document::new(
        DocumentMeta::default(),
        text.into(),
        vec![
            Marker::new(
                MarkerKind::PageBreak,
                CharRange::new(CharPos(0), CharPos(20)),
            )
            .with_label("ii"),
            Marker::new(
                MarkerKind::PageBreak,
                CharRange::new(CharPos(20), CharPos(37)),
            )
            .with_label("1"),
            Marker::new(
                MarkerKind::PageBreak,
                CharRange::new(CharPos(37), CharPos(55)),
            ),
        ],
    );
    h.tui
        .app_mut()
        .open_document(doc, DocKey::untitled(1), "Scan".into());
    h.draw();
    h.act(ActionId::Outline);
    let list = h.tui.list().expect("the page list").clone();
    assert_eq!(
        list.items,
        [
            "Page ii: Preface words here.",
            "Page 1: First page text.",
            "Page 3: Second page text."
        ]
    );
    let l = h.list_title_row();
    h.forty("page list title line", &l, "1 of 3, Pages, 3 pages");
    h.press(key(KeyCode::Down));
    let s = h.status_row();
    h.forty("page list item", &s, "2 of 3, Page 1: First page text.");
    h.press(key(KeyCode::Enter));
    let s = h.status_row();
    h.forty("page chosen", &s, "Page 1, line 2");
    assert_eq!(h.tui.app().session().unwrap().cursor, CharPos(20));
}
