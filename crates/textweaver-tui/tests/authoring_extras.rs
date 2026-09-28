//! Terminal parts of the authoring extras (Agent W4g): Unicode math in the
//! reading view.

use std::path::Path;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use textweaver_app::store::{MathDisplay, Settings};
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig};
use textweaver_tui::Tui;

const WIDTH: u16 = 70;
const HEIGHT: u16 = 30;

struct Harness {
    tui: Tui,
    term: Terminal<TestBackend>,
}

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/g")
        .join(name)
}

fn open(file: &Path, settings: Settings) -> Harness {
    let mut app = App::new(AppConfig {
        settings,
        ..AppConfig::for_tests()
    });
    app.open(file).unwrap();
    let mut h = Harness {
        tui: Tui::with_color_support(app, ColorSupport::NoColor),
        term: Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap(),
    };
    h.draw();
    h
}

impl Harness {
    fn draw(&mut self) {
        let tui = &mut self.tui;
        self.term.draw(|f| tui.draw(f)).unwrap();
    }
    fn screen(&self) -> String {
        let buf = self.term.backend().buffer();
        (0..HEIGHT)
            .map(|y| (0..WIDTH).map(|x| buf[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[test]
fn math_is_drawn_as_unicode_only_when_asked() {
    let file = fixture("math.md");
    let h = open(&file, Settings::default());
    let screen = h.screen();
    assert!(screen.contains("\\sqrt{2}"), "{screen}");
    assert!(!screen.contains('\u{221a}'), "{screen}");

    let mut settings = Settings::default();
    settings.reading.math_display = MathDisplay::Unicode;
    let h = open(&file, settings);
    let screen = h.screen();
    for shown in [
        "\u{3c0}r\u{b2}",
        "\u{221a}2",
        "\u{221b}8",
        "1\u{2044}2",
        "aᵢ",
        "ℝ",
    ] {
        assert!(screen.contains(shown), "{shown} not in\n{screen}");
    }
    assert!(!screen.contains("\\sqrt{2}"), "{screen}");
    // The text around the math keeps its place on the line.
    assert!(
        screen.contains("The square root of two is \u{221a}2, and"),
        "{screen}"
    );
}

/// Code blocks: keywords bold and comments italic, so the kinds differ by
/// more than color; the text itself is unchanged.
#[cfg(feature = "highlight")]
#[test]
fn code_blocks_are_highlighted_with_attributes() {
    use ratatui::style::Modifier;
    let file = fixture("code.md");
    let h = open(&file, Settings::default());
    let screen = h.screen();
    let buf = h.term.backend().buffer();
    let find = |needle: &str| -> (u16, u16) {
        for (y, line) in screen.lines().enumerate() {
            if let Some(x) = line.find(needle) {
                let x = line[..x].chars().count();
                return (x as u16, y as u16);
            }
        }
        panic!("{needle} not in\n{screen}");
    };
    let (x, y) = find("def add");
    assert!(buf[(x, y)].modifier.contains(Modifier::BOLD), "def is bold");
    let (x, y) = find("# Add two");
    assert!(
        buf[(x, y)].modifier.contains(Modifier::ITALIC),
        "the comment is italic"
    );
    let (x, y) = find("fn add");
    assert!(buf[(x, y)].modifier.contains(Modifier::BOLD), "fn is bold");
    // Prose is not touched.
    let (x, y) = find("Move the caret");
    assert!(!buf[(x, y)].modifier.contains(Modifier::BOLD));
}

#[test]
fn edit_mode_shows_the_source() {
    let file = fixture("math.md");
    let mut settings = Settings::default();
    settings.reading.math_display = MathDisplay::Unicode;
    let mut h = open(&file, settings);
    h.tui.app_mut().dispatch(textweaver_app::Command::Action(
        textweaver_app::keymap::ActionId::ToggleEditMode,
    ));
    h.draw();
    let screen = h.screen();
    assert!(screen.contains("\\sqrt{2}"), "{screen}");
}
