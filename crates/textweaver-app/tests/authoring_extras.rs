//! Authoring extras (Agent W4g), driven through `App::dispatch` without a
//! terminal: Markdown lint in edit mode.

use std::path::PathBuf;

use textweaver_app::keymap::ActionId;
use textweaver_app::{App, AppConfig, Command, Effect};

/// A silent app and a folder for the document.
struct Rig {
    app: App,
    dir: PathBuf,
    _tmp: tempfile::TempDir,
}

impl Rig {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        Rig {
            app: App::new(AppConfig::for_tests()),
            dir: tmp.path().to_owned(),
            _tmp: tmp,
        }
    }

    fn open(&mut self, name: &str, text: &str) {
        let path = self.dir.join(name);
        std::fs::write(&path, text).unwrap();
        self.app.open(&path).unwrap();
    }

    fn act(&mut self, a: ActionId) -> Vec<Effect> {
        let effects = self.app.dispatch(Command::Action(a));
        self.app.wait_for_writes();
        effects
    }

    fn status(&self) -> String {
        self.app.status_text().to_owned()
    }

    /// The text the editor has selected.
    fn selected(&self) -> String {
        let sel = self
            .app
            .edit_session()
            .unwrap()
            .editor()
            .unwrap()
            .selection();
        let s = self.app.session().unwrap();
        s.doc.slice(sel.range())
    }
}

#[cfg(feature = "lint")]
#[test]
fn lint_problems_are_found_selected_and_said_in_edit_mode() {
    let mut r = Rig::new();
    r.open(
        "lint.md",
        "# Title\n\n### Too deep\n\nSee https://example.org today. \n\n- one\n* two\n",
    );
    r.act(ActionId::NextLintProblem);
    assert!(
        r.status()
            .starts_with("Lint checks the Markdown you write."),
        "{}",
        r.status()
    );
    r.act(ActionId::ToggleEditMode);
    r.act(ActionId::DocumentStart);
    r.act(ActionId::NextLintProblem);
    assert_eq!(
        r.status(),
        "Lint: heading level 3 after level 1; use level 2."
    );
    assert_eq!(r.selected(), "### Too deep");
    r.act(ActionId::NextLintProblem);
    assert_eq!(
        r.status(),
        "Lint: bare web address; put it in angle brackets or make it a link with a name."
    );
    assert_eq!(r.selected(), "https://example.org");
    r.act(ActionId::NextLintProblem);
    assert_eq!(r.status(), "Lint: 1 space at the end of the line.");
    r.act(ActionId::NextLintProblem);
    assert_eq!(r.status(), "Lint: list marker star; this list uses dash.");
    r.act(ActionId::NextLintProblem);
    assert_eq!(r.status(), "No more lint problem. 4 lint problems in all.");
    r.act(ActionId::PreviousLintProblem);
    assert_eq!(r.status(), "Lint: 1 space at the end of the line.");
}

#[cfg(feature = "lint")]
#[test]
fn lint_says_so_for_plain_text() {
    let mut r = Rig::new();
    r.open("plain.txt", "Just text.  \n");
    r.act(ActionId::ToggleEditMode);
    r.act(ActionId::NextLintProblem);
    assert_eq!(
        r.status(),
        "Lint checks Markdown, and this document is not Markdown."
    );
}
